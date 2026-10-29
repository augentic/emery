# emery amend

Stage an amendment against the current revision from the change sources named on the invocation, for `emery apply` to seal as the next revision.

## Synopsis

```bash
emery amend <adapter>... [--description <adapter>=<text>] --config [<path>]
emery amend --config <path>              # the [[source]] tables of a change file
emery amend --config                     # the project-root emery.toml's sources, read as the change
emery amend --config --description <adapter>=<text>   # an inline change beside the file's sources
emery amend --config --apply             # stage and seal in one run
```

## Description

The one amend verb. [`emery specify`](specify.md) drafts a revision from its sources alone, numbering from `REQ-001` and carrying nothing of the revision before it; `amend` reads its sources as a **change** against the **current revision** — the one [`emery apply`](apply.md) sealed in the target's repository — and stages an **amendment**: every requirement of the current revision classed by what the change does to it, the specification redrafted for the requirements the change touches alone, and the design and plan derived from the current ones. `emery apply` seals the amendment as the next revision, its `parent` the revision it amends, so the history is a lineage of revisions each sealed over the last, and [`emery build`](build.md) builds the amended plan under the new label.

Each run names its own sources, as `specify` does: every positional `<adapter>` names a **workspace-backed** change source (the adapter reads a read-only view rooted at the project directory; the source's name is the adapter's), and every `--description <adapter>=<text>` (`-d`) names a **description-backed** one (the adapter extracts the inline text). The sources of a change file — an `emery.toml` of its own, or the project-root one — are read with `--config [<path>]` and layered over exactly as [`emery specify --config`](specify.md#the-emerytoml-config) layers them: the `[[source]]` entries are the base, a command-line source named as one of them takes that entry's place whole, and any other is appended after the file's. The sources are the operator's statement of the change: the edited documentation re-read whole, a brief stating one behaviour, a code tree the behaviour moved in. A run naming its sources on the command line without `--config` reads no file, so it knows no target and is refused `build-target-required` (exit `1`) before any adapter loads; `--config` alone reads the project-root `emery.toml` for both its sources and its `[target]`. A change file naming no `[target]` reads the project-root `emery.toml`'s. A run naming no source at all, and no change file, fails typed with `specify-source-required` (exit `1`). No source may be named `baseline`, the name the current revision's requirements enter the matching under.

The run reads the current revision from the repository the `[target]` names first, before any adapter loads ([`emery apply`](apply.md) has the three kinds of repository, and [`emery show`](show.md) the reading rule), then extracts every change source at once as `specify` does, and matches the claims against the revision's requirements in one grouping turn, the standing requirements listed to the model beside the change's claims. The classes are the engine's, derived by code from the groups:

- **changed** — a group holding a standing requirement and a change claim that states it: the requirement keeps its id and takes the change's statement and, where the claim's subject differs, its subject, so a behaviour restated under another stem moves with it; the requirement is redrafted whole, scenarios included.
- **added** — a group of change claims alone: a requirement the current revision does not state, numbered past every id the revision ever used.
- **retired** — a standing requirement the change withdraws: a group led by a `retire` claim — the withdrawal of a behaviour by role, which the `intent` and `documentation` adapters emit for a passage that withdraws one — or, under a `[[source]]` with `scope = "whole"`, a standing requirement under a stem a behaviour source states and no claim of it touched, since a source read whole says everything it has to say about its subject. A `retire` claim matched to no standing requirement is `retire-unmatched` (exit `1`), nothing staged: the change withdraws a behaviour the revision does not state.
- **conflict** — change sources at one rank disagreeing over one requirement, or a withdrawal and a revision at one rank; the amendment carries it, and `apply` seals it marked `[conflict]` as a `specify` seals one, since the next change is where the operator reconciles it. The current revision ranks below every change source, so a change always wins over the base, and a tie is between change sources alone.
- **unchanged** — a standing requirement no claim touched, or restated as it stands: the revision's own requirement, byte for byte.

A standing requirement the current revision holds in conflict is listed to the model as one entry whose statement is `[conflict]` followed by the competing statements, and the one claim that lands on it resolves the conflict as a `changed` requirement — `Before:` the competing statements, `After:` the claim's — under the status the change sources alone settle. A source cannot conflict with itself: its `retire` beside its own `requirement` under one id is read as the restatement, and only two sources at one rank conflict.

The specification is then redrafted for the added, changed, and conflicting requirements alone, under the ids the classes assigned, each turn held to the same checks as a `specify` draft, the preamble and every unchanged requirement kept as the current revision has them. The design is derived from the current one by code — less the type blocks of the slices the retirements emptied — and redrafted, in one turn, in `## Overview` and the sections the change's claims inform or whose paragraphs describe a retired requirement, every other section kept as it stands; a `decision` claim alone opens the design turn, so a brief stating only the technology behind a behaviour the revision has lands in the design and changes no requirement. The plan is derived from the current one by code: each slice keeps its id and name, each retired requirement leaves its slice, a requirement restated under another stem moves to the slice of its new stem, an added requirement joins the slice of its stem, a slice left with none is removed and its dependants depend on what it depended on, a design type no slice owns goes to the one slice the change touched; what no rule places — a stem no slice holds, a stem left past the slice cap, a type where no slice or more than one changed — is placed by one slicing turn, which also redrafts the brief of every slice it finds touched. A run whose change states no behaviour — `decision`, `type`, `call`, or `excerpt` claims alone — spends no grouping and no draft turn; one that retires or restates alone spends no draft turn; one that leaves every requirement unchanged and no design section or slice different **stages nothing** and says so.

The amendment is **staged** where a `specify` candidate would go, replacing any amendment staged before it, for [`emery show amendment`](show.md) to review and [`emery apply`](apply.md) to seal; `--apply` does both in one run, the stage landing first so a refused seal leaves the amendment staged for a later `emery apply`. The slot holds one item: a revision candidate staged there is never replaced, so `amend` over one is refused `amendment-pending` (exit `1`) — seal it with `emery apply`, or review it with `emery show <artifact> --staged` — and `emery specify` over a staged amendment is refused the same way, since a specify never displaces one. An amendment prepared against a revision that is no longer current when `apply` runs — another seal landed, or the head moved — is `amendment-stale` (exit `1`) at the seal, the amendment staying staged until `emery amend` prepares it again.

Before any adapter loads, the run is refused where the repository holds no revision to amend: `spec-not-generated` (exit `2`), the description naming a staged candidate (seal it with `emery apply`, then amend), a repository with no commit, or a label the head moved on without (merge or rebase it, or `emery specify --apply` seals a fresh revision over it; then amend); a `spec/` at the head whose three documents read as no revision, or as no one revision's, is `spec-mismatch` (exit `1`) saying they were edited or merged by hand ([`emery apply`](apply.md#spec-is-the-engines) has both ways out). A current revision under an older grammar than this binary reads is `spec-outdated` (exit `1`); only `emery specify` regenerates it. The source and loader refusals are `specify`'s, as the adapter put them.

This is the CLI command invoked by [`/emery:amend`](../../../plugins/emery/skills/amend/SKILL.md), which relays the staged counts and offers the review and the seal; the CLI itself has no interactive mode.

## The change file

A change file is an `emery.toml` in the grammar [`emery specify`](specify.md#the-emerytoml-config) reads — `[[source]]` entries with `name`, `adapter`, `path` or `description`, `digest`, `rank`, and optionally `repository` and `revision` — selected with `--config <path>`, and the project-root `emery.toml` is one when `--config` names no value. One key is read by `amend` alone: **`scope`**, `"delta"` or `"whole"`, `"delta"` when omitted. A source read as a **delta** states what it states and is silent on the rest, so a standing requirement no claim of it touches stands unchanged. A **behaviour** source read **whole** — a code adapter over the whole tree the behaviour lives in — states its subject's behaviour whole, so a standing requirement under a stem its claims cover that no claim touched is retired by its silence, the entry naming the source as its `scope` and ``Note: retired by scope: `<source>` was read whole and states nothing under `<stem>`.`` after the statement; the counts line says ` (N by scope)`. A prose source — `intent`, `documentation` — read whole retires nothing, since prose is silent on most of what a system does; it withdraws a behaviour through a `retire` claim alone. `specify` reads `scope` and ignores it.

```toml
# The documentation re-read whole as the change: a passage withdrawing a
# behaviour is a `retire` claim, a passage prescribing a technology a `decision`.
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

# A brief, the inline form; `--description` on the command line is the same.
[[source]]
name = "brief"
adapter = "emery:intent@1.0.0"
description = "Cancelling an order refunds it in full."

# The code read whole: every standing requirement under a stem it states that
# it no longer states is retired by scope.
[[source]]
name = "api"
adapter = "emery:typescript@1.2.0"
path = "packages/api/src"
scope = "whole"

[target]
adapter = "emery:node@1.0.0"
repository = "."
```

## Options

| Option                           | Description                                                                                                                                                                                                     |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>...` (positional)      | Change source adapters, each an exact package reference `namespace:name@version` ([Adapters](../adapters.md)), named as a workspace-backed source; beside `--config`, one takes the place of the `[[source]]` of its name. |
| `--description <adapter>=<text>` | Inline description-backed change source (repeatable); `-d` for short. Layers over `--config` as a positional adapter does.                                                                                     |
| `--config [<path>]`              | The change file whose `[[source]]` tables state the change and whose `[target]` names the repository the current revision lives in; the omitted value selects `emery.toml` (`-c` for short). A run naming its sources on the command line reads no file without it. |
| `--apply`                        | Seal the staged amendment as the current revision in the same run, as [`emery apply`](apply.md) would; a refused seal leaves it staged.                                                                        |
| `--format`                       | Global output format: `json` for structured automation output.                                                                                                                                                  |

## JSON output

When `--format json` is provided, returns:

- `amendment` — the staged amendment's id, the content hash of its one document; omitted when the change staged nothing
- `staged` — `true` while the amendment stands staged for `emery apply`; `false` when nothing was staged and once `--apply` sealed it
- `base` — `{ revision, commit }`, the current revision the amendment was prepared against and the commit it was read at
- `tally` — the counts by class, `{ added, changed, retired, conflict, unchanged, by-scope }`, `by-scope` how many of the retired a source's scope retired
- `conflicts` — the subjects of the requirements the change sources disagree over, in id order; omitted when none
- `stems` — the stems of the requirements the change touches, once each, sorted; omitted when none
- `sections` — what the amended design changes against the current one, `{ preamble, added, removed, changed }` over section kinds; omitted when it changes nothing
- `slices` — the slices the amended plan adds, changes, or removes, `{ id, name, class, brief }` entries in id order, `brief` `redrafted` or `kept`; omitted when none
- `repositories` — every change source read from a repository, as [`emery specify`](specify.md#json-output) reports them; omitted when none
- `applied` — under `--apply`, what the run sealed, in the shape [`emery apply`](apply.md#json-output) reports with `kind: "amendment"`; omitted otherwise, and where nothing was staged to seal (see [CLI output shapes](../cli-output-shapes.md#emery-amend))

Text mode prints `staged amendment <id>`, or `nothing staged: no change against the current revision`, then `  base <revision> at <commit8>`, the counts line — `  1 added, 1 changed, 1 retired (1 by scope), 0 conflict, 4 unchanged` — `  conflicts: <subjects>` and `  stems: <stems>` when any, `  design: <kinds>` when the design changed (the section kinds in document order, `preamble` first where it changed), `  slices: orders (brief kept), export (removed), rate-limit (new)` when the plan changed, one line per repository source, then `` nothing sealed; `emery show amendment` reviews it `` or, under `--apply`, `  sealed revision <id> as emery/<id> at <commit8>` over `    amends <parent>`.

## Limits

- **A design paragraph goes stale while nothing it cites is retired.** The design turn redrafts `## Overview` and the sections the change's claims inform, or whose paragraphs describe a retired requirement; a paragraph describing a requirement the change revised in a section the change does not inform stands as the base had it. A change that drafts a requirement opens the turn on the sections its claims inform alone, so an overview that describes the old behaviour of a changed requirement is reworded at the next change that informs it, or by a `decision` beside the change.
- **A slice changed by code alone keeps the base's brief.** A retirement taking a requirement out, a dependency spliced across a removed slice, a dropped type: each changes the slice without a slicing turn, so its brief is the base's, marked `(brief kept)` on the `slices:` line and noted under its `## Plan` block in `show amendment`. A slice the slicing turn finds touched, or a join names, has its brief redrafted by that turn.
- **One source name per adapter on the command line.** A positional adapter and a `--description` for the same adapter both bind a source named for the adapter's package, so the pair is refused as a source named twice (`bad_request`, exit `1`). An inline change over an adapter whose workspace source the file names is `--description` alone beside `--config`, or a named `[[source]]` entry with a `description`.
- **A build after an amendment builds every slice again.** The slices a build sealed carry the revision they were built under in their commits, and nothing is resumed across revisions; the layout stands, so nothing is initialised. A sealed amendment is a revision to build whole, as a fresh `specify` is.
- **A conflicted revision is one to resolve, not to build.** A conflict among change sources seals marked `[conflict]` with no body and the competing statements as notes, and `apply` refuses nothing for it; the way out is the next change, with the sources agreeing or the contradicting one dropped, which the grouping reads as resolving the conflict.
- **`scope = "whole"` is a behaviour source's.** A prose source read whole retires nothing; the code adapters' reading of their tree whole against a sealed revision is recorded, with what it retires, in the adapters' own examples ([`augentic/emery-adapters`](https://github.com/augentic/emery-adapters/blob/main/examples/README.md)).
- **Two branches merge by keeping one side's `spec/` whole.** The current revision is read from the repository: at the label the pointer names where it stands over the head, else from the head's tree, whose three documents are that revision when they read as one — whatever labels the repository holds or lacks for them, the pointer a cache the repository bears out or not. After a `git merge` of two branches that each sealed an amendment over one revision, `spec/spec.json` conflicts, since each seal writes it whole: keep one side's three files whole (`git checkout <branch> -- spec/`), commit, and run the other side's amendment again over the kept revision, which seals over the merge commit with that revision as its parent. A hand merge mixing the sides is `spec-mismatch` where the documents disagree over an id or a type key; one that differs in design text alone is content-consistent and reads as a new, unlabelled revision, so keeping one side whole is the rule, not a check.
- **A change's grouping is the model's.** Byte-equal ids merge by code; the rest — a brief restating a standing requirement in other words, a claim anchored where a standing requirement was read — is the grouping turn's to match, hinted by anchor and held to one group per standing requirement. A restatement the model reads as a second requirement stages it `added`; the fix is the source's wording, or the id convention the adapter's prose teaches.

## See also

- [`emery show amendment`](show.md) renders the staged amendment, [`emery apply`](apply.md) seals it, and [`emery build`](build.md) builds the amended plan; [`emery specify --fresh`](specify.md) starts the history over instead; see the [CLI reference](index.md).
