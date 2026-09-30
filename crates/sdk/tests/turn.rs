//! Every call here mines one seam, so each is one turn and its outcome passes
//! through unchanged; the fan-out and join over several are `extract.rs`'s.

use emery_sdk::{Context, Doc, Error, Evidence, Seam, SourceInput};
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
];

const VALID: &str = r#"{"claims":[
    {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
    {"kind":"decision"}
]}"#;

fn stemmed<const N: usize>(text: &str, stems: [&str; N]) -> Seam {
    Seam {
        stems: stems.into_iter().map(str::to_string).collect(),
        ..Seam::note(text)
    }
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

    let accepted = ask(&model, &SourceInput::workspace("docs", "/lend/docs"), Seam::whole())
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
            "describes.\n\n",
            "Anchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is reachable; ",
            "extract mines only this source.\n\n",
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
        emery_sdk::extract(&ctx, &[], &[Seam::whole()]).await.expect_err("no prompt to ask with");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`extract.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "nothing was asked");
}

#[tokio::test]
async fn inline_value() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole()).await.expect("accepted");
    let request = &model.seen()[0];
    assert!(request.workspace.is_none(), "no lend for an inline value");
    let user = &request.messages[0];
    assert!(user.contains("the source `brief` bound to"), "{user}");
    assert!(user.contains("no `$SOURCE_DIR` is lent:\n\nShip it.\n\n"), "{user}");
}

// A seam's text leads the turn and the input follows it; with no stems,
// nothing is appended between the input and the closing rule.
#[tokio::test]
async fn prepared_turn() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::note("THE NOTE"))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "bound to adapter `source:probe`.\n\nTHE NOTE\n\nThe bound seam is this inline \
             value; no `$SOURCE_DIR` is lent:\n\nShip it.\n\nNothing else is reachable; extract \
             mines only this source.\n\nThe claim rules"
        ),
        "{user}"
    );
}

// The files are listed in the seam's order, once each, `.` segments dropped,
// so every anchor the model answers is already root-relative; with no tree at
// the root there is nothing to lay out, so they are listed.
#[tokio::test]
async fn files_turn() {
    let model = Scripted::answering([VALID]);
    let seam = Seam::files(["guide/setup.md", "./guide/intro.md", "guide/intro.md", "api.md"]);

    ask(&model, &SourceInput::workspace("docs", "/lend/docs"), seam).await.expect("accepted");

    let request = &model.seen()[0];
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the root is lent");
    let user = &request.messages[0];
    assert!(
        user.contains(
            "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file you \
             can read. Mine these files beneath it and nothing else:\n\n\
             - `guide/setup.md`\n- `guide/intro.md`\n- `api.md`\n\n\
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

    ask(&model, &SourceInput::workspace("docs", root), Seam::files(["api.md", "guide/intro.md"]))
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

// The leading files are laid for as long as they fit within INLINE_BYTES
// together; the first past it, or one that is not text, and every file after
// it are listed. A first file that does not fit leaves every file listed.
#[tokio::test]
async fn files_listed() {
    let budget = usize::try_from(emery_sdk::INLINE_BYTES).expect("fits");
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.md", &"x".repeat(budget));
    write(tmp.path(), "b.md", "one byte over");
    write(tmp.path(), "c.md", "after");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");

    let model = Scripted::answering([VALID]);
    ask(&model, &SourceInput::workspace("docs", root), Seam::files(["a.md", "b.md", "c.md"]))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "Mine these files beneath it and nothing else. The first is laid out here whole, \
             every line led by its number, so cite `#L<n>` from the numbers shown rather than \
             reading it again; the rest are listed after it, to read from `$SOURCE_DIR` as the \
             seam reaches them:\n\n### `a.md` (1 line)"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "```\n\nThe rest of this seam's files:\n\n- `b.md`\n- `c.md`\n\nAnchor every"
        ),
        "{user}"
    );

    let model = Scripted::answering([VALID]);
    ask(&model, &SourceInput::workspace("docs", root), Seam::files(["b.md", "a.md", "c.md"]))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("The first is laid out"), "{user}");
    assert!(user.contains("### `b.md` (1 line)"), "{user}");
    assert!(user.contains("The rest of this seam's files:\n\n- `a.md`\n- `c.md`\n\n"), "{user}");

    let binary = tempfile::tempdir().expect("tempdir");
    std::fs::write(binary.path().join("blob.bin"), [0xff, 0xfe, 0x00]).expect("write");
    write(binary.path(), "a.md", "text");
    let root = binary.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    ask(&model, &SourceInput::workspace("docs", root), Seam::files(["blob.bin", "a.md"]))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("nothing else:\n\n- `blob.bin`\n- `a.md`\n\n"), "{user}");
    assert!(!user.contains("laid out"), "{user}");
}

// The turn reads text, lend, stems, anchor rule: one stem appends the stem
// rule after the lend; several are offered as a set, sorted and deduplicated.
#[tokio::test]
async fn note_stems() {
    let model = Scripted::answering([VALID, VALID]);
    let one = stemmed(
        "Surface `POST /orders` — entry `src/routes.ts` — stem `orders`.",
        ["password-reset"],
    );
    let several = stemmed("Mine the tree.", ["reset", "password-reset", "reset"]);

    ask(&model, &SourceInput::workspace("code", "/lend/code"), one).await.expect("accepted");
    ask(&model, &SourceInput::workspace("code", "/lend/code"), several).await.expect("accepted");

    let seen = model.seen();
    assert!(
        seen[0].messages[0].contains(
            "— stem `orders`.\n\n`$SOURCE_DIR` is the bound source tree, lent read-only: the \
             root of every file you can read, and the root every `path` is relative to. Walk it \
             as the prompt describes.\n\nLead every `requirement` and `criterion` id with the \
             stem `password-reset` as its first dotted segment; an id under another stem is \
             refused.\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
             reachable; extract mines only this source.\n\nThe claim rules"
        ),
        "{}",
        seen[0].messages[0]
    );
    assert!(
        seen[1].messages[0].contains(
            "as the prompt describes.\n\nLead every `requirement` and `criterion` id with one of \
             the stems `password-reset`, `reset` as its first dotted segment"
        ),
        "{}",
        seen[1].messages[0]
    );
    model.assert_exhausted();
}

// Files and stems together: the stem rule sits between the laid files and the
// anchor rule, so the model reads what to mine before what to call it.
#[tokio::test]
async fn files_stems() {
    let model = Scripted::answering([VALID]);
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/routes.ts", "export const x = 1;\n");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let seam = Seam {
        text: "Surface `POST /password-reset` — entry `src/routes.ts`.".to_string(),
        files: vec!["src/routes.ts".to_string()],
        stems: vec!["password-reset".to_string()],
        anchors: Vec::new(),
    };

    ask(&model, &SourceInput::workspace("code", root), seam).await.expect("accepted");

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "bound to adapter `source:probe`.\n\nSurface `POST /password-reset` — entry \
             `src/routes.ts`.\n\n`$SOURCE_DIR` is the bound source tree, lent read-only: the \
             root every `path` is relative to. Mine these files beneath it and nothing else. \
             Each is laid out here whole, every line led by its number, so cite `#L<n>` from the \
             numbers shown rather than reading it again:\n\n### `src/routes.ts` (1 \
             line)\n\n```\n1|export const x = 1;\n```\n\nLead every `requirement` and \
             `criterion` id with the stem `password-reset` as its first dotted segment; an id \
             under another stem is refused.\n\nAnchor every `path` relative to `$SOURCE_DIR`, \
             within these files. Nothing outside it is reachable; extract mines only this \
             source.\n\nThe claim rules"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// A stem outside the grammar is the adapter's own defect, refused before a
// turn is spent.
#[tokio::test]
async fn note_defects() {
    let model = Scripted::default();
    let error = ask(&model, &SourceInput::value("brief", "Ship it."), stemmed("Mine.", ["Orders"]))
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
    let seam = stemmed("Mine.", ["orders", "invoices"]);

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
    let seam = Seam::files(["api.md", "guide/intro.md", "guide/missing.md", "dir.md"]);

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

// A seam naming no file holds a `path` to the tree alone. A `path` on an
// inline value cites nothing.
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
    ask(&model, &SourceInput::workspace("docs", root), Seam::whole()).await.expect("corrected");
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the ghost is refused");
    assert!(correction.contains("claim 1: path `nope.md` names no regular file"), "{correction}");
    assert!(!correction.contains("claim 0:"), "{correction}");

    let model = Scripted::answering([
        r#"{"claims":[{"kind":"requirement","id":"a.one","statement":"x","path":"brief.md"}]}"#,
        VALID,
    ]);
    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole()).await.expect("corrected");
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

// A `requirement` is held to the seam's anchors: one sharing a line with a
// listed span is accepted, one at any other line — or citing a whole file
// against listed lines — is a finding naming the anchors in its file, the
// nearest sixteen of many; one in a file the seam anchors nothing in says so;
// a whole-file anchor listed covers its file; a `criterion` and the rest are
// not held. The brief says so.
#[tokio::test]
async fn anchor_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "src/orders.ts", "one\ntwo\nthree\nfour\nfive\nsix\n");
    write(tmp.path(), "src/config.ts", "export const LIMIT = 3;\n");
    write(tmp.path(), "src/lib/util.ts", "one\ntwo\n");
    write(tmp.path(), "src/long.ts", &"line\n".repeat(60));
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"claims":[
            {"kind":"requirement","id":"orders.create","statement":"x","path":"src/orders.ts#L3-L5"},
            {"kind":"requirement","id":"orders.wire","statement":"x","path":"src/orders.ts#L1"},
            {"kind":"requirement","id":"orders.whole","statement":"x","path":"src/orders.ts"},
            {"kind":"criterion","id":"orders.limit","criterion":"3","path":"src/orders.ts#L6"},
            {"kind":"requirement","id":"orders.config","statement":"x","path":"./src/config.ts"},
            {"kind":"requirement","id":"orders.past","statement":"x","path":"src/orders.ts#L9"},
            {"kind":"requirement","id":"orders.util","statement":"x","path":"src/lib/util.ts#L2"},
            {"kind":"requirement","id":"orders.long","statement":"x","path":"src/long.ts#L41"}
        ]}"#,
        r#"{"claims":[{"kind":"requirement","id":"orders.create","statement":"x","path":"src/orders.ts#L3"}]}"#,
    ]);
    let mut anchors = vec![
        "src/orders.ts#L5-L6".to_owned(),
        "src/orders.ts#L2-L3".to_owned(),
        "./src/config.ts".to_owned(),
    ];
    anchors.extend((1..=60).step_by(3).map(|line| format!("src/long.ts#L{line}")));
    let seam = Seam {
        anchors,
        ..Seam::files(["src/orders.ts", "src/config.ts", "src/lib/util.ts", "src/long.ts"])
    };

    let accepted =
        ask(&model, &SourceInput::workspace("code", root), seam).await.expect("corrected");
    assert_eq!(accepted.claims[0].path.as_deref(), Some("src/orders.ts#L3"));

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "A `requirement` anchors at one of the lines the text above lists — where its \
             behaviour starts; one anchored at any other line is refused.\n\nAnchor every `path`"
        ),
        "{user}"
    );
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray anchors are refused");
    for finding in [
        "claim 1: path `src/orders.ts#L1` is at none of the lines this seam names for a \
         `requirement`; in `src/orders.ts` it names L2-L3, L5-L6; anchor it at the one where its \
         behaviour starts, or leave it out",
        "claim 2: path `src/orders.ts` is at none of the lines this seam names for a \
         `requirement`; in `src/orders.ts` it names L2-L3, L5-L6;",
        "claim 5: path `src/orders.ts#L9` cites line 9, but `src/orders.ts` has 6 lines",
        "claim 6: path `src/lib/util.ts#L2` is at none of the lines this seam names for a \
         `requirement`, and it names none in `src/lib/util.ts`; anchor it in a file where its \
         behaviour starts, or leave it out",
        "claim 7: path `src/long.ts#L41` is at none of the lines this seam names for a \
         `requirement`; in `src/long.ts` it names L13, L16, L19, L22, L25, L28, L31, L34, L37, \
         L40, L43, L46, L49, L52, L55, L58; anchor it",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(!correction.contains("claim 0:"), "an overlapping span is no finding: {correction}");
    assert!(!correction.contains("claim 3:"), "a criterion is not held: {correction}");
    assert!(!correction.contains("claim 4:"), "a whole-file anchor covers its file: {correction}");
    assert!(!correction.contains("L10, L13"), "the farthest anchors are left out: {correction}");
    assert_eq!(
        correction.matches("claim 5:").count(),
        1,
        "a line past the file is that finding alone: {correction}"
    );
    model.assert_exhausted();
}

// An anchor outside the grammar, or any anchor over an inline value, is the
// adapter's own defect, refused before a turn is spent.
#[tokio::test]
async fn anchor_defects() {
    let model = Scripted::default();
    let seam = Seam {
        anchors: vec!["src/orders.ts#4".to_owned()],
        ..Seam::files(["src/orders.ts"])
    };
    let error = ask(&model, &SourceInput::workspace("code", "/lend/code"), seam)
        .await
        .expect_err("an anchor outside the grammar");
    assert_eq!(error.code(), "server_error");
    assert!(
        error.description().contains(
            "a seam's anchor `src/orders.ts#4` is not `<path>`, `<path>#L<n>`, or \
             `<path>#L<n>-L<n>`"
        ),
        "{error}"
    );

    let seam = Seam {
        anchors: vec!["brief.md#L1".to_owned()],
        ..Seam::whole()
    };
    let error = ask(&model, &SourceInput::value("brief", "Ship it."), seam)
        .await
        .expect_err("anchors over a value");
    assert_eq!(error.code(), "server_error");
    assert!(
        error.description().contains("a seam names anchors, but the source is an inline value"),
        "{error}"
    );
    assert!(model.seen().is_empty(), "no turn was spent");
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
// prompt and the claim rules ride this turn's system already. A read still
// answers each.
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
                arguments: r#"{"path":"extract.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole()).await.expect("accepted");
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
        Ok(r#"{"body":"SYSTEM","path":"extract.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(exchanges[4].tool, "check");
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
}

// The engine never sees the claim it would otherwise refuse.
#[tokio::test]
async fn gate_findings() {
    let model = Scripted::answering([r#"{"claims":[{"kind":"requirement"}]}"#, VALID]);

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole())
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

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole())
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

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole())
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

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::whole())
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
