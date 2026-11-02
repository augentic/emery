//! Precompiles a `wasm32-wasip2` component for the release `emery` binary.
//!
//! The root build script runs this under `--release` as
//! `emery-precompile <wasm> <output> --target <triple>`. Compile options are
//! the runtime defaults.

cfg_select! {
    not(target_arch = "wasm32") => {
        use std::path::{Path, PathBuf};

        use anyhow::{Context as _, Result, bail};
        use omnia_compile::CompileOptions;

        const USAGE: &str = "usage: emery-precompile <wasm> <output> --target <triple>";

        fn main() -> Result<()> {
            #[expect(
                clippy::disallowed_methods,
                reason = "a host tool, not a guest: argv is the build script's call"
            )]
            let args: Vec<_> = std::env::args_os().skip(1).collect();
            let (wasm, output, target) = match args.as_slice() {
                [wasm, output, flag, target] if flag == "--target" => {
                    let target = target.to_str().context("the target is not UTF-8")?;
                    (Path::new(wasm), PathBuf::from(output), target)
                }
                _ => bail!("{USAGE}"),
            };

            omnia_compile::compile(wasm, Some(output), Some(target), &CompileOptions::default())
                .with_context(|| format!("compiling {} for `{target}`", wasm.display()))
        }
    }
    _ => {
        // host-only; empty so the workspace wasm32 clippy pass can compile this member
        fn main() {}
    }
}
