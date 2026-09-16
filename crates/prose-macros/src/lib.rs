//! The procedural macro behind `emery_prose::include_prose!`.
//!
//! Nothing names this crate directly: `emery_prose::include_prose!` forwards
//! to [`include_prose!`] with the path of its `Doc` type, so the expansion
//! constructs the caller's `Doc` whatever the crate is called there.

mod tree;

use std::path::PathBuf;

use proc_macro::{Delimiter, Span, TokenStream, TokenTree};

/// Embeds the Markdown tree at a path relative to the invoking file as a `Doc` table.
///
/// The input is the path of the `Doc` type to construct, a comma, and the
/// tree as a string literal. The expansion is `&[Doc { path, body }, ..]`,
/// one entry per `.md` file beneath the tree, sorted by tree-relative path,
/// each body an `include_str!` of the file. A missing or empty tree, a
/// relative link with no target, or a symlink cycle is a compile error at the
/// literal.
#[proc_macro]
pub fn include_prose(input: TokenStream) -> TokenStream {
    match expand(input) {
        Ok(tokens) => tokens,
        Err((span, message)) => compile_error(span, &message),
    }
}

// Splits the input into the `Doc` path and the tree literal, then embeds.
fn expand(input: TokenStream) -> Result<TokenStream, (Span, String)> {
    let (doc, literal) = split(input)?;
    let span = literal.span();
    let tree = string(&literal).ok_or_else(|| {
        (span, "`include_prose!` takes the tree as a plain string literal".to_string())
    })?;

    let file = span.local_file().ok_or_else(|| {
        (span, "`include_prose!` resolves the tree relative to a file on disk".to_string())
    })?;
    let root = file.parent().map_or_else(|| PathBuf::from(&tree), |dir| dir.join(&tree));

    let entries = tree::table(&root).map_err(|err| (span, format!("{err:#}")))?;
    Ok(table(&doc, &entries))
}

// Splits `Doc, "tree"` at the first top-level comma.
fn split(input: TokenStream) -> Result<(TokenStream, TokenTree), (Span, String)> {
    let mut tokens = input.into_iter();
    let doc: TokenStream = tokens
        .by_ref()
        .take_while(|token| !matches!(token, TokenTree::Punct(p) if p.as_char() == ','))
        .collect();

    let rest: Vec<TokenTree> = tokens.collect();
    match rest.as_slice() {
        [token] => Ok((doc, unwrap(token.clone()))),
        _ => Err((
            Span::call_site(),
            "`include_prose!` takes the `Doc` path and one tree literal".to_string(),
        )),
    }
}

// Strips the transparent groups a `macro_rules!` fragment arrives wrapped in.
fn unwrap(mut token: TokenTree) -> TokenTree {
    loop {
        match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
                let mut inner = group.stream().into_iter();
                match (inner.next(), inner.next()) {
                    (Some(only), None) => token = only,
                    _ => return TokenTree::Group(group),
                }
            }
            other => return other,
        }
    }
}

// Returns the text of a plain `"…"` literal without escapes.
fn string(token: &TokenTree) -> Option<String> {
    let TokenTree::Literal(literal) = token else { return None };
    let text = literal.to_string();
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('\\')).then(|| inner.to_string())
}

// Builds `&[Doc { path: "…", body: include_str!("…") }, …]` over the entries.
fn table(doc: &TokenStream, entries: &[tree::Entry]) -> TokenStream {
    let mut items = TokenStream::new();
    for entry in entries {
        let file = entry.file.display().to_string();
        let fields: TokenStream =
            format!("path: {:?}, body: ::core::include_str!({file:?})", entry.path)
                .parse()
                .expect("a document entry is valid Rust");

        items.extend(doc.clone());
        items.extend([
            TokenTree::Group(proc_macro::Group::new(Delimiter::Brace, fields)),
            TokenTree::Punct(proc_macro::Punct::new(',', proc_macro::Spacing::Alone)),
        ]);
    }

    let mut table = TokenStream::new();
    table.extend([
        TokenTree::Punct(proc_macro::Punct::new('&', proc_macro::Spacing::Alone)),
        TokenTree::Group(proc_macro::Group::new(Delimiter::Bracket, items)),
    ]);
    table
}

// Emits `compile_error!("message")` pointing at `span`.
fn compile_error(span: Span, message: &str) -> TokenStream {
    let tokens: TokenStream = format!("::core::compile_error!({message:?})")
        .parse()
        .expect("a compile_error! invocation is valid Rust");
    tokens
        .into_iter()
        .map(|mut token| {
            token.set_span(span);
            token
        })
        .collect()
}
