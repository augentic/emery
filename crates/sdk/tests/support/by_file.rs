//! Host `ModelClient` doubles routing completions to per-seam scripts.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt as _;
use omnia_test::host::ScriptedModel;
use omnia_wasi_model::{Answer, Error as ModelError, FutureResult, ModelClient, Request, ToolHost};

#[derive(Clone, Debug)]
pub enum FailKind {
    Invalid(String),
    Backend(String),
}

impl FailKind {
    fn into_error(self) -> anyhow::Error {
        match self {
            Self::Invalid(detail) => ModelError::Invalid(detail).into(),
            Self::Backend(detail) => ModelError::Unavailable(detail).into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScriptFailures {
    steps: Arc<Mutex<VecDeque<Result<String, FailKind>>>>,
    seen: Arc<AtomicUsize>,
}

impl ScriptFailures {
    #[must_use]
    pub fn new(steps: impl IntoIterator<Item = Result<String, FailKind>>) -> Self {
        Self {
            steps: Arc::new(Mutex::new(steps.into_iter().collect())),
            seen: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[must_use]
    pub fn seen(&self) -> usize {
        self.seen.load(Ordering::SeqCst)
    }

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
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("model script exhausted"))?;
            step.map(Answer::from).map_err(FailKind::into_error)
        }
        .boxed()
    }
}

#[derive(Clone, Debug)]
pub struct FileScript {
    inner: ScriptedModel,
    fails: Arc<Mutex<VecDeque<FailKind>>>,
    attempts: Arc<AtomicUsize>,
}

impl FileScript {
    fn answering<S: Into<String>>(answers: impl IntoIterator<Item = S>) -> Self {
        Self {
            inner: ScriptedModel::answering(answers),
            fails: Arc::new(Mutex::new(VecDeque::new())),
            attempts: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn retrying(steps: impl IntoIterator<Item = Result<String, FailKind>>) -> Self {
        let mut answers = Vec::new();
        let mut fails = VecDeque::new();
        for step in steps {
            match step {
                Ok(answer) => answers.push(answer),
                Err(kind) => fails.push_back(kind),
            }
        }
        Self {
            inner: ScriptedModel::answering(answers),
            fails: Arc::new(Mutex::new(fails)),
            attempts: Arc::new(AtomicUsize::new(0)),
        }
    }
}

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
    #[must_use]
    pub fn file(mut self, file: &str, script: FileScript) -> Self {
        self.scripts.insert(file.to_owned(), script);
        self
    }

    #[must_use]
    pub fn holding(mut self, held: &str, until: &str) -> Self {
        self.hold = Some(Hold {
            held: held.to_owned(),
            until: until.to_owned(),
            gate: Arc::new(tokio::sync::Notify::new()),
        });
        self
    }

    #[must_use]
    pub fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("arrivals").clone()
    }

    #[must_use]
    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn turns(&self, file: &str) -> usize {
        self.scripts.get(file).map_or(0, |script| script.attempts.load(Ordering::SeqCst))
    }

    #[track_caller]
    pub fn assert_exhausted(&self) {
        for script in self.scripts.values() {
            script.inner.assert_exhausted();
        }
    }

    #[must_use]
    pub fn model(&self, file: &str) -> &ScriptedModel {
        &self.scripts[file].inner
    }

    fn match_file(&self, request: &Request) -> (String, FileScript) {
        let turn = request.messages.first().map_or("", |message| message.content.as_str());
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

            let fail = fails.lock().expect("fails").pop_front();
            let outcome = match fail {
                Some(fail) => Err(fail.into_error()),
                None => inner.complete(request, tool_host).await,
            };
            pending.fetch_sub(1, Ordering::SeqCst);
            outcome
        }
        .boxed()
    }
}

pub fn file_answers<S: Into<String>>(answers: impl IntoIterator<Item = S>) -> FileScript {
    FileScript::answering(answers)
}

pub fn file_retries(steps: impl IntoIterator<Item = Result<String, FailKind>>) -> FileScript {
    FileScript::retrying(steps)
}

#[must_use]
pub fn file_invalid(detail: impl Into<String>) -> FileScript {
    file_retries([Err(FailKind::Invalid(detail.into()))])
}
