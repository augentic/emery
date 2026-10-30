# Review plan: `emery amend` and `emery apply`

A step-by-step guide for reviewing the `amend` and `apply` implementation across `augentic/emery` (this repository) and `augentic/emery-adapters` (`../emery-adapters`). All code and documentation in these PRs were written by AI agents. Use this checklist to verify the implementation, log findings in Section 10, and adjust the review scope as you go.

Conventions:

- `[ ]` marks a review step. `Agent:` provides a ready-made prompt you can give an agent session to gather facts; always verify its output against the source code.
- File paths are relative to the `emery` root unless prefixed with `../emery-adapters/`.
- Do not commit, push, or open a PR during a review session. Leave fixes uncommitted in the working tree or record them in the findings log.
- Each section ends with an **Exit** criterion describing what you should understand or verify before moving on. Complete Sections 0 through 2 in order, as later sections rely on the terminology and contracts introduced there.

---

## 0. Orientation

### 0.1 Core Concepts and Glossary

The codebase and `AGENTS.md` use compressed project-specific terminology. Keep this translation table in mind as you read:

- **Revision** (`crates/engine/src/revision.rs::Revision`): A complete, versioned specification bundle consisting of three JSON documents: `spec.json` (requirements and scenarios), `design.json` (architectural sections and data types), and `plan.json` (build slices and their dependencies). Identified by a content-addressed SHA-256 hex digest (`<id>`).
- **Staging Slot** (`crates/engine/src/store.rs::Slot`, stored under `.emery/storage/`): Local scratch storage holding at most one unsealed item waiting for operator review—either a **Candidate** (a brand-new `Revision` produced by `emery specify`) or an **Amendment** (an incremental change produced by `emery amend`).
- **Sealing** (`crates/engine/src/store.rs::Sealing` / `commit`, run via `emery apply`): Committing the staged candidate or amendment into the target Git repository as `spec/{spec,design,plan}.json` in a single commit (`Revision <id>`), creating or moving the Git branch `emery/<id>`, updating the cached pointer in `.emery/storage`, and clearing the staging slot.
- **Label**: The Git branch `emery/<id>` in the target repository pointing to a sealed revision commit (or later advanced by `emery build` as slices are merged and verified).
- **Target Repository** (`crates/engine/src/vcs.rs::BuildRepo`): Where sealed revisions and built code live, configured via `[target] repository` in `emery.toml`:
  - `"."` (`BuildRepo::Project`): The current working repository.
  - `"<url>"` (`BuildRepo::Target { url: Some(..), .. }`): A remote repository cloned into `.emery/vcs/repos/<name>`.
  - Omitted (`BuildRepo::Target { url: None, .. }`): A local Git repository initialized and managed by Emery at `.emery/vcs/repos/<name>`.
- **Lineage** (`parent` field on `Spec`, `Design`, and `Plan`): The chain of revisions formed when a revision records the ID of the previous revision as its `parent`. Once a revision has a `parent` (meaning at least one amendment or second revision has been sealed), bare `emery specify` refuses to overwrite it (`lineage-exists`) unless `--fresh` is passed to start a new, parentless history.
- **Standing** (`store::Standing`, `crates/engine/src/store.rs`): The current state of the target Git repository—its `HEAD` commit, the active sealed revision (`Current`), and whether the cached `emery/<id>` branch has fallen behind `HEAD`.
- **Baseline** (`amend::baseline::Baseline`, `crates/engine/src/amend/baseline.rs`): The existing requirements from the currently sealed revision ("standing requirements"), injected into the model's grouping prompt under the reserved source name `baseline` so incoming change claims can be matched against them.
- **Stem, Sub-stem, and Floor**: Every requirement has a dotted subject like `orders.cancel` or `orders.items.add`. The first segment (`orders`) is its **stem**; the first two segments (`orders.items`) are its **sub-stem**. The build planner groups requirements into build **slices** by a minimum grouping unit called a **floor** (`crates/engine/src/specify/plan.rs::Floors`)—normally the stem, or the sub-stem if a single stem has more than `SLICE_CAP` (12) requirements.
- **Trigger** (`crates/adapter/src/trigger.rs::Trigger`): An inbound entry point attached to a requirement—either an HTTP route (`Trigger::Http`, e.g., `POST /orders`) or a message/topic name (`Trigger::Message`). Requirements sharing a trigger must be placed in the same build slice.
- **Turn / Brief**: A **turn** is a single structured LLM request (with optional automatic validation-and-correction rounds). In `crates/engine/src/brief.rs`, the `Brief` trait represents a typed prompt + JSON schema + verifier (`verify`) sent to the model. (Elsewhere, "brief" can also mean the prose paragraphs describing a build slice in `plan.json`, or a short inline text source passed via `--description`.)
- **`retire` claim & `scope = "whole"`**: Two ways an amendment removes ("retires") an existing requirement:
  1. Explicit withdrawal: A prose source (`intent` or `documentation`) emits a claim of kind `retire` naming the requirement's subject.
  2. Implicit withdrawal ("retired by scope"): A code source configured with `scope = "whole"` in `emery.toml` is treated as exhaustive for any stem it mentions; any existing requirement under that stem not matched by a claim from that source is automatically retired.

### 0.2 What shipped, and where

The feature landed on `main` of `emery` across four pull requests:

| PR | Title | Summary of changes |
| --- | --- | --- |
| [#374](https://github.com/augentic/emery/pull/374) | Apply spec before build | Moves sealed specifications into the target Git repository under `spec/`. `specify` only stages a candidate; the new `apply` subcommand commits ("seals") it to Git (`--apply` does both). Adds `BuildRepo::head`, `RevisionMessage`, `.emery/vcs/revision` working copy, the `spec-rewritten` build guard, and JSON schema version `EMERY = 8` (adding `parent`). |
| [#375](https://github.com/augentic/emery/pull/375) | Amend phase 1, diff computation | Adds the `retire` claim kind to the WIT contract, `amend.rs`, `amend/baseline.rs`, `amend/change.rs`, `revision/amendment.rs`, the `Amendment` document format and Markdown renderer (`emery show amendment`), and the initial hidden `amend` subcommand. |
| [#380](https://github.com/augentic/emery/pull/380) | Amend plans and designs | Adds incremental plan and design updates: `amend/plan.rs` (`Pruned`), `specify/plan/amend.rs` (`SliceAmend` model turn), `specify/design/amend.rs` (`DesignAmend` model turn), implicit retirement via `scope = "whole"`, and `DesignDiff` / `SliceChange` tracking on `Amendment`. |
| [#383](https://github.com/augentic/emery/pull/383) | Apply (merge) amendment to base spec | Extends `apply` to seal staged amendments, adds `amendment-stale` and `lineage-exists` guards, adds `specify --fresh`, supports reading revisions from merged Git branches, detects manual edits to `spec/` (`spec-mismatch`), exposes `amend` publicly in CLI help/completions, and updates documentation and Cursor skills. |

In the sibling `../emery-adapters` repository, branch `amend-review` is two commits (`656036a`, `96d25d6`) ahead of `main`: it updates the `emery-sdk` Git dependency to `emery` `main` (`b3682cab`) and updates the extraction prompts for `intent` and `documentation` to teach `retire` and `decision` claims (building on earlier PR [#186](https://github.com/augentic/emery-adapters/pull/186)).

Repository state at the start of review:

- `emery` branch `amend-review` is at `main` (`b3682cab`), plus this `AMEND-REVIEW.md` file.
- `emery-adapters` branch `amend-review` is at `96d25d6` (two commits ahead of `main`), with `Cargo.lock` pinning `emery-sdk` to `b3682cab`.
- Both workspaces are at version `0.39.0` (unreleased notes in `RELEASES.md`). The Cursor plugin is versioned separately at `0.32.0` (`plugins/emery/.cursor-plugin/plugin.json` and `.cursor-plugin/marketplace.json`, kept in sync by `tests/plugin.rs::versions_agree`).

### 0.3 Intentional deviations from the original design canvas

The original planning canvas (`~/.cursor/projects/Users-goldie-git-augentic-emery/canvases/emery-amend-plan.canvas.tsx`) defined scenarios S1–S13 and implementation steps. Several design details changed during implementation. The table below lists known differences so you do not waste time rediscovering them—verify during the review whether each is sound and properly documented:

| Original canvas design | Actual implementation | Section |
| --- | --- | --- |
| Overwriting a staged item required `--replace` | No `--replace` flag. `emery amend` freely overwrites a staged amendment, but refuses with `amendment-pending` if a `specify` candidate is staged. `emery specify` always refuses if an amendment is staged. | §2.1, §4 |
| A conflicted base requirement is passed to the grouping prompt as multiple separate entries | Passed as a single entry formatted as `[conflict] <statement A> \| <statement B>` (`Requirement::stated`). | §3.2 |
| Open question: how to cap the number of existing requirements sent to the grouping prompt on large specs | No cap implemented; every existing requirement in the base spec is included in the grouping prompt. | §3.3, §9 |
| Add `emery show diff` subcommand | Omitted. Diffs are shown in the JSON output and text summary of `specify` and `apply`, and via `emery show amendment`. | §2.3 |
| Open question: whether to redraft the spec preamble when an amendment adds a new stem | Never redrafted; the base spec's preamble is always copied unchanged. | §3.4 |
| Open question: whether to keep existing test scenarios when only a requirement's statement changes | Never kept; a `Changed` requirement is always redrafted from scratch (statement and scenarios). | §3.4 |
| Open question: when all requirements in a slice move to a new stem, rename the slice or remove it? | The emptied slice is removed; the slicing turn creates a new slice (or joins an existing one) and reassigns its data types. | §3.6 |
| Newly added requirement IDs are never reused across the entire revision lineage | `ReqId::free_after` simply computes `max(base_spec_ids) + 1`. If the highest-numbered requirement was retired in revision $N$, revision $N+1$ will reuse that ID. | §2.1, §9 |
| `emery apply` writes no `.gitignore` when initializing a managed repository | Confirmed: `store::commit` only writes `spec/*`; the first `emery build` writes `.gitignore` (`IGNORE`) if the target adapter's `Init` task did not scaffold one. | §2.2 |
| Keep `emery amend` hidden until final release | Publicly visible in `--help` and shell completions. | §4 |
| Separate `amend/stage.rs` module | Staging logic lives in `crates/engine/src/store.rs` (`stage_amendment`, `Slot`). | §2.1 |

### 0.4 Recommended reading order

Before diving into the Rust source, skim these documents to build context:

- [ ] `docs/reference/cli/apply.md` and `docs/reference/cli/amend.md`: The user-facing CLI specification and the clearest prose description of how `amend` and `apply` work. Keep both open during Sections 2 and 3; any mismatch between these docs and the code is a review finding.
- [ ] `examples/README.md` ("Build and run", "Amend and build again", "Two branches", "Start over"): Concrete shell transcripts showing the end-to-end workflow you will test in Section 8.
- [ ] `AGENTS.md` (in `emery`): Skim `## Vocabulary` and the `## Map` rows for `crates/engine`, `crates/cli`, and `crates/adapter`. Note that `AGENTS.md` is an agent-maintained wall of text (148 KB)—treat it as claims to audit in Section 6, not as an introductory guide.
- [ ] `../emery-adapters/AGENTS.md`: Skim `## Map` and the first two bullets of `## Invariants` to see how source and target WASM adapters are structured.

> Agent: "Give me a one-page summary of how `emery specify`, `emery amend`, `emery apply`, `emery show`, and `emery build` interact with `.emery/storage` and the target Git repository."

**Exit:** You can explain what `specify`, `amend`, `apply`, `show`, and `build` read and write, and where the canonical specification files live once sealed.

---

## 1. Set the ground

Verify that both repositories build and pass all checks before inspecting code:

- [ ] Run `make ci` in `emery` and record the result. (Runs `fmt-check`, `lint`, `test`, `test-docs`, `docs`, `vet`, and `deny`.)
- [ ] Run `make ci` in `../emery-adapters` and record the result.
- [ ] Run `cargo nextest run --workspace --all-features --test amend --test apply --test show` in `emery` to see the fast integration suites run in isolation.
- [ ] Run `cargo doc --no-deps -p emery-engine --open` and read the top-level module docs for `amend`, `apply`, `store`, and `revision`.
- [ ] Build the release binaries and WASM components needed for the Section 8 live test: `cargo build --release` and `cargo build --examples --target wasm32-wasip2 --release` in `emery`, and `cargo build --workspace --target wasm32-wasip2 --release` in `../emery-adapters`.

**Exit:** Both repositories pass `make ci`, rustdoc builds cleanly, and release binaries/WASM components are compiled.

---

## 2. Contracts and persistence

Review the data types and storage layer before reading the pipeline logic that operates on them.

### 2.1 The revision schema and staging slot

Files: `crates/engine/src/revision.rs` (239 lines), `crates/engine/src/revision/id.rs` (168), `crates/engine/src/revision/amendment.rs` (481), `crates/engine/src/store.rs` (937).

- [ ] `revision.rs`: Check schema version `EMERY = 8`, the `Document` trait (`from_json`, `to_json`, `to_markdown`), and `Revision::read` / `identify` / `documents`. Inspect `Revision::findings`, which enforces cross-document consistency across `spec.json`, `design.json`, and `plan.json` (every spec requirement appears in exactly one plan slice, every design data type is owned by exactly one slice, and `plan.json` references no nonexistent requirement or type). Question: does `Revision::findings` catch all inconsistent states caused by a manual Git merge that takes `spec.json` from one branch and `design.json` / `plan.json` from another?
- [ ] `id.rs`: `ReqId` and `SliceId` wrap `NonZeroU32`. Notice that `ReqId::free_after` simply starts at `max_id + 1` of the current base spec. Decide whether reusing a retired highest ID in a subsequent amendment is acceptable or a bug (see §0.3 and §9)—note that `docs/reference/cli/amend.md` currently claims added requirements are "numbered past every id the revision ever used".
- [ ] `amendment.rs`: Inspect the `Amendment` struct (`emery`, `base`, `changes`, `spec`, `design`, `plan`, `sections`, `slices`), its `#[serde(deny_unknown_fields)]` attribute, `identify` (SHA-256 over the single serialized JSON document), `is_change`, `stems`, `conflicts`, and `count`. Compare its `Display` implementation (which powers `emery show amendment`) against `tests/specify/6-amendment.md` and `docs/reference/cli/show.md`. Check `Change` (`before` / `after` values for each `ChangeClass`, `scope` populated only on scope-retired items) and `SliceChange::of` / `keeps_brief`.
- [ ] `store.rs` (top half): Inspect `Slot` (the JSON metadata record at `STAGED_KEY`: `id`, `kind`, `base`, `fresh`), `StagedKind::documents` / `noun`, `stage` and `stage_amendment` (compare-and-swap on `STAGED_KEY`, followed by `prune` of the replaced blobs in `STAGED_CONTAINER`), `no_amendment` and `no_candidate` (the `amendment-pending` guards), `staged` / `staged_amendment` / `staged_item`, and `clear`. Review questions:
  - If `STAGED_KEY` contains corrupt JSON, `token` treats the slot as empty (allowing `stage` to overwrite it), whereas `Slot::read` returns an error (causing `apply` and `show` to fail). Confirm that this asymmetry is intentional.
  - `swap` only prunes blobs when the new staged ID differs from the old one, so re-staging identical content does not delete the blobs just written. Confirm that `BlobStore::put` is idempotent.
  - After sealing, `clear` re-reads `STAGED_KEY` and deletes whatever is currently in the slot without compare-and-swap. Confirm whether single-process/single-operator execution is an assumed invariant.

> Agent: "List every reader and writer of `STAGED_KEY`, `STAGED_CONTAINER`, and `REVISION_KEY` across `crates/engine`, along with the function name and any precondition guard it checks."

### 2.2 Resolving the current revision and sealing to Git

Files: `crates/engine/src/store.rs` (bottom half: `Standing`, `Current`, `Sealing`, `Occupancy`, `current`, `commit`, `seal_copy`, `read_tree`), `crates/engine/src/vcs.rs` (1211 lines), `crates/engine/src/apply.rs` (158), `crates/engine/src/build.rs`.

- [ ] `store::current`: Traces how the engine locates the active sealed revision:
  1. Reads the cached branch name from `REVISION_KEY` in `.emery/storage` and queries `repo.head`.
  2. If the cached `emery/<id>` branch exists in Git and is a descendant of `HEAD` (`descends(head, label)`), checks out a temporary worktree at `.emery/vcs/revision` for that branch and reads `spec/{spec,design,plan}.json`.
  3. Otherwise, falls back to reading `spec/{spec,design,plan}.json` directly from `HEAD` (recording `behind = Some(..)` if the cached branch existed but fell behind `HEAD`).
  Check each `tracing::warn!` branch in `store::current`—because the default log level is `info`, operators will see these warnings on stderr.
- [ ] `Current::read_tree`: Inspect `Occupancy` (`Absent`, `Held` when `spec/` contains only `design.json`, `plan.json`, and `spec.json` as regular files, or `Other`). When reading from a labelled `emery/<id>` commit, any mismatch or invalid file is an immediate `spec-mismatch`. When reading from an unlabelled `HEAD` commit, unparseable JSON (for example, committed Git merge conflict markers) or cross-document inconsistencies (`Revision::findings`) are reported via `hand_merged`, which includes the exact substring `"edited or merged by hand"` in the error description. Note that the CLI hint logic in `crates/cli/src/lib.rs` matches on that substring—decide whether substring matching on error descriptions is acceptable or should use a distinct error code.
- [ ] `Sealing` constructors:
  - `Sealing::candidate`: Sets `parent` to the current revision's ID, cuts the worktree from the current revision's commit (or `HEAD`), and enforces `Standing::no_lineage` (refusing with `lineage-exists` if the current revision already has a `parent`).
  - `Sealing::fresh`: Used for candidates staged with `specify --fresh`; sets `parent: None` and cuts the worktree from `HEAD`, starting a new lineage while leaving older `emery/<id>` branches untouched.
  - `Sealing::amendment`: Verifies that the repository's current revision ID and commit still match `amendment.base` (refusing with `amendment-stale` otherwise) and builds the commit trailer summary (`Summary::of`).
  Verify that these three constructors match "Sealing a candidate" and "Sealing an amendment" in `docs/reference/cli/apply.md`.
- [ ] `store::commit` and `seal_copy`: Checks out `.emery/vcs/revision` at `sealing.over` (or initializes a new managed repository via `repo.init` when sealing the very first revision), verifies `spec/` is not `Occupancy::Other`, writes the three JSON files via `Files::write`, runs the secret scanner (`secrets::refusal`) on pending changes, creates the Git commit with `RevisionMessage`, force-updates the `emery/<id>` branch, updates `REVISION_KEY` via compare-and-swap, and clears the staging slot. Review questions:
  - The Git commit and `emery/<id>` branch are created *before* the compare-and-swap on `REVISION_KEY` and `clear`. If updating `REVISION_KEY` fails, the revision is already committed and labelled in Git. On the next run, `current` will only find it if `emery/<id>` is at `HEAD` (or matches the old `REVISION_KEY`). Is this failure ordering acceptable?
  - `seal_copy` rejects `Occupancy::Other` (e.g., an operator-created `spec/notes.md`), but silently overwrites `Occupancy::Held`. Confirm this is documented for `--fresh` seals over an existing `spec/`.
- [ ] `vcs.rs`: Inspect `SPECIFICATION`, `REVISION`, `RevisionMessage` (`Display` and `parse`, including `Parent:`, `Base:`, `Added:`, `Changed:`, `Retired:`, and `Slices:` trailers), `Summary::of`, `SliceSummary`, `BuildRepo::head`, `BuildRepo::init`, `BuildRepo::label`, `BuildRepo::is_created`, and `WorkingCopy::checkout`. Confirm that `Message::parse` (which reads slice merge commits during `emery build`) ignores `Revision <id>` commits because their first word is not a `SLICE-NNN` ID.
- [ ] `apply.rs`: `apply` calls `store::current` followed by `seal` (`seal` is also called directly by `specify --apply` and `amend --apply`). Verify the `# Errors` doc comment against `store.rs` and `docs/reference/cli/apply.md`.
- [ ] `build.rs`: Check the changes introduced in PR #374: reading `store::current` before loading the target adapter; populating `BuildOutput.candidate` when an unsealed candidate or amendment sits in the staging slot; `Progress::initialise` running `Task::Init` and writing `.gitignore` (`IGNORE`) if the repository was created by Emery and has none; and `gated()` rejecting any build or verify turn that modifies files under `spec/` (`spec-rewritten`).

> Agent: "Trace `emery apply` when sealing a staged amendment from `apply::apply` through to `store::commit`, listing every `Vcs`, `Files`, `StateStore`, and `BlobStore` call in execution order and every early error return."

### 2.3 The revision diff

File: `crates/engine/src/revision/diff.rs` (411 lines).

- [ ] `Diff::between(from, outgoing, incoming)`: When `incoming.spec.parent == Some(from)` (same lineage), requirements are paired strictly by `ReqId` and any omitted requirement is recorded in `spec.retired`. Across unrelated lineages (`parent != Some(from)`), requirements are paired heuristically by stem + overlapping source anchor (and then by ID), and omitted requirements are recorded in `spec.removed`. Also check `DesignDiff::between`, `PlanDiff::between`, and `slice_differences` (confirming `trigger` changes are detected and `BRIEF` is recorded for `SliceChange::of`).
- [ ] Decide whether omitting a standalone `emery show diff` subcommand (in favor of `emery show amendment` and the diff summaries on `specify` / `apply`) leaves any usability gap.

**Exit:** You can sketch the state transitions for both the staging slot (empty, candidate, fresh candidate, amendment) and the target repository (uninitialized, no `spec/`, labelled revision, branch behind `HEAD`, unlabelled revision on `HEAD`, hand-merged conflict), and name the error code returned for each invalid transition.

---

## 3. The amend pipeline, in execution order

Read `crates/engine/src/amend.rs` (347 lines) from top to bottom first—it orchestrates the entire `amend` workflow—and then inspect each submodule in the order `amend` calls them.

### 3.1 Preconditions and source extraction

- [ ] `amend.rs` (lines ~120–145): Check the precondition sequence before `Bound::extract_all`:
  1. `Bound::all` validates source configs and rejects any source named `"baseline"` (`specify::BASELINE`).
  2. `target_required` ensures a `[target]` is configured (`build-target-required`).
  3. `store::current` reads the target repository and `not_generated` fails with `spec-not-generated` if no sealed revision exists.
  4. `current.revision?` fails with `spec-outdated` if the sealed revision uses an older schema version.
  5. `store::no_candidate` fails with `amendment-pending` if an unsealed `specify` candidate occupies the staging slot.
  Confirm that all five checks run *before* `Bound::extract_all` (so no WASM adapter is loaded, no Git repository is cloned, and no LLM turn is spent when a precondition fails), matching the guarantees in `docs/reference/cli/amend.md`.
- [ ] `crates/engine/src/specify.rs`: Inspect `Scope` (`Delta` by default, or `Whole`), `SourceConfig.scope`, `Extract.{scope, kind, rank}`, and `Bound::extract_all` (shared between `specify` and `amend`). Confirm that `specify` ignores `Scope`.
- [ ] `crates/engine/src/authority.rs`: Check `Rank::below` (returns the next lower authority rank, saturating at ` u32::MAX`), used to rank the existing specification below all incoming change sources.

### 3.2 Standing requirements and deriving `ChangeSet`

Files: `crates/engine/src/amend/baseline.rs` (104 lines), `crates/engine/src/amend/change.rs` (226).

- [ ] `Baseline::new(&spec, &extracts)`: Assigns the existing specification an authority rank one step below the lowest-ranked change source (`max_extract_rank.below()`). `contributors()` converts each requirement in the base spec into a `Contributor` under source name `"baseline"`, setting `id` to the requirement's **subject** (not `REQ-NNN`), `statement` to `Requirement::stated()` (either the joined body paragraphs, or `[conflict] <stmt A> | <stmt B>` if the base requirement is currently conflicted), and `citations` to the requirement's original source anchors. Question: `Baseline::requirement(subject)` looks up base requirements by subject using `.find(...)`. Does the spec verifier (`specify/spec.rs` / `revision/spec.rs`) guarantee that no two requirements in a valid `Spec` share the same `subject`? If not, `matched` could resolve to the wrong requirement.
- [ ] `ChangeSet::states(&extracts)`: Returns `true` if the extracted claims contain at least one `Requirement` or `Retire` claim. When `false` (for example, an amendment containing only `decision` or `type` claims), `amend` skips both the grouping turn and the spec-drafting turns, calling `ChangeSet::unchanged(&standing)` to mark every base requirement `Unchanged`.
- [ ] `ChangeSet::derive(&standing, bases)`:
  1. First checks `Basis::withdrawals` for any group containing only `retire` claims that did not match a standing requirement; if found, fails immediately with `retire-unmatched`.
  2. For each group (`Basis`) that matched a standing requirement: strips `"baseline"` from `sources` and sets `before = matched.stated()`. Classifies the requirement as:
     - `Unchanged` if no change claims joined the group (`sources.is_empty()`), or if the winning statement, trigger, and subject are identical to the base requirement's.
     - `Retired` if the winning authority class is a `retire` claim (`basis.withdraws()`), with `after: None`.
     - `Conflict` if top-ranked change sources disagree (`basis.statement().is_none()`), with `after: None`.
     - `Changed` otherwise, taking the change claim's subject (allowing a requirement to move to a new stem).
  3. For each group with no standing requirement: assigns the next `ReqId` from `standing.free_ids()` and classifies it as `Added` (or `Conflict` if the new claims disagree).
  4. Collects `basis.amended(id, trigger)` into `drafted` for every `Added`, `Changed`, and `Conflict` entry so `SpecBrief::amending` can draft their Markdown bodies and scenarios.
  Review questions on `ChangeSet::derive`:
  - If a change claim matches a standing requirement with the exact same statement and subject, but a *different* `Trigger` (route/message), it is classified as `Changed` and sent to `SpecBrief::amending`. Does the spec-drafting prompt handle a trigger-only change cleanly?
  - In `same_trigger`, when `basis.trigger` is `None`, it returns `true` and falls back to `matched.trigger` via `.or_else(...)`. That means an amendment that rewords a requirement without re-emitting a `contract` claim preserves the base requirement's HTTP route or message trigger, and cannot remove a trigger without replacing it. Confirm this is intentional.
- [ ] `ChangeSet::retire_by_scope(&extracts)`: For every extract from a `SourceKind::Behaviour` adapter (e.g., `typescript` or `python`) configured with `Scope::Whole`, collects the stems of all `Requirement` claims emitted by that source. Any base requirement under one of those stems that is still `Unchanged` with empty `sources` is reclassified to `Retired` with `scope: Some(source_name)`. Review questions:
  - Because `covered` stems are derived from the `Requirement` claims the source emitted in this run, if an entire module/stem is deleted from the codebase so the adapter emits *zero* claims under that stem, `retire_by_scope` will not retire that stem's requirements. Decide whether to document this limitation in `docs/reference/cli/amend.md` or change how whole-scope stems are tracked.
  - Confirm that `retire_by_scope` runs *before* `Pruned::new` and `DesignAmend::new`, so scope-retired requirements are pruned from the build plan and design just like explicitly retired ones.

> Agent: "List every branch in `ChangeSet::derive` across `(matched vs. unmatched) × (sources empty vs. withdraws vs. statement Some vs. statement None) × (same vs. different subject/trigger)`, the `ChangeClass` each produces, and which test in `tests/amend.rs` exercises it."

### 3.3 Grouping claims against standing requirements

Files: `crates/engine/src/specify/basis.rs` (870 lines), `crates/engine/prose/grouping.md` (80).

- [ ] `GroupingBrief::with_standing`: Places incoming change claims first in the numbered list, followed by standing requirements under `## Standing requirements`. Note how `retire` claims are handled:
  - In `specify` (`GroupingBrief::new`), `retire` claims are ignored.
  - In `amend` (`GroupingBrief::with_standing`), if a single source emits both a `retire` claim and a `requirement` claim with the exact same ID, `assemble` drops the `retire` claim with a `tracing::debug!` (treating withdraw-and-restate from the same source as a restatement). Remaining `retire` claims become contributors with `withdraws = true`, rendered with a `(retire)` tag in the prompt.
  - `Hint::all` scans for change claims whose source file/line anchor overlaps (`Cited::overlaps`) a standing requirement's citation from the same source, emitting a non-binding hint under `## Baseline` (`claim N anchors at ..., where standing requirement M was read`).
- [ ] `GroupingBrief::verify`: Checks that no group merges two distinct standing requirements, and no equivalence class mixes a `(retire)` claim with a regular requirement statement.
- [ ] `prose/grouping.md` (`## Standing requirements`): Read the prompt instructions and worked example alongside `GroupingBrief::verify` to confirm the prompt rules and code checks agree.
- [ ] Prompt scaling: `with_standing` includes *all* standing requirements in a single grouping turn. On a large specification (e.g., 200+ requirements), this single prompt could become a bottleneck or hit token/time limits. Log this in Section 9/10 to decide whether a cap or pre-filtering step is needed later.

### 3.4 Redrafting the specification (`SpecBrief`)

Files: `crates/engine/src/specify/spec.rs` (462 lines: `SpecBrief::amending`, `SpecBrief::amend`, `Review::technology`), `crates/engine/src/specify/sections.rs`.

- [ ] `SpecBrief::amending(&extracts, &drafted)`: Splits only the `drafted` bases (`Added`, `Changed`, `Conflict`) into chunks of up to `SPEC_CHUNK` (25) requirements, omitting the preamble request. If `drafted` is empty (e.g., a retire-only amendment), returns an empty vec so zero `spec-draft` turns are run.
- [ ] `SpecBrief::amend(base, parent, &changes, drafts)`: Reconstructs the amended `Spec` in `ReqId` order—copying `base.preamble`, cloning the base `Requirement` struct directly for `Unchanged` entries, inserting the newly drafted `Requirement` for `Added`/`Changed`/`Conflict` entries, omitting `Retired` entries, and setting `parent: Some(parent)`.
- [ ] Technology lexicon check (`Review::technology`): Confirm that amended requirement drafts are checked against the `Lexicon` (built from fixed technology names plus packages in the change's `call` claims) just like `specify` drafts.

### 3.5 Updating the design (`DesignAmend`)

Files: `crates/engine/src/specify/design/amend.rs` (488 lines), `crates/engine/src/revision/design.rs` (`without_types`, `data_types`, `Block`), `crates/engine/prose/design-format.md` (`## Amending`).

- [ ] Pruning orphaned types: Before checking whether a design turn is needed, `amend` removes `Block::Type` entries whose keys belonged to slices that were completely emptied by requirement retirements (`Pruned.orphaned` → `Design::without_types`), dropping any design section left with zero blocks.
- [ ] `DesignAmend::new`: Returns `Some(brief)` if the change's claims inform at least one design section (`Outline::informed`, e.g., `decision`, `call`, `contract`, `type`, or `excerpt` claims) *or* if an existing paragraph in the pruned design cites a `Retired` requirement ID (`describes`). Otherwise returns `None` and reuses the pruned base design with zero LLM turns. When a turn *does* run, it always redrafts `## Overview` plus each informed or retirement-affected section, copying all other sections from the base design in canonical section order (`into_output`).
- [ ] `DesignAmend::tighten` and `verify`: Inspect `Outline::blocks` / `references` and the paragraph-loss check (`DesignAmend::loss`). For every base text paragraph in a redrafted section:
  - If all requirement IDs it cites were `Retired` (`withdrawn`), the model may drop or reword it freely (and if every block in a non-Overview section is `withdrawn`, the model may return an empty block list to remove the section).
  - Otherwise, the paragraph must either appear verbatim in the new section (ignoring whitespace differences) or have its 1-based index listed in `revised`.
  - Any paragraph listed in `revised` must be reworded, not deleted: every non-retired `REQ-NNN` it originally cited must still be cited somewhere in the redrafted section. Furthermore, outside `## Overview`, when the amendment drafts requirements, revising a paragraph that only cites `Unchanged` requirements is rejected as a finding.
- [ ] Compare `prose/design-format.md` (`## Amending`) against `DesignAmend::verify` and the "A design paragraph goes stale while nothing it cites is retired" bullet in `docs/reference/cli/amend.md`.
- [ ] Concurrency in `amend.rs`: Note that the `DesignAmend` turn runs concurrently with the `SpecBrief` chunks via `future::try_join`, taking `&drafted` (`Basis` outlines) rather than the finished `Spec` drafts.

### 3.6 Updating the build plan (`Pruned` and `SliceAmend`)

Files: `crates/engine/src/amend/plan.rs` (306 lines), `crates/engine/src/specify/plan/amend.rs` (559), `crates/engine/src/specify/plan.rs` (`Floors`, `SLICE_CAP`, `drafts`), `crates/engine/prose/slicing.md` (`## Amending`).

- [ ] `Pruned::new(&base_plan, &base_spec, &changes, parent)`:
  1. Removes all `Retired` requirement IDs from their slices (`without`). If a slice loses all of its requirements, the slice is removed from the plan, its owned `types` are recorded in `orphaned` (so `Design::without_types` can remove them), and any slice whose `depends_on` named the removed slice inherits the removed slice's own `depends_on` transitively (`spliced`).
  2. Removes any `Changed` requirement whose stem changed (`moved`) from its old slice. If that empties the old slice, the slice is removed and its dependencies are spliced, but its `types` are *not* added to `orphaned` (they remain in the design and become unowned keys for `own` / `SliceAmend` to reassign).
  Verify that `spliced` cannot infinite-loop on cyclic inputs (it tracks a `visited` set) and filters out self-dependencies.
- [ ] `Pruned::assemble(&spec, &types, &drafted)`: Attempts to place remaining requirements (`place`) and design data types (`own`) deterministically without an LLM turn:
  - `place`: Places each unplaced requirement into an existing slice if (a) that slice already holds another requirement with the same `Trigger`, or (b) that slice holds the requirement's floor (its stem, or its sub-stem when the stem has more than `SLICE_CAP` = 12 requirements). Any requirement under a brand-new stem, or under a stem that exceeds `SLICE_CAP` with a new sub-stem (`recut`), is left open for `SliceAmend`.
    - **Edge case to check in `place`**: Suppose a stem had 11 requirements in one slice, and an amendment adds 3 new requirements that all share an *existing* sub-stem already in that slice. Does `place` put them into the existing slice (pushing it to 14 requirements, over `SLICE_CAP`) without triggering `recut`, and does `held` (`Floors::check`) accept or fail with `server_error`? Verify how `Floors::new` and `place` interact here.
  - `own`: Drops type keys no longer in the design. If there are unowned data type keys, assigns them automatically *only* if no requirement IDs are open and *exactly one* slice's requirement list changed compared to the base plan; otherwise leaves the unowned type keys open for `SliceAmend`.
  - If no requirement IDs or type keys are open, validates the plan with `held` (`Floors::check`, turning any violation into `server_error`) and returns `Assembled::Settled(plan)` with zero slicing turns (keeping all existing slice briefs).
- [ ] `SliceAmend` (`specify/plan/amend.rs`): Used when `assemble` returns `Assembled::Open`. Renders `## Current plan` (marking slices whose requirements changed or were redrafted as `touched by this amendment`), `## Open` (unplaced stems/requirements, recut stems, unowned type keys), slicing rules, and `## Requirements`. The model answers with `SliceAmendAnswer { slices, joins, briefs }`:
  - `slices`: Brand-new slices (numbered after the base plan's highest `SliceId`).
  - `joins`: Open requirements/types added into existing slices.
  - `briefs`: Redrafted prose descriptions (`Rebrief`) for every existing slice that was either `touched` or named in `joins`.
  Check `SliceAmend::tighten`, `verify` (refusing moved already-placed items, duplicate joins, empty slices, missing or extra `briefs`, and running `Floors::check` on the combined result), and `into_output`.

### 3.7 Assembling `Amendment`, staging, and `emery show`

- [ ] `amend.rs` (lines ~190–225): Computes `sections = DesignDiff::between(&base.design, &design)` and `slices = SliceChange::of(&PlanDiff::between(&base.plan, &plan))`, constructs `Amendment`, and checks `amendment.is_change()` (true if any requirement is not `Unchanged`, or `sections` / `slices` is non-empty).
  - If `!amendment.is_change()`, nothing is written to `.emery/storage` and `--apply` is a no-op.
  - Otherwise, calls `store::stage_amendment`, and if `input.apply` is true, immediately calls `apply::seal` using the `Standing` read at the start of the run (so if sealing fails, the amendment remains staged).
- [ ] `crates/engine/src/show.rs`: Check `Artifact::Amendment` (reads `store::staged_amendment` directly without needing a `[target]`, returning `nothing-staged` if the slot is empty or holds a `specify` candidate). Check that `emery show spec|design|plan --staged` returns `amendment-pending` when an amendment is staged.

> Agent: "Trace `emery amend --apply` end to end: list the exact condition under which each of the four model steps (grouping, spec-draft, design-draft, slicing) is skipped, and list what state is left on disk or in `.emery/` if any step fails."

**Exit:** For every scenario S1–S13 in the planning canvas, you can trace the exact code path through `amend` and `apply`, and you have noted any edge cases in `ChangeSet::derive` or `Pruned::assemble`.

---

## 4. The CLI surface

Files: `crates/cli/src/lib.rs` (571 lines), `crates/cli/src/config.rs` (385), `crates/cli/src/text.rs` (314).

- [ ] `lib.rs`: Check `Verb::{Specify, Apply, Amend, Build, Show, Completions}`, `SpecifyArgs` (`--apply`, `--fresh`), `ApplyArgs` (`--config`, `--repository`), `AmendArgs` (positional sources, `--description`, `--config`, `--apply`), and `ShowArgs` (`amendment` artifact, `--staged`). Run `emery --help`, `emery amend --help`, `emery apply --help`, and `emery completions zsh | grep -c amend` to inspect the generated help text.
- [ ] `config.rs`: Check `scope` on `[[source]]` (defaults to `delta`), `[target]` resolution for `amend` (reads the `[target]` from the `--config` file, falling back to the project-root `emery.toml` if the change file omits `[target]`), and how command-line sources layer over `--config`. Note two CLI subtleties:
  1. Passing positional `<adapter>` or `--description` arguments to `amend` *without* `--config` skips reading `emery.toml` altogether, which means no `[target]` is loaded and the run immediately fails with `build-target-required`.
  2. Passing `emery amend --config` with no path reads the project-root `emery.toml`'s `[[source]]` entries as the change sources. Confirm this behavior is clear in `docs/reference/cli/amend.md`.
- [ ] `text.rs`: Inspect the plain-text output formatters for `amend` (counts line, ` (N by scope)`, `conflicts:`, `stems:`, `design:`, `slices: … (brief kept) / (removed) / (new)`, repository lines, and closing instruction), `apply` (`sealed revision … as emery/… at <commit8>`, `amends <parent>`, `repository <path>`, diff line), `specify` (` (fresh)`), and `build` (warnings when a candidate or amendment is staged). Compare each against `docs/reference/cli-output-shapes.md` and the assertions in `tests/amend.rs` and `tests/apply.rs`.
- [ ] Error hints (`fn hint` in `crates/cli/src/lib.rs`, around line 444): Check every branch for `nothing-staged`, `amendment-pending` (varies by subcommand), `amendment-stale`, `lineage-exists` (varies between `specify` and `apply`), `retire-unmatched`, `spec-not-generated` (varies by error description and subcommand), `spec-mismatch` (branches on whether the error description contains `"edited or merged by hand"`), and `build-target-required`. Read each hint from an operator's perspective and verify that every suggested command is valid.
- [ ] JSON output envelopes: Check `AmendOutput`, `ApplyOutput`, `SpecifyOutput.{staged, fresh, applied}`, and `BuildOutput.candidate`. Verify that `#[serde(skip_serializing_if = ...)]` attributes match the "omitted when" rules in `docs/reference/cli-output-shapes.md`.

**Exit:** Every error code handled in `fn hint` is documented in `cli-output-shapes.md` or the subcommand reference pages, and every plain-text output line is covered by a test.

---

## 5. Tests

The integration suites are comprehensive, but because they were also written by agents, check whether assertions verify the full contract or only happy-path status codes.

### 5.1 Test harness and fixtures

- [ ] `tests/support/`: Skim `provider.rs` (scripted `Model`, `Source`, and `Plugins` test doubles), `vcs.rs` (scripted `Vcs` recording Git operations), `files.rs`, `store.rs` (helpers for seeding a sealed revision or staged item: `project_staged`, `staged_document`), `scratch.rs`, and `verbs.rs`. Make sure you understand how tests seed a repository with an existing revision and script the model's JSON responses for each turn.
- [ ] `tests/amend/*.json` and `tests/specify/6-amendment.{json,md}`: Read the golden JSON and Markdown fixtures carefully—if a fixture encodes buggy output, the test suite will enforce that bug.

### 5.2 Scenario coverage map

Check off each row once you have verified that the listed tests genuinely assert the scenario's invariants:

| Scenario | Tests |
| --- | --- |
| S1: Add a feature (`added` + `changed`) | `amend.rs::amend_classes`, `amend_sliced`, `amend_sliced_joined`, `amend_sliced_types` |
| S2: Change a requirement value while keeping its `ReqId` | `amend_classes`, `amend_anchored` |
| S3: Retire a requirement (`retire` claim) | `amend_retired`, `amend_retired_slice`, `amend_retired_chain`, `amend_retired_design`, `amend_retired_section`, `amend_retired_overview`, `amend_retire_unmatched`, `amend_retire_own`, `amend_retire_contested[_applied]` |
| S4: Add a new stem / slice and design section | `amend_sliced*`, `amend_design*` |
| S5: Re-read code with `scope = "whole"` | `amend_scoped`, `amend_scoped_silent`; `../emery-adapters/examples/README.md` (`## Amend`) |
| S6: Change sources disagree (`conflict`) | `amend_classes` (conflict), `apply.rs::apply_amendment_conflict`, `amend_retire_contested` |
| S7: Resolve an existing conflict in the base spec | `amend_resolves_conflict`, `amend_conflict_unchanged` |
| S8: `apply` fails when base revision moved (`amendment-stale`) | `apply.rs::apply_amendment_stale` |
| S9: No-op change stages nothing | `amend_no_change`, `amend_apply_no_change` |
| S10: Requirement changes stem and moves between slices | `amend_moved`, `amend_moved_slice`, `amend_rebrief_refused` |
| S11: `specify` over an existing lineage (`lineage-exists` vs. `--fresh`) | `specify.rs::store::specify_lineage_exists`, `store::specify_fresh` (around line 3900), `apply.rs::apply_lineage_exists`, `apply_fresh`, `apply_fresh_over_baseline`, `specify_apply_fresh`; `runtime.rs` ("Start over") |
| S12: `build` after sealing an amendment | `build.rs::build_amendment_staged`, `build_candidate_staged`; `runtime.rs::amend_applied_built` |
| S13: Two Git branches seal conflicting revisions | `apply.rs::apply_over_merge`, `amend.rs::amend_hand_merged`, `show.rs::show_hand_merged`, `show_hand_mixed`; `runtime.rs::amend_branches_merged`, `amend_branches_hand_merged` |
| Staging slot and precondition guards | `amend_over_staged`, `amend_baseline_refused`, `amend_no_revision`, `amend_target_required`, `show.rs::amendment_pending`, `show_amendment_none`, `corrupt_amendment`, `apply.rs::apply_refused`, `apply_target_required`, `command.rs::apply_grammar`, `no_target` |
| Design amendment turn and paragraph-loss checks | `amend_design`, `amend_design_envelope`, `amend_design_types`, `amend_design_loss`, `amend_design_decision`, `amend_design_revised`, `amend_revised_dropped` |
| Recutting a stem that exceeds `SLICE_CAP` (12) | `amend_recut` |
| Grouping verifier rejections | `amend_merged_standing` |
| Markdown projection (`emery show amendment`) | `amend_shown`, `show.rs::show_amendment`, fixture `tests/specify/6-amendment.md` |
| `--apply` flag on `amend` and `specify` | `amend_apply`, `amend_apply_seal_refused`, `apply.rs::specify_apply*` |
| End-to-end WASM runtime integration tests | `runtime.rs::amend_apply_runtime`, `amend_applied_built`, `apply_spec_occupied`, `specify_staged`, `build_label_merged` |

### 5.3 Potential test gaps to probe

Investigate these edge cases and decide whether to add tests, update docs, or change code:

- [ ] **ID reuse after retiring the highest `ReqId`** (§2.1): Retire the highest-numbered requirement in revision 1, then add a new requirement in revision 2. Confirm that it reuses the retired ID, contradicting `docs/reference/cli/amend.md`.
- [ ] **Whole-scope code source where an entire stem is deleted** (§3.2): If a `scope = "whole"` source emits zero claims under a stem that existed in the base spec, nothing under that stem is retired.
- [ ] **Pushing a stem over `SLICE_CAP` under already-held sub-stems** (§3.6): Check what happens when additions push a stem past 12 requirements, but every added requirement shares a sub-stem already present in an existing slice.
- [ ] **Duplicate requirement subjects in the base spec** (§3.2): Check whether the spec gate prevents two requirements from having the same `subject`.
- [ ] **Trigger-only change**: An amendment where a requirement's statement and subject are unchanged, but its HTTP route or message trigger changes.
- [ ] **`apply` when `REVISION_KEY` cannot be read (`pointer = None`)**: Check compare-and-swap behavior in `store::commit` if `REVISION_KEY` already exists in `.emery/storage`.
- [ ] **`amend --apply` when `REVISION_KEY` fell behind `HEAD`**: When `store::current` falls back to reading `HEAD`'s `spec/` because the cached `emery/<id>` branch is behind `HEAD`, confirm that `Sealing::amendment` (`is_at`) validates against `HEAD`'s commit hash cleanly.
- [ ] **Re-running `emery amend --config` with unmodified `delta` sources**: Verify whether a test covers running `amend --config` against the exact same `delta` sources that produced the base revision (which should result in `nothing staged`).

> Agent: "For each `async fn amend_*` in `tests/amend.rs`, give a one-line summary of what base state it seeds, what model turns it scripts, and what it asserts."

**Exit:** You have a list of untested edge cases and a decision for each (test, document, or fix).

---

## 6. Documentation in `emery`

Audit the documentation first for factual accuracy against the Rust code, and second for readability. Note that agent-written prose across these repositories tends toward long, heavily punctuated sentences—flag passages that an experienced developer new to Emery would struggle to parse.

### 6.1 CLI reference docs (`docs/`, published via mdBook)

- [ ] `docs/reference/cli/amend.md` (109 lines): Verify the five requirement classes against `change.rs`, the `## The change file` section against `config.rs`, the options table against `crates/cli/src/lib.rs`, the JSON output fields against `AmendOutput`, the plain-text summary format against `text.rs`, and the caveats in `## Limits`. Decide whether any item in `## Limits` is a bug that should be fixed rather than documented.
- [ ] `docs/reference/cli/apply.md` (74 lines): Verify the three `[target] repository` modes, "Sealing a candidate" (`lineage-exists`, `--fresh`, branch behavior when content is identical), "Sealing an amendment" (commit trailers, `amendment-stale`, sealing conflicts), "`spec/` is the engine's" (both `spec-mismatch` causes and their remediation commands), "Refusals", and the JSON output fields.
- [ ] `docs/reference/cli/specify.md`: Check "Every run starts from new", "A lineage is amended or started over", "Where the revision lives", `--apply`, `--fresh`, and that `scope` in `emery.toml` is ignored by `specify`.
- [ ] `docs/reference/cli/show.md`: Check the `amendment` artifact argument, `--staged`, how `show` locates the current revision, and the `amendment-pending` error.
- [ ] `docs/reference/cli/build.md`: Check that `build` always reads the sealed revision (never the staging slot), prints a notice when a candidate or amendment is staged, rebuilds all slices from scratch after an amendment, and enforces `spec-rewritten`.
- [ ] `docs/reference/cli/index.md`, `docs/index.md`, `docs/reference/index.md`, `docs/SUMMARY.md`: Confirm `amend` and `apply` are listed and linked properly.
- [ ] `docs/reference/cli-output-shapes.md` (225 lines): Check the JSON schemas/examples for `emery amend` and `emery apply` and the error code table (every error code handled in `crates/cli/src/lib.rs::hint` should be documented).
- [ ] Run `mdbook build docs` and verify that all internal anchor links (`amend.md#limits`, `apply.md#spec-is-the-engines`, `specify.md#the-emerytoml-config`) resolve cleanly.

### 6.2 Root README, example walkthrough, and release notes

- [ ] `README.md` ("The live surface"): Check the overview of the five main subcommands (`specify`, `apply`, `amend`, `build`, `show`).
- [ ] `examples/README.md` (220 lines): Read the shell walkthrough before executing it in Section 8.
- [ ] `examples/source/prose/extract.md` and `examples/source/prose/references/greeting.md`: Check how the mock source adapter prompt teaches `retire` and standalone `decision` claims for the walkthrough.
- [ ] `RELEASES.md` (`0.39.0` section): Verify that all breaking changes are clearly called out for operators: specifications now live in the target Git repository under `spec/`, `specify` only stages until `apply` is run, `[target]` is required for sealing, schema version bumped to `EMERY = 8`, the WIT `claim-kind` enum added `retire` (requiring all WASM adapters to be rebuilt), and `specify --fresh`. Consider whether the 45 KB release notes section needs a concise executive summary at the top.

### 6.3 Agent instructions and Cursor plugin skills

- [ ] `AGENTS.md`: Spot-check the `crates/engine`, `crates/cli`, `crates/adapter`, and `tests/` rows (plus the tail of the file past 100 KB) against the implementation. Because future agent sessions treat `AGENTS.md` as ground truth, any inaccurate claim here will cause regressions in future sessions.
- [ ] `plugins/emery/`: Check `rules/emery.mdc`, `README.md`, and `skills/{amend,apply,specify,build}/SKILL.md`. Ensure `tests/plugin.rs` passes and read `skills/amend/SKILL.md` to confirm its instructions accurately describe `emery amend`'s output and flags.
- [ ] `.cursor-plugin/marketplace.json` and `plugins/emery/.cursor-plugin/plugin.json`: Both are at `0.32.0`. Decide whether adding the `/emery:amend` and `/emery:apply` skills warrants bumping the plugin version.

> Agent: "Compare `docs/reference/cli/amend.md`, the `///` doc comment on `crates/engine/src/amend.rs::amend`, and the `crates/engine` section of `AGENTS.md`. List any behavioral claims that disagree or appear in only one place."

**Exit:** All user-facing documentation and agent instructions match the code, and any overly dense prose passages are logged for cleanup.

---

## 7. `emery-adapters`

While the `emery` engine owns the amendment logic, `../emery-adapters` defines how prose adapters emit `retire` and `decision` claims and pins the `emery-sdk` dependency.

### 7.1 Dependency pin and WIT contract

- [ ] `../emery-adapters/Cargo.toml`: Check `emery-prose = "0.39.0"`, `emery-sdk = "0.39.0"`, and the `[patch.crates-io]` git entries; verify `Cargo.lock` resolves `emery-sdk` to `b3682cab` (`emery` `main`). Note that `../emery-adapters/AGENTS.md` claims `emery-lexicon` is also listed in `[patch.crates-io]`, whereas `Cargo.toml` only patches `emery-prose` and `emery-sdk` (logged in Section 10).
- [ ] Run `git diff main` in `../emery-adapters`: Check the `supply-chain/config.toml` and `supply-chain/imports.lock` updates from the dependency bump (verified by `cargo vet` during `make ci`).
- [ ] In `emery`, inspect `wit/emery.wit` (`claim-kind` enum variant `retire`) and `crates/adapter/src/source/evidence.rs` (`ClaimKind::Retire`, `requires_id`, `is_subject`, `required_extras` requiring `statement`, `refused_extras` rejecting `criterion`, `callee`, `signature`, `route`, and `message`, and `Evidence::findings`). Note that a `path` source anchor is permitted on `retire` claims (used by `documentation`). Check the unit tests in `crates/adapter/tests/claims.rs`.
- [ ] In `emery`, check `crates/sdk/prose/claims.md` (`## Withdrawals` and the fail-closed gate bullet), which is shared across all source adapters at runtime.

### 7.2 Adapter extraction prompts (`retire` and `decision`)

- [ ] `../emery-adapters/adapters/intent/prose/extract.md`: Inspect the `retire` row in the claim-kind table and the rules for withdrawals:
  - A directive stating that an existing behavior remains unchanged is a `requirement` (restatement), not a `retire`.
  - A directive that withdraws and replaces a behavior in one step is a single `requirement` under that behavior's ID, not a `retire` plus a `requirement`.
  - A directive that withdraws a behavior while also prescribing a technology emits both a `retire` claim and a `decision` claim.
  - A directive whose only content is choosing a technology for an existing behavior emits a `decision` claim alone (no `requirement`).
  - Under `## id derivation`, directives about an existing concept must reuse that concept's stem.
- [ ] `../emery-adapters/adapters/documentation/prose/extract.md`: Check the corresponding rules for documentation sources, including `path` anchors on `retire` claims and the `password-reset.security-questions` `retire` claim in the worked example.
- [ ] Run `cargo nextest run -p emery-adapters --test prose` and `cargo nextest run -p emery-adapters --test source -E 'test(/retire/)'` (`documentation_retire` and `intent_retire`).
- [ ] Confirm that the code adapters (`typescript` and `python`) were not modified for `amend` (`rg -n "retire|scope" adapters/typescript adapters/python`), since whole-scope retirement for code sources is handled entirely by the engine.

### 7.3 Adapter examples and docs

- [ ] `../emery-adapters/examples/README.md` (`## Amend`): Check the walkthrough that runs `emery:typescript` with `scope = "whole"` over the `orders` fixture, deletes the `cancel` method, runs `emery amend`, and inspects the `retired (N by scope)` and `slices: order-service (brief kept)` output.
- [ ] `../emery-adapters/examples/amend/emery.toml`: Confirm the adapter package versions (`emery:typescript@0.12.0`, `emery:omnia@0.12.0`) match the crate versions in `adapters/*/Cargo.toml`.
- [ ] `../emery-adapters/RELEASES.md`: Check whether an unreleased section is needed for the `0.12.0` / `0.39.0` updates.

**Exit:** `emery-adapters` is pinned to the `emery` commit under review, the `intent` and `documentation` prompts agree with `claims.md`, and the adapter tests pass.

---

## 8. Live end-to-end testing

Run the CLI workflows in a temporary scratch directory outside both repositories using the compiled release binary and WASM components. Record any unexpected output or unhelpful error messages in Section 10.

### 8.1 Engine walkthrough (`emery/examples/README.md`)

- [ ] **"Build and run"**: Run `specify`, `show spec --staged`, `apply`, `show spec`, and `build` (twice, confirming the second run is a no-op), then `git merge --ff-only emery/<id>`. Check `git log -1 --format=%B emery/<id>` to verify the `Revision:` and `Base:` commit trailers.
- [ ] **"Amend and build again"**: Modify `docs/greeting.md`, run `amend`, `show amendment`, `apply`, and `build`. Verify the `Parent:`, `Base:`, `Added:`, `Changed:`, `Retired:`, and `Slices:` commit trailers and confirm the previous `emery/<v0>` branch still exists.
- [ ] **Inline `--description` and conflicts**: Run `amend --config --description ...` first with a new farewell endpoint + PostgreSQL decision, and then with a conflicting greeting string. Verify that the conflict appears on the `conflicts:` output line and under `## Conflict` in `show amendment`, that `apply` seals it with `[conflict]` in `show spec`, and that a subsequent `amend` resolves it.
- [ ] **"Two branches"**: Clone the scratch project, seal a different amendment in each clone, merge both `emery/<id>` branches into `main` to trigger a merge conflict on `spec/spec.json`, resolve by keeping one side's `spec/` directory whole (`git checkout <branch> -- spec/`), and re-run the other amendment on top of the merge commit. Confirm that `show spec` works in the second clone even without local `.emery/storage` state.
- [ ] **"Start over"**: Confirm bare `emery specify` fails immediately with `lineage-exists` without loading any adapters, then run `emery specify --fresh --apply` and verify that the new commit has no `Parent:` trailer while all earlier `emery/*` branches remain intact.

### 8.2 Adapters walkthrough (`../emery-adapters/examples/README.md` `## Amend`)

- [ ] Using `examples/amend/emery.toml`, seal the baseline revision from the `orders` fixture, remove `cancel` from the TypeScript fixture, and run `emery amend`, `emery show amendment`, and `emery apply`. Verify `retired (N by scope)`, `Note: retired by scope: …`, `slices: order-service (brief kept)`, and `Note: the brief is the base's …`. Restore the fixture afterward with `git checkout`.

### 8.3 Negative and edge-case tests

- [ ] Run `emery amend --config` against an unchanged `emery.toml` (default `delta` scope): confirm it reports `nothing staged: no change against the current revision`.
- [ ] Run `emery apply` twice in a row: confirm the second invocation fails with `nothing-staged`.
- [ ] Stage a candidate with `emery specify`, then run `emery amend`: confirm it fails with `amendment-pending` explaining that a candidate is already staged. Stage an amendment with `emery amend`, then run `emery specify`: confirm it fails with `amendment-pending`.
- [ ] Pass an inline `--description` that withdraws a nonexistent behavior (e.g., `"Stop sending the weekly digest."`): confirm `emery amend` fails with `retire-unmatched` and hints `emery show spec`.
- [ ] Manually edit `spec/spec.json` in the working branch, commit the edit, and run `emery show spec`: check the `spec-mismatch` error message and hint.
- [ ] Create an arbitrary file `spec/notes.md` in a fresh repository and run `emery apply`: confirm it fails with `spec-mismatch` (`"the directory is the operator's"`).

**Exit:** All walkthroughs and negative tests behave as documented.

---

## 9. Open design questions to resolve

Each item below requires an explicit maintainer decision (fix code, update docs, or defer as future work):

1. **Requirement ID reuse after retirement (§2.1)**: `ReqId::free_after` starts at `max(base_spec_ids) + 1`. If the highest-numbered requirement is retired in revision $N$, adding a requirement in revision $N+1$ reuses that retired `ReqId`, contradicting `docs/reference/cli/amend.md` ("numbered past every id the revision ever used"). **Decision needed:** update the documentation to match the code, or persist a high-water mark in `Spec`?
2. **Whole-scope retirement when an entire stem is deleted (§3.2)**: `ChangeSet::retire_by_scope` only retires requirements under stems where the `scope = "whole"` source emitted at least one `Requirement` claim. Deleting an entire module/stem retires nothing. **Decision needed:** document this in `docs/reference/cli/amend.md#limits`, or change how whole-scope stems are tracked?
3. **Unbounded standing requirements in the grouping prompt (§3.3)**: `GroupingBrief::with_standing` includes every requirement from the base spec in a single LLM request. **Decision needed:** document the scale limit for now, or design a pre-filter/chunking mechanism?
4. **Uniqueness of requirement `subject` in `Spec` (§3.2)**: `Baseline::requirement(subject)` assumes each requirement in the base spec has a unique `subject`. **Decision needed:** verify whether `specify/spec.rs` enforces unique subjects, or add a check.
5. **Dropping a route/message trigger during an amendment (§3.2)**: Because `same_trigger` treats `basis.trigger == None` as unchanged and falls back to `matched.trigger`, an amendment cannot strip a route/message trigger from an existing requirement without replacing it. **Decision needed:** confirm this is the intended behavior.
6. **Exceeding `SLICE_CAP` via deterministic placement (§3.6)**: If an amendment adds requirements under sub-stems already held by a slice, pushing the stem past `SLICE_CAP` (12), check whether `Pruned::assemble` triggers a `server_error` in `held` (`Floors::check`). **Decision needed:** verify and fix if reachable.
7. **Git commit vs. `.emery/storage` update ordering in `store::commit` (§2.2)**: `apply` creates the Git commit and `emery/<id>` branch before updating `REVISION_KEY` and clearing `STAGED_KEY`. **Decision needed:** confirm that relying on `HEAD` fallback if the storage write fails is acceptable.
8. **Substring matching in CLI error hints (§2.2, §4)**: `crates/cli/src/lib.rs::hint` checks `description.contains("edited or merged by hand")` to choose between two `spec-mismatch` hints. **Decision needed:** keep as-is or split into distinct error codes?
9. **Prose readability across docs and `AGENTS.md` (§6)**: Much of the agent-authored documentation uses dense, multi-clause sentences. **Decision needed:** schedule a dedicated copy-editing pass on `docs/reference/cli/*.md` and `AGENTS.md`.
10. **Deliberate feature cuts from the planning canvas (§0.3)**: Confirm that omitting `--replace` and `emery show diff` is final.
11. **Cursor plugin version (§6.3)**: `plugins/emery/.cursor-plugin/plugin.json` is still `0.32.0` after adding `/emery:amend` and `/emery:apply`. **Decision needed:** bump plugin version before release?
12. **`emery-adapters/AGENTS.md` drift (§7.1)**: Mentions `emery-lexicon` in `[patch.crates-io]`, which is not in `Cargo.toml`.
13. **Live eval coverage for `amend` (§7.3)**: `evals/` in `emery-adapters` currently benchmarks `specify` and `build`, not `amend`. Log as future work.

---

## 10. Findings log

Record findings here as you work through the review.
- `Severity`: `defect` (incorrect behavior), `doc` (inaccurate or missing documentation), `test` (missing test coverage), `style` (readability/code style), `question` (requires maintainer decision).
- `Status`: `open`, `fixed-uncommitted`, `deferred`, `wontfix`.

| # | Phase | Repository | Location | Severity | Finding | Status |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 0 | emery | `crates/engine/src/revision/id.rs::free_after`, `docs/reference/cli/amend.md` | question | Added IDs start at `max(base_spec_ids) + 1`; if the highest `ReqId` was retired, the next amendment reuses it, contradicting `amend.md`. | open |
| 2 | 0 | emery | `crates/engine/src/amend/change.rs::retire_by_scope` | question | A `scope = "whole"` source that emits zero claims under a stem (e.g., deleted module) retires nothing under that stem; not noted in `amend.md#limits`. | open |
| 3 | 0 | emery | `crates/engine/src/specify/basis.rs::with_standing` | question | All standing requirements from the base spec are sent in a single grouping prompt with no cap. | open |
| 4 | 0 | emery-adapters | `AGENTS.md` | doc | Claims `emery-lexicon` is patched in `[patch.crates-io]`, but `Cargo.toml` only patches `emery-prose` and `emery-sdk`. | open |
| 5 | 0 | emery | `plugins/emery/.cursor-plugin/plugin.json` | question | Plugin version remained `0.32.0` after adding the `amend` and `apply` skills. | open |

---

## Appendix A. File inventory by phase

Line counts at the time of writing, so you can budget.

| Phase | Files |
| --- | --- |
| 2 | `crates/engine/src/revision.rs` 239 · `revision/id.rs` 168 · `revision/amendment.rs` 481 · `revision/diff.rs` 411 · `store.rs` 937 · `apply.rs` 158 · `vcs.rs` 1211 (parts) · `build.rs` 971 (parts) |
| 3 | `amend.rs` 347 · `amend/baseline.rs` 104 · `amend/change.rs` 226 · `amend/plan.rs` 306 · `specify.rs` 525 (parts) · `specify/basis.rs` 870 (parts) · `specify/spec.rs` 462 (parts) · `specify/design/amend.rs` 488 · `specify/plan/amend.rs` 559 · `specify/plan.rs` 664 (parts) · `show.rs` 176 · `prose/grouping.md` 80 · `prose/slicing.md` 205 · `prose/design-format.md` 45 · `crates/sdk/prose/claims.md` 66 |
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
