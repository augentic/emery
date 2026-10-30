# Install, pin, and vendor Wasm adapters

This guide shows how to manage WebAssembly (Wasm) adapters in `~/.emery/adapters`, pin adapter SHA-256 digests in `emery.toml`, install third-party or local development adapters, and pre-populate the store for offline or CI environments.

## Contents

- [How Emery resolves adapters](#how-emery-resolves-adapters)
- [Pin an adapter to a SHA-256 digest](#pin-an-adapter-to-a-sha-256-digest)
- [Install a third-party adapter with `wkg`](#install-a-third-party-adapter-with-wkg)
- [Install a locally built `-dev` adapter](#install-a-locally-built--dev-adapter)
- [Pre-populate `~/.emery/adapters` for offline or CI runs](#pre-populate-emeryadapters-for-offline-or-ci-runs)
- [Replace or remove cached adapters](#replace-or-remove-cached-adapters)
- [See also](#see-also)

---

## How Emery resolves adapters

Every adapter reference has the exact form `<namespace>:<name>@<version>` (for example, `emery:typescript@0.39.0`). When a command runs, Emery looks up the adapter in two steps:

1. **Local store first (`~/.emery/adapters`):** Emery checks for `~/.emery/adapters/<namespace>_<name>@<version>.wasm` (with the colon `:` replaced by an underscore `_`). If the file exists, Emery verifies its `digest` (if configured) and loads it immediately with zero network traffic.
2. **Compiled-in registry (`emery` namespace only):** If the file is missing from `~/.emery/adapters`, Emery fetches `emery:*` packages from `augentic.io`, caches the `.wasm` file in `~/.emery/adapters/`, and loads it. Any namespace other than `emery` is never fetched over the network by Emery and must be placed in `~/.emery/adapters/` ahead of time.

---

## Pin an adapter to a SHA-256 digest

To guarantee that every developer and CI runner executes the exact same `.wasm` binary, add a `digest` field to your `[[source]]` or `[target]` entry in `emery.toml`:

1. Compute the SHA-256 hash of the cached `.wasm` file in `~/.emery/adapters/`:

   ```bash
   shasum -a 256 ~/.emery/adapters/emery_typescript@0.39.0.wasm
   # e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  ...
   ```

2. Add `digest = "sha256:<hex>"` to `emery.toml`:

   ```toml
   [[source]]
   name = "api"
   adapter = "emery:typescript@0.39.0"
   path = "src/"
   digest = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
   ```

If the file on disk (or fetched from the registry) does not match the pinned hash, Emery aborts before instantiating the component with `refused` (exit `1`).

> [!IMPORTANT]
> If you reference the same adapter package in multiple `[[source]]` entries within one run, they must use the same `@version` and (if pinned) the same `digest`.

---

## Install a third-party adapter with `wkg`

Because Emery only routes the `emery` namespace to `augentic.io` (and ignores `~/.emery/wasm-pkg.toml` so project files cannot redirect fetches), adapters under any other namespace (such as `acme:ledger@2.1.0`) must be fetched into `~/.emery/adapters/` using [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools):

```bash
mkdir -p ~/.emery/adapters
wkg get acme:ledger@2.1.0 -o ~/.emery/adapters/
```

`wkg get -o ~/.emery/adapters/` writes `~/.emery/adapters/acme_ledger@2.1.0.wasm`, which matches the exact filename Emery looks for.

---

## Install a locally built `-dev` adapter

When developing or testing a custom adapter (or a modified build of a first-party adapter from `augentic/emery-adapters`), build the `wasm32-wasip2` component and copy the raw `.wasm` file into `~/.emery/adapters/` under a `-dev` version tag:

```bash
# Build the component for wasm32-wasip2
cargo build -p emery-typescript --target wasm32-wasip2 --release

# Copy into ~/.emery/adapters/<namespace>_<name>@<version>.wasm
mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/emery_typescript.wasm \
  ~/.emery/adapters/emery_typescript@0.40.0-dev.wasm
```

Then reference `adapter = "emery:typescript@0.40.0-dev"` in your `emery.toml`.

> [!CAUTION]
> Always copy the raw `.wasm` component—never a precompiled `.cwasm` file. The plugin loader rejects precompiled artifacts with `refused`.

---

## Pre-populate `~/.emery/adapters` for offline or CI runs

To run Emery in an air-gapped environment or cache adapters in CI:

1. Pre-fetch all required adapters into `~/.emery/adapters/` during your environment setup or CI cache step:

   ```bash
   mkdir -p ~/.emery/adapters
   wkg get emery:intent@0.39.0 -o ~/.emery/adapters/
   wkg get emery:typescript@0.39.0 -o ~/.emery/adapters/
   wkg get emery:omnia@0.39.0 -o ~/.emery/adapters/
   ```

2. Because Emery always checks `~/.emery/adapters/` before making any network request, subsequent `emery specify`, `emery amend`, and `emery build` runs will load the cached `.wasm` files directly from disk.

---

## Replace or remove cached adapters

A file in `~/.emery/adapters/` is treated as immutable once present—Emery never re-fetches a version that already exists in the store.

- When iterating on a local `-dev` adapter, re-run `cp` after every `cargo build` to overwrite the `.wasm` file in `~/.emery/adapters/`.
- To force Emery to re-download a published `emery:*` release from `augentic.io`, delete the cached file first:

  ```bash
  rm ~/.emery/adapters/emery_typescript@0.39.0.wasm
  ```

---

## See also

- [Adapters reference](../reference/adapters.md) — package reference rules, store invariants, and error codes.
- [Author a custom source or target adapter](author-an-adapter.md) — write your own adapter in Rust with `emery-sdk`.
