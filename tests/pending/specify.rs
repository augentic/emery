//! Remainder of `emery specify` scenarios — ported incrementally from this file (A24).
//!
//! Lines 1–3382 are live in `tests/specify.rs` after A23.

// --- repository sources ---

mod repository {
    use omnia_sdk::vcs::Error;

    use super::*;

    const URL: &str = "https://example.com/acme/shop.git";
    // `URL` normalised and hashed, the clone every run of it shares.
    const CLONE: &str = "./.emery/vcs/repos/f4c82940a714c3c5";
    const CHECKOUT: &str = "./.emery/vcs/sources/upstream";

    fn upstream(scratch: &Scratch, body: &str) -> String {
        scratch.config(&format!(
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"{URL}\"\n{body}"
        ))
    }

    // A source naming a repository is read in a working copy of it at the
    // revision: the clone made or fetched after the adapters load, the copy cut
    // and lent, at the `path` within it, and removed once the source answered.
    #[tokio::test]
    async fn repository_source() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v1.2.0\"\npath = \"docs/api\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            stdout.contains(&format!("\n  upstream read from {URL} at v1.2.0: v1.2.0-commit\n")),
            "{stdout}"
        );
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} v1.2.0"),
                format!("add {CLONE} {CHECKOUT} v1.2.0-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        let calls = provider.source.calls();
        let (_, input) = calls.first().expect("one extract dispatch");
        assert_eq!(input.name, "upstream");
        assert_eq!(
            input.content,
            SourceContent::Workspace(format!("{CHECKOUT}/docs/api")),
            "the lend is the path within the working copy"
        );
        assert_eq!(
            provider.source.metadata.lock().expect("metadata").len(),
            1,
            "the adapter loads before the repository is touched"
        );
        drop(calls);
        provider.model.assert_exhausted();

        // the JSON envelope names what was read, and a path omitted lends the copy whole
        let config = upstream(&scratch, "revision = \"main\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        provider.vcs.clones.script(CLONE, Err(Error::Exists(CLONE.to_owned())));
        let resp =
            cli_ok(&provider, &["emery", "--format", "json", "specify", "--config", &config]).await;
        let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
        assert_eq!(
            envelope["repositories"],
            serde_json::json!([
                {"source": "upstream", "repository": URL, "revision": "main", "commit": "main-commit"}
            ]),
            "{envelope}"
        );
        assert_eq!(
            provider.vcs.calls()[..2],
            [format!("clone {URL} {CLONE}"), format!("fetch {CLONE} origin")],
            "found, so fetched"
        );
        let calls = provider.source.calls();
        assert_eq!(calls[0].1.content, SourceContent::Workspace(CHECKOUT.to_owned()));
        drop(calls);
        provider.model.assert_exhausted();
    }

    // Two sources naming one repository, however they spell its URL, share
    // one clone made once and are read in working copies of their own.
    #[tokio::test]
    async fn repo_shared_clone() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"HTTPS://Example.com/acme/shop/\"\nrevision = \"v2\"\npath = \"api\"\n",
        );
        let grouping = baseline_grouping(2);
        let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} ./.emery/vcs/sources/docs main-commit"),
                format!("resolve {CLONE} v2"),
                format!("add {CLONE} ./.emery/vcs/sources/api v2-commit"),
                "remove ./.emery/vcs/sources/docs".to_owned(),
                "remove ./.emery/vcs/sources/api".to_owned(),
            ]
        );
        let roots: Vec<SourceContent> =
            provider.source.calls().into_iter().map(|(_, input)| input.content).collect();
        assert_eq!(
            roots,
            [
                SourceContent::Workspace("./.emery/vcs/sources/docs".to_owned()),
                SourceContent::Workspace("./.emery/vcs/sources/api/api".to_owned()),
            ]
        );
        provider.model.assert_exhausted();
    }

    // The working copy goes whether or not the source answered; a stale one
    // from a failed run is removed before the copy is cut again.
    #[tokio::test]
    async fn repo_extract_error() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"main\"\n");
        let mut provider = Provider::idle();
        provider
            .source
            .evidence
            .insert("upstream".to_string(), Err(bad_gateway!("the adapter exploded")));
        provider.vcs.adds.script(CHECKOUT, Err(Error::Exists(CHECKOUT.to_owned())));

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;

        assert_eq!(envelope["message"], "the adapter exploded");
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        provider.vcs.assert_exhausted();
    }

    // A revision the repository lacks, or a URL that names none, is
    // `revision-not-found` before any source is asked; a remote that cannot be
    // reached is the gateway's.
    #[tokio::test]
    async fn repo_missing_revision() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v9\"\n");
        let provider = Provider::idle();

        provider.vcs.resolves.script(CLONE, Err(Error::NotFound("v9".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 2, "revision-not-found")
                .await;
        assert_message(&envelope, &format!("repository `{URL}` has no `v9`"));
        assert!(
            envelope["hint"].as_str().is_some_and(|hint| hint.contains("`revision`")),
            "{envelope}"
        );
        assert!(provider.source.calls().is_empty(), "no source is asked");

        provider.vcs.clones.script(CLONE, Err(Error::Access("authentication failed".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;
        assert_message(&envelope, &format!("repository `{URL}`: authentication failed"));
        provider.vcs.assert_exhausted();
    }

    // A repository is read at a revision and at a path within it, never
    // beside inline text or above the clone; the file is refused before any
    // adapter loads.
    #[tokio::test]
    async fn repository_config() {
        let cases: &[(&str, &str)] = &[
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\n",
                "source `upstream` sets `repository` without `revision`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 revision = \"main\"\n",
                "source `upstream` sets `revision` without `repository`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:intent@1.0.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 description = \"Ship it.\"\n",
                "source `upstream` sets both `repository` and `description`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 path = \"../sibling\"\n",
                "source `upstream`: path `../sibling` must lie within the repository",
            ),
        ];
        for (body, fragment) in cases {
            let scratch = Scratch::new();
            let config = scratch.config(body);
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
            assert_message(&envelope, fragment);
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a refused list gates nothing: {body}"
            );
            assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
        }
    }
}

// --- adapter references ---

mod reference {
    use std::fs;

    use super::{
        DESIGN_ANSWER, Provider, SPEC_ANSWER, Scratch, assert_message, cli_ok, fail, guest, shown,
    };

    // The exact reference loads as the guest it names without its version,
    // and the source is named for the package.
    #[tokio::test]
    async fn exact_ref() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "emery:demo@1.2.0"]).await;

        let gated = provider.source.metadata.lock().expect("metadata").clone();
        assert_eq!(gated, ["emery:demo"], "the package is gated without its version");
        let calls = provider.source.calls();
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo", "the adapter id is the registered guest");
        assert_eq!(input.name, "demo", "the source name is the package name");
        drop(calls);
        provider.model.assert_exhausted();
    }

    // A reference parses whole or not at all: no default namespace, nothing
    // resolving to a latest, no path, no bare name, no source checkout.
    #[tokio::test]
    async fn malformed_ref() {
        let cases: &[(&str, &str)] = &[
            ("emery:demo", "adapter `emery:demo` names no version"),
            ("demo@1.2.0", "adapter `demo@1.2.0` names no namespace"),
            ("demo", "adapter `demo` names no version"),
            ("emery:demo@main", "adapter `emery:demo@main` has an invalid version `main`"),
            ("emery:@1.2.0", "adapter `emery:@1.2.0` is not `namespace:name@version`"),
            ("Emery:demo@1.2.0", "adapter `Emery:demo@1.2.0` is not `namespace:name@version`"),
            ("./demo.wasm", "adapter `./demo.wasm` names no version"),
            ("/tmp/demo.wasm", "adapter `/tmp/demo.wasm` names no version"),
            ("https://github.com/acme/api", "GitHub URLs are not supported"),
        ];
        for (reference, fragment) in cases {
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", reference], 1, "adapter-reference").await;
            assert_message(&envelope, fragment);
            assert_eq!(
                envelope["hint"],
                "an adapter is an exact package reference, `namespace:name@version`",
                "{envelope}"
            );
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a malformed reference loads nothing: {reference}"
            );
        }
    }

    // Routing is the deployment's, so the file reserves no table for it.
    #[tokio::test]
    async fn registries_table() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n\n\
             [registries]\nacme = \"registry.acme.io\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, "unknown field `registries`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a file that does not parse loads nothing"
        );
    }

    // Two versions of one package would register as one guest, the second
    // load attesting the first's bytes; the list is refused before either
    // loads.
    #[tokio::test]
    async fn version_collision() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.3.0\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(
            &envelope,
            "adapters `emery:documentation@1.2.0` and `emery:documentation@1.3.0`",
        );
        assert_message(&envelope, "would both register as `emery:documentation`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a colliding list loads nothing"
        );

        // the same two on the command line are two sources of one package name
        let envelope = fail(
            &provider,
            &["emery", "specify", "emery:documentation@1.2.0", "emery:documentation@1.3.0"],
            1,
            "bad_request",
        )
        .await;
        assert_message(&envelope, "source `documentation` appears twice");
    }

    // An argv run reads no project-root file: one that does not parse, or
    // whose source is located outside its root, refuses the discovered run
    // alone. The CWD move is hermetic under nextest's process-per-test
    // isolation.
    #[tokio::test]
    async fn argv_with_bad_config() {
        let project = tempfile::TempDir::new().expect("project dir");
        std::env::set_current_dir(project.path()).expect("enter project");
        let argv = ["emery", "specify", "acme:ledger@2.1.0"];

        for (entry, refusal) in [
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nbranch = \
                 \"main\"\n",
                "unknown field `branch`",
            ),
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
                 \"../../outside\"\n",
                "must be relative to the project root",
            ),
            ("[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n", "names no version"),
        ] {
            fs::write(project.path().join("emery.toml"), entry).expect("write emery.toml");
            let envelope = fail(&Provider::idle(), &["emery", "specify"], 1, "bad_request").await;
            assert_message(&envelope, refusal);
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

            cli_ok(&provider, &argv).await;

            let gated = provider.source.metadata.lock().expect("metadata").clone();
            assert_eq!(gated, [guest("ledger")], "argv names the run's only source: {entry}");
            assert!(
                shown(&provider, "spec").await.contains("Sources: [ledger:greeting.behaviour]"),
                "the file's `[[source]]` entries stay out of an argv run"
            );
            provider.model.assert_exhausted();
        }
    }
}

mod digest {
    use super::{Provider, SOURCE, Scratch, assert_message, digest, fail};

    // One adapter loads once under one pin, so two entries pinning it differently
    // cannot both be honoured; the list is refused before either is asked for.
    #[tokio::test]
    async fn conflict() {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n",
            digest("cd"),
            digest("ab")
        ));
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, &format!("adapter `{SOURCE}` is pinned to two digests"));
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a conflicting pin loads nothing"
        );
    }
}

mod store {
    use std::sync::Arc;

    use emery_engine::vcs::{REVISION, RevisionMessage, SPECIFICATION};
    use emery_engine::{EntryKind, REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
    use omnia_test::guest::{Memory, Namespaced};
    use serde_json::Value;

    use super::support::vcs::HEAD;
    use super::{
        DESIGN_ANSWER, PROJECT_CONFIG, Provider, SOURCE, SPEC_ANSWER, SPEC_RENDERED, SPEC_REVISION,
        assert_message, cli_ok, committed, fail, project_staged, projection, reference, seed,
        shown, shown_sealed, staged,
    };

    const SPEC: &[u8] = include_bytes!("build/spec.json");
    const DESIGN: &[u8] = include_bytes!("build/design.json");
    const PLAN: &[u8] = include_bytes!("build/plan.json");

    const LABELLED: &str = "e1f2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4";

    // The pointer is a cache over the repository: bytes at it that name no
    // label the repository holds, or that decode to no id, are passed over
    // and the head's tree read, so `show` refuses typed where that holds no
    // revision, the next `specify` stages with the diff suppressed, and
    // `apply` swaps over them.
    #[tokio::test]
    async fn corrupt_current() {
        for bytes in [&b"0123456789abcdef"[..], &b"\xff\xfe"[..]] {
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
            provider.storage.insert_state(REVISION_KEY, bytes);
            fail(
                &provider,
                &["emery", "show", "spec", "--config", PROJECT_CONFIG],
                2,
                "spec-not-generated",
            )
            .await;

            let resp = cli_ok(
                &provider,
                &["emery", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
            )
            .await;
            let stdout = String::from_utf8_lossy(&resp.stdout);
            assert!(!stdout.contains("diff vs"), "a corrupt pointer yields no diff: {stdout}");
            let id = staged(&provider.storage);
            assert!(
                provider.storage.object(STAGED_CONTAINER, &format!("{id}/spec.json")).is_some()
            );

            cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
            assert_eq!(committed(&provider.storage), id, "the seal swaps over the corruption");
            cli_ok(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG]).await;
            provider.model.assert_exhausted();
        }
    }

    // Bytes at the staged key that name no candidate are corruption: `show
    // --staged` and `apply` fail closed, and the next `specify` stages over
    // them.
    #[tokio::test]
    async fn corrupt_staged() {
        for bytes in [&b"0123456789abcdef"[..], &b"\xff\xfe"[..]] {
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
            provider.storage.insert_state(STAGED_KEY, bytes);
            fail(&provider, &["emery", "show", "spec", "--staged"], 3, "server_error").await;
            fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 3, "server_error")
                .await;

            cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
            let id = staged(&provider.storage);
            assert_ne!(id.as_bytes(), bytes, "the candidate stands in the slot");
            cli_ok(&provider, &["emery", "show", "spec", "--staged"]).await;
            provider.model.assert_exhausted();
        }
    }

    // A document edited under the label no longer hashes to the revision the
    // label names, so every verb that reads the current revision refuses
    // typed, nothing is staged over it, and the hint names the restore;
    // once the tree holds the revision again, `show` renders it.
    #[tokio::test]
    async fn tampered_revision() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        cli_ok(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
        )
        .await;
        let id = committed(&provider.storage);
        let sealed = super::committed_document(&provider, "spec");
        provider.files.holds(REVISION, &format!("{SPECIFICATION}/spec.json"), "{}\n");
        for _ in 0..3 {
            provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
        }

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert_message(
            &envelope,
            &format!("labelled `emery/{id}`, does not match revision `{id}`"),
        );
        assert!(
            envelope["hint"]
                .as_str()
                .unwrap_or("")
                .contains("git checkout emery/<revision> -- spec/"),
            "{envelope}"
        );
        fail(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged over it");

        provider.files.holds_bytes(REVISION, &format!("{SPECIFICATION}/spec.json"), &sealed);
        assert!(shown_sealed(&provider, "spec").await.contains("hello"), "show renders it again");
        provider.model.assert_exhausted();
    }

    // A `spec/` of the operator's own at the head, holding anything but the
    // three documents, is where the specification cannot live: every verb
    // over the current revision refuses typed and nothing is sealed there.
    #[tokio::test]
    async fn spec_occupied() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        provider.files.lists(REVISION, ".", &[(SPECIFICATION, EntryKind::Directory)]);
        provider.files.lists(REVISION, SPECIFICATION, &[("openapi.yaml", EntryKind::File)]);

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert_message(&envelope, "holds `openapi.yaml`, where the three documents");
        fail(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
        assert!(provider.files.writes().is_empty(), "nothing is written");

        // a bare specify needs no repository and stages regardless
        cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
        assert!(provider.storage.state(STAGED_KEY).is_some());
        provider.model.assert_exhausted();
    }

    // Outdated is not corrupt: `show` refuses typed, and the next `specify`
    // regenerates over it with the diff suppressed.
    #[tokio::test]
    async fn spec_outdated() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let outdated = seed(
            &provider.files,
            br#"{"emery": 1, "requirements": []}"#,
            br#"{"emery": 1, "sections": []}"#,
            br#"{"emery": 1, "slices": []}"#,
        );

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-outdated",
        )
        .await;
        assert_message(&envelope, "grammar 1");
        assert!(
            envelope["hint"].as_str().unwrap_or("").contains("emery specify"),
            "the hint names the way out: {envelope}"
        );

        let resp = cli_ok(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
        )
        .await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            !stdout.contains("diff vs"),
            "an outdated outgoing revision yields no diff: {stdout}"
        );
        let id = committed(&provider.storage);
        assert_ne!(id, outdated, "the regenerated revision is current");
        cli_ok(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG]).await;
        provider.model.assert_exhausted();
    }

    // A revision id no repository holds, for a parent the lineage names.
    const EARLIER: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    // `tests/build/spec.json` as an amendment sealed it, its `parent` naming
    // `EARLIER`, so a repository holding it holds a lineage.
    fn lineage() -> Vec<u8> {
        let spec = std::str::from_utf8(SPEC).expect("spec.json is UTF-8");
        spec.replace("\"parent\": null", &format!("\"parent\": \"{EARLIER}\"")).into_bytes()
    }

    // A specify that knows the target reads the current revision before any
    // load, and one that amends an earlier revision refuses the run there:
    // nothing loads, nothing is staged, and the hint names `emery amend` and
    // `--fresh` as the ways out. The parent is read off the specification's
    // own key, so a lineage under an outdated grammar refuses the same, and
    // a run naming no target reads no repository and stages regardless.
    #[tokio::test]
    async fn existing_lineage() {
        let provider = Provider::idle();
        let held = seed(&provider.files, &lineage(), DESIGN, PLAN);

        for argv in [
            vec!["emery", "specify", SOURCE, "--config", PROJECT_CONFIG],
            vec!["emery", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
        ] {
            let envelope = fail(&provider, &argv, 1, "lineage-exists").await;
            assert_message(&envelope, &format!("the current revision `{held}` amends `{EARLIER}`"));
            let hint = envelope["hint"].as_str().expect("a hint");
            assert!(hint.contains("emery amend"), "{argv:?}: {envelope}");
            assert!(hint.contains("emery specify --fresh"), "{argv:?}: {envelope}");
        }
        assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");
        assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");

        // a lineage under an outdated grammar is a lineage still
        let provider = Provider::idle();
        seed(
            &provider.files,
            format!(r#"{{"emery": 1, "parent": "{EARLIER}", "requirements": []}}"#).as_bytes(),
            br#"{"emery": 1, "sections": []}"#,
            br#"{"emery": 1, "slices": []}"#,
        );
        fail(
            &provider,
            &["emery", "specify", SOURCE, "--config", PROJECT_CONFIG],
            1,
            "lineage-exists",
        )
        .await;

        // a run with no target reads no repository
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        seed(&provider.files, &lineage(), DESIGN, PLAN);
        cli_ok(&provider, &["emery", "specify", SOURCE]).await;
        assert!(provider.vcs.calls().is_empty(), "a run with no target reads no repository");
        provider.model.assert_exhausted();
    }

    // `--fresh` starts the history over: the run proceeds over the lineage,
    // the candidate is staged with its record marked fresh, the text and
    // the envelope say so, and `apply` seals it with no parent over the
    // head, the diff still reported against the revision it supersedes.
    #[tokio::test]
    async fn specify_fresh() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let outgoing = seed(&provider.files, &lineage(), DESIGN, PLAN);

        let resp =
            cli_ok(&provider, &["emery", "specify", SOURCE, "--fresh", "--config", PROJECT_CONFIG])
                .await;

        let id = staged(&provider.storage);
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.starts_with(&format!("staged candidate {id} (fresh)\n")), "{stdout}");
        assert!(stdout.contains(&format!("  diff vs {outgoing}: ")), "{stdout}");
        assert!(stdout.ends_with("  nothing sealed; `emery apply` seals it\n"), "{stdout}");
        let record: Value =
            serde_json::from_slice(&provider.storage.state(STAGED_KEY).expect("staged record"))
                .expect("staged record JSON");
        assert_eq!(record["fresh"], true, "the record marks the candidate fresh: {record}");

        cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
        assert_eq!(committed(&provider.storage), id);
        let message =
            RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
        assert_eq!(message.parent, None, "a fresh seal records no parent");
        assert_eq!(message.base.as_deref(), Some(HEAD), "sealed over the head");
        provider.model.assert_exhausted();

        // the envelope carries the flag
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        seed(&provider.files, &lineage(), DESIGN, PLAN);
        let resp = cli_ok(
            &provider,
            &[
                "emery",
                "--format",
                "json",
                "specify",
                SOURCE,
                "--fresh",
                "--config",
                PROJECT_CONFIG,
            ],
        )
        .await;
        let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
        assert_eq!(envelope["revision"], id, "{envelope}");
        assert_eq!(envelope["fresh"], true, "{envelope}");
        assert_eq!(envelope["staged"], true, "{envelope}");
        assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
        provider.model.assert_exhausted();

        // a run over no lineage takes the flag too, staging as any other
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let resp = cli_ok(&provider, &["emery", "specify", SOURCE, "--fresh"]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.starts_with(&format!("staged candidate {id} (fresh)\n")), "{stdout}");
        provider.model.assert_exhausted();
    }

    // Isolation is host policy over the engine's flat keys.
    #[tokio::test]
    async fn multi_project() {
        // one shared store, two project-scoped views
        let shared = Memory::default();
        let alpha = Provider::over(
            Arc::new(Namespaced::new("alpha", shared.clone())),
            [SPEC_ANSWER, DESIGN_ANSWER],
        );
        let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
        let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
        let beta = Provider::over(
            Arc::new(Namespaced::new("beta", shared.clone())),
            [beta_spec, beta_design],
        );

        cli_ok(&alpha, &["emery", "specify", SOURCE]).await;
        cli_ok(&beta, &["emery", "specify", SOURCE]).await;

        // every write landed under its project prefix
        assert!(shared.state(STAGED_KEY).is_none(), "no unprefixed staged id exists");
        assert!(shared.objects(STAGED_CONTAINER).is_empty(), "no unprefixed candidate exists");

        let id_alpha = project_staged(&shared, "alpha");
        let id_beta = project_staged(&shared, "beta");
        assert_ne!(id_alpha, id_beta, "distinct documents stage distinct candidates");

        // each project shows its own candidate
        let spec_alpha = shared
            .object(&format!("alpha/{STAGED_CONTAINER}"), &format!("{id_alpha}/spec.json"))
            .expect("spec.json");
        let spec_beta = shared
            .object(&format!("beta/{STAGED_CONTAINER}"), &format!("{id_beta}/spec.json"))
            .expect("spec.json");
        assert_eq!(
            String::from_utf8_lossy(&spec_alpha),
            SPEC_REVISION,
            "alpha staged the revision"
        );
        assert!(String::from_utf8_lossy(&spec_beta).contains("howdy"));
        assert_eq!(
            shown(&alpha, "spec").await,
            projection(SPEC_RENDERED, &id_alpha),
            "alpha shows its own revision"
        );
        assert!(shown(&beta, "spec").await.contains("howdy"), "beta shows its own revision");

        // a seal's pointer lands under its prefix too
        cli_ok(&alpha, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
        assert!(shared.state(REVISION_KEY).is_none(), "no unprefixed current id exists");
        assert_eq!(
            shared.state(&format!("alpha/{REVISION_KEY}")).as_deref(),
            Some(id_alpha.as_bytes()),
            "alpha sealed its own candidate"
        );
        assert_eq!(project_staged(&shared, "beta"), id_beta, "beta's candidate stands");

        alpha.model.assert_exhausted();
        beta.model.assert_exhausted();
    }
}

// --- helpers ---

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

fn committed(storage: &Memory) -> String {
    String::from_utf8(storage.state(REVISION_KEY).expect("current")).expect("utf-8 revision id")
}

fn project_staged(shared: &Memory, project: &str) -> String {
    staged_at(shared, &format!("{project}/{STAGED_KEY}"))
}

fn staged_document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage
        .object(STAGED_CONTAINER, &format!("{id}/{name}"))
        .unwrap_or_else(|| panic!("staged {name}"))
}

fn committed_document(provider: &Provider, name: &str) -> Vec<u8> {
    written(&provider.files, name)
}

fn projection(fixture: &str, id: &str) -> String {
    fixture.replace("<revision>", id)
}

async fn shown<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document, "--staged"]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}

async fn shown_sealed<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document, "--config", PROJECT_CONFIG]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}
