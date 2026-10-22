# emery show

Print a reviewable artifact of the current revision, of the staged candidate, or the staged amendment, to stdout.

## Synopsis

```bash
emery show spec
emery show design
emery show plan
emery show spec --staged             # the candidate emery specify staged
emery show spec --config [<path>]    # the [target] naming the repository the revision lives in
emery show amendment                 # the amendment emery amend staged
```

## Description

The one read verb: renders the named artifact — a verifiable, non-authoritative projection of the stored revision, never a second authority. `spec`, `design`, and `plan` are the whole revision; `amendment` is the staged change against it; there is no `show sources` or `show receipts`. The specification states what the system does by the role each thing it reaches plays, and names no product, protocol, framework, or host; the design's `## APIs and integrations` is where the technology behind each role is read — what the source ran against, and what a brief or decision record prescribes — so a target maps the role onto its own platform from the design alone.

A bare `show` renders the **current revision**, read from the repository the specification lives in: the one the `[target]` table of `--config [<path>]`, or of the discovered project-root `emery.toml`, names ([`emery apply`](apply.md) has the three kinds), where `emery apply` sealed it under `spec/`. The revision is read at the label `emery/<id>` the pointer in `.emery/storage` names, where that label stands over the repository's head, and else from the head's own tree — a label merged into the branch, or a fresh clone with no pointer at all, reads the same — and in each case the three documents are held to the id their label names. `--staged` renders the **staged candidate** instead, the one [`emery specify`](specify.md) left for `emery apply`, and needs no target.

`show amendment` renders the **staged amendment**: the change `emery amend` prepared against the current revision and staged where a candidate would go, awaiting review. It is read from the staging slot alone — an amendment is never current, so `--staged` changes nothing and no `[target]` is read. The projection opens with the base it was prepared against, the revision id and the commit it was read at, then a count of its changes by class, then one section per class with entries, in the order `Added`, `Changed`, `Retired`, `Conflict`, `Unchanged`, each entry a requirement with its id, the claims behind it, and its statement — `Before:` and `After:` for a changed one, the withdrawn statement for a retired one, and `Note: Operator reconciliation required.` for a conflict, whose competing claims the sources list. While an amendment is staged, `emery specify` and `emery apply` are refused `amendment-pending` (exit `1`), as is `show spec|design|plan --staged`, since the slot holds no revision candidate: review it with `show amendment`, or re-run `emery amend` to prepare it again.

Text output is the Markdown projection alone — a deliberate exception to the result-line convention so `emery show spec > spec.md` is the document byte for byte. The projection opens with two lines of front matter, `emery: <grammar>` and `revision: <id>`, then the body rendered from the stored revision. The JSON envelope carries the revision id, the projection, and the typed revision itself (`document`).

Neither the projection nor the revision is edited by hand. Changing the specification means changing a *source* — the intent text, the workspace the adapters extract, or the adapter list — and re-running [`emery specify`](specify.md). The documents under `spec/` in the target repository are the sealed revision, not a source either: a document rewritten there, and a `spec/` the operator holds that is not a revision, fail closed as `spec-mismatch` (exit `1`), the hint naming `git checkout emery/<revision> -- spec/` to restore the labelled documents or moving the directory aside.

Before any revision is sealed the verb fails typed with `spec-not-generated` (exit `2`), the description naming a staged candidate where one stands (`emery apply` seals it), a repository with no commit, and a label the repository's head moved on without (merge or rebase `emery/<revision>` onto the head, or `emery specify --apply` seals a fresh revision over it). `--staged` with no candidate fails typed with `nothing-staged` (exit `2`), as does `show amendment` with no amendment staged — a staged candidate is no amendment — the hint naming `emery amend`. A bare `show` with no `[target]` to read is `build-target-required` (exit `1`). A revision or candidate written under an older grammar than this binary reads fails typed with `spec-outdated` (exit `1`); the next `specify` regenerates over it. A current revision id naming missing or unreadable documents fails closed with `server_error` (exit `3`) — corruption is never an empty result.

## Options

| Option | Description |
|--------|-------------|
| `spec` \| `design` \| `plan` \| `amendment` (positional) | Which reviewable artifact to print; `amendment` is the staged amendment, read from the slot alone. |
| `--staged` | Read the staged candidate in place of the current revision; needs no `[target]`. Changes nothing for `amendment`. |
| `--config [<path>]` | Operator-owned config naming the `[target]` whose repository the revision lives in; the omitted value selects `emery.toml` (`-c` for short). A bare `show` naming none discovers the project-root file. |
| `--format` | Global output format: `json` wraps the projection with its revision id and the typed revision. |

## JSON output

When `--format json` is provided, returns:

- `revision` — the current revision id, the staged candidate's under `--staged`, or the staged amendment's for `amendment`
- `body` — the Markdown projection, byte for byte what text mode prints
- `document` — the stored document the projection was rendered from: the typed specification (`emery`, `parent`, `preamble`, `requirements` with `id`, `subject`, `status`, `covered`, `trigger`, `sources` as `{ source, claim, path }` entries, `body`, `losers`, `scenarios`), design (`emery`, `parent`, `preamble`, `sections` of `kind` and `blocks`), plan (`emery`, `parent`, `preamble`, `slices` with `id`, `name`, `requirements`, `types`, `depends-on`, `brief`), or amendment (`emery`, `base` as `{ revision, commit }`, `changes` with `id`, `subject`, `class` of `added` \| `changed` \| `retired` \| `conflict` \| `unchanged`, `before`, `after`, `sources`); see [CLI output shapes](../cli-output-shapes.md#emery-show-specdesignplanamendment)

## See also

- [`emery specify`](specify.md) stages the candidate `--staged` renders, [`emery apply`](apply.md) seals the revision a bare `show` renders, and [`emery build`](build.md) builds its plan; see the [CLI reference](index.md).
