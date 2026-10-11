//! Precompiles the engine guest into the artifact the release `emery` binary embeds.
//!
//! `emery-precompile <wasm> <output> --target <triple>` compiles the
//! `wasm32-wasip2` component at `<wasm>` for `<triple>` under the runtime's
//! default compile settings, so the build shell's environment steers nothing,
//! and writes it to `<output>`. The crate's build script builds and runs this
//! tool in a release build alone: it depends on the compiler and nothing of
//! the runtime, and no check or test of the workspace links either through
//! it.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        use std::ffi::OsString;
        use std::path::PathBuf;

        use anyhow::{Context as _, Result, bail};
        use omnia_compile::CompileOptions;

        const USAGE: &str = "usage: emery-precompile <wasm> <output> --target <triple>";

        fn main() -> Result<()> {
            #[expect(
                clippy::disallowed_methods,
                reason = "a host tool, not a guest: argv is the build script's call"
            )]
            let args: Vec<OsString> = std::env::args_os().skip(1).collect();
            let [wasm, output, flag, triple] = args.as_slice() else {
                bail!("{USAGE}");
            };
            if flag != "--target" {
                bail!("{USAGE}");
            }
            let triple =
                triple.to_str().with_context(|| format!("{USAGE}: the triple is not UTF-8"))?;

            let wasm = PathBuf::from(wasm);
            let output = PathBuf::from(output);
            omnia_compile::compile(&wasm, Some(output), Some(triple), &CompileOptions::default())
                .with_context(|| format!("compiling {} for `{triple}`", wasm.display()))
        }
    }
    _ => {
        fn main() {}
    }
}
