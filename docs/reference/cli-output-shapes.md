# CLI output shapes

Canonical JSON envelope shapes for the `emery *` commands that skills shell out to. Skills should **link** to the relevant section here rather than embedding multi-line JSON examples in their `SKILL.md` body. The v1 verb catalogue is archived at git tag `v1`.

## Conventions

- `--format json` responses are a **flat body**: every successful body is a single JSON object carrying the command-specific fields **at the top level** — there is no `ok` discriminant, no `data` wrapper, and no top-level envelope-version stamp.
- Failures keep the same flat shape with three extra top-level keys:
  - `error` — a discriminant string: kebab-case for the six recovery codes (`specify-source-required`, `build-target-required`, `adapter-reference`, `unsupported-version`, `spec-not-generated`, `spec-outdated`), snake_case for the Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`). The discriminant is grep-stable and forms part of the public contract.
  - `message` — humanised one-liner suitable for direct rendering.
  - `exit-code` — the integer the binary returns (see [Exit codes](#exit-codes)).
- Paths are emitted as plain strings relative to the repo root unless the field name says otherwise.
- All keys are `kebab-case`. Body shapes are pinned by the typed `*Output` DTOs in `emery-engine` (`Serialize`) and change only with the CLI's own versioning; the failure envelope is `emery-cli`'s.
- Stream roles: the semantic result body (text or JSON) is **stdout**; the failure envelope and live tracing are **stderr**. Tracing is one level for the whole run — `info` bare, one step up per `-v` and down per `-q`, a process `RUST_LOG` refining a bare run.

## Exit codes

`omnia_sdk::Error::exit_code` maps the four `Error` variants 1:1; there is no exit table in this repository's code.

| Code | Class          | When                                                                                                                                                                 |
| ---- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | success        | Command succeeded.                                                                                                                                                   |
| 1    | `bad_request`  | Operator or input refusal. The `error` field is `specify-source-required`, `build-target-required`, `adapter-reference`, `unsupported-version`, `spec-outdated`, the loader's `refused`, or `bad_request`. |
| 2    | `not_found`    | Missing resource. The `error` field is `spec-not-generated` or `not_found`.                                                                                          |
| 3    | `server_error` | Evidence the claim gate rejects, a build report the report gate rejects, or an unclassified failure: I/O, storage, conversions. The `error` field is `server_error` or the loader's `internal`. |
| 4    | `bad_gateway`  | Upstream model, adapter, or component-acquisition failure. The `error` field is `bad_gateway` or the loader's `unavailable`.                                          |
| 64   | usage          | Clap usage error (unknown verb or flag, missing argument), rendered by clap on stderr with no envelope. `EX_USAGE`, so exit 2 always means a `not_found` envelope.    |

Skills branch on the exit code first and on the five kebab-case recovery discriminants second. On `unsupported-version` (exit `1`), tell the operator to update the installed binary through its install channel; a skill that sees exit `64` has built a bad argv.

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

The success body names the committed revision, the shape of its plan, and its reviewable set:

```json
{
  "revision": "9f8e7d6c…",
  "waves": [["SLICE-001", "SLICE-003"], ["SLICE-002"]],
  "diff": {
    "from": "1a2b3c4d…",
    "spec": {
      "preamble": true,
      "added": [{ "id": "REQ-003", "subject": "access.audit" }],
      "removed": [],
      "changed": [
        { "id": "REQ-002", "subject": "session.timeout", "fields": ["body", "scenarios"] },
        { "id": "REQ-004", "subject": "orders.create", "was": "REQ-003", "fields": ["id"] }
      ]
    },
    "design": { "preamble": false, "added": [], "removed": [], "changed": ["domain-model"] },
    "plan": {
      "preamble": false,
      "added": [{ "id": "SLICE-002", "name": "access" }],
      "removed": [],
      "changed": [{ "id": "SLICE-001", "name": "authentication", "fields": ["requirements", "brief"] }]
    }
  },
  "repositories": [
    { "source": "upstream-api", "repository": "https://github.com/acme/api.git", "revision": "v2.3.0", "commit": "4e5f6a7b…" }
  ]
}
```

`waves` is the committed plan's slices grouped into the sets ready to build at once: the first wave every slice that depends on nothing, each wave after it every slice whose `depends-on` names only slices in the waves before, each wave in id order. Their count is the plan's longest dependency chain, the widest of them how many slices could build at once, and `emery build` builds them flattened in that order. Text mode prints them as one line beneath the revision — `  plan: 3 slices in 2 waves, widest 2`.

`repositories` lists every source read from a repository, in declaration order: the source's name, the URL as `emery.toml` spelled it, the `revision` asked for, and the commit it resolved to on this run. It is omitted when no source names a repository. Text mode prints one line per entry beneath the plan line — `  upstream-api read from https://github.com/acme/api.git at v2.3.0: 4e5f6a7b…`.

`diff` is the re-mine diff against the outgoing current revision, computed by typed equality over the two revisions: each document's `preamble` flags whether its preamble changed; `spec` lists requirements as `{ id, subject }` entries, matched first by where they anchor — the same stem and a cited `path` in common (same source and file, line ranges that meet), one to one, the pair sharing the most anchors first — and then by `id` (ids are positional — `REQ-001` onward in source order — so a requirement no anchor matches whose place moved reads as a removal and an addition), each `changed` entry naming the fields that differ (`id`, `subject`, `status`, `covered`, `sources`, `body`, `losers`, `scenarios`) and, when the match crossed ids, carrying `was`, the id the requirement held in the outgoing revision; `design` lists sections by their kebab-case key; `plan` lists slices matched by `id` as `{ id, name }` entries (ids are positional too — `SLICE-001` onward by each slice's lowest requirement), each `changed` entry naming the fields that differ (`name`, `requirements`, `types`, `depends-on`, `brief`). It is absent on a first run; on a byte-stable re-run `from` equals `revision`, every `preamble` flag is false, and every list is empty; nothing is persisted for it. Text mode prints a one-line summary of those counts beneath the plan line — `  diff vs 1a2b3c4d: spec +1 -0 ~1 preamble, design +0 -0 ~1, plan +1 -0 ~1` — so a run never spans more than three lines; the per-requirement and per-slice entries ride `--format json` alone.

`emery specify` with no source — and no project-root `emery.toml` to discover — fails with `error: "specify-source-required"` (exit 1); mixing `--config` with positional adapters or `--description`, or naming an absolute or escaping source `path` (one above the project), fails with `error: "bad_request"` (exit 1); an adapter that is not an exact package reference `namespace:name@version` — a bare name, a path, a URL, a reference without a version — fails with `error: "adapter-reference"` (exit 1); a release the store lacks under a namespace the binary routes nowhere fails with the loader's `error: "refused"` (exit 1), and one its registry cannot supply with `error: "unavailable"` (exit 4). `--config` without a value explicitly selects the project-relative `emery.toml`. A `[[source]]` with a `repository` and no `revision`, a `revision` and no `repository`, a `repository` beside a `description`, or a `path` climbing out of the repository fails with `error: "bad_request"` (exit 1); a `revision` the repository lacks, or a `repository` URL that names none, fails with `error: "revision-not-found"` (exit 2) before any source is asked, and a remote that refuses or cannot be reached with `error: "bad_gateway"` (exit 4). A model draft (grouping, spec, design, or slicing) that still fails its check once the backend's rounds are spent exits 1 with `error: "bad_request"` carrying the last correction and its findings; a model failure exits 4 with `error: "bad_gateway"`. The first source to fail ends the run — the sources still extracting are not waited for — and its failure is the envelope, as the adapter put it: a source refusing its input exits 1 with `error: "bad_request"` carrying the adapter's own description, an adapter failing upstream exits 4 with `error: "bad_gateway"`, and evidence the claim gate rejects exits 3 with `error: "server_error"` naming the findings.

### `emery build`

The success body names the revision whose plan was built, the plan's waves, the base and the repository the build landed in, the `Initialise` commit the adapter's layout was sealed as when this run sealed one, the slices resumed from the label or the integration working copy, every slice this run merged with its wave and merge commit, each wave verified with the slices merged in it and the head it was verified at, what each wave's verification sealed and the waves it repaired, and the label on the integrated head:

```json
{
  "revision": "9f8e7d6c…",
  "waves": [["SLICE-001", "SLICE-003"], ["SLICE-002"]],
  "base": "1a2b3c4d…",
  "repository": "./.emery/vcs/repos/shop",
  "initialised": { "commit": "2b3c4d5e…", "paths": [".gitignore", "Cargo.toml", "src/lib.rs"] },
  "resumed": ["SLICE-001"],
  "slices": [
    { "id": "SLICE-003", "name": "catalogue", "wave": 1, "covered": ["REQ-005"], "uncovered": [], "written": ["src/catalogue.rs"], "commit": "5e6f7a8b…" },
    { "id": "SLICE-002", "name": "orders", "wave": 2, "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["src/lib.rs", "src/orders.rs", "src/orders/create.rs"], "resolved": ["src/lib.rs"], "commit": "9c0d1e2f…" }
  ],
  "verified": [
    { "wave": 1, "slices": ["SLICE-001", "SLICE-003"], "head": "5e6f7a8b…" },
    { "wave": 2, "slices": ["SLICE-002"], "head": "9c0d1e2f…" }
  ],
  "sealed": [{ "wave": 2, "commit": "7a8b9c0d…", "paths": ["Cargo.lock", "src/orders/create.rs"] }],
  "repaired": [2],
  "head": "9c0d1e2f…",
  "label": "emery/9f8e7d6c…"
}
```

`waves` is the plan's shape as `emery specify` reported it when the revision was committed. `base` is the commit the build started from — the project checkout's head under `[target] repository = "."`, the default branch of a `[target] repository` URL as its origin holds it, or the `Initialise` root commit of the repository the engine created when the build named none. `repository` is where the build landed and the label stands: `.` for the project checkout, else `./.emery/vcs/repos/<name>`, `<name>` the `[target] name` or the adapter's package name. `initialised` is the `Initialise` commit this run sealed over the base with the files the adapter's `init` scaffolded, `paths` those files relative to the tree root and sorted; omitted for a build resumed from the label or from staged slices, for a repository the engine created, whose `base` is that commit, and for a tree the layout added nothing to. `resumed` lists the slices not built again, in id order: those the label `emery/<revision>` already held over the base, and those the integration working copy `.emery/vcs/integration` held merged over the labelled head from a run that ended before their wave was verified, which this run verified in their wave instead; it is omitted when none were. `slices` is every slice this run built, in merge order: each carries `wave`, the wave it merged in from one; the requirement ids the target adapter reported implemented (`covered`) and those of the slice it left out (`uncovered`), both in id order; `written`, the paths its build changed in its working copy — written, edited, or removed — relative to the copy's root and sorted, read from the copy rather than the report; `resolved`, the paths its merge conflicted at under the adapter's merge rules and the model resolved before it was merged again, sorted, omitted when none; and `commit`, the merge commit that brought what it changed into the integrated head, `null` when it changed nothing. `verified` is every wave this run verified and labelled, in wave order: `wave`, from one; `slices`, the ids merged in it, those this run built and those an earlier run staged in the integration working copy, in id order; and `head`, the integrated commit it was verified at; omitted when every slice was resumed from the label. `sealed` is what each wave's verification left in the integrated tree, sealed as its `Wave <k> verified` commit before the head was labelled: the wave, the commit, and the paths changed, sorted, omitted when no wave left anything. `repaired` lists the waves whose seal carries a path the adapter's `ours` merge rules do not cover — a lockfile a check regenerated under one is no repair — omitted when none. `head` is the integrated commit and `label` the branch `emery/<revision>` set on it; nothing is pushed. Text mode prints the revision, the plan line — `  plan: 3 slices in 2 waves, widest 2` — the base — `  base 1a2b3c4d…` — `  repository ./.emery/vcs/repos/shop` when the build landed outside the project checkout, `  initialised 2b3c4d5e, 3 files` when the run sealed the layout, `  resumed: SLICE-001` when any slice was skipped, one block per wave — `  wave 2: 1 slice verified at 9c0d1e2f, repaired` (`, repaired` only when the verification repaired the tree, `, sealed` when what it left was under the `ours` rules alone) over one line per slice, `    SLICE-002 orders: covered 1/2 (uncovered REQ-004), written 3 files, merged 9c0d1e2f…, resolved (src/lib.rs)`, or `…, nothing to merge`, or `    SLICE-001: staged by an earlier run, not built again` for a slice the integration working copy held merged — then `  labelled emery/9f8e7d6c… at 9c0d1e2f…`.

`emery build` with no adapter — and no project-root `emery.toml` carrying a `[target]` table — fails with `error: "build-target-required"` (exit 1); mixing `--config` with a positional adapter, a `[target] name` that is not kebab-case, or a `branch` or `remote` key in the `[target]` table, which the grammar no longer carries, fails with `error: "bad_request"` (exit 1); `--jobs 0` is a usage error (exit 64, no envelope). Before any revision is committed it fails with `error: "spec-not-generated"` (exit 2), and over a stored revision under an older grammar with `error: "spec-outdated"` (exit 1), neither loading the adapter. Under `[target] repository = "."`, a project directory that is no repository fails with `error: "repository-required"` (exit 1), and a checkout holding pending changes outside `.emery/` and `.git/`, or no commit, with `error: "base-not-sealed"` (exit 1), the `message` listing the paths; a `repository` URL that names none, one with no default branch, or one named over a directory the engine created under the same `name` fails with `error: "revision-not-found"` (exit 2), and a remote that refuses or cannot be reached with `error: "bad_gateway"` (exit 4). An initialisation the adapter refuses, or whose files carry a credential (`error: "secret-detected"`), ends the run the same way, before any slice. The first failure ends the run and is the envelope. A slice's failure is as the adapter put it — a refusal exits 1 with `error: "bad_request"`, an upstream failure exits 4 with `error: "bad_gateway"` once the slice has been put twice, as a wave's verification is, and a turn the model backend's time budget ended is put twice the same way before its `bad_request` is the run's — the `message` naming the slice, its wave, and the working copy the slices merged before it, those built beside it in its wave included, stay in (``slice `SLICE-002` (orders) failed in wave 2; SLICE-001 merged before it stays committed in `./.emery/vcs/integration`: …``); a slice whose merge conflicts at a path the adapter's rules leave and the model cannot resolve — one side removed it or holds a link, a directory, or a file that is not text, or the answer leaves it unresolved within the available rounds — exits 1 with `error: "merge-unresolved"`, the `message` naming the paths and the reason (``…: its merge conflicts at src/lib.rs, and the conflict cannot be resolved: the slice removed `src/lib.rs` ``); a slice whose change set, or whose merge's resolution, carries a credential exits 1 with `error: "secret-detected"`, the `message` listing each path with what it carries (``slice `SLICE-001` (authentication) failed in wave 1: what the build wrote carries a credential, so none of it is sealed:\n- src/auth.rs: a PEM private key block``), as does a wave whose verification left one in the integrated tree, under the wave's own framing; a wave the adapter does not verify exits 1 with `error: "verify-failed"`, the `message` naming the wave, the slices merged in it, where the label stands (``wave 2 failed; SLICE-002 merged in it stays committed in `./.emery/vcs/integration`; `emery/9f8e7d6c…` stays at `5e6f7a8b…`: verification failed: …``), and each failing check; a report the report gate rejects — a covered id outside the slice, a written path escaping the root or under `.emery/` or `.git/`, either named twice — or a verdict whose `passed` disagrees with its `failures` exits 3 with `error: "server_error"` naming the findings, as does a stored plan whose remaining slices wait on one another.

### `emery show <spec|design|plan>`

The success body carries the revision id, the Markdown projection, and the typed revision it was rendered from; text mode is the projection alone (see the exception above).

```json
{
  "revision": "9f8e7d6c…",
  "body": "---\nemery: 6\nrevision: 9f8e7d6c…\n---\n\n# Specification\n…",
  "document": {
    "emery": 6,
    "preamble": ["…"],
    "requirements": [
      {
        "id": "REQ-001",
        "subject": "session.timeout",
        "status": "divergence",
        "covered": true,
        "sources": [{ "source": "intent", "claim": "session.timeout", "path": null }, { "source": "code", "claim": "session-expiry", "path": "src/session.ts#L12-L30" }],
        "body": ["Sessions must expire after 30 minutes of inactivity."],
        "losers": [{ "sources": ["code"], "kind": "behaviour", "rank": 3, "claim": "session-expiry", "statement": "…" }],
        "scenarios": [{ "name": "Session expires", "given": [], "when": "…", "then": "…", "and": [] }]
      }
    ]
  }
}
```

`document` is the revision document exactly as stored: for `spec`, `emery` (the grammar stamp), `preamble`, and `requirements`; for `design`, `emery`, `preamble`, and `sections` (each a `kind` and its `blocks`, `{ "text": "<paragraph>" }` or `{ "type": { "key", "signature", "role" } }`, `role` being `data`, a shape a build declares and one slice owns, or `mechanism`, a class of the source's machinery that no slice owns); for `plan`, `emery`, `preamble`, and `slices` (each an `id`, `name`, `requirements`, `types`, `depends-on`, and `brief`). The revision's serde shape is pinned by `emery-engine`'s `revision` types; its canonical bytes hash to `revision`.

`plan.md` renders each slice as a `## Slice: <name>` block: its `ID:` and `Requirements:` lines, a `Types:` line where the slice owns a design type (a `data` type; a `mechanism` type is owned by no slice), a `Depends on:` line where it is built after another slice, then the brief's paragraphs.

Before any revision is committed the verb fails with `error: "spec-not-generated"` (exit 2); a current revision id naming missing documents fails with `error: "server_error"` (exit 3); a stored revision under an older grammar fails with `error: "spec-outdated"` (exit 1). The `spec-not-generated` hint names both ways out: ``run `emery specify <adapter>...` to commit a revision, then re-run show or build``.

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
