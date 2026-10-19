//! Defines the `emery-dev` runtime binary, the shipped runtime over a local model.
//!
//! The same deployment as `emery` with `omnia-genai` answering the model in
//! place of Cursor, so a build or a verify runs against any
//! `OpenAI`-compatible endpoint. Built under the `genai` feature alone and
//! never released:
//!
//! ```text
//! cargo build --release --features genai --bin emery-dev
//!
//! # an Ollama server: a bare name routes to genai's Ollama adapter, which
//! # speaks Ollama's own API at the endpoint and needs no key
//! GENAI_MODEL=qwen3-coder GENAI_ENDPOINT=http://127.0.0.1:11434/ \
//!   target/release/emery-dev build --config emery.toml
//!
//! # a vLLM or any OpenAI-compatible server: `openai::` names the adapter,
//! # the key whatever the server expects, any value where it checks none
//! GENAI_MODEL=openai::qwen3-coder GENAI_ENDPOINT=http://127.0.0.1:8000/v1/ \
//!   OPENAI_API_KEY=none target/release/emery-dev build --config emery.toml
//! ```
//!
//! - `GENAI_MODEL` is the name the server serves, under the adapter's
//!   namespace where the name alone would route elsewhere. The Ollama
//!   adapter sends no answer schema, so the prompt alone steers the shape;
//!   the `OpenAI` adapter sends it.
//! - `GENAI_ENDPOINT` is the server's base URL.
//! - `GENAI_MAX_ROUNDS` (128) bounds the provider round-trips of one turn,
//!   `GENAI_MAX_TOOL_CALLS` (128) its tool calls, and
//!   `GENAI_SHELL_TIMEOUT_SECS` (600) one shell command.
//!
//! The shell runs `sh -c` per command inside the kernel boundary the shipped
//! runtime confines Cursor's worker to, on the same two roots, and what it
//! reaches beyond the lent tree is the target adapter's `grants`, as under
//! `emery`.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia::FromEnv as _;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_genai::{Client as GenAi, ConnectOptions as GenAiOptions};
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
                WasiModel: GenAi(GenAiOptions {
                    shell_roots: vec![".emery/vcs/worktrees".into(), ".emery/vcs/integration".into()],
                    ..GenAiOptions::load_env()?
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
