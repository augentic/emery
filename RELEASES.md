## 0.38.0

Unreleased

### Added

- The `specify` re-mine diff flags a changed preamble: `SpecDiff` / `DesignDiff` carry `preamble: bool`, the JSON envelope carries it per document, and text mode prints `spec.md ~ preamble` / `design.md ~ preamble`.

### Changed

- `emery_prose::registry::body` returns `Option<&'static str>` instead of panicking on a path the build did not embed; `emery_prose::emit` writes the `docs()` accessor beside the table it generates and the `registry!` macro only includes that file — the SDK's `SourceAdapter::prompt` reads the extraction prompt from `docs()` and reports a miss as `server_error`.
- `emery-source` exposes every public item at the crate root (`emery_source::{Source, SourceInput, SourceContent, AdapterMetadata, Evidence, Claim, ClaimKind, Authority, Backing, CLAIM_ID_REGEX, is_kebab}`); the `types` and `claims` modules are gone, as are the test-only `SourceInput::workspace` / `SourceInput::value` constructors.
- `emery-adapter` reshapes the `SourceAdapter` trait around a provided `evidence` call: an adapter declares `const SOURCE` (the noun the turn names its source by), `docs()`, and `extract(model, ctx)`, and asks for evidence with `Self::evidence(model, ctx, Material::Bound | Material::Prepared(note))`. `Context` is `{ adapter_id, input }`; `EvidenceTurn`, the free `evidence` fn, `content_note`, and the `types` module are deleted; the crate re-exports the contract's types at its root and omnia's `model` module in place of fourteen individual model types.
- The SDK's user turn always offers the `list_docs` / `read_doc` reference tools (every adapter embeds at least its prompt) and describes a bound workspace as "the `<SOURCE>` source tree".

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.38.x](https://github.com/augentic/emery/blob/release-0.38.0/RELEASES.md)
* [0.37.x](https://github.com/augentic/emery/blob/release-0.37.0/RELEASES.md)
* [0.36.x](https://github.com/augentic/emery/blob/release-0.36.0/RELEASES.md)
* [0.35.x](https://github.com/augentic/emery/blob/release-0.35.0/RELEASES.md)
* [0.34.x](https://github.com/augentic/emery/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
