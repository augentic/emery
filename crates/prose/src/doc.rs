/// An embedded Markdown document.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// The path relative to the document tree, such as `references/ids.md`.
    pub path: &'static str,
    /// The document's Markdown body.
    pub body: &'static str,
}

const TREE: &[u8] = b"prose/";

#[doc(hidden)]
#[must_use]
pub const fn within(path: &'static str) -> &'static str {
    let bytes = path.as_bytes();
    let mut start = 0;
    while start + TREE.len() <= bytes.len() {
        let at_segment = start == 0 || bytes[start - 1] == b'/';
        if at_segment && names_tree(bytes, start) {
            return path.split_at(start + TREE.len()).1;
        }
        start += 1;
    }
    panic!("a listed document must sit beneath a `prose/` directory");
}

const fn names_tree(bytes: &[u8], at: usize) -> bool {
    let mut i = 0;
    while i < TREE.len() {
        if bytes[at + i] != TREE[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns the document at `path`, if the table embeds one.
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.iter().find(|doc| doc.path == path)
}

/// Returns the body of the document at `path`, if the table embeds one.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}
