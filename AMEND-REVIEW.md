# Review plan: `emery amend` and `emery apply`

A top-to-bottom plan for the human review of the amend/apply feature across `augentic/emery` (this repository) and `augentic/emery-adapters` (the sibling checkout at `../emery-adapters`). The code and every document it touched were agent-written. This plan is the reviewer's: tick items off, add findings to the log at the end, and reshape the plan as the review teaches you where the risk is.

Conventions:

- `[ ]` is a step for you. `Agent:` is a prompt you can hand to an agent session to do the legwork; you still read the answer against the code.
- Paths are repository-relative. Anything under `../emery-adapters/` is the sibling repository.
- Never commit, push, or open a PR from a review session. Leave fixes uncommitted for the maintainer, as `AGENTS.md` requires of agents, or note them in the findings log and fix later.
- Each phase ends with an **Exit** line stating what you should be able to say before moving on. Do not skip phases 0 to 2; everything after them assumes their vocabulary.

---

## 0. Orientation

### 0.1 What shipped, and where

The feature landed on `main` of `emery` in four pull requests, in this order. Every one is agent-written; read the PR titles as the phases of the plan canvas, not as a review unit.

| PR | Title | What it carried |
| --- | --- | --- |
| [#374](https://github.com/augentic/emery/pull/374) | Apply spec before build | The specification moves into the target repository under `spec/`. `specify` stages, the new `apply` seals, `--apply` composes them. `BuildRepo::head`, `RevisionMessage`, `vcs::REVISION`, `spec-rewritten`, grammar 8 (`parent`). |
| [#375](https://github.com/augentic/emery/pull/375) | Amend phase 1, diff computation | The `retire` claim kind. `amend.rs`, `amend/standing.rs`, `amend/change.rs`, `revision/amendment.rs`, the `Amendment` document and its projection, `show amendment`, the hidden verb. |
| [#380](https://github.com/augentic/emery/pull/380) | Amend plans and designs | `amend/plan.rs` (`Pruned`), `specify/plan/amend.rs` (`SliceAmend`), `specify/design/amend.rs` (`DesignAmend`), retirement by `scope = "whole"`, `DesignDiff` / `SliceChange` on the amendment. |
| [#383](https://github.com/augentic/emery/pull/383) | Apply (merge) amendment to base spec | `apply` seals an amendment, `amendment-stale`, `lineage-exists`, `specify --fresh`, the two-branch reading, the by-hand `spec-mismatch`, the verb made public, docs, skills, release notes. |

The adapters repository carries two uncommitted-to-main commits on `amend-review` (`656036a`, `96d25d6`): the `emery-sdk` re-pin to `emery` `main` at `b3682cab` and the prose rules for `retire` and `decision` in `intent` and `documentation`. PR [#186](https://github.com/augentic/emery-adapters/pull/186) before them is the pin move alone. The `retire` claim itself reached the adapters earlier (`tests/source.rs::documentation_retire`, `intent_retire`).

State at the time of writing:

- `emery` branch `amemd-review` is at `main` (`b3682cab`), nothing uncommitted.
- `emery-adapters` branch `amend-review` is two commits ahead of `main`; `Cargo.lock` pins `emery-sdk` at `b3682cab`, so the two trees are in step.
- Workspace version `0.39.0` in both; the 0.39.0 section of `RELEASES.md` is unreleased and holds every bullet of the four PRs. The Cursor plugin is versioned apart (`0.32.0`, `plugins/emery/.cursor-plugin/plugin.json`, held equal to `.cursor-plugin/marketplace.json` by `tests/plugin.rs::versions_agree`).

### 0.2 The plan canvas, and where the code is known to have drifted

The canvas "Emery Amend Plan" (`~/.cursor/projects/Users-goldie-git-augentic-emery/canvases/emery-amend-plan.canvas.tsx`) is the brief the work started from. It is not canonical. Its scenarios S1 to S13 and steps A, B, C, 1a, 1b, 2a, 2b, 3a, 3b, 3c are a useful checklist of *intent*; the table below records where the shipped code is known to differ, so you do not rediscover each one. Confirm or correct each as you reach the relevant phase.

| Canvas said | Code does | Where to confirm |
| --- | --- | --- |
| `amend` over a staged item needs `--replace`; `specify` over a staged amendment needs `--replace` | No flag. `amend` replaces a staged amendment freely and refuses over a candidate (`amendment-pending`); `specify` always refuses over a staged amendment. | §2.2, §4.1 |
| A conflicted standing requirement contributes each statement as a separate contributor | One contributor whose statement is `[conflict] <a> \| <b>` (`Requirement::stated`) | §3.2 |
| "Which standing requirements enter the brief" was an open design question for large specifications | Every standing requirement is listed, no cap | §3.3, §9 |
| A candidate `emery show diff` | Not built; the diff is `apply`'s and `specify`'s JSON `diff` and the text summary line | §2.3 |
| Preamble: kept or redrafted when the change adds a stem (open) | Always the base's | §3.4 |
| Changed requirement: scenarios kept when only the body changed (open) | Always redrafted whole | §3.4 |
| A slice all of whose requirements moved stem: renamed or recreated (open) | Removed; the slicing turn drafts the new home and re-owns its types | §3.6 |
| Added ids are "the first of `sequence()` no revision in the lineage has held, never reused" | `ReqId::free_after` is max-held-id + 1 over the *base spec alone*; a retired id that was the highest is reused by the next amendment | §3.2, §9 |
| `apply` creating a repository writes no `.gitignore` | Confirm against `store::commit` and `build.rs` `IGNORE` | §2.2 |
| Hidden verb until step 3c | Public, in `--help` and completions | §4.1 |
| `amend/stage.rs` module | Staging lives in `store.rs` (`stage_amendment`, `Slot`) | §2.1 |

### 0.3 Reading order for someone new to these repositories

You know Rust; you do not yet know how these two repositories are laid out. Read in this order, skimming, before any code:

- [ ] `AGENTS.md` here: `## Vocabulary` (short), then the `## Map` rows for `crates/engine`, `crates/cli`, `crates/adapter`, `tests/` only. The `crates/engine` row is enormous (the file is 148 KB) and agent-written; treat it as a claimed specification to verify in phase 6, not as ground truth now. Read it once for the nouns: *revision*, *candidate*, *amendment*, *standing*, *sealing*, *slot*, *label*, *lineage*, *fresh*, *scope*.
- [ ] `docs/reference/cli/apply.md` and `docs/reference/cli/amend.md`, whole. These are the public contract and the clearest statement of intended behaviour. Keep them open through phases 2 and 3; every "the code does X" below should match a sentence there, and a mismatch is a finding against one or the other.
- [ ] `examples/README.md`, sections "Build and run", "Amend and build again", "Two branches", "Start over". This is the live journey you will run in phase 8.
- [ ] `../emery-adapters/AGENTS.md`: `## Map` and the first two `## Invariants` bullets, for how an adapter is shaped and where its prose lives.
- [ ] The canvas, the `SCENARIOS` array (S1 to S13) and the `STEPS` array. Fifteen minutes; it gives you the vocabulary the agents used.

> Agent: "Give me a one-page glossary of the nouns `AGENTS.md` uses for the amend feature (revision, candidate, amendment, standing, sealing, slot, label, lineage, fresh, scope, baseline, by scope, brief kept) with the one file and type that defines each."

**Exit:** you can say what `emery specify`, `emery apply`, `emery amend`, `emery show amendment`, and `emery build` each read and write, and where the specification lives after a seal.

---

## 1. Set the ground

- [ ] `make ci` in `emery`. Record the time and the result. If anything is red, stop and log it; the review assumes green.
- [ ] `make ci` in `../emery-adapters`. The same.
- [ ] `cargo nextest run --workspace --all-features --test amend --test apply --test show` here, so you know the names of the amend suites and that they pass in isolation.
- [ ] `cargo doc --no-deps -p emery-engine --open` and read the `emery_engine::amend`, `apply`, `store`, and `revision` module docs. The `///` and `//!` are agent-written; you are reading them for a first impression of accuracy and tone, which phase 6 checks line by line.
- [ ] Confirm the build artefacts the live journey needs exist or can be built: `cargo build --release` here (the guest and the two example mocks), and in `../emery-adapters` `cargo build --workspace --target wasm32-wasip2 --release`, which the phase 8 journey copies into `~/.emery/adapters/`.

**Exit:** both repositories green under `make ci`, docs built, components built.

---

## 2. Contracts and persistence

Read these before the pipeline because the pipeline is written in their terms.

### 2.1 The revision grammar and the staging slot

Files: `crates/engine/src/revision.rs` (239), `crates/engine/src/revision/id.rs` (168), `crates/engine/src/revision/amendment.rs` (481), `crates/engine/src/store.rs` (937).

- [ ] `revision.rs`: `EMERY = 8`; the `Document` trait (`from_json`, `to_json`, `to_markdown`); `Revision::read` / `identify` / `documents`; `Revision::findings`, the cross-document rule (each requirement placed in one slice, each design data type owned by one slice, nothing else named). Ask: is `findings` complete for what a hand merge can produce (one side's `spec.json` beside the other's `design.json` and `plan.json`)?
- [ ] `id.rs`: `ReqId` / `SliceId` over `NonZeroU32`; `free_after` is `max + 1`. Decide whether id reuse after retiring the highest id is acceptable or a defect (see §0.2 and §9). The amend doc says "numbered past every id the revision ever used", which the code does not guarantee.
- [ ] `amendment.rs`: the `Amendment` shape (`emery`, `base`, `changes`, `spec`, `design`, `plan`, `sections`, `slices`), `deny_unknown_fields`, `identify` over one body, `is_change`, `stems`, `conflicts`, `count`. The `Display` impl is the `show amendment` projection: check it against `tests/specify/6-amendment.md` and against `docs/reference/cli/show.md`. Check the `Change` type: `before` / `after` semantics per class, `scope` set only on `Retired`, `Display` arms. Check `SliceChange::of` and `keeps_brief`.
- [ ] `store.rs`, top half: `Slot` (the one staged record: `id`, `kind`, `base`, `fresh`), `StagedKind::documents` / `noun`, `stage` / `stage_amendment` (compare-and-swap, `prune` of the displaced item), `no_amendment` / `no_candidate` (the two `amendment-pending` guards, read from the raw token so an undecodable record is swapped over rather than refused), `staged` / `staged_amendment` / `staged_item`, `clear`. Questions:
  - A `Slot` that decodes as neither kind is treated as empty by `token` and refused by `Slot::read`. Is that asymmetry intended? (`stage` swaps over corruption; `apply`'s `staged_item` fails on it.)
  - `swap` prunes the displaced item only when its id differs; a re-stage of identical content keeps the documents. Fine, but confirm `put` is idempotent.
  - `clear` after a seal re-reads the slot and deletes whatever is there. Between `staged_item` and `clear`, another process could have staged something else. Decide whether the single-operator assumption is stated anywhere.

> Agent: "List every reader and writer of `STAGED_KEY`, `STAGED_CONTAINER`, and `REVISION_KEY` across `crates/engine`, with the function and the guard it applies."

### 2.2 Where the current revision lives, and the seal

Files: `store.rs`, bottom half (`Standing`, `Current`, `Sealing`, `Occupancy`, `current`, `commit`, `seal_copy`, `read_tree`); `crates/engine/src/vcs.rs` (1211; the parts named below); `crates/engine/src/apply.rs` (158).

- [ ] `store::current`: pointer read (a cache); `repo.head`; the label the pointer names, taken only where `labelled` finds it and `descends(head, label)`; else the head's tree; `behind` recorded. Trace each `tracing::warn!` branch and ask whether the operator ever sees it (they are `warn`, so yes at bare `info`).
- [ ] `Current::read_tree`: `Occupancy` (absent / exactly `design.json`, `plan.json`, `spec.json` as files / other); `expect` held where a label named the commit; at an unlabelled head, "no revision" (parse failure that is not `BadRequest`, i.e. not `spec-outdated`) and "no one revision's" (`Revision::findings`) both become `hand_merged`, whose phrase `edited or merged by hand` the CLI hint matches on by string. Decide whether string-matching an error description in the CLI is acceptable or should be a code.
- [ ] `Sealing::candidate` (parent = current id, `over` = current commit else head, `no_lineage` guard), `Sealing::fresh` (no parent, `over` = head), `Sealing::amendment` (`is_at(base)` else `amendment-stale`; `Summary::of`). Confirm the three match the three paragraphs of `docs/reference/cli/apply.md` "Sealing a candidate" / "Sealing an amendment".
- [ ] `store::commit`: the cut at `over`, else `repo.init` for a created repository (`WorkingCopy::checkout` with no cut, `cut = false` so nothing is removed), else `base-not-sealed` / `server_error`; `seal_copy` (occupancy check, three `Files::write`, `pending` to the secret gate, commit, a `None` commit with `over` = "the repository already holds the revision", label forced); the pointer CAS against `standing.pointer`; `clear`. Questions:
  - The pointer CAS uses the token read at `current`. If `StateStore::get` failed there, `pointer` is `None` and the CAS fails against a present key. Is that the right failure (the seal has already landed in git at that point)? Note the ordering: git commit and label first, then pointer, then clear. A failure after the commit leaves a labelled revision the pointer does not name, which `current` then reads from the head only if the label stands over it. Decide whether this is acceptable and whether `apply.md` says so.
  - `seal_copy` refuses `Occupancy::Other` but overwrites `Held` silently. For a `fresh` seal over a head whose `spec/` holds another lineage's documents, that is the intent; confirm it is documented.
- [ ] `vcs.rs`: `SPECIFICATION`, `REVISION`, `RevisionMessage` (`Display` and `parse`, the `Parent:` / `Base:` trailers, the `Summary` trailers `Added:` / `Changed:` / `Retired:` / `Slices:`, a malformed summary read as no message), `Summary::of`, `SliceSummary`, `BuildRepo::head` (the three kinds), `BuildRepo::init`, `BuildRepo::label` (forced), `BuildRepo::is_created`, `WorkingCopy::checkout`. Check `Message::parse` passes over a `Revision …` first line (its first word is no slice id) so `merged_under` and `staged` never mistake a seal for a slice.
- [ ] `apply.rs`: `apply` is `current` then `seal`; `seal` is the shared step `specify --apply` and `amend --apply` call with the `Standing` their run read. Confirm the `# Errors` list against `store.rs` and against `apply.md` "Refusals". Note `ApplyOutput.kind` / `amendment` / `diff`.
- [ ] `build.rs`: the parts #374 touched. `store::current` read before the target loads; `BuildOutput.candidate` (`Candidate { id, kind }`); `Progress::initialise` conditions (label's chain names no slice and the copy stages none); the `IGNORE` write when the init scaffolded no `.gitignore`; `gated()` and `spec-rewritten` at every seal point (`initialise`, `integrate`, `UnresolvedMerge::resolve`, `verify`). Confirm the canvas claim that `apply` creating a repository writes no `.gitignore` and the first build does.

> Agent: "Walk me through `emery apply` over a staged amendment from `apply::apply` to the git commit, naming every `Vcs`, `Files`, `StateStore`, and `BlobStore` call in order, and every early return with its error code."

### 2.3 The revision diff

File: `crates/engine/src/revision/diff.rs` (411).

- [ ] `Diff::between(from, outgoing, incoming)`: within one history (incoming `parent` names `from`) pairs by id alone and a dropped requirement is `retired`; across histories pairs by stem and shared anchor then by id and a dropped one is `removed`. `DesignDiff::between`, `PlanDiff::between`, `differences` / `slice_differences` (every field compared? `trigger` included per the 0.39.0 notes). `BRIEF` is the field name `SliceChange::of` reads.
- [ ] Decide whether the absence of a `show diff` verb (canvas 1b candidate) is a gap worth logging or a reasonable cut.

**Exit:** you can draw the state machine of the slot (empty, candidate, fresh candidate, amendment) and of the repository (no commit, head with no `spec/`, labelled revision over head, label behind head, unlabelled revision at head, hand-merged), and name the refusal at every transition.

---

## 3. The amend pipeline, in execution order

Read `crates/engine/src/amend.rs` (347) first, top to bottom; it is the spine, and its `///` on `amend` is the longest statement of intended behaviour outside the docs. Then follow each step into its module in the order `amend` calls them.

### 3.1 Preconditions and extraction

- [ ] `amend.rs` lines ~120–145: `Bound::all`, the `BASELINE` name refusal, `target_required`, `store::current`, `not_generated`, the revision grammar (`current.revision` is a `Result`, so an outdated grammar refuses here), `store::no_candidate`. Order matters: confirm every refusal precedes `Bound::extract_all` (no adapter loads, no clone, no turn). Compare with the "before any adapter loads" sentences in `amend.md`.
- [ ] `crates/engine/src/specify.rs`: `Scope` (`Delta` default, `Whole`), `SourceConfig.scope`, `Extract.scope` and `.kind` and `.rank`, `Extracted` / `Bound::extract_all` (shared with `specify`), `BASELINE`. Check `specify` itself ignores `scope` as the docs say.
- [ ] `crates/engine/src/authority.rs`: `Rank::below` (one step down, lowest answers itself). Used by `Standing::new`.

### 3.2 The standing requirements and the change set

Files: `amend/standing.rs` (104), `amend/change.rs` (226).

- [ ] `Standing::new`: rank = `max(extract ranks).below()`, default `Behaviour` when there are no extracts. `contributors()`: one per base requirement with a `stated()` statement; `kind` nominally `Documentation`; `id` is the **subject**; `citations` for the anchor hints. `requirement(subject)` finds by subject with `find` (first match). Ask: can a base spec hold two requirements with one subject? If the spec gate does not forbid it, `matched` can land on the wrong one. Check `specify/spec.rs::verify` and `revision/spec.rs` for a uniqueness rule.
- [ ] `ChangeSet::states`: any `Requirement` or `Retire` claim. `ChangeSet::unchanged`: every base requirement `Unchanged` with empty `sources`.
- [ ] `ChangeSet::derive`: the `retire-unmatched` pass first; then per basis: `sources` = contributors minus `BASELINE`; matched → `before = stated()`, class by (`sources.is_empty()` → `Unchanged`; `withdraws` → `Retired`; `statement() None` → `Conflict`; same statement + trigger + subject → `Unchanged`; else `Changed`), subject = base's for `Retired` else the basis's; unmatched → next free id, `Added` or `Conflict`. `drafted` holds `basis.amended(id, trigger)` for drafted classes. Questions:
  - A matched group where the change's statement equals the base's but the **trigger** differs is `Changed`. Is a trigger-only change drafted sensibly (the spec-draft is told the requirement amends the current one)?
  - `same_trigger` treats a basis with no trigger as "same". A change that drops a route (standing has `Route:`, change has none) therefore reads as unchanged on the trigger axis and the base's trigger is carried (`.or_else(matched.trigger)`). Is that the intended asymmetry?
  - A conflict among change sources matched to a standing requirement: `after = None`, class `Conflict`, `before` = base's statement. The amended spec then drafts it under the kept id with the competing statements as notes. Confirm against `tests/amend.rs::amend_retire_contested`.
  - `Added` ids: see the reuse question in §2.1.
- [ ] `ChangeSet::retire_by_scope`: only `Scope::Whole` and `SourceKind::Behaviour`; `covered` = stems of the source's `Requirement` claim ids; a change is retired when `Unchanged` with empty `sources` and its stem is covered. Consequences to judge:
  - A whole-scope code source that stops stating an entire stem (the module was deleted) covers nothing under it and retires nothing. The docs state the rule ("under a stem its claims cover") but not this consequence. Decide whether to log it as a limitation to document or a behaviour to change.
  - Retirement by scope happens **after** the grouping and before the design and plan steps, so a by-scope retirement flows through `Pruned` and `DesignAmend::describes` like a claim's. Confirm in `amend.rs` ordering and in `amend_scoped`.
  - `change.scope` names the first whole source in declaration order to cover the stem; a second covering source is skipped because the class is no longer `Unchanged`. Fine; note it.

> Agent: "Enumerate every combination of (matched / unmatched) × (sources empty / statement Some / statement None / withdraws) × (same subject / different subject) × (same trigger / different trigger / no trigger) that `ChangeSet::derive` can see, and the class each yields. Flag any combination the tests in `tests/amend.rs` do not cover."

### 3.3 The grouping with standing requirements

Files: `crates/engine/src/specify/basis.rs` (870; the amend-relevant parts), `crates/engine/prose/grouping.md` (80).

- [ ] `GroupingBrief::with_standing`: the change claims first, standing contributors appended and re-indexed, `criteria`; a `retire` is a `Contributor { withdraws: true }` only here (`new` passes it over); a `retire` beside its own source's `requirement` under one id is passed over with a `tracing::debug!`. `Hint::all` (the `## Baseline` anchor hints from `Cited::overlaps`). `derive` always puts the turn when any standing exist.
- [ ] `verify`: refuses a group merging two standing requirements; refuses a class mixing a `(retire)` with a statement. `Basis::of` ranking: the base is always below, so a change wins; `Basis::withdraws`, `withdrawals`, `statement()` (`None` for conflict or withdrawal), `amended(id, trigger)` (drops `BASELINE` contributors).
- [ ] `grouping.md` `## Standing requirements` and its worked example. Read it as the model will. Does it say everything `verify` enforces, and nothing `verify` does not? Does the worked example's JSON pass `verify`? (`tests/prose` or the brief's corpus test should hold the example; confirm which test.)
- [ ] Scale: every standing requirement is listed in one turn. For a 200-requirement specification that is one large prompt per amend. Log as a known limitation with whatever cap you think is needed; the canvas flagged it and no cap shipped.

### 3.4 The amended specification

Files: `crates/engine/src/specify/spec.rs` (462): `SpecBrief::amending`, `SpecBrief::amend`, `Review::technology`; `crates/engine/src/specify/sections.rs`.

- [ ] `amending(extracts, drafted)`: chunks over the drafted bases alone, no preamble, each told it amends the current specification; none for no basis. `amend(base, parent, changes, drafts)`: base preamble, `Unchanged` requirements byte for byte, drafted ones in id order, `Retired` left out, `parent` set. Confirm "byte for byte" is literally the base's `Requirement` value (clone), not a re-render.
- [ ] The preamble is never redrafted and a changed requirement's scenarios are always redrafted: both were open in the canvas and are now settled by code. Confirm `amend.md` states both (it states the preamble; check scenarios).
- [ ] The `Lexicon` gate applies to amend drafts as to specify drafts (`Review::technology`). Confirm the lexicon is derived from the change's `call` claims alone (it is `SpecBrief::chunked`'s rule); a change with no `call` claims gets the fixed lexicon.

### 3.5 The amended design

Files: `crates/engine/src/specify/design/amend.rs` (488), `crates/engine/src/revision/design.rs` (`without_types`, `data_types`, `Block`), `crates/engine/prose/design-format.md` `## Amending`.

- [ ] `DesignAmend::new` returns `Some` when a section is informed (`Outline::informed` over claim kinds) or a paragraph cites a `Retired` id (`describes`); `None` otherwise, the pruned base standing. Confirm `## Overview` is always redrafted when the turn runs.
- [ ] `tighten`: `sections` held to the redrafted kinds, `revised` capped at the longest redrafted base section, `TypeRef` keys limited where `## Domain model` is redrafted. `verify`: `Outline::blocks` / `references` and the `loss` check (kept verbatim with whitespace collapsed, or named in `revised`; a named one still describes every non-retired id it cited; a revised paragraph citing only `Unchanged` ids is a finding when the change drafts a requirement; `withdrawn` paragraphs may be dropped unnamed; `withdrawable` sections may be answered empty and leave the design).
- [ ] `into_output`: redrafted sections replace the base's, every other section and the preamble copied, vocabulary order. A redrafted section answered with no block is left out.
- [ ] Read `design-format.md` `## Amending` as the model will, against `verify`. Note the "Limits" bullet in `amend.md` about a paragraph going stale when nothing it cites is retired: confirm it is an honest statement of what `loss` does not check.
- [ ] `amend.rs`: the design turn runs under `future::try_join` beside the spec chunks and reads `drafted` (the bases, not the drafts). Confirm the brief does not need the drafted text.

### 3.6 The amended plan

Files: `crates/engine/src/amend/plan.rs` (306), `crates/engine/src/specify/plan/amend.rs` (559), `crates/engine/src/specify/plan.rs` (`Floors`, `SLICE_CAP`, `drafts`), `crates/engine/prose/slicing.md` `## Amending`.

- [ ] `Pruned::new`: `without(retired)` → `orphaned` type keys from emptied slices; then `without(moved)` where moved = `Changed` whose base stem ≠ change stem. `without` removes a slice all of whose requirements leave (and not an already-empty one), splices `depends_on` through removed slices transitively (`spliced`), dedupes and sorts. Check `spliced` cannot loop (visited set) and removes self-dependency.
- [ ] `Pruned::assemble` → `place` then `own`, then `SliceAmend` if anything is open, else `held` (the slicing gate over the settled plan; a finding is `server_error`, since nothing of it is the model's). Questions on `place`:
  - `past` counts requirements of the stem in the **amended** spec. A stem that was under the cap and goes over it by additions is recut only if an addition falls under a sub-stem no slice holds; otherwise the additions join the holding slice by floor and the slice silently exceeds `SLICE_CAP`. Is that intended? Compare with `Floors::check` in `held`, which may then refuse the settled plan as `server_error`. This is the kind of edge a desk check would not hit.
  - Trigger placement before floor placement: an added requirement whose trigger matches a requirement in a slice of **another** stem joins that slice. That is "one trigger is one slice" and deliberate; confirm `slicing.md` says so for amendments.
- [ ] `own`: drop keys the design no longer holds; unowned keys go to the single changed slice when nothing is open and exactly one slice changed; else open. Check the "exactly one changed" rule against `amend_sliced_types`.
- [ ] `SliceAmend` (`specify/plan/amend.rs`): `Display` (the `## Current plan` list with `touched by this amendment`, `## Open`, recut stems by sub-stem, the floor/trigger/cap rules, `## Requirements`), `tighten`, `verify` (the refusals listed in `AGENTS.md` and `slicing.md`), `into_output` (joins, rebriefs, numbering new slices after the base's highest id by lowest requirement, `depends-on` names to ids, preamble kept). Check `touched` and `is_drafted` against the `briefs` rule: every touched or joined slice must get a rebrief; no untouched slice may.
- [ ] `slicing.md` `## Amending` and its worked example against `verify`.

### 3.7 Assembly, diffs, staging, and `show`

- [ ] `amend.rs` lines ~190–225: `DesignDiff::between`, `SliceChange::of(PlanDiff::between)`, the `Amendment` value, `Tally::of`, `stage` (no change → nothing staged, nothing sealed; else `stage_amendment` then, under `apply`, `apply::seal` over the `Standing` the run read, so a refused seal leaves it staged). `AmendOutput.staged` is `id.is_some() && applied.is_none()`.
- [ ] `show.rs`: `Artifact::Amendment` reads the slot alone (`staged_amendment`; `nothing-staged` with none or a candidate), `staged` and `target` unread; `--staged` over an amendment is `amendment-pending`; a bare `show spec|design|plan` reads `store::current` and raises the revision `Result`.

> Agent: "Trace `emery amend --apply` end to end and list every model turn it can open (grouping, spec-draft chunks, design-draft, slicing) with the condition under which each is skipped. Then list every point after extraction where a refusal leaves state behind (a staged amendment, a cut working copy, a clone)."

**Exit:** for each of S1–S13 in the canvas you can point at the code path and the class(es) it produces, and you have a list of edge cases the desk checks did not cover.

---

## 4. The CLI surface

Files: `crates/cli/src/lib.rs` (571), `crates/cli/src/config.rs` (385), `crates/cli/src/text.rs` (314).

- [ ] `lib.rs`: `Verb::{Specify, Apply, Amend, Build, Show, Completions}`, `SpecifyArgs` (`--apply`, `--fresh`), `ApplyArgs` (`--config`, `--repository`), `AmendArgs` (sources, `--description`, `--config`, `--apply`), `ShowArgs` (`amendment` artifact, `--staged`). Run `emery --help`, `emery amend --help`, `emery apply --help`, `emery completions zsh | grep -c amend` and read the help text as an operator would.
- [ ] `config.rs`: `scope` on `[[source]]` (`delta` default); `[target]` resolution for `amend` (the change file's, else the discovered `emery.toml`'s, only under `--config`); argv-over-file layering; an argv-only `amend` knows no target and is refused `build-target-required`. Confirm `amend --config` with no value reads the project-root `emery.toml`'s `[[source]]` entries **as the change** (the synopsis in `amend.md` says so); decide whether that is a foot-gun worth a note.
- [ ] `text.rs`: `amend` (the counts line, ` (N by scope)`, `conflicts:`, `stems:`, `design:`, `slices: … (brief kept) / (removed) / (new)`, the repository lines, the closing line), `apply` (`sealed revision … as emery/… at <commit8>`, ` amends <parent>`, ` repository <path>`, the diff line), `specify` (` (fresh)`), `build` (the candidate / amendment staged lines). Compare each against `docs/reference/cli-output-shapes.md` and against what `tests/amend.rs` / `tests/apply.rs` assert.
- [ ] The hint table (`fn hint`, ~line 444): `nothing-staged`, `amendment-pending` (three readings by verb), `amendment-stale`, `lineage-exists` (two readings), `retire-unmatched`, `spec-not-generated` (description-driven), `spec-mismatch` (the by-hand phrase match), `build-target-required` (three readings). Read every hint as the operator who just got the error. Are the commands it names real and complete?
- [ ] JSON envelopes: `AmendOutput`, `ApplyOutput`, `SpecifyOutput.{staged, fresh, applied}`, `BuildOutput.candidate`. Check `skip_serializing_if` choices against the "omitted when" sentences in the docs.

**Exit:** every code in the hint table has a documented meaning in `cli-output-shapes.md` or a verb page, and every text line has a test asserting it.

---

## 5. Tests

The suites are the strongest evidence you have that the agents understood the behaviour; they are also agent-written, so read assertions for what they *do not* check.

### 5.1 How the suites work

- [ ] `tests/support/`: `provider.rs` (the scripted `Model`, `Source`, `Plugins`), `vcs.rs` (a scripted `Vcs` recording calls), `files.rs`, `store.rs` (seeding a current revision and a staged item; `project_staged`, `staged_document`), `scratch.rs`, `verbs.rs`. Understand how a test seeds "a repository holding revision X at commit Y" and how model answers are scripted per turn.
- [ ] `tests/amend/*.json` and `tests/specify/6-amendment.{json,md}`: read the fixtures. They are the expected outputs; a wrong fixture makes a wrong behaviour pass.

### 5.2 Scenario coverage map

Tick each once you have read the test and agree it asserts the scenario, not merely that it runs.

| Canvas scenario | Tests |
| --- | --- |
| S1 add a feature (added + changed) | `amend.rs::amend_classes`, `amend_sliced`, `amend_sliced_joined`, `amend_sliced_types` |
| S2 change a value, id kept | `amend_classes`, `amend_anchored` |
| S3 remove a behaviour (`retire`) | `amend_retired`, `amend_retired_slice`, `amend_retired_chain`, `amend_retired_design`, `amend_retired_section`, `amend_retired_overview`, `amend_retire_unmatched`, `amend_retire_own`, `amend_retire_contested[_applied]` |
| S4 add an existing service (new stems) | `amend_sliced*`, `amend_design*` |
| S5 re-read code, scope whole | `amend_scoped`, `amend_scoped_silent`; adapters `examples/README.md` Amend section |
| S6 change sources disagree | `amend_classes` (conflict), `apply.rs::apply_amendment_conflict`, `amend_retire_contested` |
| S7 resolve an existing conflict | `amend_resolves_conflict`, `amend_conflict_unchanged` |
| S8 apply out of date | `apply.rs::apply_amendment_stale` |
| S9 nothing changes | `amend_no_change`, `amend_apply_no_change` |
| S10 move a requirement to another stem | `amend_moved`, `amend_moved_slice`, `amend_rebrief_refused` |
| S11 specify after amendments (`--fresh`) | `specify.rs::store::specify_lineage_exists`, `store::specify_fresh` (nested module near line 3900), `apply.rs::apply_lineage_exists`, `apply_fresh`, `apply_fresh_over_baseline`, `specify_apply_fresh`; `runtime.rs` "Start over" |
| S12 build after an amendment | `build.rs::build_amendment_staged`, `build_candidate_staged`; `runtime.rs::amend_applied_built` |
| S13 two branches | `apply.rs::apply_over_merge`, `amend.rs::amend_hand_merged`, `show.rs::show_hand_merged`, `show_hand_mixed`; `runtime.rs::amend_branches_merged`, `amend_branches_hand_merged` |
| Slot and guards | `amend_over_staged`, `amend_baseline_refused`, `amend_no_revision`, `amend_target_required`, `show.rs::amendment_pending`, `show_amendment_none`, `corrupt_amendment`, `apply.rs::apply_refused`, `apply_target_required`, `command.rs::apply_grammar`, `no_target` |
| Design turn | `amend_design`, `amend_design_envelope`, `amend_design_types`, `amend_design_loss`, `amend_design_decision`, `amend_design_revised`, `amend_revised_dropped` |
| Recut past the cap | `amend_recut` |
| Grouping refusals | `amend_merged_standing` |
| Projection | `amend_shown`, `show.rs::show_amendment`, fixture `6-amendment.md` |
| `--apply` composition | `amend_apply`, `amend_apply_seal_refused`, `apply.rs::specify_apply*` |
| Runtime journey | `runtime.rs::amend_apply_runtime`, `amend_applied_built`, `apply_spec_occupied`, `specify_staged`, `build_label_merged` |

### 5.3 Gaps to probe

Read the suites with these in mind and log what is missing:

- [ ] Id reuse after retiring the highest id (§2.1). No test; write one to see what happens (keep it in `/tmp` or uncommitted).
- [ ] A whole-scope source that states nothing under a stem the base holds (module deleted). Expected by the code: nothing retired.
- [ ] An addition that pushes a stem over `SLICE_CAP` while every addition falls under a held sub-stem (§3.6).
- [ ] Two base requirements with one subject (§3.2), if the spec gate allows it.
- [ ] A trigger-only change (statement unchanged, route changed).
- [ ] `apply` where the pointer is unreadable (`pointer = None`) over a present key.
- [ ] `amend` where `store::current` reads the head's tree because the pointer's label fell behind (`behind`), then `--apply`: `is_at` compares against the head commit; confirm the stale check still means what the doc says.
- [ ] `amend --config` with the original `emery.toml` under default `delta` scope: is the result "nothing staged", and does the test suite exercise that reading?
- [ ] Concurrency: none is tested and none is claimed; confirm the docs say one operator, one slot.

> Agent: "For each `async fn amend_*` in `tests/amend.rs`, summarise in one line what it seeds, what it scripts, and what it asserts, and mark any assertion that only checks the run succeeded."

**Exit:** a list of untested behaviours, each with your judgement: document it, test it, or change it.

---

## 6. Documentation in `emery`

Review for accuracy against the code you have now read, then for readability by an operator who has not. Agent-written prose in this codebase runs to very long sentences with many em-dashes; judge whether a first-time reader can follow it, not only whether it is true.

### 6.1 Reference docs (mdBook, published)

- [ ] `docs/reference/cli/amend.md` (109): the class bullets against `change.rs`; the change-file section against `config.rs`; the options table against clap; the JSON list against `AmendOutput`; the text-mode sentence against `text.rs`; the `## Limits` list, which is where the honest caveats live. Is anything in `## Limits` actually a defect you would rather fix?
- [ ] `docs/reference/cli/apply.md` (74): the three repository kinds; the seal paragraph; "Sealing a candidate" (`lineage-exists`, `--fresh`, the label moved on identical content); "Sealing an amendment" (trailers, `amendment-stale`, conflicts seal); "`spec/` is the engine's" (both by-hand readings and their hints); "Refusals"; the JSON list.
- [ ] `docs/reference/cli/specify.md`: "Every run starts from new", "A lineage is amended or started over", "Where the revision lives", `--apply`, `--fresh`, `scope` read and ignored.
- [ ] `docs/reference/cli/show.md`: `amendment` artifact, `--staged`, the reading rule, the `amendment-pending` refusal.
- [ ] `docs/reference/cli/build.md`: a build never reads the candidate; `candidate` / `amendment … is staged` lines; "every slice built again" after an amendment; `spec-rewritten`.
- [ ] `docs/reference/cli/index.md`: the verb list and one-liners.
- [ ] `docs/reference/cli-output-shapes.md` (225): the `emery amend` and `emery apply` shapes and the failure envelope codes. Every code in `lib.rs::hint` should appear.
- [ ] `docs/index.md`, `docs/reference/index.md`, `docs/SUMMARY.md`: `amend` and `apply` listed and linked.
- [ ] `mdbook build docs` and click through the new pages; check cross-links (`amend.md#limits`, `apply.md#spec-is-the-engines`, `specify.md#the-emerytoml-config`).

### 6.2 Root and examples

- [ ] `README.md` "The live surface": the five verbs and the journey sentence.
- [ ] `examples/README.md` (220): the full journey script. You will run it in phase 8; read it now for whether an operator could follow it cold.
- [ ] `examples/source/prose/extract.md` and `examples/source/prose/references/greeting.md`: the mock source teaches `retire` and the `decision`-alone rule the journey depends on.
- [ ] `RELEASES.md` 0.39.0 (`### Added`, `### Changed`, `### Fixed`): every bullet of the four PRs is here. Check that the breaking changes an operator must act on are named plainly: the specification moves into the target repository, `specify` stages until `apply`, the `[target]` every sealing verb needs, grammar 8, the WIT `claim-kind` move (every component rebuilt), `--fresh`. Decide whether a 45 KB release section is readable or needs a short summary at the top.

### 6.3 Agent instructions

- [ ] `AGENTS.md`: the `crates/engine` row (the `amend`, `apply`, `store`, and revision paragraphs), the `crates/cli` row (verbs, config, text, hints), the `crates/adapter` row (`retire`), the `tests/` row (the `amend.rs` and `apply.rs` scenario lists), and the `## Invariants` and `## Testing` sections (the file is truncated past 100 KB in the agent context; read the tail in the editor). Verify sampled claims against the code: pick ten sentences at random and check each. This file is what future agent sessions read as truth; an error here propagates.
- [ ] `plugins/emery/rules/emery.mdc`, `plugins/emery/README.md`, `plugins/emery/skills/{amend,apply,specify,build}/SKILL.md`: the skill wrappers. `tests/plugin.rs::rule_mentions` and `every_skill` hold the mentions; read the amend skill's "Relay" section for whether an agent following it would say something false. Preview with `cursor-agent --plugin-dir plugins/emery` if you have it.
- [ ] `.cursor-plugin/marketplace.json` and `plugins/emery/.cursor-plugin/plugin.json`: `0.32.0`, agreeing with each other. Decide whether the plugin version should have moved with the new `amend`/`apply` skills.

> Agent: "Diff the behaviour described in `docs/reference/cli/amend.md` against `crates/engine/src/amend.rs`'s `///` on `amend` and against the `crates/engine` row of `AGENTS.md`; list each statement that appears in one and contradicts or is absent from the others."

**Exit:** no published sentence you know to be false; a list of sentences you would rewrite for a first-time reader.

---

## 7. `emery-adapters`

The engine owns the amend semantics; the adapters own how a source states a withdrawal or a technology decision, and the pin. Start in `../emery-adapters`.

### 7.1 The pin and the contract

- [ ] `Cargo.toml`: `emery-prose = "0.39.0"`, `emery-sdk = "0.39.0"`, `[patch.crates-io]` git pins; `Cargo.lock` resolves `emery-sdk` to `b3682cab` (emery `main`). Note that `AGENTS.md` says `emery-lexicon` is patched beside them and `Cargo.toml` does not patch it (it resolves beneath `emery-sdk` anyway). Log the doc drift.
- [ ] `git diff main` on `amend-review`: `supply-chain/config.toml` and `imports.lock` changed with the re-pin; `make ci` runs `vet`, so if phase 1 was green this is settled.
- [ ] `wit/emery.wit` in `emery` (`claim-kind` with `retire`) and `crates/adapter/src/source/evidence.rs` (`ClaimKind::Retire`, `requires_id`, `is_subject`, `required_extras` → `statement`, `refused_extras` → `criterion`, `callee`, `signature`, `route`, `message`; `Evidence::findings`). A `path` anchor is allowed on a `retire` (the documentation adapter uses one). Confirm the gate tests in `crates/adapter/tests/claims.rs`.
- [ ] `crates/sdk/prose/claims.md` `## Withdrawals` and the fail-closed gate bullet: the rule every adapter prompt links.

### 7.2 Prose: the `retire` and `decision` rules

- [ ] `adapters/intent/prose/extract.md`: the kind table row for `retire`; the paragraph on withdrawals (a "stays as it is" directive is a `requirement`; withdraw-and-restate is a `requirement` alone; `retire` + `decision` for a technology-bearing withdrawal); the `decision`-alone rule for a directive whose whole content is the technology behind an existing behaviour; the `## id derivation` bullets (stay under the existing concept's stem, never a respelling); the output contract. Read the `git diff main` hunk and ask whether the new sentences are consistent with each other and with `claims.md`.
- [ ] `adapters/documentation/prose/extract.md`: the same rules in the documentation adapter's terms; the `path` anchor on a `retire`; the worked example's `retire` claim (`password-reset.security-questions`).
- [ ] `tests/prose.rs` holds every `## Worked example` fence to the gate and every link to `PROSE` or the SDK runtime references; `cargo nextest run -p emery-adapters --test prose`.
- [ ] `tests/source.rs::documentation_retire` and `intent_retire`: what each scripts and asserts (the id leads with the stem, the statement is by role, `path` present / absent).
- [ ] `typescript` and `python` do not change for amend; `scope` is the engine's. Confirm with `rg -n "retire|scope" adapters/typescript adapters/python` that nothing crept in.

### 7.3 Examples and docs

- [ ] `examples/README.md`: the intro sentence, the `## Amend` section (the `orders` fixture, `scope = "whole"`, the python one-liner that deletes `cancel`, the expected `retired (N by scope)` and `slices: order-service (brief kept)` output, the trailers), the `[amend](amend/emery.toml)` link. You will run it in phase 8.
- [ ] `examples/amend/emery.toml`: `emery:typescript@0.12.0` over `../../evals/cases/orders/fixture/` with `scope = "whole"`, `[target] adapter = "emery:omnia@0.12.0"` with no `repository` (so the engine creates one under `.emery/vcs/repos/omnia`). Confirm the adapter versions match the built components' package versions.
- [ ] `AGENTS.md` in the adapters repository: the two-line diff on `amend-review`; the `## Invariants` bullet on `retire` (`a `retire` for the one that withdraws a behaviour … never one for a behaviour the source is merely silent on, a directive saying a behaviour stays as it is being a `requirement` …`); the `evals/` row says nothing about amend (the eval is specify-only). Decide whether an amend eval case is wanted and log it as future work.
- [ ] `RELEASES.md` in the adapters repository: is there an unreleased entry for the prose rule change and the pin? If not, log it.

**Exit:** the adapters' pin is the engine commit under review; the two prose adapters state the withdrawal rules consistently with `claims.md`; the live example is runnable.

---

## 8. Live desk check

Run the journeys end to end with the built binary and components. Use a scratch directory outside both repositories; the scripts in the two `examples/README.md` files are the recipe. Record the exact output of every step in the findings log; the text lines are part of the contract.

### 8.1 The engine journey (`emery/examples/README.md`)

- [ ] "Build and run": `specify` stages, `show … --staged`, `apply` seals, `show spec` reads the label, `build` builds, the second `build` has nothing to do, the fast-forward merge. Confirm `git log --format=%B emery/<id> -1` shows the `Revision` message with `Revision:` and `Base:` trailers and no `Parent:`.
- [ ] "Amend and build again": edit `docs/greeting.md`, `amend --config`, `show amendment`, `apply`, `build`. Confirm the `Parent:`, `Base:`, `Added:`, `Changed:`, `Retired:`, `Slices:` trailers and that `emery/<v0>` still stands.
- [ ] The inline brief beside the docs (`--config --description …`) and the contradicting brief (the conflict): confirm `conflicts:` on the counts line, `## Conflict` in the projection, and that `apply` seals it `[conflict]`.
- [ ] "Two branches": two clones, two amendments, the merge conflict on `spec/spec.json`, `spec-mismatch` with the by-hand hint, keep one side whole, the other amendment re-run over the merge. Watch what `show spec` says in the clone with no `.emery/storage` (the head's tree is read).
- [ ] "Start over": bare `specify` refused `lineage-exists` before any load (check the log shows no adapter loaded), `specify --fresh --apply`, the earlier labels standing, `build` over the lineage's code.

### 8.2 The adapters journey (`emery-adapters/examples/README.md` `## Amend`)

- [ ] Seal the baseline from the `orders` fixture with `emery:typescript` and `emery:omnia`, delete `cancel`, `amend`, `show amendment`, `apply`. Confirm `retired (N by scope)`, the `Note: retired by scope: …` lines, `slices: order-service (brief kept)`, and the `## Plan` block's `Note: the brief is the base's …`. Restore the fixture with `git checkout`.
- [ ] Optionally run `build` over the amended revision with the `omnia` target to see a shrunk slice rebuilt (S12). This is a long model run; decide whether it is in scope for this review.

### 8.3 Things to try that the examples do not

- [ ] `emery amend --config` with the unmodified `emery.toml` (default `delta` scope): expect `nothing staged`.
- [ ] `emery apply` twice: the second is `nothing-staged`.
- [ ] `emery amend` over a staged candidate: `amendment-pending` with the "candidate … where the amendment would go" description; `emery specify` over a staged amendment: `amendment-pending` with the other description.
- [ ] A `retire` of a behaviour the revision lacks (an intent brief "stop sending the weekly digest" over the greeting revision): `retire-unmatched`, the hint naming `emery show spec`.
- [ ] Hand-edit `spec/spec.json` on the labelled commit's branch after merging it, commit, and run `show spec`: which `spec-mismatch` description appears, and does the hint fit?
- [ ] Put a `spec/notes.md` in a fresh repository and run the first `apply`: `spec-mismatch` "the directory is the operator's".

**Exit:** every journey ran; every refusal you provoked printed the description and hint the docs promise.

---

## 9. Cross-cutting questions and seeded observations

Carry these through the phases; each needs a decision, not just a reading.

1. **Id reuse.** `ReqId::free_after` restarts at the base's highest id + 1. Retire the highest requirement, amend again, and the next added requirement takes the retired id. The docs claim ids are never reused. Decide: change the docs, or carry a high-water mark (the base's `parent` chain is not walked, and the spec stores no "ever used" set).
2. **Whole-scope retirement needs a surviving claim under the stem.** A deleted module retires nothing. Decide whether to document or extend (the source would need to state the stems it covers, which the WIT does not carry).
3. **No cap on standing requirements in the grouping turn.** Decide a limit or document the cost.
4. **Subject uniqueness** in the base spec is assumed by `Standing::requirement`. Confirm or guard.
5. **`same_trigger` treats "no trigger in the change" as "same".** Decide whether a change that drops a route should read as `Changed`.
6. **Settled-plan rule breaks are `server_error`.** `Pruned::assemble` can produce a plan `Floors::check` refuses (a stem grown past the cap by additions under held sub-stems). Confirm reachable or not; if reachable, a `server_error` is the wrong class.
7. **Seal ordering.** Git commit and label land before the pointer CAS and the slot clear. Decide whether the recovery story (the pointer is a cache; the head's tree is read) is enough and documented.
8. **CLI hint matches on an error description string** (`edited or merged by hand`). Decide whether a dedicated code is cleaner.
9. **Agent-written prose density.** The `///` docs, `AGENTS.md`, and the reference pages are accurate where sampled but long. Decide a standard for the public docs and apply it in a follow-up, not in this review.
10. **No `--replace` flag** and **no `show diff`** (canvas). Confirm both cuts are deliberate and the docs do not promise them.
11. **Plugin version** stayed at `0.32.0` through two new skills.
12. **Adapters `AGENTS.md`** names `emery-lexicon` in `[patch.crates-io]`; `Cargo.toml` does not.
13. **Eval coverage.** `evals/` grades `specify` only; amend has no live eval. Log as future work.

---

## 10. Findings log

Add a row per finding as you go. `Severity`: `defect` (wrong behaviour), `doc` (a false or missing statement), `test` (a gap), `style`, `question` (needs the maintainer). `Status`: `open`, `fixed-uncommitted`, `deferred`, `wontfix`.

| # | Phase | Repository | Location | Severity | Finding | Status |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 | emery | `crates/engine/src/revision/id.rs::free_after`, `docs/reference/cli/amend.md` | question | Added ids restart at base max + 1; a retired highest id is reused, docs say never reused | open |
| 2 | 0 | emery | `crates/engine/src/amend/change.rs::retire_by_scope` | question | A whole-scope source stating nothing under a stem retires nothing under it; not documented as a consequence | open |
| 3 | 0 | emery | `crates/engine/src/specify/basis.rs::with_standing` | question | Every standing requirement enters the grouping turn; no cap | open |
| 4 | 0 | emery-adapters | `AGENTS.md` root paragraph | doc | Says `emery-lexicon` is patched in `[patch.crates-io]`; `Cargo.toml` patches `emery-prose` and `emery-sdk` only | open |
| 5 | 0 | emery | `plugins/emery/.cursor-plugin/plugin.json` | question | Plugin version `0.32.0` unchanged across the new `amend` and `apply` skills | open |

---

## Appendix A. File inventory by phase

Line counts at the time of writing, so you can budget.

| Phase | Files |
| --- | --- |
| 2 | `crates/engine/src/revision.rs` 239 · `revision/id.rs` 168 · `revision/amendment.rs` 481 · `revision/diff.rs` 411 · `store.rs` 937 · `apply.rs` 158 · `vcs.rs` 1211 (parts) · `build.rs` 971 (parts) |
| 3 | `amend.rs` 347 · `amend/standing.rs` 104 · `amend/change.rs` 226 · `amend/plan.rs` 306 · `specify.rs` 525 (parts) · `specify/basis.rs` 870 (parts) · `specify/spec.rs` 462 (parts) · `specify/design/amend.rs` 488 · `specify/plan/amend.rs` 559 · `specify/plan.rs` 664 (parts) · `show.rs` 176 · `prose/grouping.md` 80 · `prose/slicing.md` 205 · `prose/design-format.md` 45 · `crates/sdk/prose/claims.md` 66 |
| 4 | `crates/cli/src/lib.rs` 571 · `config.rs` 385 · `text.rs` 314 |
| 5 | `tests/amend.rs` 3112 · `tests/apply.rs` 1131 · `tests/show.rs` 224 · `tests/specify.rs` 4120 (parts) · `tests/build.rs` 3466 (parts) · `tests/runtime.rs` 2952 (parts) · `tests/command.rs` 277 · `tests/plugin.rs` 246 · `tests/support/*` ~1250 · `tests/amend/*.json` · `tests/specify/6-amendment.{json,md}` |
| 6 | `docs/reference/cli/*.md` ~490 · `docs/reference/cli-output-shapes.md` 225 · `README.md` · `examples/README.md` 220 · `RELEASES.md` 0.39.0 section · `AGENTS.md` (rows named) · `plugins/emery/**` ~300 |
| 7 | `../emery-adapters/`: `Cargo.toml`, `Cargo.lock`, `AGENTS.md` (parts), `adapters/intent/prose/extract.md`, `adapters/documentation/prose/extract.md`, `tests/source.rs` (two tests), `tests/prose.rs`, `examples/README.md`, `examples/amend/emery.toml`; `emery/wit/emery.wit`, `emery/crates/adapter/src/source/evidence.rs` 664 (parts) |

## Appendix B. Useful commands

```bash
# emery
make ci
cargo nextest run --workspace --all-features --test amend --test apply --test show
cargo nextest run --workspace --all-features --test runtime -E 'test(/amend|apply|fresh|branch/)'
cargo doc --no-deps -p emery-engine --open
mdbook build docs
git log --oneline 626d0462..main                      # the four PRs and what came between
git show --stat d947499a 106960df 7ac91bdb b3682cab  # per-PR file lists

# emery-adapters
make ci
cargo nextest run -p emery-adapters --test source -E 'test(/retire/)'
cargo nextest run -p emery-adapters --test prose
git diff main --stat                                 # the re-pin and the prose rules

# in a sealed scratch project
git log --format=%B emery/<id> -1                    # the Revision message and trailers
git branch --list 'emery/*'
emery show amendment
emery --format json amend --config | jq .tally
```
