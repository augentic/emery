# Deployment profiles

How the `emery` runtime binds engine storage and mounts the filesystem, and how deployments other than the shipped local binary swap those bindings without touching engine code. A profile is host policy: one `omnia::runtime!` invocation choosing what backs the `wasi:keyvalue` / `wasi:blobstore` capability imports and which directories the guests see. The engine ships fixed key, container, and mount formulas and never learns which backing or host directory it runs over.

## The storage boundary

Engine state — the revision store and its current revision id — is reachable only through the storage capabilities (`omnia_sdk::StateStore` / `BlobStore` on the guest side). The names the engine uses are flat, deployment-neutral formulas:

| Surface             | Kind                            | Name                                                    |
| ------------------- | ------------------------------- | ------------------------------------------------------- |
| Current revision id | keyvalue key                    | `current-revision`                                      |
| Revision documents  | blobstore container `revisions` | `<id>/spec.json`, `<id>/design.json`, `<id>/plan.json` |

The host side of the boundary is a backend type implementing `omnia::Backend` (connection options compiled into the `hosts:` row, or loaded from the environment when the row carries none) plus the host context traits `WasiKeyValueCtx` and `WasiBlobstoreCtx`. Bucket and container identifiers cross the boundary exactly once — on `open_bucket` and the container methods — which is where a profile may rewrite them.

The `spec.md` / `design.md` / `plan.md` projections a project keeps beside its code are not engine state either: they are `emery show` output the skill writes into the working tree for review, and nothing reads them back. Nor is the tree `emery build` writes: a target adapter's output is the project's, and the engine keeps no record of it. Loaded components are not engine state: every adapter loads through the deployment's `omnia:plugins/loader` capability at the location the engine names, inside the grant the shipped runtime fixes at compile time. A local component is read fresh on every run through the read-only `adapters` mount, at the path beneath the adapters root the reference names — a mount a guest could write is never one code is loaded through; a package is fetched fresh on every run from the registry the deployment routes its namespace to — `~/.emery/wasm-pkg.toml`, read natively at startup, with `emery` compiled in as `augentic.io` unless a line there re-routes it — the engine naming no registry on the load and the shipped profile keeping no project cache, so no fetched component persists between runs and resolution is always fresh-release-preferred. Which registry a package is fetched from is therefore the operator's machine-level decision, never the project's: `emery.toml` names which package and which bytes, and a build that rewrites it cannot redirect a fetch; an operator who must bound registry egress bounds what `~/.emery/wasm-pkg.toml` routes. Earlier cache trees (`.omnia/cache/wasm-pkg`, `.omnia/storage/plugins/`) are orphaned — nothing reads them; delete them freely. Integrity binds the resolved sha256 digest to the exact bytes the host executes — a `[[source]] digest` pins it — never an engine-owned mutable sidecar. A bare name loads as a guest the deployment declares, attested rather than fetched, and implies no stored component.

## The shipped profile: local filesystem

The `hosts:` block of the `omnia::runtime!` invocation in [`src/main.rs`](../../src/main.rs) binds both storage hosts to `omnia_filesystem::Client` with the root compiled into the invocation: a durable, network-free store at `.omnia/storage` under the invocation directory (`blobstore/` and `keyvalue/`). The root is deployment policy, not an environment tunable — retargeting it means shipping a different profile, never setting `FILESYSTEM_ROOT`. One invocation directory is one project; isolation between projects is the filesystem root itself. Revisions survive restart; what a build writes into the working tree is the target adapter's, never the engine's.

### The mounts

The `mounts:` block fixes two directories, after the W^X rule that what a run can write it never loads code from:

| Mount      | Host directory                      | Access     | Role                                                                                                                                                      |
| ---------- | ----------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.`        | the invocation directory            | writable   | The project: the tree `specify` lends its source adapters to read, and the tree `build` lends its target adapter to write. `.omnia/storage` sits beneath it. |
| `adapters` | `~/.emery/adapters`                 | read-only  | The adapters root: every local `.wasm` component a reference names is a path beneath it, and omnia's loader admits a component from no writable mount.    |

The adapters root is `$HOME`'s, apart from any project, so a component is installed once for every project on the machine (`install target.wasm ~/.emery/adapters/`) and no run writes where the next one loads from. The runtime creates the directory on first use; one that cannot be created is reported when the mount opens, before any verb runs. omnia refuses a writable mount that contains or coincides with a read-only one, so running `emery` from `$HOME` itself is refused at startup — a project is a directory of its own.

A source adapter's extraction turn lends the project tree the same way it always has — the lend's path and the brief's `$SOURCE_DIR` are unchanged — but the mount beneath it is now writable, so an extraction turn that wrote would not be stopped by the mount; the SDK's prompts ask for claims alone, the claim gate accepts nothing but them, and no extraction or survey turn declares a write tool. A build turn lends the whole project tree writable, `.omnia/` included, and writes it through the SDK's `write_file` tool, which the guest serves through this mount: the tool refuses a path under `.omnia/` or naming a projection, so the store is written by no build, while the mount itself would permit it — the revision store still detects a document rewritten beneath it and fails closed on the next read. Narrowing the lend to exclude `.omnia/` is a profile decision this one has not taken.

## Project-id-keyed shared backings

A multi-project deployment can scope every bucket and container under a project id. This requires a genuinely shared backing: a command-mode process over in-memory defaults creates one fresh store and one project id, so it cannot demonstrate cross-project isolation or persistence.

The `multi_project` scenario in [`tests/specify.rs`](../../tests/specify.rs) exercises the invariant directly: two project-scoped views over one shared scripted store commit and show independent revisions, and no unprefixed key is written. A concrete deployment supplies its shared backend clients and rewrites identifiers at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary; that host-specific configuration does not belong in the engine.

## Remote backings

`omnia-backends` ships host clients that drop into the same `hosts:` table:

| Backend            | keyvalue | blobstore | Environment configuration |
| ------------------ | -------- | --------- | ------------------------- |
| `omnia-filesystem` | yes      | yes       | `FILESYSTEM_ROOT`         |
| `omnia-redis`      | yes      | —         | `REDIS_URL`               |
| `omnia-nats`       | yes      | yes       | `NATS_ADDR`               |
| `omnia-mongodb`    | —        | yes       | `MONGODB_URL`             |
| `omnia-azure-blob` | —        | yes       | `AZURE_BLOB_ENDPOINT`     |

The environment variables apply to a bare `hosts:` row; a row carrying compiled-in connect options (`Backend(options)`, as the shipped profile does for its filesystem root) ignores them. Credentials and endpoints live in the host binding's environment, never in engine state or operator files.

> [!WARNING]
> Identifier grammar is backend policy. The filesystem backend rejects `/` inside a bucket or container name (path-traversal fencing), so a project-id prefix targeting it needs a single-segment delimiter (for example `<project>--revisions`) or per-project roots. The in-memory and remote backends accept `/`-separated identifiers.

Remote-binding performance is unmeasured: the numbers stay unconfirmed until a remote backing is deployed and wall-clocked.
