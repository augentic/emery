//! Defines the shipped `emery` runtime: the engine over the operator's package store.
//!
//! The runtime embeds the engine guest under grants fixed at compile time.
//! The invocation directory is mounted as `.`, writable: the tree a run reads
//! its sources from and the repository a build starts from, its working
//! copies cut beneath `.emery/vcs`. Every adapter is a package, read from
//! the store `~/.emery/adapters` before anything is fetched — the file
//! `namespace_name@version.wasm` there is that release on this machine — and
//! fetched through the one route compiled in here otherwise: the `emery`
//! namespace to `augentic.io`. A release the binary routes nowhere arrives by
//! `wkg get <reference> -o ~/.emery/adapters/`. The store lies apart from the
//! project, so a component is never loaded from a tree a run can write, and
//! a run from the operator's home is refused at startup.
//!
//! Version control is the operator's own `git`, run by the git backend at
//! the paths the engine lends beneath `.`: the repository a build labels and
//! the clones a repository source or target is read from under `.emery/vcs`.
//!
//! Cursor answers the model, and every worker it runs for a lent tree is
//! confined by the kernel: it reads the system, the tree, and its own
//! scratch, writes the tree and the scratch, and reaches the network through
//! the backend's proxy alone. The shell runs on a build's working copies
//! alone — `.emery/vcs/worktrees` and `.emery/vcs/integration`, compiled in
//! here — so a source adapter's lend is read-only. What a worker may reach
//! beside that is the operator's environment: `CURSOR_READ_ROOTS` for a
//! toolchain outside the system's own, `CURSOR_EGRESS` for the hosts an
//! install fetches from, `CURSOR_WORKER_ENV` for the variables it carries.
//! Nothing in the environment turns the boundary off.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia::FromEnv as _;
        use omnia_cursor::{Client as Cursor, ConnectOptions as CursorOptions};
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_git::Client as Git;
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_vcs::WasiVcs;

        omnia::runtime!({
            mode: command,
            guests: [{ path: env!("EMERY_GUEST") }],
            plugins: {
                store: "~/.emery/adapters",
                registries: include_str!("wasm-pkg.toml"),
            },
            mounts: [{ name: ".", path: ".", writable: true }],
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor(CursorOptions {
                    shell_roots: vec![".emery/vcs/worktrees".into(), ".emery/vcs/integration".into()],
                    ..CursorOptions::load_env()?
                }),
                WasiKeyValue: Filesystem(ConnectOptions { root: ".emery/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".emery/storage".into() }),
                WasiVcs: Git,
            },
        });
    }
    _ => {
        fn main() {}
    }
}
