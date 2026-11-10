# Glossary

This glossary defines the core concepts and terminology used across the `emery` CLI, configuration files, and documentation. Use it as a quick translation reference when reading command help, error hints, or architectural guides.

## Contents

- [Specifications and revisions](#specifications-and-revisions)
  - [Revision](#revision)
  - [Staging slot (Candidate and Amendment)](#staging-slot-candidate-and-amendment)
  - [Sealing](#sealing)
  - [Layout](#layout)
  - [Label](#label)
  - [Lineage](#lineage)
  - [Standing](#standing)
  - [Baseline](#baseline)
- [Requirements and planning](#requirements-and-planning)
  - [Stem and floor](#stem-and-floor)
  - [Trigger](#trigger)
  - [Retire claim and whole-scope retirement](#retire-claim-and-whole-scope-retirement)
  - [Part](#part)
  - [Budget, schedule, and over-budget slice](#budget-schedule-and-over-budget-slice)
  - [Slice](#slice)
  - [Wave](#wave)
- [Adapters, extraction, and execution](#adapters-extraction-and-execution)
  - [Source adapter and target adapter](#source-adapter-and-target-adapter)
  - [Seam](#seam)
  - [Target repository](#target-repository)
  - [Turn and brief](#turn-and-brief)

---

## Specifications and revisions

### Revision

A complete, versioned specification bundle consisting of three JSON documents:

- `spec.json`: Technology-neutral behavioural requirements (`REQ-001`, `REQ-002`, ...) and their verification scenarios.
- `design.json`: Concrete architectural sections, external integrations, and data type signatures.
- `plan.json`: Dependency-ordered logical parts (`PART-001`, `PART-002`, ...) that partition the requirements and data types for code generation.

Every revision is identified by a content-addressed SHA-256 hex digest (`<id>`) computed across all three documents. A sealed revision also carries the body of every [layout](#layout) its design names under `spec/layouts/<hex>`; the bodies are not part of `<id>`, but each is bound to it through the `sha256:<hex>` digest the design names it by. Markdown views (`spec.md`, `design.md`, `plan.md`) are read-only projections rendered on demand by [`emery show`](reference/cli/show.md).

### Staging slot (Candidate and Amendment)

Local scratch storage under `.emery/storage/` that holds at most one unsealed item awaiting review. The item currently staged is one of two kinds:

- **Candidate:** A full `Revision` produced from scratch by [`emery specify`](reference/cli/specify.md). Inspect it with `emery show <spec|design|plan> --staged`.
- **Amendment:** An incremental change against the current revision produced by [`emery amend`](reference/cli/amend.md). Inspect it with `emery show amendment`.

To prevent accidental data loss, `emery specify` will not overwrite a staged amendment, and `emery amend` will not overwrite a staged candidate (returning `amendment-pending` instead). Running [`emery apply`](reference/cli/apply.md) commits the staged item to Git and clears the staging area.

### Sealing

The act of committing the currently staged candidate or amendment to the target Git repository via [`emery apply`](reference/cli/apply.md) (or `--apply` on `specify` and `amend`). Sealing writes `spec/spec.json`, `spec/design.json`, and `spec/plan.json`, plus `spec/layouts/<hex>` for every [layout](#layout) the design names (removing any the design no longer names), in a single Git commit (`Revision <id>`), creates or advances the Git branch `emery/<id>`, updates the cached revision pointer in `.emery/storage`, and clears the staging area. Each layout body is taken from `.emery/storage`, where `emery specify` or `emery amend` stored it, or else from `spec/layouts/` in the commit the seal lands over (so an amendment seals on a fresh clone); when neither holds it, sealing fails with `layout-missing` before anything is written and the staged item stays staged. An amend that records a layout for a source replaces the layouts that source already contributed, rather than keeping both.

### Layout

A UI layout a source adapter recorded whole (for example, the rendered layout of a web page), carried by content rather than by path. The design's `## UI / layout` section names each layout by its `sha256:<hex>` digest, media type, and size, and shows review images rendered from it; the body itself is opaque to the engine. [`emery specify`](reference/cli/specify.md) stores each body under `.emery/storage`, sealing writes it to `spec/layouts/<hex>` (no file extension, since the body may be compressed), and [`emery build`](reference/cli/build.md) reads it from the sealed commit, held to its digest, and hands it whole to every slice. A sealed `spec/layouts/` must hold exactly the layouts the design names; a missing, extra, or rewritten file is `spec-mismatch`.

### Label

The local Git branch `emery/<id>` in the target repository, where `<id>` is the full 64-character revision digest. When a revision is first sealed, `emery/<id>` points to the `Revision <id>` commit. When [`emery build`](reference/cli/build.md) runs, it advances `emery/<id>` as each wave of slices is merged and verified.

### Lineage

The chain of revisions formed when an amended revision records the digest of the previous revision in its `parent` field. Once the current revision in a repository has a `parent` (meaning at least one amendment or second revision has been sealed), bare `emery specify` refuses to overwrite the history (`lineage-exists`). You must either evolve the specification with [`emery amend`](reference/cli/amend.md) or pass `emery specify --fresh` to intentionally start a new, parentless history over `HEAD`.

### Standing

The resolved state of the target Git repository when Emery inspects it before a command runs. It records:

- The repository's current `HEAD` commit.
- The active sealed revision (resolved from the cached `emery/<id>` branch if it descends from `HEAD`, or directly from `spec/` on `HEAD` after a branch merge).
- Whether the cached `emery/<id>` branch has fallen behind `HEAD`.

### Baseline

The existing requirements in the currently sealed revision when running [`emery amend`](reference/cli/amend.md). Emery injects these existing requirements into the grouping step under the reserved source name `baseline` at an authority rank one step below the lowest-ranked change source, so incoming change claims can match, update, or retire them while preserving their `REQ-NNN` identifiers.

---

## Requirements and planning

### Stem and floor

Every requirement has a dotted subject identifier such as `orders.cancel` or `orders.items.add`:

- **Stem:** The first segment of the subject (for example, `orders`).
- **Floor:** The minimum grouping unit used when partitioning requirements into logical plan parts. A requirement's floor is its **stem**—all requirements sharing a stem stay together in one part, and a stem is never split across parts during `specify` or `amend`.

### Trigger

An inbound entry point attached to a requirement by code analysis—never guessed by the language model. A trigger is either:

- **HTTP route:** Method and normalized path pattern (for example, `POST /orders/{id}/cancel`), rendered in the specification as `Route: POST /orders/{id}/cancel`.
- **Message:** A subscribed message or topic name (for example, `order.created`), rendered as `Message: order.created`.

Requirements that share a trigger are always placed in the same part in `plan.json` and kept together in the same build slice when the schedule is cut, regardless of their stems.

### Retire claim and whole-scope retirement

An amendment can remove ("retire") an existing requirement in two ways:

1. **Explicit withdrawal (`retire` claim):** A prose source adapter (`intent` or `documentation`) emits a claim of kind `retire` whose subject matches an existing requirement (for example, when an operator note states "Remove legacy CSV export").
2. **Implicit withdrawal (`scope = "whole"`):** A code source configured with `scope = "whole"` in `emery.toml` is treated as exhaustive for every stem its extracted `requirement` claims mention. Any existing requirement under one of those stems that is left untouched by the change is automatically retired ("retired by scope").

### Part

A logical domain module (`PART-001`, `PART-002`, ...) stored in `plan.json` by [`emery specify`](reference/cli/specify.md) and [`emery amend`](reference/cli/amend.md). Each part has a kebab-case `name`, a list of `requirements` (`REQ-NNN`), zero or more owned `data` type keys (`types`) from `design.json`, zero or more `data` type keys its code `uses` without owning, an engine-derived list of other parts it `depends-on` (the owners of the types its owned signatures name and the types in `uses`), and a prose `brief`. Parts carry no requirement count cap; during [`emery build`](reference/cli/build.md), each part is cut into one or more budget-sized [slices](#slice).

### Budget, schedule, and over-budget slice

- **Budget:** The maximum number of requirements (`REQ-NNN`) to implement in a single build slice during [`emery build`](reference/cli/build.md). Resolved in priority order from `--budget <N>` / `EMERY_BUDGET` (`flag`), `[target] budget = <N>` in `emery.toml` (`config`), the target adapter's `TargetMetadata.budget` (`adapter`), or the engine fallback of `12` (`default`).
- **Schedule:** The deterministic sequence of build slices (`SLICE-001`, `SLICE-002`, ...) and dependency waves cut from `plan.json` under the resolved budget, with no language-model turn spent. Inspect it ahead of time with [`emery show schedule [--budget <N>]`](reference/cli/show.md#previewing-the-build-schedule-emery-show-schedule).
- **Over-budget slice (`over-budget`):** A slice whose requirement count exceeds the budget because a single un-splittable group (such as one HTTP route or message handler) has more requirements than the budget on its own. Emery keeps the group intact as one slice and reports it under `over budget:` (text) and `over-budget` (JSON) as a warning rather than failing the build.

### Slice

A single budget-sized unit of build work (`SLICE-001`, `SLICE-002`, ...) cut from a [part](#part) in the build [schedule](#budget-schedule-and-over-budget-slice). Each slice carries its parent `part` ID (`PART-NNN`), the part's logical `name` and `brief`, its 1-based position within the part (`index` of `count`), its slice of `requirements`, and (on the first slice of an unbuilt part) the part's owned data `types`. During [`emery build`](reference/cli/build.md), slices within a part run in serial waves while independent parts run in parallel, each slice building in its own isolated temporary Git worktree (`.emery/vcs/worktrees/<slice-id>`).

### Wave

A group of build slices whose dependency parts and earlier intra-part slices have all been merged. [`emery build`](reference/cli/build.md) executes all ready slices in a wave concurrently (bounded by `--jobs`), merges their commits in slice ID order into `.emery/vcs/integration`, and runs a single verification pass over the integrated tree before moving to the next wave.

---

## Adapters, extraction, and execution

### Source adapter and target adapter

Sandboxed WebAssembly (Wasm) components compiled against the `emery:adapter` WIT (WebAssembly Interface Types) contract and executed under WASI (WebAssembly System Interface):

- **Source adapter:** Implements the `source-adapter` world (`metadata` + `extract`). It reads files from your project directory (read-only) or an inline `--description` string and emits an `Evidence` document of typed claims (`requirement`, `criterion`, `retire`, `decision`, `contract`, `type`, `call`, `excerpt`). Prose sources may include ADRs (Architecture Decision Records), Markdown runbooks, or short intent notes.
- **Target adapter:** Implements the `target-adapter` world (`metadata` + `run`). It declares file merge rules and worker sandbox grants, and executes three task kinds against a writable Git worktree:
  - `init`: Scaffolds missing project layout files deterministically without calling the language model.
  - `build(slice)`: Implements one slice's requirements and owned data types.
  - `verify`: Runs the workspace build, linter, and test suite across the integrated wave, repairing any failures before returning a `Verdict`.

Adapters are loaded from `~/.emery/adapters/` under a strict W^X (Write XOR Execute) boundary: the project directory is writable by the engine and build workers, while the adapter store is read-only and unreachable by guests. See [Adapters](reference/adapters.md).

### Seam

A bounded extraction unit chosen by a source adapter's survey pass before querying the language model. Each seam specifies which files to inspect, which requirement stems are permitted, which source line spans (`path#Lstart-Lend`) claims may anchor to, and the technology lexicon that keeps extracted requirements platform-neutral.

### Target repository

The Git repository where [`emery apply`](reference/cli/apply.md) seals specifications under `spec/` and where [`emery build`](reference/cli/build.md) commits generated code on branch `emery/<id>`. Configured via `[target] repository` in `emery.toml` (or `--repository` on the CLI):

| `[target] repository` setting | Where revisions and builds land |
| --- | --- |
| `"."` | Directly in your current project repository (the working tree outside `.emery/` must be clean before `emery build`). |
| `"<url>"` | In a clone managed by Emery at `.emery/vcs/repos/<name>`, starting from `origin/HEAD`. |
| Omitted | In a local Git repository initialized and managed by Emery at `.emery/vcs/repos/<name>`, where the first `Revision <id>` commit serves as the root commit. |

### Turn and brief

A **turn** is a single structured request sent to the language model, including automatic validation and self-correction rounds when a response violates an engine or SDK gate. In the engine, each turn is constructed from a **brief**—a typed prompt paired with a JSON schema and a deterministic verifier. (In `plan.json`, each part also carries a short prose `brief` describing what the part delivers.)
