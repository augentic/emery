# Deployment profiles

How the `emery` runtime binds engine storage and version control and mounts the filesystem, and how deployments other than the shipped local binary swap those bindings without touching engine code. A profile is host policy: one `omnia::runtime!` invocation choosing what backs the `wasi:keyvalue` / `wasi:blobstore` / `omnia:vcs` capability imports and which directories the guests see. The engine ships fixed key, container, mount, and repository-path formulas and never learns which backing or host directory it runs over.

## The storage boundary

Engine state — the revision store and its current revision id — is reachable only through the storage capabilities (`omnia_sdk::StateStore` / `BlobStore` on the guest side). The names the engine uses are flat, deployment-neutral formulas:

| Surface             | Kind                            | Name                                                    |
| ------------------- | ------------------------------- | ------------------------------------------------------- |
| Current revision id | keyvalue key                    | `current-revision`                                      |
| Revision documents  | blobstore container `revisions` | `<id>/spec.json`, `<id>/design.json`, `<id>/plan.json` |

The host side of the boundary is a backend type implementing `omnia::Backend` (connection options compiled into the `hosts:` row, or loaded from the environment when the row carries none) plus the host context traits `WasiKeyValueCtx` and `WasiBlobstoreCtx`. Bucket and container identifiers cross the boundary exactly once — on `open_bucket` and the container methods — which is where a profile may rewrite them.

The `spec.md` / `design.md` / `plan.md` projections a project keeps beside its code are not engine state either: they are `emery show` output the skill writes into the working tree for review, and nothing reads them back. Nor is what `emery build` commits: the labelled history is the repository's, and the engine keeps no record of it. Loaded components are not engine state: every adapter is an exact package reference, loaded through the deployment's `omnia:plugins/loader` capability inside the grant the shipped runtime fixes at compile time — the `plugins:` block of the `runtime!` invocation, a package store and the registries it routes. The store, `~/.emery/adapters`, is the authority for a release on this machine: a reference it holds is read from it, fresh on every run and with no network, and one it lacks is fetched from the registry the deployment routes its namespace to, written to the store once, and read from there after ([Adapters](adapters.md)). The routing is compiled in — `emery` to `augentic.io`, nothing else — and no file on the machine redirects it; a namespace the binary routes nowhere is filled by hand (`wkg get <reference> -o ~/.emery/adapters/`), which is also how an operator who must bound registry egress mirrors the releases a project names. `emery.toml` names which package and, through `digest`, which bytes, and a build that rewrites it cannot redirect a fetch. Earlier trees (`.omnia/`, the engine's root before `.emery/`, with its `cache/wasm-pkg` and `storage/plugins/`; `~/.emery/wasm-pkg.toml`) are orphaned — nothing reads them; delete them freely. Integrity binds the resolved sha256 digest to the exact bytes the host executes — a `[[source]] digest` pins it, checked on the bytes that answered, stored or fetched — never an engine-owned mutable sidecar.

## The shipped profile: local filesystem

The `hosts:` block of the `omnia::runtime!` invocation in [`src/main.rs`](../../src/main.rs) binds both storage hosts to `omnia_filesystem::Client` with the root compiled into the invocation: a durable, network-free store at `.emery/storage` under the invocation directory (`blobstore/` and `keyvalue/`). The root is deployment policy, not an environment tunable — retargeting it means shipping a different profile, never setting `FILESYSTEM_ROOT`. One invocation directory is one project; isolation between projects is the filesystem root itself. Revisions survive restart; what a build commits is the repository's, never the engine's.

The same block binds `omnia:vcs` to `omnia_git::Client`, one `git` process per operation over the `git` on the host's `PATH` (`GIT_BINARY` names another; 2.5 is the oldest that runs). The engine names every repository and working copy by a path beneath the project mount — the project itself as `.`, its clones under `.emery/vcs/repos/`, the working copies it cuts under `.emery/vcs/sources/<name>`, `.emery/vcs/worktrees/<slice>`, and `.emery/vcs/integration` — and the host resolves each against the mount before git sees it; a guest never holds a host path, a credential, or a git process. Credentials are git's own: whatever lets `git clone <url>` and `git push` run where `emery` runs — an SSH agent, a credential helper — serves a `[[source]] repository`, a `[target] repository`, and a `remote`. A deployment without git on the host refuses at startup, before any verb runs.

### The mount and the store

The `mounts:` block fixes one directory, and the `plugins:` block one more, after the W^X rule that what a run can write it never loads code from:

| Root       | Host directory                      | Access     | Role                                                                                                                                                      |
| ---------- | ----------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.`        | the invocation directory            | writable   | The project, mounted: the tree `specify` lends its source adapters to read, and the repository `build` starts from. `.emery/storage` and `.emery/vcs/` sit beneath it. |
| the store  | `~/.emery/adapters`                 | host-only  | The package store, no mount at all: every adapter a reference names is a file beneath it, read by the host's loader and reachable by no guest.             |

The store is `$HOME`'s, apart from any project, so a release is fetched or copied once for every project on the machine and no run writes where the next one loads from. A missing store is an empty one, created on the first fetch. omnia refuses a store that lies beneath a writable mount — a guest could rewrite what the deployment loads — so running `emery` from `$HOME` itself is refused at startup, before any verb runs: a project is a directory of its own.

Beneath the mount, `.emery/vcs/` is the engine's: `repos/<key>` is one clone per repository URL a run names (the key a hash of the URL normalised), cloned on the first run that names it, fetched on every one after, and kept between runs; `sources/<name>` is the working copy a repository source is read in, `worktrees/<slice>` the one each slice of a wave is built in, and `integration` the one a build merges its slices into and verifies, each cut for the run and removed at its end — the integration copy, and the slice copies of a wave that failed, left in place when a run fails, for inspection, and removed by the next build. Nothing of the tree is state the engine reads back: delete `.emery/vcs/` and the next run clones again, and what a build resumes from is the label's history in the repository, not the tree. Add `.emery/` to the project's `.gitignore`; a build never counts it against the base, but git would list it.

A source adapter's extraction turn lends the project tree, or the working copy a repository source is read in, the same way it always has — the lend's path and the brief's `$SOURCE_DIR` are unchanged — but the mount beneath it is writable, so an extraction turn that wrote would not be stopped by the mount; the SDK's prompts ask for claims alone, the claim gate accepts nothing but them, and no extraction or survey turn declares a write tool. A build turn lends a slice's working copy writable and writes it through the SDK's `write_files` tool, which the guest serves through this mount: the tool refuses a path under `.emery/` or `.git/` or naming a projection, a call naming one writing nothing, so neither the store nor the repository's own directory is written by a build, while the mount itself would permit it — the revision store still detects a document rewritten beneath it and fails closed on the next read. A build turn runs with the shell too, inside the worker boundary below, so what its install and checks leave in the copy lands in the change set beside what `write_files` wrote. A verify turn lends the integration working copy with the shell and the same `write_files` tool, so it repairs what its checks find; what it writes and what its checks leave behind is sealed as the wave's own commit. Nothing is sealed unread: every path a build or a verification left changed goes through the secret gate first, and a credential among them — a key file or a `.npmrc` by name, a PEM block or a provider's token by content — refuses the slice or the wave with `secret-detected`, the copy left as it was written ([build](cli/build.md)). The project checkout is never lent to a build at all: what a build changes reaches the checkout only as the operator merges the label.

### What a build's worker may reach

Every turn's worker — the `cursor-sdk-bridge` the Cursor backend spawns for a completion — runs inside the kernel boundary [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor) lays with omnia-confine: Seatbelt on macOS, Landlock and seccomp on Linux, and a refusal at startup where neither runs. Inside, the worker reads the system (`/usr`, `/bin`, `/opt/homebrew`, and their kin), the lent tree, and a private scratch that stands in for `HOME` and `TMPDIR`; writes the tree and the scratch alone; and reaches the network through the backend's loopback `CONNECT` proxy, which admits Cursor's own hosts and nothing else until the operator lists more. The shipped profile compiles two grants into the row beside the store and the registries:

| Grant | Value | Role |
| --- | --- | --- |
| shell roots | `.emery/vcs/worktrees`, `.emery/vcs/integration` | A lend beneath one runs with the `shell` tool: a slice's build and a wave's verify run the tree's own install and tests. A source adapter's lend sits elsewhere and reads alone. `CURSOR_SHELL_ROOTS` in the environment is ignored, since the row fixes them. |
| substrate | `Auto` | The platform's kernel boundary, set in code; nothing in the environment turns it off. |

What differs from one machine to the next is the operator's, read from the environment the row leaves open:

| Variable | Value | When |
| --- | --- | --- |
| `CURSOR_READ_ROOTS` | directories, `:`-separated, each existing | A toolchain outside the system's own: `node` under nvm, volta, or fnm lies under `$HOME`, which the scratch `HOME` hides, so a build that needs it names its root here (`~/.nvm/versions/node/v22.0.0`, `~/.volta`). A package cache the install may read goes here too. |
| `CURSOR_EGRESS` | hosts, `,`-separated; `*.suffix` for a domain | What an install fetches from: `registry.npmjs.org` for npm, `*.pypi.org,files.pythonhosted.org` for pip, `crates.io,static.crates.io,index.crates.io` for cargo. A loopback mirror is listed by its IP literal and named to the tool through `CURSOR_WORKER_ENV` (`npm_config_registry=http://127.0.0.1:4873`). |
| `CURSOR_WORKER_ENV` | variable names, `,`-separated | Further variables the worker's environment carries from the host's, which is otherwise built from nothing: `PATH`, `LANG`, `TZ`, `TERM`, the proxy variables, and the bridge's own settings. `CURSOR_API_KEY` is refused; no `GIT_*` variable crosses. |

The scratch `HOME` means a worker reads no `~/.npmrc`, `~/.gitconfig`, `~/.ssh`, or `~/.aws`: a credential the install would have found there is not there, which is what the gate above relies on, and a tool that needs one is handed it through `CURSOR_WORKER_ENV` by name or does without. On Linux the posture is looser in ways the operator weighs: Landlock fences TCP by port alone, so egress is cooperative — the proxy variables, which npm, pip, cargo, git, and curl honour — and a client that ignores them and carries its own address reaches the network; `/proc` is readable whole, so other same-uid processes' environments are readable inside; and `.git` and `.emery` beneath the lend are reachable where macOS denies them. omnia-cursor's README states the posture in full; nothing of it is emery's to restate.

## Project-id-keyed shared backings

A multi-project deployment can scope every bucket and container under a project id. This requires a genuinely shared backing: a command-mode process over in-memory defaults creates one fresh store and one project id, so it cannot demonstrate cross-project isolation or persistence.

The `multi_project` scenario in [`tests/specify.rs`](../../tests/specify.rs) exercises the invariant directly: two project-scoped views over one shared scripted store commit and show independent revisions, and no unprefixed key is written. A concrete deployment supplies its shared backend clients and rewrites identifiers at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary; that host-specific configuration does not belong in the engine.

## Remote backings

`omnia-backends` ships host clients that drop into the same `hosts:` table:

| Backend            | keyvalue | blobstore | vcs | Environment configuration |
| ------------------ | -------- | --------- | --- | ------------------------- |
| `omnia-filesystem` | yes      | yes       | —   | `FILESYSTEM_ROOT`         |
| `omnia-redis`      | yes      | —         | —   | `REDIS_URL`               |
| `omnia-nats`       | yes      | yes       | —   | `NATS_ADDR`               |
| `omnia-mongodb`    | —        | yes       | —   | `MONGODB_URL`             |
| `omnia-azure-blob` | —        | yes       | —   | `AZURE_BLOB_ENDPOINT`     |
| `omnia-git`        | —        | —         | yes | `GIT_BINARY`              |

The environment variables apply to a bare `hosts:` row; a row carrying compiled-in connect options (`Backend(options)`, as the shipped profile does for its filesystem root) ignores them. Credentials and endpoints live in the host binding's environment, never in engine state or operator files. `omnia-git` is the one `omnia:vcs` backend; a profile that swaps it supplies another `WasiVcsCtx` over the same deployment-local paths.

The model binding reads its own environment the same way, around the two grants the row compiles in. The shipped `WasiModel: Cursor` row takes `CURSOR_API_KEY`, `CURSOR_MODEL`, `CURSOR_TIMEOUT_SECS`, `CURSOR_INACTIVITY_SECS`, `CURSOR_MAX_AGENTS`, `CURSOR_MAX_TOOL_CALLS` (default `128`), the last the guest tool calls one completion may make — a build turn's `write_files` and reference reads among them — before the host ends it as `budget-exhausted`, and the three the worker boundary reads, `CURSOR_READ_ROOTS`, `CURSOR_EGRESS`, and `CURSOR_WORKER_ENV` ([above](#what-a-builds-worker-may-reach)); `CURSOR_SHELL_ROOTS` is ignored, since the row fixes the shell roots. The engine sets none of them.

> [!WARNING]
> Identifier grammar is backend policy. The filesystem backend rejects `/` inside a bucket or container name (path-traversal fencing), so a project-id prefix targeting it needs a single-segment delimiter (for example `<project>--revisions`) or per-project roots. The in-memory and remote backends accept `/`-separated identifiers.

Remote-binding performance is unmeasured: the numbers stay unconfirmed until a remote backing is deployed and wall-clocked.
