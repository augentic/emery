# Glossary

This glossary defines the core concepts and terminology used across the `emery` CLI, configuration files, and documentation. Use it as a quick translation reference when reading command help, error hints, or architectural guides.

## Contents

- [Specifications and revisions](#specifications-and-revisions)
  - [Revision](#revision)
  - [Staging slot (Candidate and Amendment)](#staging-slot-candidate-and-amendment)
  - [Sealing](#sealing)
  - [Label](#label)
  - [Lineage](#lineage)
  - [Standing](#standing)
  - [Baseline](#baseline)
- [Requirements and planning](#requirements-and-planning)
  - [Stem, sub-stem, and floor](#stem-sub-stem-and-floor)
  - [Trigger](#trigger)
  - [Retire claim and whole-scope retirement](#retire-claim-and-whole-scope-retirement)
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
- `plan.json`: Dependency-ordered build slices that partition the requirements and data types for code generation.

Every revision is identified by a content-addressed SHA-256 hex digest (`<id>`) computed across all three documents. Markdown views (`spec.md`, `design.md`, `plan.md`) are read-only projections rendered on demand by [`emery show`](reference/cli/show.md).

### Staging slot (Candidate and Amendment)

Local scratch storage under `.emery/storage/` that holds at most one unsealed item awaiting review. The item currently staged is one of two kinds:

- **Candidate:** A full `Revision` produced from scratch by [`emery specify`](reference/cli/specify.md). Inspect it with `emery show <spec|design|plan> --staged`.
- **Amendment:** An incremental change against the current revision produced by [`emery amend`](reference/cli/amend.md). Inspect it with `emery show amendment`.

To prevent accidental data loss, `emery specify` will not overwrite a staged amendment, and `emery amend` will not overwrite a staged candidate (returning `amendment-pending` instead). Running [`emery apply`](reference/cli/apply.md) commits the staged item to Git and clears the staging area.

### Sealing

The act of committing the currently staged candidate or amendment to the target Git repository via [`emery apply`](reference/cli/apply.md) (or `--apply` on `specify` and `amend`). Sealing writes `spec/spec.json`, `spec/design.json`, and `spec/plan.json` in a single Git commit (`Revision <id>`), creates or advances the Git branch `emery/<id>`, updates the cached revision pointer in `.emery/storage`, and clears the staging area.

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

### Stem, sub-stem, and floor

Every requirement has a dotted subject identifier such as `orders.cancel` or `orders.items.add`:

- **Stem:** The first segment of the subject (for example, `orders`).
- **Sub-stem:** The first two segments of the subject (for example, `orders.items`).
- **Floor:** The minimum grouping unit used when partitioning requirements into build slices. Normally, a requirement's floor is its **stem**—all requirements sharing a stem stay together in at least one slice. When a single stem exceeds 12 requirements (`SLICE_CAP`), the planner lowers the floor for that stem to its **sub-stems** so the stem can be split cleanly along sub-stem boundaries.

### Trigger

An inbound entry point attached to a requirement by code analysis—never guessed by the language model. A trigger is either:

- **HTTP route:** Method and normalized path pattern (for example, `POST /orders/{id}/cancel`), rendered in the specification as `Route: POST /orders/{id}/cancel`.
- **Message:** A subscribed message or topic name (for example, `order.created`), rendered as `Message: order.created`.

Requirements that share a trigger are always placed in the same build slice, regardless of their stems.

### Retire claim and whole-scope retirement

An amendment can remove ("retire") an existing requirement in two ways:

1. **Explicit withdrawal (`retire` claim):** A prose source adapter (`intent` or `documentation`) emits a claim of kind `retire` whose subject matches an existing requirement (for example, when an operator note states "Remove legacy CSV export").
2. **Implicit withdrawal (`scope = "whole"`):** A code source configured with `scope = "whole"` in `emery.toml` is treated as exhaustive for every stem its extracted `requirement` claims mention. Any existing requirement under one of those stems that is left untouched by the change is automatically retired ("retired by scope").

### Slice

A single unit of work (`SLICE-001`, `SLICE-002`, ...) in `plan.json`. Each slice owns a subset of requirements, zero or more data type definitions from `design.json`, a prose brief, and a list of slices it `depends-on`. During [`emery build`](reference/cli/build.md), each slice is handed to the target adapter in its own isolated temporary Git worktree.

### Wave

A group of build slices whose `depends-on` slices have all been merged. [`emery build`](reference/cli/build.md) executes all ready slices in a wave concurrently (bounded by `--jobs`), merges their commits in slice ID order into `.emery/vcs/integration`, and runs a single verification pass over the integrated tree before moving to the next wave.

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

A **turn** is a single structured request sent to the language model, including automatic validation and self-correction rounds when a response violates an engine or SDK gate. In the engine, each turn is constructed from a **brief**—a typed prompt paired with a JSON schema and a deterministic verifier. (In `plan.json`, each slice also carries a short prose `brief` describing what the slice builds.)
