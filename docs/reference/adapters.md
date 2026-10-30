# Adapters

This page is the reference for how Emery names, stores, resolves, and verifies WebAssembly (Wasm) source and target adapters. It is for developers configuring adapters in `emery.toml` or managing the local `~/.emery/adapters` package store.

## Contents

- [Package reference syntax](#package-reference-syntax)
- [The local adapter store (`~/.emery/adapters`)](#the-local-adapter-store-emeryadapters)
- [Resolution order and registry routing](#resolution-order-and-registry-routing)
  - [Installing third-party or offline adapters](#installing-third-party-or-offline-adapters)
- [Source adapters vs. target adapters](#source-adapters-vs-target-adapters)
  - [Source adapter contract (`emery:adapter/source`)](#source-adapter-contract-emeryadaptersource)
  - [Target adapter contract (`emery:adapter/target`)](#target-adapter-contract-emeryadaptertarget)
- [W^X separation between store and project](#wx-separation-between-store-and-project)
- [Refusals at a glance](#refusals-at-a-glance)
- [See also](#see-also)

---

## Package reference syntax

Every adapter in Emery—whether passed on the command line or declared in [`emery.toml`](config.md)—must be an exact package reference in the form:

```text
<namespace>:<name>@<version>
```

For example:

- First-party source adapters: `emery:intent@0.39.0`, `emery:documentation@0.39.0`, `emery:typescript@0.39.0`, `emery:python@0.39.0`
- First-party target adapters: `emery:node@0.39.0`, `emery:omnia@0.39.0`
- Local development or third-party adapters: `emery:typescript@0.40.0-dev`, `acme:ledger@2.1.0`

Emery does not support bare names (`typescript`), unversioned references (`emery:typescript`), floating tags (`@latest`), or file paths (`./adapter.wasm`). Any non-conforming reference is rejected before loading with `adapter-reference` / `bad_request` (exit `1`).

In `emery.toml`, you can optionally pair an `adapter` with `digest = "sha256:<64-hex>"` to pin the exact SHA-256 content hash of the `.wasm` file.

---

## The local adapter store (`~/.emery/adapters`)

Emery stores all adapter releases flat inside a single directory on the host machine: `~/.emery/adapters`.

Each release is stored as a single raw `.wasm` file where the namespace and package name are joined by an underscore (`_`):

```text
~/.emery/adapters/
  emery_documentation@0.39.0.wasm
  emery_typescript@0.39.0.wasm
  emery_typescript@0.40.0-dev.wasm
  acme_ledger@2.1.0.wasm
```

Key store invariants:

- **One invertible filename convention:** `<namespace>_<name>@<version>.wasm` is the exact filename `wkg get <reference> -o ~/.emery/adapters/` writes. Files named any other way or placed in subdirectories are ignored.
- **Immutable once stored:** If `~/.emery/adapters/<namespace>_<name>@<version>.wasm` exists on disk, Emery reads it directly without contacting any network registry. To force a fresh download or update a custom `-dev` build, overwrite or remove the file in `~/.emery/adapters/`.
- **Raw Wasm only:** The loader accepts only uncompiled `.wasm` WebAssembly components. Precompiled `.cwasm` files are rejected with `refused` (exit `1`), even if a `digest` pin matches.

---

## Resolution order and registry routing

When a command needs an adapter `namespace:name@version`, the host loader (`omnia:plugins/loader`) resolves it in two steps:

1. **Local store check:** Looks for `~/.emery/adapters/<namespace>_<name>@<version>.wasm`. If present, verifies its `digest` (if pinned) and loads the file with zero network traffic.
2. **Registry fetch (`emery` namespace only):** If the file is not in the store, the loader checks its compiled-in registry routing table:
   - The `emery` namespace routes to `augentic.io` (compiled into `src/wasm-pkg.toml`; `~/.emery/wasm-pkg.toml` is never read). The loader fetches the release, verifies the registry digest, writes the file to `~/.emery/adapters/`, and loads it.
   - Any namespace other than `emery` has no compiled-in route and fails immediately with `refused` (exit `1`) if missing from `~/.emery/adapters/`.

### Installing third-party or offline adapters

To use an adapter under a custom namespace (such as `acme:ledger@2.1.0`) or to pre-populate `~/.emery/adapters` for offline/air-gapped environments, fetch or copy the `.wasm` file into `~/.emery/adapters/`:

```bash
mkdir -p ~/.emery/adapters

# Fetch from a registry configured in ~/.config/wasm-pkg/config.toml:
wkg get acme:ledger@2.1.0 -o ~/.emery/adapters/

# Or copy a locally built component directly:
cp target/wasm32-wasip2/release/emery_typescript.wasm \
  ~/.emery/adapters/emery_typescript@0.40.0-dev.wasm
```

---

## Source adapters vs. target adapters

Every adapter is a `wasm32-wasip2` component implementing one of the two worlds in the `emery:adapter` WebAssembly Interface Types (WIT) package (`wit/emery.wit`). Before dispatching any call, Emery verifies that the loaded component exports the interface required by the command (`Axis::require`).

### Source adapter contract (`emery:adapter/source`)

Used by [`emery specify`](cli/specify.md) and [`emery amend`](cli/amend.md):

| Export | Purpose |
| --- | --- |
| `metadata() -> SourceMetadata` | Returns the adapter's name, version, `emery-version` compatibility pin, and source `kind` (`intent`, `documentation`, or `behaviour`), which sets its default authority rank (`1`, `2`, or `3`). |
| `extract(input: SourceInput) -> result<Evidence, Error>` | Inspects the read-only workspace directory or inline description text and returns an `Evidence` document of typed claims (`requirement`, `criterion`, `retire`, `contract`, `decision`, `type`, `call`, `excerpt`). |

### Target adapter contract (`emery:adapter/target`)

Used by [`emery build`](cli/build.md):

| Export / Task | Purpose |
| --- | --- |
| `metadata() -> TargetMetadata` | Returns the adapter's name, version, `emery-version`, and path-glob `merge_rules` (`union`, `ours`, `theirs`) used when merging slice commits into `.emery/vcs/integration`. |
| `run(..., Task::Init)` | Runs once per repository before the first slice builds. Deterministically scaffolds missing project layout files with `create_new` (no model turn) and returns `Outcome::Initialised`. |
| `run(..., Task::Build(Slice))` | Runs once per slice in `.emery/vcs/worktrees/<slice-id>`. Implements the slice's requirements and owned data types and returns `Outcome::Built(Report)` (`covered` requirement IDs and `written` file paths). |
| `run(..., Task::Verify)` | Runs once per wave in `.emery/vcs/integration`. Runs workspace-wide checks (compiler, linter, tests), repairs issues, and returns `Outcome::Verified(Verdict)` (`passed` and `failures`). |

---

## W^X separation between store and project

Emery enforces a strict Write XOR Execute (W^X) boundary:

- The project directory (`.`) is mounted writable for `.emery/storage/` and `.emery/vcs/` worktrees, and is **never** used to load executable WebAssembly components.
- The adapter store (`~/.emery/adapters`) is read by the host loader only and is **never** mounted into any guest or model worker.
- Running `emery` directly from `$HOME` (which would place `~/.emery/adapters` inside the writable `.` mount) is refused at startup.

See [Security, sandboxing, and gates](../explanation/security-and-sandbox.md#1-wx-separation-project-mount-vs-adapter-store).

---

## Refusals at a glance

| Condition | Error code | Exit | Recovery |
| --- | --- | --- | --- |
| Reference is missing namespace or `@<semver>` (or is a file path / `@latest`) | `adapter-reference` / `bad_request` | `1` | Use `namespace:name@version` syntax. |
| Two different versions of the same package in one run, or one version pinned to two different digests | `bad_request` | `1` | Align all entries of that package to one version and digest in `emery.toml`. |
| Component exports the wrong axis (for example, a source adapter passed to `emery build`) | `bad_request` | `1` | Pass a source adapter to `specify`/`amend` and a target adapter to `build`. |
| Non-`emery` namespace missing from `~/.emery/adapters` | `refused` | `1` | Run `wkg get <reference> -o ~/.emery/adapters/`. |
| Stored or fetched bytes do not match `digest` in `emery.toml` | `refused` | `1` | Verify the `digest` value or replace the file in `~/.emery/adapters/`. |
| Stored or fetched file is precompiled (`.cwasm`) or has a mismatched WIT signature | `refused` | `1` | Replace with a raw `.wasm` component built against the current WIT contract. |
| Adapter's `emery-version` is newer than the installed `emery` binary | `unsupported-version` | `1` | Upgrade `emery` (`brew upgrade emery` or `cargo install`). |
| Registry (`augentic.io`) is unreachable or version is unpublished | `unavailable` | `4` | Check network and version number, or place the `.wasm` file in `~/.emery/adapters/`. |

---

## See also

- [Configuration (`emery.toml`)](config.md) — configuring `[[source]]` and `[target]` adapters and `digest` pins.
- [Deployment profiles](deployment-profiles.md) — host plugin loader configuration and worker grants.
- [Security, sandboxing, and gates](../explanation/security-and-sandbox.md) — W^X enforcement and worker confinement.
