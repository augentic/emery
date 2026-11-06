//! Defines layouts a source records whole, which the engine stores by digest.
//!
//! A [`Layout`] is opaque to the engine: it never parses the body. The
//! design names the layout by its digest and shows its [`LayoutView`]s, and
//! every slice carries the body to the target.

/// A layout a source recorded whole, carried by content rather than by path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    /// The body's media type, which tells a target how to read it.
    pub media_type: String,
    /// The recorded layout, opaque to the engine.
    pub body: Vec<u8>,
    /// Images rendered from the body, in the order the design shows them.
    pub views: Vec<LayoutView>,
}

/// An image rendered from a [`Layout`] for review.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutView {
    /// What the image shows, such as `1280px wide`.
    pub label: String,
    /// The image's media type, such as `image/svg+xml`.
    pub media_type: String,
    /// The image as text, such as an SVG document.
    pub body: String,
}

impl Layout {
    // Every rule the layout breaks, each naming the layout by `index`.
    pub(crate) fn findings(&self, index: usize) -> Vec<String> {
        let mut findings = Vec::new();
        if !is_media_type(&self.media_type) {
            findings.push(format!(
                "layout {index}: media type `{}` is not `<type>/<subtype>`",
                self.media_type
            ));
        }
        if self.body.is_empty() {
            findings.push(format!("layout {index}: the body is empty"));
        }
        for (position, view) in self.views.iter().enumerate() {
            if view.label.trim().is_empty() || view.label.contains('\n') {
                findings
                    .push(format!("layout {index}, view {position}: the label is not one line"));
            }
            if !view.media_type.starts_with("image/") || !is_media_type(&view.media_type) {
                findings.push(format!(
                    "layout {index}, view {position}: media type `{}` is not an `image/` type",
                    view.media_type
                ));
            }
            if view.body.is_empty() {
                findings.push(format!("layout {index}, view {position}: the image is empty"));
            }
        }
        findings
    }
}

fn is_media_type(value: &str) -> bool {
    value.split_once('/').is_some_and(|(kind, subtype)| {
        let token = |part: &str| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&b))
        };
        token(kind) && token(subtype)
    })
}
