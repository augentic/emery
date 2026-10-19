//! Defines the deployment the `emery` and `emery-dev` runtimes share.
//!
//! The policy is compiled in once: the current directory mounted writable as
//! `.`, adapters loaded from `~/.emery/adapters` or fetched from
//! `augentic.io`, state under `.emery/storage`, version control via host
//! `git`, and model workers whose shell runs on [`SHELL_ROOTS`] alone. The
//! one thing a runtime chooses is the backend answering the model.

/// The roots beneath the mount a model turn's shell runs on.
pub const SHELL_ROOTS: [&str; 2] = [".emery/vcs/worktrees", ".emery/vcs/integration"];

/// Expands to the shared runtime over `$backend`, the client answering the model.
///
/// `$options` is the backend's connect options, read from the environment
/// through `FromEnv` with its `shell_roots` set to [`SHELL_ROOTS`].
macro_rules! runtime {
    ($backend:ident, $options:ident) => {
        use ::omnia::FromEnv as _;
        use ::omnia_filesystem::{Client as Filesystem, ConnectOptions as FilesystemOptions};
        use ::omnia_git::Client as Git;
        use ::omnia_wasi_blobstore::WasiBlobstore;
        use ::omnia_wasi_keyvalue::WasiKeyValue;
        use ::omnia_wasi_model::WasiModel;
        use ::omnia_wasi_otel::{OtelDefault, WasiOtel};
        use ::omnia_wasi_vcs::WasiVcs;

        ::omnia::runtime!({
            mode: command,
            guests: [{ path: env!("EMERY_GUEST") }],
            plugins: {
                store: "~/.emery/adapters",
                registries: include_str!("wasm-pkg.toml"),
            },
            mounts: [{ name: ".", path: ".", writable: true }],
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: $backend($options {
                    shell_roots: $crate::deployment::SHELL_ROOTS
                        .iter()
                        .map(::std::path::PathBuf::from)
                        .collect(),
                    ..$options::load_env()?
                }),
                WasiKeyValue: Filesystem(FilesystemOptions { root: ".emery/storage".into() }),
                WasiBlobstore: Filesystem(FilesystemOptions { root: ".emery/storage".into() }),
                WasiVcs: Git,
            },
        });
    };
}

pub(crate) use runtime;
