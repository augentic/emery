//! Every call here mines one seam, so each is one turn and its outcome passes
//! through unchanged; the fan-out and join over several are `extract.rs`'s.

use emery_sdk::{Context, Doc, Error, Evidence, Note, Seam, SourceInput};
use omnia_sdk::model::{Error as ModelError, ToolCall};
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const PROSE: &[Doc] = &[
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/greeting.md",
        body: "Greet warmly.",
    },
    Doc {
        path: "survey.md",
        body: "SURVEY",
    },
];

const VALID: &str = r#"{"claims":[
    {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
    {"kind":"decision"}
]}"#;

fn files<const N: usize>(paths: [&str; N]) -> Seam {
    Seam::Files(paths.into_iter().map(str::to_string).collect())
}

async fn ask(model: &Scripted, input: &SourceInput, seam: Seam) -> Result<Evidence, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
        model,
    };
    emery_sdk::extract(&ctx, PROSE, &[seam]).await
}

#[tokio::test]
async fn request_shape() {
    let model = Scripted::answering([VALID]);

    let accepted = ask(&model, &SourceInput::workspace("docs", "/lend/docs"), Seam::Whole)
        .await
        .expect("a valid answer is accepted first time");
    assert_eq!(accepted.claims.len(), 2);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    let claims = emery_sdk::body(emery_sdk::RUNTIME, "claims.md").expect("embedded");
    let system = format!("SYSTEM\n\n---\n\n{claims}");
    assert_eq!(request.system.as_deref(), Some(system.as_str()));
    assert!(
        !request.messages[0].contains("/lend/docs"),
        "the lend carries the root, not the brief: {}",
        request.messages[0]
    );
    assert_eq!(
        request.messages,
        [concat!(
            "Extract the claim set of the source `docs` bound to adapter `source:probe`.\n\n",
            "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file you ",
            "can read, and the root every `path` is relative to. Walk it as the prompt ",
            "describes. Nothing outside it is reachable; extract mines only this source.\n\n",
            "The claim rules (`claims.md`) are already in the system prompt; the prompt's ",
            "further references are available through this call's `read_doc` tool (`list_docs` ",
            "enumerates them); load referenced bodies on demand.\n\n",
            "Answer with one JSON object matching the gated claims schema. The caller persists ",
            "the document; do not write it yourself."
        )]
    );
    assert!(request.check, "acceptance is the check, not the reply text");
    assert_eq!(request.tools, ["list_docs", "read_doc"], "the corpus is offered through tools");
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the lend follows the input");

    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("evidence is steered by schema");
    };
    assert_eq!(name, "evidence-docs-0", "the question is labelled by source and seam");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("generated schema parses");
    assert!(schema.pointer("/properties/kind").is_none(), "the answer is claims alone: {schema}");
    let claim = schema.pointer("/$defs/Claim").expect("Claim definition");
    assert_eq!(
        claim.pointer("/properties/id/pattern").and_then(serde_json::Value::as_str),
        Some("^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$")
    );
    assert!(claim.pointer("/properties/backing").is_some(), "schema tracks the DTO");
    assert_ne!(
        claim.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "open claim extras stay admitted"
    );
    model.assert_exhausted();
}

// A corpus without `extract.md` is the adapter build's own defect.
#[tokio::test]
async fn missing_prompt() {
    let model = Scripted::default();
    let input = SourceInput::workspace("docs", ".");
    let ctx = Context {
        adapter_id: "source:mute",
        input: &input,
        model: &model,
    };

    let error =
        emery_sdk::extract(&ctx, &[], &[Seam::Whole]).await.expect_err("no prompt to ask with");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`extract.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "nothing was asked");
}

#[tokio::test]
async fn inline_value() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole).await.expect("accepted");
    let request = &model.seen()[0];
    assert!(request.workspace.is_none(), "no lend for an inline value");
    let user = &request.messages[0];
    assert!(user.contains("the source `brief` bound to"), "{user}");
    assert!(user.contains("no `$SOURCE_DIR` is lent:\n\nShip it.\n\n"), "{user}");
}

// A `Note` seam stands where the SDK's rendering of the input would be; with
// no stems, nothing is appended.
#[tokio::test]
async fn prepared_turn() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "ignored"), Seam::Note("THE NOTE".into()))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("\n\nTHE NOTE\n\nThe claim rules"), "{user}");
    assert!(!user.contains("ignored"), "the note replaces the input rendering");
}

// The files are listed sorted, once each, `.` segments dropped, so every anchor
// the model answers is already root-relative; with no tree at the root there
// is nothing to lay out, so they are listed.
#[tokio::test]
async fn files_turn() {
    let model = Scripted::answering([VALID]);
    let seam = files(["guide/setup.md", "./guide/intro.md", "guide/intro.md", "api.md"]);

    ask(&model, &SourceInput::workspace("docs", "/lend/docs"), seam).await.expect("accepted");

    let request = &model.seen()[0];
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the root is lent");
    let user = &request.messages[0];
    assert!(
        user.contains(
            "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file you \
             can read. Mine these files beneath it and nothing else:\n\n\
             - `api.md`\n- `guide/intro.md`\n- `guide/setup.md`\n\n\
             Anchor every `path` relative to `$SOURCE_DIR`, within these files."
        ),
        "{user}"
    );
    assert!(!user.contains("/lend/docs"), "the lend carries the root, not the brief: {user}");
    model.assert_exhausted();
}

// Files that fit within INLINE_BYTES together ride the turn whole, numbered
// from 1, in a fence no run of backticks inside can close.
#[tokio::test]
async fn files_laid() {
    let model = Scripted::answering([VALID]);
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "api.md", "# API\n\n```ts\nexport const x = 1;\n```\n");
    write(tmp.path(), "guide/intro.md", "# Intro\nOne line, no trailing newline");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");

    ask(&model, &SourceInput::workspace("docs", root), files(["guide/intro.md", "api.md"]))
        .await
        .expect("accepted");

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "Mine these files beneath it and nothing else. Each is laid out here whole, every \
             line led by its number, so cite `#L<n>` from the numbers shown rather than reading \
             it again:\n\n\
             ### `api.md` (5 lines)\n\n````\n1|# API\n2|\n3|```ts\n4|export const x = 1;\n5|```\n````\n\n\
             ### `guide/intro.md` (2 lines)\n\n```\n1|# Intro\n2|One line, no trailing newline\n```\n\n\
             Anchor every `path` relative to `$SOURCE_DIR`, within these files."
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// Past INLINE_BYTES together, or with a file that is not text, the files are
// listed as they would be with no tree at all.
#[tokio::test]
async fn files_listed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.md", &"x".repeat(usize::try_from(emery_sdk::INLINE_BYTES).expect("fits")));
    write(tmp.path(), "b.md", "one byte over");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");

    let model = Scripted::answering([VALID]);
    ask(&model, &SourceInput::workspace("docs", root), files(["a.md", "b.md"]))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("nothing else:\n\n- `a.md`\n- `b.md`\n\n"), "{user}");
    assert!(!user.contains("laid out"), "{user}");

    let binary = tempfile::tempdir().expect("tempdir");
    std::fs::write(binary.path().join("blob.bin"), [0xff, 0xfe, 0x00]).expect("write");
    write(binary.path(), "a.md", "text");
    let root = binary.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    ask(&model, &SourceInput::workspace("docs", root), files(["a.md", "blob.bin"]))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("nothing else:\n\n- `a.md`\n- `blob.bin`\n\n"), "{user}");
}

// One stem appends the stem rule after the text; several are offered as a
// set, sorted and deduplicated.
#[tokio::test]
async fn note_stems() {
    let model = Scripted::answering([VALID, VALID]);
    let one = Seam::Note(Note {
        text: "Surface `POST /orders` — entry `src/routes.ts` — stem `orders`.".to_string(),
        stems: vec!["password-reset".to_string()],
    });
    let several = Seam::Note(Note {
        text: "Mine the tree.".to_string(),
        stems: vec!["reset".to_string(), "password-reset".to_string(), "reset".to_string()],
    });

    ask(&model, &SourceInput::workspace("code", "/lend/code"), one).await.expect("accepted");
    ask(&model, &SourceInput::workspace("code", "/lend/code"), several).await.expect("accepted");

    let seen = model.seen();
    assert!(
        seen[0].messages[0].contains(
            "— stem `orders`.\n\nLead every `requirement` and `criterion` id with the stem \
             `password-reset` as its first dotted segment; an id under another stem is \
             refused.\n\nThe claim rules"
        ),
        "{}",
        seen[0].messages[0]
    );
    assert!(
        seen[1].messages[0].contains(
            "Mine the tree.\n\nLead every `requirement` and `criterion` id with one of the stems \
             `password-reset`, `reset` as its first dotted segment"
        ),
        "{}",
        seen[1].messages[0]
    );
    model.assert_exhausted();
}

// A stem outside the grammar is the adapter's own defect, refused before a
// turn is spent.
#[tokio::test]
async fn note_defects() {
    let model = Scripted::default();
    let stemmed = Seam::Note(Note {
        text: "Mine.".to_string(),
        stems: vec!["Orders".to_string()],
    });
    let error = ask(&model, &SourceInput::value("brief", "Ship it."), stemmed)
        .await
        .expect_err("a stem outside the grammar");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("stem `Orders` is not lowercase kebab-case"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// A `requirement` or `criterion` id under another stem is a finding; other
// kinds and the stemless are not held.
#[tokio::test]
async fn stem_findings() {
    let model = Scripted::answering([
        r#"{"claims":[
            {"kind":"requirement","id":"orders.create","statement":"Creates."},
            {"kind":"criterion","id":"order.created","criterion":"201."},
            {"kind":"example","id":"users.create","replay-digest":"sha256:00"},
            {"kind":"requirement","statement":"Unnamed."}
        ]}"#,
        r#"{"claims":[{"kind":"requirement","id":"orders.create","statement":"Creates."}]}"#,
    ]);
    let seam = Seam::Note(Note {
        text: "Mine.".to_string(),
        stems: vec!["orders".to_string(), "invoices".to_string()],
    });

    let accepted =
        ask(&model, &SourceInput::value("brief", "Ship it."), seam).await.expect("corrected");
    assert_eq!(accepted.claims.len(), 1);

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray stem is refused");
    assert!(
        correction.contains(
            "claim 1: id `order.created` leads with `order`, not a stem of this seam \
             (`invoices`, `orders`)"
        ),
        "{correction}"
    );
    let findings = correction.split("## Findings").nth(1).expect("a findings section");
    assert!(!findings.contains("claim 2:"), "an example is not held: {findings}");
    assert!(findings.contains("claims require an id"), "the gate's findings ride too");
    model.assert_exhausted();
}

// A `path` is held to the lend: the file exists, is within the seam's files,
// and holds the cited lines.
#[tokio::test]
async fn path_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "api.md", "one\ntwo\nthree\n");
    write(tmp.path(), "guide/intro.md", "intro");
    write(tmp.path(), "notes/todo.md", "todo");
    std::fs::create_dir_all(tmp.path().join("dir.md")).expect("a directory named like a file");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"claims":[
            {"kind":"requirement","id":"a.one","statement":"x","path":"api.md#L1-L3"},
            {"kind":"requirement","id":"a.two","statement":"x","path":"./api.md#L4"},
            {"kind":"requirement","id":"a.three","statement":"x","path":"notes/todo.md"},
            {"kind":"requirement","id":"a.four","statement":"x","path":"guide/missing.md"},
            {"kind":"requirement","id":"a.five","statement":"x","path":"dir.md"},
            {"kind":"requirement","id":"a.six","statement":"x","path":"guide/intro.md#L1"},
            {"kind":"requirement","id":"a.seven","statement":"x","path":"../x.md"}
        ]}"#,
        r#"{"claims":[{"kind":"requirement","id":"a.one","statement":"x","path":"api.md#L3"}]}"#,
    ]);
    let seam = files(["api.md", "guide/intro.md", "guide/missing.md", "dir.md"]);

    let accepted =
        ask(&model, &SourceInput::workspace("docs", root), seam).await.expect("corrected");
    assert_eq!(accepted.claims[0].path.as_deref(), Some("api.md#L3"));

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the anchors are refused");
    for finding in [
        "claim 1: path `./api.md#L4` cites line 4, but `api.md` has 3 lines",
        "claim 2: path `notes/todo.md` is outside this seam; anchor within the files it mines",
        "claim 3: path `guide/missing.md` names no regular file under the lent tree",
        "claim 4: path `dir.md` names no regular file under the lent tree",
        "claim 6: path `../x.md` escapes the source root",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(!correction.contains("claim 0:"), "a sound anchor is no finding: {correction}");
    assert!(!correction.contains("claim 5:"), "the last line is within the file: {correction}");
    assert_eq!(
        correction.matches("claim 6:").count(),
        1,
        "a malformed anchor is the gate's finding alone: {correction}"
    );
    model.assert_exhausted();
}

// A whole-tree seam holds a `path` to the tree alone, as a `Note` does. A
// `path` on an inline value cites nothing.
#[tokio::test]
async fn path_whole() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "api.md", "one\n");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"claims":[
            {"kind":"requirement","id":"a.one","statement":"x","path":"api.md#L1"},
            {"kind":"requirement","id":"a.two","statement":"x","path":"nope.md"}
        ]}"#,
        r#"{"claims":[{"kind":"requirement","id":"a.one","statement":"x","path":"api.md"}]}"#,
    ]);
    ask(&model, &SourceInput::workspace("docs", root), Seam::Whole).await.expect("corrected");
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the ghost is refused");
    assert!(correction.contains("claim 1: path `nope.md` names no regular file"), "{correction}");
    assert!(!correction.contains("claim 0:"), "{correction}");

    let model = Scripted::answering([
        r#"{"claims":[{"kind":"requirement","id":"a.one","statement":"x","path":"brief.md"}]}"#,
        VALID,
    ]);
    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole).await.expect("corrected");
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("a path on a value");
    assert!(
        correction.contains(
            "claim 0: path `brief.md` cites a file, but the source is an inline value with no \
             tree lent"
        ),
        "{correction}"
    );
}

fn write(root: &std::path::Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, body).expect("write");
}

// Answered from the adapter's corpus, then from the SDK's runtime references,
// which the adapter never lists. No system document is offered, since the
// claim rules ride this turn's system and the survey prompt steered a turn
// already spent. A read still answers each.
#[tokio::test]
async fn doc_refs() {
    let model = Scripted::answering([VALID]).calling(
        0,
        [
            ToolCall {
                id: "1".to_string(),
                name: "list_docs".to_string(),
                arguments: "{}".to_string(),
            },
            ToolCall {
                id: "2".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"references/greeting.md"}"#.to_string(),
            },
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"claims.md"}"#.to_string(),
            },
            ToolCall {
                id: "4".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"survey.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/greeting.md","reconciliation.md"]}"#),
        "`list_docs` lists no system document"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"Greet warmly.","path":"references/greeting.md"}"#)
    );
    let runtime: serde_json::Value =
        serde_json::from_str(exchanges[2].outcome.as_deref().expect("the runtime table answers"))
            .expect("a JSON answer");
    assert_eq!(runtime["path"], "claims.md");
    assert_eq!(
        runtime["body"],
        emery_sdk::body(emery_sdk::RUNTIME, "claims.md").expect("embedded")
    );
    assert_eq!(
        exchanges[3].outcome.as_deref(),
        Ok(r#"{"body":"SURVEY","path":"survey.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(exchanges[4].tool, "check");
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
}

// The engine never sees the claim it would otherwise refuse.
#[tokio::test]
async fn gate_findings() {
    let model = Scripted::answering([r#"{"claims":[{"kind":"requirement"}]}"#, VALID]);

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect("the second candidate passes the gate");
    assert_eq!(accepted.claims.len(), 2);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one rejection, one acceptance");
    assert_eq!(exchanges[0].tool, "check");
    let correction = exchanges[0].outcome.as_ref().expect_err("the first candidate is rejected");
    assert!(correction.contains("## Previous answer (rejected)"), "{correction}");
    assert!(correction.contains("## Findings"), "{correction}");
    assert!(correction.contains("`requirement` claims require an id"), "{correction}");
    assert!(correction.contains("missing extra `statement`"), "{correction}");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    assert_eq!(model.requests().len(), 2, "one scripted answer per attempt");
}

// The last findings surface, so the host's own error is never the adapter's answer.
#[tokio::test]
async fn rounds_exhausted() {
    let model = Scripted::answering([
        r#"{"claims":[{"kind":"criterion","id":"Not.Valid","criterion":"x"}]}"#,
    ]);

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect_err("the only candidate fails the gate");
    let Error::BadRequest { code, description } = error else {
        panic!("spent rounds are a bad request: {error}");
    };
    assert_eq!(code, "bad_request");
    assert!(description.contains("budget exhausted"), "{description}");
    assert!(description.contains("id `Not.Valid` does not match"), "{description}");
    assert_eq!(model.exchanges().len(), 1, "one check, rejected");
}

#[tokio::test]
async fn invalid_request() {
    let model = Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]);

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect_err("the host refused");
    assert!(
        matches!(&error, Error::BadRequest { description, .. } if description == "invalid request: no such model"),
        "{error}"
    );
    assert!(model.exchanges().is_empty(), "nothing to check");
}

// The kind of source is the adapter's metadata, never the model's to state.
#[tokio::test]
async fn stray_kind() {
    let model = Scripted::answering([r#"{"kind":"intent","claims":[{"kind":"decision"}]}"#, VALID]);

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect("the second candidate is claims-only");
    assert_eq!(accepted.claims.len(), 2);

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray key is refused");
    assert!(
        correction.contains("schema") || correction.contains("unknown field"),
        "a stray kind key is a schema miss: {correction}"
    );
    model.assert_exhausted();
}
