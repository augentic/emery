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
        #[path = "deployment.rs"]
        mod deployment;

        use omnia_genai::{Client as GenAi, ConnectOptions as GenAiOptions};

        deployment::runtime!(GenAi, GenAiOptions);
    }
    _ => {
        fn main() {}
    }
}
