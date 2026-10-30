# Security, sandboxing, and gates

This page explains Emery's security model, including Write XOR Execute (W^X) separation between your project and the adapter store, WebAssembly guest isolation, kernel sandboxing for model workers, no-follow path guards, and the pre-commit gates that block specification tampering and secret leaks. It is for developers, security reviewers, and platform operators.

## Contents

- [Defense in depth overview](#defense-in-depth-overview)
- [1. W^X separation: project mount vs. adapter store](#1-wx-separation-project-mount-vs-adapter-store)
  - [Pinned registry routing and digest verification](#pinned-registry-routing-and-digest-verification)
- [2. WebAssembly guest isolation](#2-webassembly-guest-isolation)
- [3. Kernel confinement for model workers](#3-kernel-confinement-for-model-workers)
  - [Filesystem and network confinement](#filesystem-and-network-confinement)
  - [Target adapter worker grants](#target-adapter-worker-grants)
  - [Platform differences: macOS vs. Linux](#platform-differences-macos-vs-linux)
- [4. Safe file operations and symlink protection](#4-safe-file-operations-and-symlink-protection)
- [5. Pre-commit gates](#5-pre-commit-gates)
  - [The specification guard (`spec-rewritten`)](#the-specification-guard-spec-rewritten)
  - [The secret gate (`secret-detected`)](#the-secret-gate-secret-detected)
- [See also](#see-also)

---

## Defense in depth overview

Because Emery executes third-party or language-specific adapters and invokes language-model workers that run build toolchains (`cargo`, `npm`, test suites), it enforces five concentric security boundaries:

1. **W^X (Write XOR Execute) mount separation:** No directory that a run can write to is ever used to load executable adapter code.
2. **WebAssembly (Wasm) component sandbox:** Both the engine and all adapters run as WASI (WebAssembly System Interface) guests with no ambient host filesystem, network, or process access.
3. **Kernel worker confinement:** Every language-model worker process runs inside an OS kernel sandbox (Seatbelt on macOS, Landlock and seccomp on Linux) with a private `HOME` directory and a loopback network proxy.
4. **No-follow path validation (`beneath` and `standing`):** File tools reject path traversal, reserved directories (`.git`, `.emery`, `spec`), and symbolic links along any write path.
5. **Pre-commit gates (`spec-rewritten` and `secret-detected`):** Before Emery commits any change from initialization, slice builds, merge resolutions, or wave verifications, it verifies that `spec/` was not touched and scans all modified paths and symlink targets for credentials.

---

## 1. W^X separation: project mount vs. adapter store

The shipped `emery` runtime enforces a strict Write XOR Execute rule between your project directory and the package store where WebAssembly adapters live:

| Location | Host path | Guest mount | Access policy |
| --- | --- | --- | --- |
| **Project root** | Invocation directory (`.`) | Mounted as `.` | Writable by the engine guest (for `.emery/storage/` and `.emery/vcs/` worktrees). Never used to load WebAssembly components. |
| **Adapter store** | `~/.emery/adapters` | Not mounted | Read-only to the host plugin loader. Unreachable by the engine guest, adapters, or model workers. |

At startup, the Omnia runtime verifies that the plugin store does not sit inside any writable mount. If you invoke `emery` directly from `$HOME` (which would place `~/.emery/adapters` inside the writable `.` mount), the binary refuses to start before any command runs.

### Pinned registry routing and digest verification

A build turn can edit files inside its worktree, including `emery.toml`. To ensure a modified project configuration can never redirect network fetches or execute untrusted machine code:

- **Compiled-in registry routing:** `emery.toml` can name an adapter package (`namespace:name@version`) and an optional `sha256:` content `digest`, but it cannot configure registry URLs. The `emery` binary compiles in a single registry route (`emery` -> `augentic.io`) and ignores `~/.emery/wasm-pkg.toml`.
- **Raw Wasm only:** The plugin loader accepts only uncompiled `.wasm` components and rejects precompiled `.cwasm` artifacts (`refused`), even if a digest matches.
- **Digest pinning:** When `digest = "sha256:..."` is set on a `[[source]]` or `[target]` entry, the host loader verifies the SHA-256 hash of the `.wasm` bytes—whether read from `~/.emery/adapters` or freshly fetched—before compiling and instantiating the component.

---

## 2. WebAssembly guest isolation

The Emery engine and every source or target adapter are compiled to `wasm32-wasip2` components:

- **No ambient host access:** Guests cannot spawn processes, open network sockets, or read environment variables unless explicitly provided through typed WIT interfaces.
- **Host-mediated Git (`omnia:vcs`):** Guests never hold Git credentials, host filesystem paths, or `git` subprocesses. The engine passes deployment-local relative paths (such as `.`, `.emery/vcs/repos/<name>`, or `.emery/vcs/worktrees/<slice>`) to the `omnia:vcs` host capability, which resolves them against the project mount and invokes `git` on the host.

---

## 3. Kernel confinement for model workers

When the Cursor model backend (`omnia-cursor`) spawns a `cursor-sdk-bridge` worker process for a model turn (or when `emery-dev` runs a local `sh -c` shell turn under the `genai` feature), the worker is confined inside an operating-system kernel sandbox via `omnia-confine` (**Seatbelt** on macOS, **Landlock + seccomp** on Linux). If neither kernel mechanism is available, the runtime refuses to start.

### Filesystem and network confinement

Inside the kernel boundary, a model worker is restricted to the minimum surface needed for its turn:

- **Shell roots:** The runtime compiles in `shell_roots = [".emery/vcs/worktrees", ".emery/vcs/integration"]` (`CURSOR_SHELL_ROOTS` in the environment is ignored). Only turns lent a directory beneath those two roots—namely `Task::Build` and `Task::Verify`—receive the `shell` tool. Source extraction and survey turns are lent read-only views outside those roots and have no shell or write tools.
- **Writable paths:** During a build or verify turn, the worker can write **only** to the lent worktree (`.emery/vcs/worktrees/<slice>` or `.emery/vcs/integration`) and to a private per-turn scratch directory that replaces `HOME` and `TMPDIR`. Your checked-out project files are never lent to a build worker.
- **Isolated `HOME` and environment:** Because `HOME` points to an empty private scratch directory, tools executed by the worker cannot read your personal `~/.ssh`, `~/.aws`, `~/.npmrc`, `~/.gitconfig`, or `~/.cargo/credentials.toml`, and operator environment variables are stripped except for those explicitly declared in the adapter's worker grant.
- **Loopback egress proxy:** Direct outbound network connections are blocked or routed through a per-worker loopback HTTP `CONNECT` proxy that permits only Cursor's relay hosts plus the exact registry hostnames declared in the target adapter's worker grant.

### Target adapter worker grants

A target adapter knows which compiler and package manager its prompts run, so it declares its required reach in `TargetAdapter::metadata().grants` (`emery_sdk::target::Grant`):

| Adapter | Granted egress hosts | Granted read-only `~/` roots | Granted environment variables |
| --- | --- | --- | --- |
| Source adapters (`intent`, `documentation`, `typescript`, `python`) | *(none)* | *(none)* | *(none)* |
| `emery:node` | `registry.npmjs.org` | `~/.nvm/versions/node`, `~/.volta`, `~/.local/share/fnm` | `VOLTA_HOME=~/.volta` |
| `emery:omnia` | `index.crates.io`, `static.crates.io` | `~/.rustup`, `~/.cargo/bin` *(never `~/.cargo` whole)* | `RUSTUP_HOME=~/.rustup` |

Before starting any turn, the SDK and backend validate every grant (`Grant::findings`):

- If a `~/` root does not exist on the host machine (for example, `~/.volta` on a machine using `nvm`), it is silently skipped.
- If a grant requests a root outside `~/`, overlaps a sensitive credential directory (`~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.azure`, `~/.kube`, `~/.docker`, `~/.config/gh`, `~/.config/gcloud`, `~/.password-store`, `~/Library/Keychains`, or any parent directory above them), or attempts to set `CURSOR_API_KEY`, the backend rejects the turn with `invalid-request`.

### Platform differences: macOS vs. Linux

Operators deploying on Linux should note three differences between macOS Seatbelt and Linux Landlock:

1. **Cooperative TCP egress:** Landlock filters TCP by port rather than destination hostname, so hostname filtering on Linux relies on standard proxy environment variables (`HTTPS_PROXY`), which `cargo`, `npm`, `pip`, `git`, and `curl` honour.
2. **`/proc` visibility:** `/proc` remains readable inside the Linux sandbox, meaning environment variables of other processes running under the same OS user ID may be visible.
3. **Subdirectory exclusions:** macOS Seatbelt can deny `.git` and `.emery` subdirectories inside a writable mount, whereas Landlock grants access at the mount root and relies on Emery's pre-commit gates to catch unauthorized writes.

---

## 4. Safe file operations and symlink protection

All file writes performed by the SDK's `write_files` and `edit_files` tools and by the engine's `Files::write` implementation pass through two shared guards in `emery-adapter`:

1. **Lexical path guard (`beneath`):** Every path must be a relative path containing no `..` segments, naming a file outside `.git/` and `.emery/` (`SKIP_DIRS`), outside `spec.md`, `design.md`, and `plan.md` (`SKIP_FILES`), and not starting with the root-level `spec/` directory (`SPECIFICATION`).*(A deeper directory such as `src/spec/` is permitted.)*
2. **No-follow filesystem walk (`standing`, `make_dirs`, `write_whole`):** Before creating parent directories or opening a file, Emery inspects every path component one segment at a time using `symlink_metadata` (`lstat`). If any parent directory or the target file itself is a symbolic link (`NoFollow::Link` or `NoFollow::Symlink`), the write is refused immediately. Parent directories are created one level at a time with `create_dir`, and new files are opened with `create_new`. This prevents a symlink planted by a repository or shell command from redirecting file writes outside the worktree.

---

## 5. Pre-commit gates

Even though the SDK's `write_files` and `edit_files` tools block prohibited paths, a build or verification turn also has access to a shell inside its worktree. To guarantee that shell commands cannot smuggle unauthorized changes or credentials into Git history, Emery runs two pre-commit gates (`build::gated` and `secrets::findings`) before every commit in `apply` and `build`:

- During `emery apply` (sealing `spec/`).
- During `emery build` after `Task::Init` (`Initialise` commit).
- During `emery build` after each slice's `Task::Build` and any AI merge resolution.
- During `emery build` after each wave's `Task::Verify` (`Wave <k> verified` commit).

### The specification guard (`spec-rewritten`)

Before reading any file contents during a build, Emery checks the list of modified, added, or deleted paths in the worktree (`vcs::pending`). If any changed path falls under the root-level `spec/` directory (`BadPath::Specification`), Emery aborts the commit and fails with `spec-rewritten` (exit code `1`), listing every offending path. The worktree is left intact for inspection, and nothing is committed.

### The secret gate (`secret-detected`)

Next, Emery scans every added or modified path in the worktree (`crates/engine/src/secrets.rs`). Each path is read through `Files::read` exactly as Git would commit it—regular files as their raw bytes, and symbolic links as their link target string (never following the symlink)—so even a broken symlink whose target string contains a token is caught.

A change is rejected with `secret-detected` (exit code `1`) if it matches either rule:

#### 1. Credential carrier filenames

Any path whose filename matches a known credential file is rejected immediately without reading its contents:

- SSH private keys: `id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` *(public keys ending in `.pub` are allowed)*
- Package and tool credential files: `.npmrc`, `.netrc`, `.git-credentials`, `.pgpass`, `.pypirc`, `.env`
- Cloud credential files: `.aws/credentials`

#### 2. Credential content patterns

For all other added or modified files (and symlink targets), Emery scans the raw bytes for:

| Pattern | Detection rule | Reported finding |
| --- | --- | --- |
| `-----BEGIN <LABEL>-----` where `<LABEL>` ends in `PRIVATE KEY` on the same line | PEM header check (catches multi-line blocks and single-line escaped strings) | `a PEM private key block` |
| `AKIA` + 16+ uppercase/digit characters | Prefix + run | `an AWS access key id` |
| `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_` + 36+ alphanumeric characters | Prefix + run | `a GitHub token` |
| `github_pat_` + 22+ token characters (`[A-Za-z0-9_-]`) | Prefix + run | `a GitHub token` |
| `npm_` + 36+ alphanumeric characters | Prefix + run | `an npm token` |
| `_authToken=` + 20+ token characters (`$` excluded, so `_authToken=${NPM_TOKEN}` passes) | Prefix + run | `an npm _authToken line` |
| `xoxa-`, `xoxb-`, `xoxp-`, `xoxr-`, `xoxs-` + 10+ token characters | Prefix + run | `a Slack token` |
| `sk_live_`, `rk_live_` + 24+ alphanumeric characters | Prefix + run | `a Stripe live key` |
| `AIza` + 35+ token characters | Prefix + run | `a Google API key` |
| `glpat-` + 20+ token characters | Prefix + run | `a GitLab token` |

When any secret is detected, Emery refuses to commit, prints every offending `<path>: <what>`, and leaves the worktree on disk so you can inspect and remove the credential.

---

## See also

- [Wave-based builds and verification](wave-builds.md) — how slice worktrees and wave verifications run and commit.
- [Revision lifecycle](revision-lifecycle.md) — how `spec/` is sealed and verified in the target repository.
- [Adapters reference](../reference/adapters.md) — how `~/.emery/adapters` stores and resolves `.wasm` packages.
- [Deployment profiles](../reference/deployment-profiles.md) — runtime host bindings, storage mounts, and environment configuration.
