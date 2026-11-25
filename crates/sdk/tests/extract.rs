//! `extract` on the component rung: one gated turn per seam.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_adapter::source::Backing;
use support::by_file::{
    ByFile, FailKind, ScriptFailures, file_answers, file_invalid, file_retries, single_model,
};
use support::harness::{drive, extract};
use test_programs::protocol::{ExtractScenario, FileSpec, InputSpec, SeamSpec, WorkspaceInput};

// Every SDK driver guest must have a matching test here.
test_programs::foreach_sdk!();

const LINE: &str = "the line\n";

fn note(file: &str) -> String {
    format!(
        r#"{{"claims":[{{"kind":"decision","path":"{file}#L1","backing":{{"path":"{file}"}}}}]}}"#
    )
}

fn workspace(files: &[&str]) -> WorkspaceInput {
    WorkspaceInput {
        name: "docs".to_owned(),
        subpath: None,
        files: files
            .iter()
            .map(|path| FileSpec {
                path: (*path).to_owned(),
                body: LINE.to_owned(),
            })
            .collect(),
    }
}

fn scenario(input: InputSpec, seams: Vec<SeamSpec>) -> ExtractScenario {
    ExtractScenario {
        adapter_id: "probe".to_owned(),
        input,
        seams,
        omit_extract_prompt: false,
    }
}

fn paths(evidence: &emery_adapter::source::Evidence) -> Vec<&str> {
    evidence.claims.iter().map(|claim| claim.path.as_deref().unwrap_or_default()).collect()
}

#[tokio::test]
async fn sdk_extract() {
    three_seams().await;
    concurrent().await;
    head_of_line().await;
    largest_first().await;
    retried().await;
    retry_spent().await;
    bad_request_seam().await;
    no_seams().await;
    escaping_path().await;
    dot_path().await;
    files_value().await;
    two_seams_fail().await;
    single_seam_error().await;
}

async fn three_seams() {
    let files = ["a/x.md", "b/y.md", "c/z.md"];
    let model = ByFile::default()
        .file("a/x.md", file_answers([note("a/x.md")]))
        .file("b/y.md", file_answers([note("b/y.md")]))
        .file("c/z.md", file_answers([note("c/z.md")]));
    let run = extract(
        &scenario(
            InputSpec::Workspace(workspace(&files)),
            files
                .iter()
                .map(|file| SeamSpec::Files {
                    files: vec![(*file).to_owned()],
                })
                .collect(),
        ),
        model.clone(),
    )
    .await;
    let evidence = run.evidence();
    assert_eq!(paths(&evidence), files.iter().map(|file| format!("{file}#L1")).collect::<Vec<_>>(),);
    let backings: Vec<_> = evidence.claims.iter().map(|claim| claim.backing.clone()).collect();
    assert_eq!(
        backings,
        [
            Some(Backing::Path("a/x.md".to_owned())),
            Some(Backing::Path("b/y.md".to_owned())),
            Some(Backing::Path("c/z.md".to_owned())),
        ]
    );
    for file in files {
        let seen = model.model(file).seen();
        assert_eq!(seen.len(), 1, "one turn per seam");
        let user = &seen[0].messages[0];
        assert!(user.contains("`$SOURCE_DIR` is the bound source tree, lent read-only:"), "{user}");
        assert!(
            user.contains(&format!("### `{file}` (1 line)\n\n```\n1|the line\n```\n\n")),
            "the one file is laid out: {user}"
        );
    }
    model.assert_exhausted();
}

async fn concurrent() {
    let files: Vec<String> = (0..=4).map(|i| format!("d{i}/f.md")).collect();
    let mut model = ByFile::default();
    for file in &files {
        model = model.file(file, file_answers([note(file)]));
    }
    let run = extract(
        &scenario(
            InputSpec::Workspace(workspace(&files.iter().map(String::as_str).collect::<Vec<_>>())),
            files
                .iter()
                .map(|file| SeamSpec::Files {
                    files: vec![file.clone()],
                })
                .collect(),
        ),
        model.clone(),
    )
    .await;
    assert_eq!(model.peak(), 4, "5 seams hold at most 4 completions pending");
    let expected: Vec<String> = files.iter().map(|file| format!("{file}#L1")).collect();
    assert_eq!(paths(&run.evidence()), expected);
    model.assert_exhausted();
}

async fn head_of_line() {
    let files: Vec<String> = (0..=4).map(|i| format!("d{i}/f.md")).collect();
    let mut model = ByFile::default().holding("d0/f.md", "d4/f.md");
    for file in &files {
        model = model.file(file, file_answers([note(file)]));
    }
    let run = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        extract(
            &scenario(
                InputSpec::Workspace(workspace(
                    &files.iter().map(String::as_str).collect::<Vec<_>>(),
                )),
                files
                    .iter()
                    .map(|file| SeamSpec::Files {
                        files: vec![file.clone()],
                    })
                    .collect(),
            ),
            model.clone(),
        ),
    )
    .await
    .expect("the fifth seam is dispatched while the first is still pending");
    let expected: Vec<String> = files.iter().map(|file| format!("{file}#L1")).collect();
    assert_eq!(paths(&run.evidence()), expected);
    model.assert_exhausted();
}

async fn largest_first() {
    let files = ["a/1.md", "n/1.md", "b/1.md", "b/2.md", "b/3.md", "c/1.md", "c/2.md"];
    let model = ByFile::default()
        .file("a/1.md", file_answers([note("a/1.md")]))
        .file("n/1.md", file_answers([note("n/1.md")]))
        .file("b/1.md", file_answers([note("b/1.md")]))
        .file("c/1.md", file_answers([note("c/1.md")]));
    let run = extract(
        &scenario(
            InputSpec::Workspace(workspace(&files)),
            vec![
                SeamSpec::Files {
                    files: vec!["a/1.md".to_owned()],
                },
                SeamSpec::Note {
                    text: "Mine the surface entered at `n/1.md`.".to_owned(),
                },
                SeamSpec::Files {
                    files: vec!["b/1.md".to_owned(), "b/2.md".to_owned(), "b/3.md".to_owned()],
                },
                SeamSpec::Files {
                    files: vec!["c/1.md".to_owned(), "c/2.md".to_owned()],
                },
            ],
        ),
        model.clone(),
    )
    .await;
    assert_eq!(
        model.arrivals(),
        ["n/1.md", "b/1.md", "c/1.md", "a/1.md"],
        "the unsized seam first, then the seams naming files by descending count"
    );
    assert_eq!(
        paths(&run.evidence()),
        ["a/1.md#L1", "n/1.md#L1", "b/1.md#L1", "c/1.md#L1"],
        "the document reads in seam order"
    );
    model.assert_exhausted();
}

async fn retried() {
    let model = ByFile::default().file("a/x.md", file_answers([note("a/x.md")])).file(
        "b/y.md",
        file_retries([Err(FailKind::Backend("down".to_owned())), Ok(note("b/y.md"))]),
    );
    let run = extract(
        &scenario(
            InputSpec::Workspace(workspace(&["a/x.md", "b/y.md"])),
            vec![
                SeamSpec::Files {
                    files: vec!["a/x.md".to_owned()],
                },
                SeamSpec::Files {
                    files: vec!["b/y.md".to_owned()],
                },
            ],
        ),
        model.clone(),
    )
    .await;
    assert_eq!(paths(&run.evidence()), ["a/x.md#L1", "b/y.md#L1"]);
    assert_eq!(model.turns("b/y.md"), 2, "the failed seam was put twice");
    assert_eq!(model.turns("a/x.md"), 1, "the answered seam once");
    model.assert_exhausted();
}

async fn retry_spent() {
    let model = ByFile::default().file("a/x.md", file_answers([note("a/x.md")])).file(
        "b/y.md",
        file_retries([
            Err(FailKind::Backend("down".to_owned())),
            Err(FailKind::Backend("still down".to_owned())),
        ]),
    );
    let error = extract(
        &scenario(
            InputSpec::Workspace(workspace(&["a/x.md", "b/y.md"])),
            vec![
                SeamSpec::Files {
                    files: vec!["a/x.md".to_owned()],
                },
                SeamSpec::Files {
                    files: vec!["b/y.md".to_owned()],
                },
            ],
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "unavailable");
    assert_eq!(error.description(), "`docs`: 1 of 2 seams failed:\n- seam 1: still down");
    assert_eq!(model.turns("b/y.md"), 2);
    model.assert_exhausted();
}

async fn bad_request_seam() {
    let model = ByFile::default()
        .file("a/x.md", file_invalid("no such model"))
        .file("b/y.md", file_answers([note("b/y.md")]))
        .file("c/z.md", file_answers([r#"{"claims":[{"kind":"requirement","id":"c"}]}"#]));
    let error = extract(
        &scenario(
            InputSpec::Workspace(workspace(&["a/x.md", "b/y.md", "c/z.md"])),
            ["a/x.md", "b/y.md", "c/z.md"]
                .into_iter()
                .map(|file| SeamSpec::Files {
                    files: vec![file.to_owned()],
                })
                .collect(),
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "invalid");
    assert!(error.description().starts_with("`docs`: 2 of 3 seams failed:\n"), "{error}");
    assert_eq!(model.turns("a/x.md"), 1);
    assert_eq!(model.turns("c/z.md"), 1);
    model.assert_exhausted();
}

async fn no_seams() {
    let model = single_model(Vec::<String>::new());
    let error = extract(&scenario(InputSpec::Workspace(workspace(&[])), Vec::new()), model.clone())
        .await
        .error();
    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("nothing to extract"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

async fn escaping_path() {
    let model = single_model(Vec::<String>::new());
    let error = extract(
        &scenario(
            InputSpec::Workspace(workspace(&["a/x.md"])),
            vec![
                SeamSpec::Files {
                    files: vec!["a/x.md".to_owned()],
                },
                SeamSpec::Files {
                    files: vec!["../secret.md".to_owned()],
                },
            ],
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("`../secret.md` escapes the root"), "{error}");
    assert_eq!(model.seen(), Vec::new());
}

async fn dot_path() {
    let model = single_model(Vec::<String>::new());
    let error = extract(
        &scenario(
            InputSpec::Workspace(workspace(&[])),
            vec![SeamSpec::Files {
                files: vec!["./".to_owned()],
            }],
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("names no file"), "{error}");
    assert_eq!(model.seen(), Vec::new());
}

async fn files_value() {
    let model = single_model(Vec::<String>::new());
    let error = extract(
        &scenario(
            InputSpec::Value {
                name: "brief".to_owned(),
                text: "Ship it.".to_owned(),
            },
            vec![SeamSpec::Files {
                files: vec!["a/x.md".to_owned()],
            }],
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("inline value with no tree to lend"), "{error}");
    assert_eq!(model.seen(), Vec::new());
}

async fn two_seams_fail() {
    let model = ByFile::default()
        .file("a/x.md", file_invalid("no such model"))
        .file("b/y.md", file_answers([note("b/y.md")]))
        .file(
            "c/z.md",
            file_retries([
                Err(FailKind::Backend("down".to_owned())),
                Err(FailKind::Backend("down".to_owned())),
            ]),
        );
    let error = extract(
        &scenario(
            InputSpec::Workspace(workspace(&["a/x.md", "b/y.md", "c/z.md"])),
            ["a/x.md", "b/y.md", "c/z.md"]
                .into_iter()
                .map(|file| SeamSpec::Files {
                    files: vec![file.to_owned()],
                })
                .collect(),
        ),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "invalid");
    assert_eq!(
        error.description(),
        "`docs`: 2 of 3 seams failed:\n- seam 0: no such model\n- \
         seam 2: down"
    );
    model.assert_exhausted();
}

async fn single_seam_error() {
    let model = ScriptFailures::new([
        Err(FailKind::Backend("down".to_owned())),
        Err(FailKind::Backend("still down".to_owned())),
    ]);
    let error = extract(
        &scenario(InputSpec::Workspace(workspace(&[])), vec![SeamSpec::Whole]),
        model.clone(),
    )
    .await
    .error();
    assert_eq!(error.code(), "unavailable");
    assert_eq!(error.description(), "still down");
    assert_eq!(model.seen(), 2);
    model.assert_exhausted();
}

// Completeness arms for driver guests whose suites live in other files.
#[tokio::test]
async fn sdk_survey() {}

#[tokio::test]
async fn sdk_build() {
    let scenario = test_programs::protocol::BuildScenario {
        op: "stub".to_owned(),
    };
    let (response, _) = drive::<serde_json::Value, _>(
        test_programs::SDK_BUILD,
        (omnia_test::host::scratch(), serde_json::to_string(&scenario).unwrap()),
        single_model(Vec::<String>::new()),
    )
    .await;
    assert!(matches!(response, test_programs::protocol::Response::Ok { .. }));
}

#[tokio::test]
async fn sdk_verify() {}
