//! Scripts the `Vcs` capability and records every call the engine makes.
//!
//! Answers are queued per location and consumed in order; an unqueued call
//! answers a default that lets a journey through, so a scenario scripts the
//! exchanges it bends and asserts the whole sequence it consumed. The one
//! operation the engine never makes, `init`, fails the scenario.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};

use omnia_sdk::Vcs;
use omnia_sdk::vcs::{Change, CloneOptions, Entry, Error, Merged, Rule};

/// The commit every unscripted `head` answers.
pub const HEAD: &str = "9f8e7d6c5b4a39281706f5e4d3c2b1a0f9e8d7c6";

type Queued<T> = BTreeMap<String, VecDeque<Result<T, Error>>>;

/// One merge as the script saw it: the message whole and the policy it rode.
pub type Merge = (String, Vec<Rule>);

/// Answers queued per location, consumed in call order.
#[derive(Clone, Debug, Default)]
pub struct Queue<T>(Arc<Mutex<Queued<T>>>);

impl<T> Queue<T> {
    /// Queues `answer` for the next call at `location`.
    pub fn script(&self, location: &str, answer: Result<T, Error>) {
        self.0.lock().expect("queue").entry(location.to_owned()).or_default().push_back(answer);
    }

    fn take(&self, location: &str) -> Option<Result<T, Error>> {
        self.0.lock().expect("queue").get_mut(location).and_then(VecDeque::pop_front)
    }

    fn drained(&self) -> Vec<String> {
        let queues = self.0.lock().expect("queue");
        queues.iter().filter(|(_, left)| !left.is_empty()).map(|(at, _)| at.clone()).collect()
    }
}

/// A scripted `Vcs` with a record of every call.
///
/// Unscripted answers:
///
/// - `pending` holds nothing;
/// - `head` is the revision `add` cut the working copy at, until a commit or
///   a merge lands there; [`HEAD`] otherwise;
/// - `resolve` is `<revision>-commit`;
/// - `labelled` is `NotFound`: a fresh build;
/// - `fetched` is `NotFound`: nothing fetched from the remote yet;
/// - `descends` holds;
/// - `log` holds nothing;
/// - `commit` seals `<first word of the message>-commit`;
/// - `merge` merges as `m:<first word of the message>`;
/// - every other operation succeeds.
#[derive(Clone, Debug, Default)]
pub struct VcsScript {
    /// `pending` answers by working copy.
    pub pending: Queue<Vec<Change>>,
    /// `head` answers by working copy.
    pub heads: Queue<String>,
    /// `resolve` answers by repository.
    pub resolves: Queue<String>,
    /// `labelled` answers by repository.
    pub labelleds: Queue<String>,
    /// `fetched` answers by repository.
    pub fetcheds: Queue<String>,
    /// `descends` answers by repository.
    pub descends: Queue<bool>,
    /// `log` answers by repository.
    pub logs: Queue<Vec<Entry>>,
    /// `commit` answers by working copy.
    pub commits: Queue<Option<String>>,
    /// `merge` answers by working copy.
    pub merges: Queue<Merged>,
    /// `fetch` answers by repository.
    pub fetches: Queue<()>,
    /// `clone` answers by clone location.
    pub clones: Queue<()>,
    /// `add` answers by working copy.
    pub adds: Queue<()>,
    /// `remove` answers by working copy.
    pub removes: Queue<()>,
    /// `label` answers by repository.
    pub labels: Queue<()>,
    /// `push` answers by repository.
    pub pushes: Queue<()>,
    /// Every call, in order, spelled `<operation> <arguments>`; a commit or
    /// merge carries its message's first line.
    pub calls: Arc<Mutex<Vec<String>>>,
    /// Every commit message whole, in order.
    pub messages: Arc<Mutex<Vec<String>>>,
    /// Every merge, in order.
    pub merged: Arc<Mutex<Vec<Merge>>>,
    // the revision each working copy was cut at, while nothing has landed there
    cut: Arc<Mutex<BTreeMap<String, String>>>,
}

impl VcsScript {
    /// Returns every call so far, in order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }

    /// Returns every commit message so far, in order.
    pub fn messages(&self) -> Vec<String> {
        self.messages.lock().expect("messages").clone()
    }

    /// Returns every merge so far, in order.
    pub fn merged(&self) -> Vec<Merge> {
        self.merged.lock().expect("merged").clone()
    }

    /// Asserts that every queued answer was consumed.
    pub fn assert_exhausted(&self) {
        let left: Vec<String> = [
            ("pending", self.pending.drained()),
            ("head", self.heads.drained()),
            ("resolve", self.resolves.drained()),
            ("labelled", self.labelleds.drained()),
            ("fetched", self.fetcheds.drained()),
            ("descends", self.descends.drained()),
            ("log", self.logs.drained()),
            ("commit", self.commits.drained()),
            ("merge", self.merges.drained()),
            ("fetch", self.fetches.drained()),
            ("clone", self.clones.drained()),
            ("add", self.adds.drained()),
            ("remove", self.removes.drained()),
            ("label", self.labels.drained()),
            ("push", self.pushes.drained()),
        ]
        .into_iter()
        .flat_map(|(op, ats)| ats.into_iter().map(move |at| format!("{op} {at}")))
        .collect();
        assert!(left.is_empty(), "scripted answers never consumed: {left:?}");
    }

    fn record(&self, call: String) {
        self.calls.lock().expect("calls").push(call);
    }

    fn forget(&self, at: &str) {
        self.cut.lock().expect("cut").remove(at);
    }
}

impl Vcs for VcsScript {
    fn resolve(
        &self, repo: &str, revision: &str,
    ) -> impl Future<Output = Result<String, Error>> + Send {
        self.record(format!("resolve {repo} {revision}"));
        let answer = self.resolves.take(repo).unwrap_or_else(|| Ok(format!("{revision}-commit")));
        async move { answer }
    }

    fn descends(
        &self, repo: &str, ancestor: &str, descendant: &str,
    ) -> impl Future<Output = Result<bool, Error>> + Send {
        self.record(format!("descends {repo} {ancestor} {descendant}"));
        let answer = self.descends.take(repo).unwrap_or(Ok(true));
        async move { answer }
    }

    fn head(&self, at: &str) -> impl Future<Output = Result<String, Error>> + Send {
        self.record(format!("head {at}"));
        let answer = self.heads.take(at).unwrap_or_else(|| {
            let cut = self.cut.lock().expect("cut").get(at).cloned();
            Ok(cut.unwrap_or_else(|| HEAD.to_owned()))
        });
        async move { answer }
    }

    fn commit(
        &self, at: &str, message: &str,
    ) -> impl Future<Output = Result<Option<String>, Error>> + Send {
        let first = message.lines().next().unwrap_or_default();
        self.record(format!("commit {at} {first}"));
        self.messages.lock().expect("messages").push(message.to_owned());
        let answer = self.commits.take(at).unwrap_or_else(|| {
            let word = first.split_whitespace().next().unwrap_or_default();
            Ok(Some(format!("{word}-commit")))
        });
        if matches!(answer, Ok(Some(_))) {
            self.forget(at);
        }
        async move { answer }
    }

    fn merge(
        &self, at: &str, revision: &str, message: &str, policy: &[Rule],
    ) -> impl Future<Output = Result<Merged, Error>> + Send {
        let first = message.lines().next().unwrap_or_default();
        self.record(format!("merge {at} {revision} {first}"));
        self.merged.lock().expect("merged").push((message.to_owned(), policy.to_vec()));
        let answer = self.merges.take(at).unwrap_or_else(|| {
            let word = first.split_whitespace().next().unwrap_or_default();
            Ok(Merged {
                commit: Some(format!("m:{word}")),
                conflicts: Vec::new(),
            })
        });
        if matches!(&answer, Ok(merged) if merged.commit.is_some()) {
            self.forget(at);
        }
        async move { answer }
    }

    fn log(
        &self, repo: &str, revision: &str, base: &str,
    ) -> impl Future<Output = Result<Vec<Entry>, Error>> + Send {
        self.record(format!("log {repo} {revision} {base}"));
        let answer = self.logs.take(repo).unwrap_or_else(|| Ok(Vec::new()));
        async move { answer }
    }

    fn init(&self, at: &str) -> impl Future<Output = Result<(), Error>> + Send {
        let call = format!("initialised `{at}`");
        async move { panic!("the engine never inits, yet {call}") }
    }

    fn add(
        &self, repo: &str, at: &str, revision: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.record(format!("add {repo} {at} {revision}"));
        let answer = self.adds.take(at).unwrap_or(Ok(()));
        if answer.is_ok() {
            self.cut.lock().expect("cut").insert(at.to_owned(), revision.to_owned());
        }
        async move { answer }
    }

    fn remove(&self, at: &str) -> impl Future<Output = Result<(), Error>> + Send {
        self.record(format!("remove {at}"));
        let answer = self.removes.take(at).unwrap_or(Ok(()));
        self.forget(at);
        async move { answer }
    }

    fn pending(&self, at: &str) -> impl Future<Output = Result<Vec<Change>, Error>> + Send {
        self.record(format!("pending {at}"));
        let answer = self.pending.take(at).unwrap_or_else(|| Ok(Vec::new()));
        async move { answer }
    }

    fn clone_repo(
        &self, url: &str, at: &str, options: CloneOptions,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        assert_eq!(options.depth, None, "a run clones whole: every label must resolve");
        self.record(format!("clone {url} {at}"));
        let answer = self.clones.take(at).unwrap_or(Ok(()));
        async move { answer }
    }

    fn fetch(&self, repo: &str, remote: &str) -> impl Future<Output = Result<(), Error>> + Send {
        self.record(format!("fetch {repo} {remote}"));
        let answer = self.fetches.take(repo).unwrap_or(Ok(()));
        async move { answer }
    }

    fn label(
        &self, repo: &str, name: &str, revision: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.record(format!("label {repo} {name} {revision}"));
        let answer = self.labels.take(repo).unwrap_or(Ok(()));
        async move { answer }
    }

    fn labelled(
        &self, repo: &str, name: &str,
    ) -> impl Future<Output = Result<String, Error>> + Send {
        self.record(format!("labelled {repo} {name}"));
        let answer =
            self.labelleds.take(repo).unwrap_or_else(|| Err(Error::NotFound(name.to_owned())));
        async move { answer }
    }

    fn fetched(
        &self, repo: &str, remote: &str, name: &str,
    ) -> impl Future<Output = Result<String, Error>> + Send {
        self.record(format!("fetched {repo} {remote} {name}"));
        let answer =
            self.fetcheds.take(repo).unwrap_or_else(|| Err(Error::NotFound(name.to_owned())));
        async move { answer }
    }

    fn push(
        &self, repo: &str, remote: &str, label: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.record(format!("push {repo} {remote} {label}"));
        let answer = self.pushes.take(repo).unwrap_or(Ok(()));
        async move { answer }
    }
}
