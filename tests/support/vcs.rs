//! Scripted `omnia:vcs` backend recording version-control operations.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use omnia::futures::FutureExt;
use omnia_wasi_vcs::{
    Change, CloneOptions, Entry, Error, FutureResult, Merged, Place, Rule, VcsClient,
};

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

/// A scripted [`VcsClient`] with a record of every call.
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
    /// `init` answers by repository.
    pub inits: Queue<()>,
    /// `clone` answers by clone location.
    pub clones: Queue<()>,
    /// `add` answers by working copy.
    pub adds: Queue<()>,
    /// `remove` answers by working copy.
    pub removes: Queue<()>,
    /// `label` answers by repository.
    pub labels: Queue<()>,
    /// Every call, in order, spelled `<operation> <arguments>`.
    pub calls: Arc<Mutex<Vec<String>>>,
    /// Every commit message whole, in order.
    pub messages: Arc<Mutex<Vec<String>>>,
    /// Every merge, in order.
    pub merged: Arc<Mutex<Vec<Merge>>>,
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
            ("descends", self.descends.drained()),
            ("log", self.logs.drained()),
            ("commit", self.commits.drained()),
            ("merge", self.merges.drained()),
            ("fetch", self.fetches.drained()),
            ("init", self.inits.drained()),
            ("clone", self.clones.drained()),
            ("add", self.adds.drained()),
            ("remove", self.removes.drained()),
            ("label", self.labels.drained()),
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

impl VcsClient for VcsScript {
    fn resolve(&self, repo: Place, revision: String) -> FutureResult<String> {
        let key = held(&repo);
        self.record(format!("resolve {key} {revision}"));
        let answer = self.resolves.take(&key).unwrap_or_else(|| Ok(format!("{revision}-commit")));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn descends(&self, repo: Place, ancestor: String, descendant: String) -> FutureResult<bool> {
        let key = held(&repo);
        self.record(format!("descends {key} {ancestor} {descendant}"));
        let answer = self.descends.take(&key).unwrap_or(Ok(true));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn head(&self, at: Place) -> FutureResult<String> {
        let key = held(&at);
        self.record(format!("head {key}"));
        let answer = self.heads.take(&key).unwrap_or_else(|| {
            let cut = self.cut.lock().expect("cut").get(&key).cloned();
            Ok(cut.unwrap_or_else(|| HEAD.to_owned()))
        });
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn commit(&self, at: Place, message: String) -> FutureResult<Option<String>> {
        let key = held(&at);
        let first = message.lines().next().unwrap_or_default().to_owned();
        self.record(format!("commit {key} {first}"));
        self.messages.lock().expect("messages").push(message);
        let answer = self.commits.take(&key).unwrap_or_else(|| {
            let word = first.split_whitespace().next().unwrap_or_default();
            Ok(Some(format!("{word}-commit")))
        });
        if matches!(&answer, Ok(Some(_))) {
            self.forget(&key);
        }
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn merge(
        &self, at: Place, revision: String, message: String, policy: Vec<Rule>,
    ) -> FutureResult<Merged> {
        let key = held(&at);
        let first = message.lines().next().unwrap_or_default().to_owned();
        self.record(format!("merge {key} {revision} {first}"));
        self.merged.lock().expect("merged").push((message, policy));
        let answer = self.merges.take(&key).unwrap_or_else(|| {
            let word = first.split_whitespace().next().unwrap_or_default();
            Ok(Merged {
                commit: Some(format!("m:{word}")),
                conflicts: Vec::new(),
            })
        });
        if matches!(&answer, Ok(merged) if merged.commit.is_some()) {
            self.forget(&key);
        }
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn log(&self, repo: Place, revision: String, base: String) -> FutureResult<Vec<Entry>> {
        let key = held(&repo);
        self.record(format!("log {key} {revision} {base}"));
        let answer = self.logs.take(&key).unwrap_or_else(|| Ok(Vec::new()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn init(&self, at: Place) -> FutureResult<()> {
        let key = held(&at);
        self.record(format!("init {key}"));
        let answer = self.inits.take(&key).unwrap_or(Ok(()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn add(&self, repo: Place, at: Place, revision: String) -> FutureResult<()> {
        let repo_key = held(&repo);
        let at_key = held(&at);
        self.record(format!("add {repo_key} {at_key} {revision}"));
        let answer = self.adds.take(&at_key).unwrap_or(Ok(()));
        if answer.is_ok() {
            self.cut.lock().expect("cut").insert(at_key, revision);
        }
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn remove(&self, at: Place) -> FutureResult<()> {
        let key = held(&at);
        self.record(format!("remove {key}"));
        let answer = self.removes.take(&key).unwrap_or(Ok(()));
        self.forget(&key);
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn pending(&self, at: Place) -> FutureResult<Vec<Change>> {
        let key = held(&at);
        self.record(format!("pending {key}"));
        let answer = self.pending.take(&key).unwrap_or_else(|| Ok(Vec::new()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn clone_repo(&self, url: String, at: Place, options: CloneOptions) -> FutureResult<()> {
        assert_eq!(options.depth, None, "a run clones whole: every label must resolve");
        let key = held(&at);
        self.record(format!("clone {url} {key}"));
        let answer = self.clones.take(&key).unwrap_or(Ok(()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn fetch(&self, repo: Place, remote: String) -> FutureResult<()> {
        let key = held(&repo);
        self.record(format!("fetch {key} {remote}"));
        let answer = self.fetches.take(&key).unwrap_or(Ok(()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn label(&self, repo: Place, name: String, revision: String) -> FutureResult<()> {
        let key = held(&repo);
        self.record(format!("label {key} {name} {revision}"));
        let answer = self.labels.take(&key).unwrap_or(Ok(()));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn labelled(&self, repo: Place, name: String) -> FutureResult<String> {
        let key = held(&repo);
        self.record(format!("labelled {key} {name}"));
        let answer = self.labelleds.take(&key).unwrap_or(Err(Error::NotFound(name)));
        async move { answer.map_err(Into::into) }.boxed()
    }

    fn fetched(&self, repo: Place, remote: String, name: String) -> FutureResult<String> {
        let key = held(&repo);
        let call = format!("read `{remote}`'s `{name}` in `{key}`");
        async move { panic!("the engine never reads a remote's label, yet {call}") }.boxed()
    }

    fn push(&self, repo: Place, remote: String, label: String) -> FutureResult<()> {
        let key = held(&repo);
        let call = format!("pushed `{label}` to `{remote}` from `{key}`");
        async move { panic!("the engine never pushes, yet {call}") }.boxed()
    }
}

fn held(place: &Place) -> String {
    place.path().display().to_string()
}
