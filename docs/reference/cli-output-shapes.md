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

The success body names the revision whose plan was built, the plan's waves, the base, the slices resumed from the label, every slice this run merged with its wave and merge commit, the head each wave was verified at, and the label on the integrated head:

```json
{
  "revision": "9f8e7d6c…",
  "waves": [["SLICE-001", "SLICE-003"], ["SLICE-002"]],
  "base": "1a2b3c4d…",
  "resumed": ["SLICE-001"],
  "slices": [
    { "id": "SLICE-003", "name": "catalogue", "wave": 1, "covered": ["REQ-005"], "uncovered": [], "written": ["src/catalogue.rs"], "commit": "5e6f7a8b…" },
    { "id": "SLICE-002", "name": "orders", "wave": 2, "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["src/orders.rs", "src/orders/create.rs"], "commit": "9c0d1e2f…", "conflicts": ["src/lib.rs"] }
  ],
  "verified": ["5e6f7a8b…", "9c0d1e2f…"],
  "head": "9c0d1e2f…",
  "label": "emery/9f8e7d6c…",
  "pushed": "origin"
}
```

`waves` is the plan's shape as `emery specify` reported it when the revision was committed — the plan's own projection, which a conflicted slice's rebuild in a later wave departs from. `base` is the commit the build started from — the project checkout's head, or the `[target] branch`'s commit in the clone. `resumed` lists the slices the label `emery/<revision>` already held over the base, in id order, which were not built again; it is omitted when none were. `slices` is every slice this run built, in merge order: each carries `wave`, the wave it merged in from one; the requirement ids the target adapter reported implemented (`covered`) and those of the slice it left out (`uncovered`), both in id order; the files it reported written, relative to its working copy's root, in the order it named them; `commit`, the merge commit that brought what it changed into the integrated head, `null` when it changed nothing; and `conflicts`, the paths an earlier build of it conflicted at before it was built again over the merged head, omitted when none. `verified` is the integrated head each wave was verified and labelled at, in wave order, omitted when every slice was resumed. `head` is the integrated commit, `label` the branch `emery/<revision>` set on it, and `pushed` the remote it went to, omitted when the target names none. Text mode prints the revision, the plan line — `  plan: 3 slices in 2 waves, widest 2` — the base — `  base 1a2b3c4d…` — `  resumed: SLICE-001` when any slice was skipped, one block per wave — `  wave 2: 1 slice verified at 9c0d1e2f` over one line per slice, `    SLICE-002 orders: covered 1/2 (uncovered REQ-004), written 2 files, merged 9c0d1e2f…, conflicted (src/lib.rs)`, or `…, nothing to merge` — then `  labelled emery/9f8e7d6c… at 9c0d1e2f…` and, after a push, `  pushed to origin`.

`emery build` with no adapter — and no project-root `emery.toml` carrying a `[target]` table — fails with `error: "build-target-required"` (exit 1); mixing `--config` with a positional adapter, or a `[target] repository` without a `branch` or a `branch` without a `repository`, fails with `error: "bad_request"` (exit 1); `--jobs 0` is a usage error (exit 64, no envelope). Before any revision is committed it fails with `error: "spec-not-generated"` (exit 2), and over a stored revision under an older grammar with `error: "spec-outdated"` (exit 1), neither loading the adapter. With no `[target] repository`, a project directory that is no repository fails with `error: "repository-required"` (exit 1), and a checkout holding pending changes outside `.emery/` and `.git/`, or no commit, with `error: "base-not-sealed"` (exit 1), the `message` listing the paths; a `branch` or `remote` the repository lacks, or a `repository` URL that names none, fails with `error: "revision-not-found"` (exit 2), and a remote that refuses or cannot be reached with `error: "bad_gateway"` (exit 4). The first failure ends the run and is the envelope. A slice's failure is as the adapter put it — a refusal exits 1 with `error: "bad_request"`, an upstream failure exits 4 with `error: "bad_gateway"` — the `message` naming the slice, its wave, and the working copy the slices merged before it stay in (``slice `SLICE-002` (orders) failed in wave 2; SLICE-001 merged before it stays committed in `./.emery/vcs/integration`: …``); a slice whose merge conflicts on its second build exits 1 with `error: "slice-conflict"`, the paths named; a wave the adapter does not verify exits 1 with `error: "verify-failed"`, the `message` naming the wave, the slices merged in it, where the label stands (``wave 2 failed; SLICE-002 merged in it stays committed in `./.emery/vcs/integration`; `emery/9f8e7d6c…` stays at `5e6f7a8b…`: verification failed: …``), and each failing check; a report the report gate rejects — a covered id outside the slice, a written path escaping the root or under `.emery/` or `.git/`, either named twice — or a verdict whose `passed` disagrees with its `failures` exits 3 with `error: "server_error"` naming the findings, as does a stored plan whose remaining slices wait on one another.

### `emery show <spec|design|plan>`

The success body carries the revision id, the Markdown projection, and the typed revision it was rendered from; text mode is the projection alone (see the exception above).

```json
{
  "revision": "9f8e7d6c…",
  "body": "---\nemery: 4\nrevision: 9f8e7d6c…\n---\n\n# Specification\n…",
  "document": {
    "emery": 4,
    "preamble": ["…"],
    "requirements": [
      {
        "id": "REQ-001",
        "subject": "session.timeout",
        "status": "divergence",
        "covered": true,
        "sources": [{ "source": "intent", "claim": "session.timeout", "path": null }, { "source": "code", "claim": "session-expiry", "path": "src/session.ts#L12-L30" }],
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
