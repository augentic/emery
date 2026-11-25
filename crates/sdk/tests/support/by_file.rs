//! Host [`ModelClient`] that routes each completion to a per-seam script.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt as _;
use omnia_test::host::ScriptedModel;
use omnia_wasi_model::{Answer, Error as ModelError, FutureResult, ModelClient, Request, ToolHost};

/// A model failure the host surfaces to the guest.
#[derive(Clone, Debug)]
pub enum FailKind {
    /// An `invalid` completion refusal.
    Invalid(String),
    /// A backend failure retried once by [`emery_sdk::extract`].
    Backend(String),
}

/// A single [`ModelClient`] script for one-off seam tests.
#[derive(Clone, Debug, Default)]
pub struct ScriptFailures {
    steps: Arc<Mutex<Vec<Result<String, FailKind>>>>,
    seen: Arc<AtomicUsize>,
}

impl ScriptFailures {
    /// A FIFO script of answers and failures.
    #[must_use]
    pub fn new(steps: impl IntoIterator<Item = Result<String, FailKind>>) -> Self {
        Self {
            steps: Arc::new(Mutex::new(steps.into_iter().collect())),
            seen: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// How many completions this script served.
    #[must_use]
    pub fn seen(&self) -> usize {
        self.seen.load(Ordering::SeqCst)
    }

    /// Asserts the script is fully consumed.
    #[track_caller]
    pub fn assert_exhausted(&self) {
        assert!(
            self.steps.lock().expect("steps").is_empty(),
            "script still holds unconsumed steps"
        );
    }
}

impl ModelClient for ScriptFailures {
    fn complete(&self, _request: Request, _tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let steps = self.steps.clone();
        let seen = self.seen.clone();
        async move {
            seen.fetch_add(1, Ordering::SeqCst);
            let step = steps
                .lock()
                .expect("steps")
                .drain(..1)
                .next()
                .ok_or_else(|| anyhow::anyhow!("model script exhausted"))?;
            match step {
                Ok(answer) => Ok(Answer::from(answer)),
                Err(FailKind::Invalid(detail)) => Err(ModelError::Invalid(detail).into()),
                Err(FailKind::Backend(detail)) => Err(ModelError::Unavailable(detail).into()),
            }
        }
        .boxed()
    }
}

/// One completion script for a single seam file.
#[derive(Clone, Debug)]
pub struct FileScript {
    inner: ScriptedModel,
    fails: Arc<Mutex<Vec<FailKind>>>,
    attempts: Arc<AtomicUsize>,
}

impl FileScript {
    fn answering<S: Into<String>>(answers: impl IntoIterator<Item = S>) -> Self {
        Self {
            inner: ScriptedModel::answering(answers),
            fails: Arc::new(Mutex::new(Vec::new())),
            attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn failing(steps: Vec<Result<String, FailKind>>) -> Self {
        let mut answers = Vec::new();
        let mut fails = Vec::new();
        for step in steps {
            match step {
                Ok(answer) => answers.push(answer),
                Err(kind) => fails.push(kind),
            }
        }
        Self {
            inner: ScriptedModel::answering(answers),
            fails: Arc::new(Mutex::new(fails)),
            attempts: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// Routes completions to the [`ScriptedModel`] keyed by the seam file named in
/// the turn, recording concurrency for the fan-out tests.
#[derive(Clone, Default)]
pub struct ByFile {
    scripts: BTreeMap<String, FileScript>,
    arrivals: Arc<Mutex<Vec<String>>>,
    pending: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    hold: Option<Hold>,
}

impl fmt::Debug for ByFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByFile")
            .field("files", &self.scripts.keys().collect::<Vec<_>>())
            .field("hold", &self.hold.as_ref().map(|hold| (&hold.held, &hold.until)))
            .finish_non_exhaustive()
    }
}

impl ByFile {
    /// Registers one seam file's script.
    #[must_use]
    pub fn file(mut self, file: &str, script: FileScript) -> Self {
        self.scripts.insert(file.to_owned(), script);
        self
    }

    /// Withholds `held`'s answer until `until`'s turn arrives.
    #[must_use]
    pub fn holding(mut self, held: &str, until: &str) -> Self {
        self.hold = Some(Hold {
            held: held.to_owned(),
            until: until.to_owned(),
            gate: Arc::new(tokio::sync::Notify::new()),
        });
        self
    }

    /// Arrival order recorded across completions.
    #[must_use]
    pub fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("arrivals").clone()
    }

    /// Peak in-flight completions.
    #[must_use]
    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    /// How many turns one seam file spent.
    #[must_use]
    pub fn turns(&self, file: &str) -> usize {
        self.scripts.get(file).map_or(0, |script| script.attempts.load(Ordering::SeqCst))
    }

    /// Asserts every per-file script was consumed.
    #[track_caller]
    pub fn assert_exhausted(&self) {
        for script in self.scripts.values() {
            script.inner.assert_exhausted();
        }
    }

    /// Accesses one file's underlying [`ScriptedModel`].
    #[must_use]
    pub fn model(&self, file: &str) -> &ScriptedModel {
        &self.scripts[file].inner
    }

    fn match_file(&self, request: &Request) -> (String, FileScript) {
        let turn = request.messages.first().map(|message| message.content.as_str());
        let turn = turn.unwrap_or_default();
        self.scripts.iter().find(|(file, _)| turn.contains(&format!("`{file}`"))).map_or_else(
            || panic!("no script for the turn:\n{turn}"),
            |(file, script)| (file.clone(), script.clone()),
        )
    }
}

#[derive(Clone)]
struct Hold {
    held: String,
    until: String,
    gate: Arc<tokio::sync::Notify>,
}

impl ModelClient for ByFile {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let (file, script) = self.match_file(&request);
        let arrivals = self.arrivals.clone();
        let pending = self.pending.clone();
        let peak = self.peak.clone();
        let hold = self.hold.clone();
        let inner = script.inner.clone();
        let fails = script.fails.clone();
        let attempts = Arc::clone(&script.attempts);

        async move {
            attempts.fetch_add(1, Ordering::SeqCst);
            arrivals.lock().expect("arrivals").push(file.clone());
            let live = pending.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(live, Ordering::SeqCst);

            tokio::task::yield_now().await;
            tokio::task::yield_now().await;
            if let Some(hold) = &hold {
                if file == hold.until {
                    hold.gate.notify_one();
                }
                if file == hold.held {
                    hold.gate.notified().await;
                }
            }

            let fail = {
                let mut pending_fails = fails.lock().expect("fails");
                if pending_fails.is_empty() { None } else { Some(pending_fails.remove(0)) }
            };
            let outcome = if let Some(fail) = fail {
                match fail {
                    FailKind::Invalid(detail) => Err(ModelError::Invalid(detail).into()),
                    FailKind::Backend(detail) => Err(ModelError::Unavailable(detail).into()),
                }
            } else {
                inner.complete(request, tool_host).await
            };
            pending.fetch_sub(1, Ordering::SeqCst);
            outcome
        }
        .boxed()
    }
}

/// One seam-file script of plain answers.
pub fn file_answers<S: Into<String>>(answers: impl IntoIterator<Item = S>) -> FileScript {
    FileScript::answering(answers)
}

/// One seam-file script mixing failures and a final answer.
pub fn file_retries(steps: impl IntoIterator<Item = Result<String, FailKind>>) -> FileScript {
    FileScript::failing(steps.into_iter().collect())
}

/// One invalid completion for a seam file.
#[must_use]
pub fn file_invalid(detail: impl Into<String>) -> FileScript {
    file_retries([Err(FailKind::Invalid(detail.into()))])
}

/// One backend failure for a seam file.
#[must_use]
pub fn file_backend(detail: impl Into<String>) -> FileScript {
    file_retries([Err(FailKind::Backend(detail.into()))])
}

/// A single-seam [`ScriptedModel`] for scenarios that do not key by file.
#[must_use]
pub fn single_model(answers: impl IntoIterator<Item = impl Into<String>>) -> ScriptedModel {
    ScriptedModel::answering(answers)
}
