//! Embeds the synthesis prompt corpus under `prose/` and checks its links.
//!
//! A prompt that references a missing document is a build failure rather
//! than a run-time surprise.

fn main() {
    emery_prose::emit("prose");
}
