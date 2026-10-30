# Deployment profiles

This page is the reference for how the `emery` runtime binds WebAssembly System Interface (WASI) storage, version control, filesystem mounts, and model worker sandboxes, and how custom host deployments can swap those backings. It is for platform engineers and operators deploying or embedding Emery.

## Contents

- [Host capability bindings](#host-capability-bindings)
- [The storage boundary](#the-storage-boundary)
- [The shipped profile: local filesystem and Git](#the-shipped-profile-local-filesystem-and-git)
  - [Filesystem mounts and W^X policy](#filesystem-mounts-and-wx-policy)
  - [Directory layout under `.emery/`](#directory-layout-under-emery)
- [What a build's worker may reach](#what-a-builds-worker-may-reach)
- [Environment variables](#environment-variables)
  - [Shipped binary (`emery`)](#shipped-binary-emery)
  - [Local-model development binary (`emery-dev`, `--features genai`)](#local-model-development-binary-emery-dev---features-genai)
- [Alternative and remote backings](#alternative-and-remote-backings)
- [See also](#see-also)

---

## Host capability bindings

A deployment profile is defined by a single `omnia::runtime!` macro invocation (in [`src/main.rs`](../../src/main.rs) for the shipped binary). It binds the engine guest's capability imports (`wasi:keyvalue`, `wasi:blobstore`, `omnia:vcs`, `omnia:plugins/loader`, and `omnia:model`) and declares filesystem mounts without changing any engine crate code.

| Capability | Guest trait (`emery-engine`) | Shipped host implementation (`src/main.rs`) |
| --- | --- | --- |
| Key-value store (`wasi:keyvalue`) | `omnia_sdk::StateStore` | `omnia_filesystem::Client` at `.emery/storage/keyvalue/` |
| Blob store (`wasi:blobstore`) | `omnia_sdk::BlobStore` | `omnia_filesystem::Client` at `.emery/storage/blobstore/` |
| Version control (`omnia:vcs`) | `omnia_sdk::Vcs` | `omnia_git::Client` (invokes host `git` subprocesses) |
| Plugin loader (`omnia:plugins/loader`) | `omnia_sdk::Plugins` | Host Wasmtime loader reading `~/.emery/adapters` and routing `emery` -> `augentic.io` |
| Language model (`omnia:model`) | `omnia_sdk::Model` | `omnia_cursor::Client` (or `omnia_genai::Client` with `--features genai`) |
| Project filesystem (`wasi:filesystem`) | `emery_engine::Files` | Preopened `.` directory via `std::fs` no-follow helpers |

---

## The storage boundary

The engine accesses local staging state and the cached revision pointer exclusively through `StateStore` and `BlobStore` using fixed, deployment-neutral identifiers:

| Surface | Capability | Container / Bucket | Key / Object name | Description |
| --- | --- | --- | --- | --- |
| Staged item record | `StateStore` | default bucket | `staged` | JSON record (`id`, `kind`, `base`, `fresh`) for the currently staged candidate or amendment. |
| Cached revision pointer | `StateStore` | default bucket | `current-revision` | Cached `emery/<id>` label name (always verified against the target Git repository). |
| Staged candidate blobs | `BlobStore` | `staged` | `<id>/spec.json`, `<id>/design.json`, `<id>/plan.json` | Unsealed candidate revision documents. |
| Staged amendment blob | `BlobStore` | `staged` | `<id>/amendment.json` | Unsealed `Amendment` document. |

> [!NOTE]
> Sealed revisions do **not** live in `.emery/storage/`. Once [`emery apply`](cli/apply.md) runs, the authoritative revision lives in the target Git repository as `spec/{spec,design,plan}.json` on branch `emery/<id>`, and the staging container is cleared.

---

## The shipped profile: local filesystem and Git

### Filesystem mounts and W^X policy

The shipped `src/main.rs` profile configures one writable guest mount and one host-only adapter store under a strict Write XOR Execute (W^X) rule:

| Location | Host path | Access | Purpose |
| --- | --- | --- | --- |
| Project mount (`.`) | Current working directory | Writable by engine guest | Reading workspace sources and managing `.emery/storage/` and `.emery/vcs/`. |
| Adapter store | `~/.emery/adapters` | Host-only (unmounted) | Loading `.wasm` adapter components. Unreachable by guests and model workers. |

If you invoke `emery` from `$HOME` itself, the runtime refuses to start because `~/.emery/adapters` would sit inside the writable `.` mount.

### Directory layout under `.emery/`

All local state created by Emery inside your project directory sits under `.emery/` (which should be listed in your `.gitignore`):

| Path | Lifecycle | Purpose |
| --- | --- | --- |
| `.emery/storage/` | Persistent cache | Holds the single staged candidate/amendment and the `current-revision` cache pointer. |
| `.emery/vcs/repos/<key>` | Persistent cache | Bare/cached Git clones for remote `[[source]] repository` URLs (keyed by normalized URL hash). |
| `.emery/vcs/repos/<name>` | Persistent repo | Target Git repository when `[target] repository` is a URL clone or omitted (engine-managed repo). When omitted, this directory holds your sealed specifications and built branches—do not delete it unless you want to discard those builds. |
| `.emery/vcs/revision` | Ephemeral worktree | Temporary Git worktree used by `emery apply` and `store::current` to read or write `spec/`. Removed immediately after use. |
| `.emery/vcs/sources/<name>` | Ephemeral worktree | Temporary Git worktree checked out at `[[source]] revision` during `specify` and `amend`. Removed as soon as extraction finishes. |
| `.emery/vcs/worktrees/<slice>` | Ephemeral worktree | Isolated Git worktree for building one slice (`SLICE-NNN`). Removed on merge; kept on disk for inspection if the slice fails. |
| `.emery/vcs/integration` | Wave worktree | Git worktree where slices in a wave are merged and verified. Removed when all waves pass; kept on disk if a wave fails so the next `emery build` can resume already-merged slices. |

---

## What a build's worker may reach

Every model completion worker (`cursor-sdk-bridge` under the default `omnia-cursor` backend, or `sh -c` under `--features genai`) runs inside an OS kernel sandbox (`omnia-confine`: Seatbelt on macOS, Landlock + seccomp on Linux).

1. **Compiled-in shell roots:** `src/main.rs` compiles in `shell_roots: [".emery/vcs/worktrees", ".emery/vcs/integration"]` (`CURSOR_SHELL_ROOTS` in the environment is ignored). Only `Task::Build` and `Task::Verify` turns run inside these directories and receive the `shell` tool; source extraction and survey turns have no shell or file-write tools.
2. **Writable scope:** A worker can write only to its lent worktree and a private per-turn scratch directory that replaces `HOME` and `TMPDIR`.
3. **Target adapter worker grants (`TargetAdapter::metadata().grants`):** Each target adapter declares the exact registry hosts, read-only `~/` toolchain paths, and environment variables its toolchain requires:

| Target adapter | Egress hosts (via loopback proxy) | Read-only `~/` roots (granted if present on host) | Environment variables |
| --- | --- | --- | --- |
| `emery:node` | `registry.npmjs.org` | `~/.nvm/versions/node`, `~/.volta`, `~/.local/share/fnm` | `VOLTA_HOME=~/.volta` |
| `emery:omnia` | `index.crates.io`, `static.crates.io` | `~/.rustup`, `~/.cargo/bin` | `RUSTUP_HOME=~/.rustup` |

Any grant requesting a path outside `~/`, overlapping a credential directory (`~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.kube`, `~/.docker`, `~/.config/gh`, `~/.config/gcloud`, `~/.password-store`, `~/Library/Keychains`), or setting `CURSOR_API_KEY` is rejected with `invalid-request` before the turn starts. For full security details and macOS vs. Linux differences, see [Security, sandboxing, and gates](../explanation/security-and-sandbox.md#3-kernel-confinement-for-model-workers).

---

## Environment variables

### Shipped binary (`emery`)

| Variable | Default | Description |
| --- | --- | --- |
| `CURSOR_API_KEY` | *(required)* | API key for the Cursor model backend (`omnia-cursor`). |
| `CURSOR_MODEL` | Backend default | Override the model slug used by `omnia-cursor`. |
| `CURSOR_TIMEOUT_SECS` | Backend default | Wall-clock timeout in seconds for a single model turn. |
| `CURSOR_INACTIVITY_SECS` | Backend default | Inactivity timeout in seconds for a model turn stream. |
| `CURSOR_MAX_AGENTS` | `8` | Maximum concurrent model turns (caps wave parallelism alongside `--jobs`). |
| `CURSOR_MAX_TOOL_CALLS` | `128` | Maximum guest tool calls (`write_files`, `edit_files`, `read_doc`, `list_docs`) per turn before failing with `budget-exhausted`. |
| `EMERY_JOBS` | Unset (all ready slices) | Maximum concurrent slice builds per wave (equivalent to `emery build --jobs <N>`). |
| `EMERY_FORMAT` | `text` | Default CLI output format (`text` or `json`; equivalent to `--format`). |
| `GIT_BINARY` | `git` (on `PATH`) | Path to the host `git` executable used by `omnia-git` (requires Git 2.5+). |
| `RUST_LOG` | `info` | Tracing filter when neither `-v` nor `-q` is passed. |

### Local-model development binary (`emery-dev`, `--features genai`)

When compiled with `cargo build --release --features genai`, the binary uses `omnia-genai` in place of `omnia-cursor`:

| Variable | Description |
| --- | --- |
| `GENAI_MODEL` | Model identifier (bare name for Ollama, or `openai::<name>` for vLLM / OpenAI-compatible servers). |
| `GENAI_ENDPOINT` | Base URL for the local model server. |
| `OPENAI_API_KEY` | API key when using an `openai::<name>` model. |
| `GENAI_MAX_ROUNDS` | Maximum correction rounds per question. |
| `GENAI_MAX_TOOL_CALLS` | Maximum tool calls per completion turn. |
| `GENAI_SHELL_TIMEOUT_SECS` | Per-command timeout in seconds for `sh -c` shell calls. |

---

## Alternative and remote backings

Custom deployments can replace `omnia-filesystem` in the `hosts:` block of `omnia::runtime!` with other clients from `omnia-backends`:

| Backend crate | `wasi:keyvalue` | `wasi:blobstore` | `omnia:vcs` | Default environment variable |
| --- | --- | --- | --- | --- |
| `omnia-filesystem` | Yes | Yes | — | `FILESYSTEM_ROOT` *(ignored when compiled in, as in `emery`)* |
| `omnia-redis` | Yes | — | — | `REDIS_URL` |
| `omnia-nats` | Yes | Yes | — | `NATS_ADDR` |
| `omnia-mongodb` | — | Yes | — | `MONGODB_URL` |
| `omnia-azure-blob` | — | Yes | — | `AZURE_BLOB_ENDPOINT` |
| `omnia-git` | — | — | Yes | `GIT_BINARY` |

Multi-tenant deployments sharing a single backing store can isolate projects by rewriting bucket and container names at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary (verified by the `multi_project` test in `tests/specify.rs`). Note that `omnia-filesystem` forbids `/` inside bucket/container names, so filesystem-backed prefixing must use a single-segment delimiter such as `<project>--staged`.

---

## See also

- [Security, sandboxing, and gates](../explanation/security-and-sandbox.md) — W^X separation, kernel confinement, and pre-commit security gates.
- [Adapters](adapters.md) — adapter package references and `~/.emery/adapters` resolution.
- [`emery build`](cli/build.md) — how builds use worktrees, worker grants, and concurrency limits.
