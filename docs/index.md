# Emery

Emery is a specification synthesis and build engine for software teams. This book explains how to extract requirements from your sources, review and seal specifications in Git, evolve them with incremental amendments, and build them through WebAssembly (Wasm) adapters.

## Contents

- [How Emery works](#how-emery-works)
- [Quickstart in 5 commands](#quickstart-in-5-commands)
- [Navigating this documentation](#navigating-this-documentation)
- [Historical archive (v1)](#historical-archive-v1)

## How Emery works

Emery separates specification authoring from code generation through a five-step lifecycle:

1. **Specify (`emery specify`):** Runs source adapters over your inputs (such as natural-language intent, Markdown documentation, or existing TypeScript and Python code). It groups related claims, resolves disagreements using source authority ranks, and stages a candidate **revision** containing a behavioural specification (`spec`), an architectural design (`design`), and a dependency-ordered build plan (`plan`).
2. **Review (`emery show`):** Renders any artifact from the staged candidate (`--staged`), staged amendment (`amendment`), or currently sealed revision as clean Markdown or structured JSON.
3. **Seal (`emery apply`):** Commits the currently staged candidate or amendment into your target Git repository as `spec/{spec,design,plan}.json` on branch `emery/<id>`, then clears the staging area.
4. **Build (`emery build`):** Executes the sealed build plan in dependency waves using a target adapter. Each slice builds in an isolated temporary Git worktree, merges into an integration worktree, and passes workspace-wide verification before advancing the `emery/<id>` branch.
5. **Amend (`emery amend`):** Matches new or updated source claims against the existing requirements in the current revision and stages an incremental `Amendment` for review and sealing.

## Quickstart in 5 commands

```bash
# 1. Extract claims from configured sources and stage a candidate revision
emery specify --config

# 2. Review the staged specification as Markdown
emery show spec --staged

# 3. Seal the staged candidate to the target Git repository under spec/ on branch emery/<id>
emery apply

# 4. Build the sealed plan wave by wave through the target adapter
emery build

# 5. Stage an incremental change against the sealed revision
emery amend --description emery:intent@0.39.0="Add order cancellation" --config
```

## Navigating this documentation

- **[Glossary](glossary.md):** Plain-language definitions of Emery's core concepts, including revisions, staging, sealing, labels, lineage, stems, floors, triggers, slices, waves, and adapters.
- **Tutorials:** Step-by-step hands-on walkthroughs for new users:
  - **[Your first specification and build](tutorials/quickstart.md):** Extract a specification from Markdown docs, seal it into Git, and build it with a mock target adapter.
  - **[Evolving a specification with `emery amend`](tutorials/amending-a-spec.md):** Update requirements, add technology decisions, review staged amendments, and rebuild.
- **How-To Guides:** Task-focused recipes for common workflows:
  - **[Configure local, inline, and remote Git sources](how-to/configure-sources.md):** Set up `path`, `description`, and `repository` sources and layer CLI flags with `--config`.
  - **[Set authority ranks and resolve source conflicts](how-to/resolve-conflicts.md):** Prioritize sources with `rank` and resolve `[conflict]` requirements.
  - **[Retire requirements explicitly or by scope](how-to/retire-requirements.md):** Withdraw obsolete requirements via `retire` claims or `scope = "whole"`.
  - **[Merge specifications across Git branches](how-to/merge-branches.md):** Resolve `spec/` conflicts when two branches seal amendments independently.
  - **[Install, pin, and vendor Wasm adapters](how-to/manage-adapters.md):** Manage `~/.emery/adapters`, pin SHA-256 digests, and run offline.
  - **[Author a custom source or target adapter](how-to/author-an-adapter.md):** Build a custom Wasm adapter in Rust with `emery-sdk`.
- **Explanation:** Deep architectural guides covering how Emery works under the hood:
  - **[Revision lifecycle](explanation/revision-lifecycle.md):** How `.emery/storage` stages candidates and amendments, how `emery apply` seals `spec/` onto `emery/<id>`, the three target repository modes, and revision lineages.
  - **[Synthesis, authority, and the technology boundary](explanation/synthesis-and-authority.md):** How source adapters extract claims, how authority ranks resolve `[agreed]`, `[divergence]`, and `[conflict]`, and how the `Lexicon` gate keeps specifications technology-neutral.
  - **[Wave-based builds and verification](explanation/wave-builds.md):** How requirements are sliced by stem and sub-stem floor (`SLICE_CAP = 12`), built in parallel Git worktrees, merged, verified, and resumed.
  - **[Security, sandboxing, and gates](explanation/security-and-sandbox.md):** W^X separation between `.` and `~/.emery/adapters`, kernel worker sandboxing, worker grants, and the `spec-rewritten` and `secret-detected` gates.
- **[Reference](reference/index.md):** Detailed lookup documentation for the shipped system:
  - **[Configuration (`emery.toml`)](reference/config.md):** Complete schema and CLI layering rules for `[[source]]` and `[target]`.
  - **[CLI Reference](reference/cli/index.md):** Flags, configuration tables, and behaviour for [`emery specify`](reference/cli/specify.md), [`emery amend`](reference/cli/amend.md), [`emery apply`](reference/cli/apply.md), [`emery build`](reference/cli/build.md), and [`emery show`](reference/cli/show.md).
  - **[CLI Output Shapes](reference/cli-output-shapes.md):** JSON output schemas, exit codes, and recovery error codes for every command.
  - **[Adapters](reference/adapters.md):** Package reference syntax (`namespace:name@version`), local adapter storage (`~/.emery/adapters`), and registry resolution.
  - **[Deployment Profiles](reference/deployment-profiles.md):** Runtime sandboxing, worker grants, storage mounts, and environment variables.
- **Repository guides:** For a live shell walkthrough over the mock adapters, see [`examples/README.md`](../examples/README.md). For contributor workflows and workspace invariants, see [`CONTRIBUTING.md`](../CONTRIBUTING.md) and [`AGENTS.md`](../AGENTS.md).

## Historical archive (v1)

The v1 delivery engine (`plan → refine → execute → finalize`) is archived at Git tag `v1`. Retrieve it with:

```bash
git worktree add ../emery-v1 v1
```
