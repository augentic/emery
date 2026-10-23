//! Verifies `emery apply` and `emery specify --apply` over scripted storage and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::sync::Arc;

use emery_engine::vcs::{REVISION, RevisionMessage};
use emery_engine::{EntryKind, REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
use omnia_sdk::vcs::{Change, ChangeKind, Error};
use serde_json::Value;
use support::{HEAD, Provider, Scratch, carry, cli, cli_ok, document, fail, seed, stage};

// The one-source reference `specify` names; the scripted loader lands any
// exact package reference.
const SOURCE: &str = "acme:source@1.0.0";
// The target every scenario seals through, into the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";
// The target adapter reference the created-repository scenarios name.
const BUILDER: &str = "acme:builder@2.1.0";
const TARGET_REPO: &str = "./.emery/vcs/repos/builder";

const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");

// A staged candidate becomes the current revision: the repository the
// target names is read for the revision it holds, the three documents are
// written under `spec/` in a working copy cut at its head, sealed as one
// commit under a `Revision` message, and labelled; the pointer names the
// revision, the staged slot is cleared and its documents pruned, and the
// output names the sealed revision alone, since a first seal displaces
// nothing.
#[tokio::test]
async fn apply_seals() {
    let provider = Provider::idle();
    let id = stage(&provider.storage, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id} as emery/{id} at Revision\n")
    );
    assert_eq!(current(&provider), id, "the pointer names the sealed revision");
    assert_eq!(
        provider.vcs.calls(),
        [
            "head .".to_owned(),
            format!("add . {REVISION} {HEAD}"),
            format!("remove {REVISION}"),
            format!("add . {REVISION} {HEAD}"),
            format!("pending {REVISION}"),
            format!("commit {REVISION} Revision {id}"),
            format!("label . emery/{id} Revision-commit"),
            format!("remove {REVISION}"),
        ],
        "read, then written, sealed, and labelled in a working copy cut at the head"
    );
    assert_eq!(
        provider.files.writes(),
        [
            (REVISION.to_owned(), document("spec"), SPEC.to_vec()),
            (REVISION.to_owned(), document("design"), DESIGN.to_vec()),
            (REVISION.to_owned(), document("plan"), PLAN.to_vec()),
        ],
        "the three documents, byte for byte"
    );
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(
        message,
        RevisionMessage {
            revision: id.clone(),
            parent: None,
            base: Some(HEAD.to_owned()),
        }
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");

    // the sealed revision is what show and a second apply see
    cli_ok(&provider, &["emery", "show", "plan", "--config", PROJECT_CONFIG]).await;
    fail(&provider, &["emery", "show", "plan", "--staged"], 2, "nothing-staged").await;
    fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 2, "nothing-staged").await;
}

// Sealing over a current revision displaces it: the new commit lands over
// the one holding the outgoing revision and names it as its parent, the
// output carries the diff against it, and the envelope names the parent,
// the commit, the label, and the kind of candidate sealed.
#[tokio::test]
async fn apply_displaces() {
    let provider = Provider::idle();
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    assert_ne!(outgoing, candidate);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], candidate, "{envelope}");
    assert_eq!(envelope["parent"], outgoing, "{envelope}");
    assert_eq!(envelope["commit"], "Revision-commit", "{envelope}");
    assert_eq!(envelope["label"], format!("emery/{candidate}"), "{envelope}");
    assert_eq!(envelope["repository"], ".", "{envelope}");
    assert_eq!(envelope["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
    assert_eq!(
        envelope["diff"]["plan"]["added"],
        serde_json::json!([{"id": "SLICE-003", "name": "orders"}]),
        "the diff is against the revision displaced: {envelope}"
    );
    assert_eq!(current(&provider), candidate);
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(message.parent.as_deref(), Some(outgoing.as_str()), "the parent is the outgoing");
    assert_eq!(message.base.as_deref(), Some(HEAD), "sealed over the commit holding it");
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");

    // the text render carries the same diff on one line
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, PLAN);
    stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {candidate} as emery/{candidate} at Revision\n  diff vs {outgoing}: \
             spec +0 -0 ~0, design +0 -0 ~0, plan +1 -0 ~2 preamble\n"
        )
    );
}

// Sealing a candidate identical to the current revision writes the same
// bytes, so the working copy has nothing pending and no commit is made: the
// label is set at the commit already holding the revision, and the pointer
// keeps the id.
#[tokio::test]
async fn apply_same_revision() {
    let provider = Provider::idle();
    let id = seed(&provider.files, SPEC, DESIGN, PLAN);
    assert_eq!(stage(&provider.storage, SPEC, DESIGN, PLAN), id);
    provider.vcs.commits.script(REVISION, Ok(None));

    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {id} as emery/{id} at 9f8e7d6c\n  diff vs {id}: none (byte-stable)\n"
        )
    );
    assert_eq!(current(&provider), id);
    assert!(
        provider.vcs.calls().contains(&format!("label . emery/{id} {HEAD}")),
        "labelled at the commit already holding it: {:?}",
        provider.vcs.calls()
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    provider.vcs.assert_exhausted();
}

// A target naming no repository seals into one the engine creates under the
// target's name: created now, the revision its root commit, the envelope
// naming the repository. A later seal finds it and lands over its head.
#[tokio::test]
async fn apply_creates_repository() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!("[target]\nadapter = \"{BUILDER}\"\n"));
    let provider = Provider::idle();
    let id = stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(TARGET_REPO, Err(Error::NotARepository));

    let resp = cli_ok(&provider, &["emery", "apply", "--config", &config]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id} as emery/{id} at Revision\n  repository {TARGET_REPO}\n")
    );
    assert_eq!(
        provider.vcs.calls(),
        [
            format!("head {TARGET_REPO}"),
            format!("init {TARGET_REPO}"),
            format!("pending {TARGET_REPO}"),
            format!("commit {TARGET_REPO} Revision {id}"),
            format!("label {TARGET_REPO} emery/{id} Revision-commit"),
        ],
        "created, then the revision sealed as its root commit"
    );
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(message.base, None, "a root commit has no base");
    assert_eq!(
        provider
            .files
            .writes()
            .iter()
            .map(|(at, path, _)| (at.as_str(), path.as_str()))
            .collect::<Vec<_>>(),
        [
            (TARGET_REPO, document("spec").as_str()),
            (TARGET_REPO, document("design").as_str()),
            (TARGET_REPO, document("plan").as_str()),
        ],
        "written into the repository's own checkout"
    );
    assert_eq!(current(&provider), id);

    // the repository is kept: the next seal lands over its head
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    provider.vcs.heads.script(TARGET_REPO, Ok("root-commit".to_owned()));
    let before = provider.vcs.calls().len();
    cli_ok(&provider, &["emery", "apply", "--config", &config]).await;
    let calls = provider.vcs.calls();
    assert_eq!(
        calls[before..before + 5],
        [
            format!("head {TARGET_REPO}"),
            format!("labelled {TARGET_REPO} emery/{id}"),
            format!("add {TARGET_REPO} {REVISION} root-commit"),
            format!("remove {REVISION}"),
            format!("add {TARGET_REPO} {REVISION} root-commit"),
        ],
        "read, then sealed over the head: {calls:?}"
    );
    assert!(!calls[before..].iter().any(|call| call.starts_with("init ")), "created once");
    assert_eq!(current(&provider), candidate);
    provider.vcs.assert_exhausted();
}

// The seal is refused, and nothing sealed, where the specification cannot
// live in the repository: a `spec/` of the operator's own at the head, a
// project checkout that is no repository or has no commit, and a credential
// among what the seal would commit.
#[tokio::test]
async fn apply_refused() {
    // the operator's own `spec/`
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.files.lists(REVISION, ".", &[("spec", EntryKind::Directory)]);
    provider.files.lists(REVISION, "spec", &[("openapi.yaml", EntryKind::File)]);
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains(
            "`spec/` at commit `9f8e7d6c5b4a39281706f5e4d3c2b1a0f9e8d7c6` holds `openapi.yaml`"
        )),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert!(provider.files.writes().is_empty(), "nothing is written");

    // a project directory that is no repository
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(".", Err(Error::NotARepository));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "repository-required")
            .await;
    assert_eq!(envelope["message"], "the project directory is not a repository", "{envelope}");

    // a project repository with no commit
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(".", Err(Error::NotFound("HEAD".to_owned())));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "base-not-sealed")
            .await;
    assert_eq!(
        envelope["message"], "the project repository has no commit to hold the specification",
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");

    // a credential the seal would commit
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.pending.script(
        REVISION,
        Ok(vec![Change {
            path: ".npmrc".to_owned(),
            kind: ChangeKind::Added,
        }]),
    );
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.starts_with(
            "what the seal wrote carries a credential, so none of it is sealed:\n- .npmrc"
        )),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert_eq!(provider.vcs.calls().last(), Some(&format!("remove {REVISION}")), "the copy goes");
    provider.vcs.assert_exhausted();
}

// The verbs that read or seal the current revision need the repository it
// lives in: `apply`, `show`, and `specify --apply` are refused typed where
// no `[target]` names one, before anything is read, while `show --staged`
// and a bare `specify` need none.
#[tokio::test]
async fn apply_target_required() {
    let scratch = Scratch::new();
    let config = scratch.config("[[source]]\nname = \"source\"\nadapter = \"acme:source@1.0.0\"\n");
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    stage(&provider.storage, SPEC, DESIGN, PLAN);

    for argv in [
        vec!["emery", "apply", "--config", config.as_str()],
        vec!["emery", "apply"],
        vec!["emery", "show", "spec", "--config", config.as_str()],
        vec!["emery", "show", "spec"],
        vec!["emery", "specify", "--apply", "--config", config.as_str()],
        vec!["emery", "specify", SOURCE, "--apply"],
    ] {
        let envelope = fail(&provider, &argv, 1, "build-target-required").await;
        assert!(
            envelope["message"]
                .as_str()
                .is_some_and(|message| message.contains("has no repository to live in")),
            "{argv:?}: {envelope}"
        );
        assert!(
            envelope["hint"].as_str().is_some_and(|hint| hint.contains("[target]")),
            "{argv:?}: {envelope}"
        );
    }
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");

    cli_ok(&provider, &["emery", "show", "spec", "--staged"]).await;
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    assert!(provider.vcs.calls().is_empty(), "a run with no target reads no repository");
    provider.model.assert_exhausted();
}

// `specify --apply` stages the candidate and seals it in one run: the
// output carries both lines, the envelope says the candidate is no longer
// staged and names what was sealed, and the slot and the repository stand
// as after `apply`.
#[tokio::test]
async fn specify_apply() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    let resp =
        cli_ok(&provider, &["emery", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG])
            .await;

    let id = current(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged candidate {id}\n  plan: 1 slice in 1 wave, widest 1\n  sealed revision {id} \
             as emery/{id} at Revision\n"
        )
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");
    assert_eq!(provider.vcs.messages().len(), 1, "one revision commit");
    assert!(provider.vcs.calls().contains(&format!("label . emery/{id} Revision-commit")));
    provider.model.assert_exhausted();

    // the envelope under a re-run that seals the same bytes over the head
    // now holding them
    let sealed = Provider::over(Arc::clone(&provider.storage), [SPEC_ANSWER, DESIGN_ANSWER]);
    assert_eq!(carry(&provider.files, &sealed.files), id, "the written bytes are the id");
    let provider = sealed;
    provider.vcs.commits.script(REVISION, Ok(None));
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(envelope["applied"]["revision"], id, "{envelope}");
    assert_eq!(envelope["applied"]["parent"], id, "{envelope}");
    assert_eq!(envelope["applied"]["commit"], HEAD, "nothing to seal: {envelope}");
    assert_eq!(envelope["applied"]["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], id, "the diff is against the revision displaced");
    assert_eq!(current(&provider), id);
    provider.model.assert_exhausted();
}

// The stage lands before the seal is attempted: a seal the repository
// refuses fails the run after the candidate is staged, so `emery apply` can
// seal it once the repository answers, and nothing of the current revision
// moves.
#[tokio::test]
async fn specify_apply_seal_refused() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    provider.vcs.commits.script(REVISION, Err(Error::Access("index.lock held".to_owned())));

    let resp = cli(
        &provider,
        &["emery", "--format", "json", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    assert_eq!(resp.exit, 4, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "bad_gateway", "{envelope}");
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains("index.lock held")),
        "{envelope}"
    );
    let candidate =
        String::from_utf8(provider.storage.state(STAGED_KEY).expect("staged")).expect("utf-8 id");
    assert_ne!(candidate, outgoing);
    assert!(
        provider.storage.object(STAGED_CONTAINER, &format!("{candidate}/spec.json")).is_some(),
        "the candidate stands staged"
    );
    assert!(provider.storage.state(REVISION_KEY).is_none(), "the pointer does not move");
    assert!(
        !provider.vcs.calls().iter().any(|call| call.starts_with("label ")),
        "nothing is labelled"
    );
    provider.model.assert_exhausted();

    // once the repository answers, apply seals what the run left staged
    let provider = Provider::over(Arc::clone(&provider.storage), Vec::<String>::new());
    seed(&provider.files, SPEC, DESIGN, PLAN);
    cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(current(&provider), candidate);
    provider.vcs.assert_exhausted();
}

// `--apply` with no source is refused as a bare `specify` is, before any
// load, and nothing is staged or sealed.
#[tokio::test]
async fn specify_apply_no_source() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    fail(&provider, &["emery", "specify", "--apply"], 1, "specify-source-required").await;
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.storage.state(STAGED_KEY).is_none());
    assert!(provider.storage.state(REVISION_KEY).is_none());
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
}

fn current(provider: &Provider) -> String {
    String::from_utf8(provider.storage.state(REVISION_KEY).expect("current"))
        .expect("utf-8 revision id")
}
