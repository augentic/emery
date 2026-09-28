# CLI output shapes

Canonical JSON envelope shapes for the `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body. The v1 verb catalogue is archived at git tag `v1`.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a discriminant string: kebab-case for the four recovery codes (`specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`), snake_case for the Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The discriminant is grep-stable and forms part of the public contract; see [cli-contract.md](../standards/cli-contract.md#exit-codes) for the exit-code table.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns.
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise.
- All keys are `kebab-case`. Body shapes are pinned by the typed `*Output` DTOs in `emery-engine` (`Serialize`) and change only with the CLI's own versioning; the failure envelope is `emery-cli`'s.
- Stream roles: the semantic result body (text or JSON) is **stdout**; the failure envelope and live tracing are **stderr**. Tracing is one level for the whole run — `info` bare, one step up per `-v` and down per `-q`, a process `RUST_LOG` refining a bare run (see [cli-contract.md](../standards/cli-contract.md)).

## Text-mode style

Every body's render fn (its text mode, in `crates/cli/src/text.rs`) follows one convention so operators can scan any command's output the same way:

- **Result line first, lowercase, verb-first**: `committed revision 9f8e7d6c…`.
- **Detail lines are indented `label: value` pairs** with kebab-case labels: `  diff vs 1a2b3c4d: none (byte-stable)`.
- **Names in backticks**, paths bare.
- **No trailing periods** on result or detail lines.
- **`hint:` is recovery guidance** (what to fix); **`resume:` is the literal next command** (what to run). A line is one or the other, never both.
- **Every empty state prints a lowercase line** — silence is never the empty rendering.

One documented exception: `emery show` renders the Markdown projection alone in text mode — no result line — so its stdout redirects as the document itself (`emery show spec > spec.md`). Its revision id rides the projection's front matter and the JSON envelope.

## Shapes

The examples below are hand-curated illustrations of the happy path; the accept/reject variant set is exercised by the integration suites under `crates/*/tests/`.

### `emery specify`

The success body names the committed revision and its reviewable set:

```json
{
  "revision": "9f8e7d6c…",
  "diff": {
    "from": "1a2b3c4d…",
    "spec": {
      "preamble": true,
      "added": [{ "id": "REQ-003", "subject": "access.audit" }],
      "removed": [],
      "changed": [{ "id": "REQ-002", "subject": "session.timeout", "fields": ["body", "scenarios"] }]
    },
    "design": { "preamble": false, "added": [], "removed": [], "changed": ["domain-model"] },
    "plan": {
      "preamble": false,
      "added": [{ "id": "SLICE-002", "name": "access" }],
      "removed": [],
      "changed": [{ "id": "SLICE-001", "name": "authentication", "fields": ["requirements", "brief"] }]
    }
  }
}
```

`diff` is the re-mine diff against the outgoing current revision, computed by typed equality over the two revisions: each document's `preamble` flags whether its preamble changed; `spec` lists requirements matched by `id` as `{ id, subject }` entries (ids are positional — `REQ-001` onward in source order — so a requirement whose place moved reads as a change), each `changed` entry naming the fields that differ (`subject`, `status`, `covered`, `sources`, `body`, `losers`, `scenarios`); `design` lists sections by their kebab-case key; `plan` lists slices matched by `id` as `{ id, name }` entries (ids are positional too — `SLICE-001` onward by each slice's lowest requirement), each `changed` entry naming the fields that differ (`name`, `requirements`, `types`, `depends-on`, `brief`). It is absent on a first run; on a byte-stable re-run `from` equals `revision`, every `preamble` flag is false, and every list is empty; nothing is persisted for it. Text mode prints a one-line summary of those counts beneath the revision — `  diff vs 1a2b3c4d: spec +1 -0 ~1 preamble, design +0 -0 ~1, plan +1 -0 ~1` — so a run never spans more than two lines; the per-requirement and per-slice entries ride `--format json` alone.

`emery specify` with no source — and no project-root `emery.toml` to discover — fails with `error: "specify-source-required"` (exit 1); mixing `--config` with positional adapters or `--description`, or naming an absolute or project-escaping local path, fails with `error: "bad_request"` (exit 1). `--config` without a value explicitly selects the project-relative `emery.toml`. A GitHub URL source fails with `error: "bad_request"`. A model draft (grouping, spec, design, or slicing) that still fails its check once the backend's rounds are spent exits 1 with `error: "bad_request"` carrying the last correction and its findings; a model failure exits 4 with `error: "bad_gateway"`. The first source to fail ends the run — the sources still extracting are not waited for — and its failure is the envelope, as the adapter put it: a source refusing its input exits 1 with `error: "bad_request"` carrying the adapter's own description, an adapter failing upstream exits 4 with `error: "bad_gateway"`, and evidence the claim gate rejects exits 3 with `error: "server_error"` naming the findings.

### `emery show <spec|design|plan>`

The success body carries the revision id, the Markdown projection, and the typed revision it was rendered from; text mode is the projection alone (see the exception above).

```json
{
  "revision": "9f8e7d6c…",
  "body": "---\nemery: 3\nrevision: 9f8e7d6c…\n---\n\n# Specification\n…",
  "document": {
    "emery": 3,
    "preamble": ["…"],
    "requirements": [
      {
        "id": "REQ-001",
        "subject": "session.timeout",
        "status": "divergence",
        "covered": true,
        "sources": [{ "source": "intent", "claim": "session.timeout" }, { "source": "code", "claim": "session-expiry" }],
        "body": ["Sessions must expire after 30 minutes of inactivity."],
        "losers": [{ "sources": ["code"], "kind": "behaviour", "claim": "session-expiry", "statement": "…" }],
        "scenarios": [{ "name": "Session expires", "given": [], "when": "…", "then": "…", "and": [] }]
      }
    ]
  }
}
```

`document` is the revision document exactly as stored: for `spec`, `emery` (the grammar stamp), `preamble`, and `requirements`; for `design`, `emery`, `preamble`, and `sections` (each a `kind` and its `blocks`, `{ "text": "<paragraph>" }` or `{ "type": { "key", "signature" } }`); for `plan`, `emery`, `preamble`, and `slices` (each an `id`, `name`, `requirements`, `types`, `depends-on`, and `brief`). The revision's serde shape is pinned by `emery-engine`'s `revision` types; its canonical bytes hash to `revision`.

`plan.md` renders each slice as a `## Slice: <name>` block: its `ID:` and `Requirements:` lines, a `Types:` line where the slice owns a design type, a `Depends on:` line where it is built after another slice, then the brief's paragraphs.

Before any revision is committed the verb fails with `error: "spec-not-generated"` (exit 2); a current revision id naming missing documents fails with `error: "server_error"` (exit 3); a stored revision under an older grammar fails with `error: "spec-outdated"` (exit 1).

### `emery completions <shell>`

Emits the shell completion script on stdout (no JSON envelope; the output is the script itself).

## Failure envelope

Every failing verb emits the same flat envelope on stderr:

```json
{
  "error": "specify-source-required",
  "message": "no sources",
  "exit-code": 1,
  "hint": "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root"
}
```

An optional `hint` key carries a static recovery hint when the error defines one; the `message` is transport-neutral (it names the rule, path, or adapter), and flag-vocabulary recovery text lives in the hint. Text mode prints the same envelope as `error[specify-source-required]: no sources` followed by a `hint:` line when one is defined; the discriminant is grep-stable in both formats, so a `message` never repeats it.
