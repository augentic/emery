//! Extracts leaf routine spans, imports, and top-level values from brace-delimited code.

use super::Scanned;
use super::lexer::{Dialect, Keyword, Lexer, Pairs, Token, TokenKind};
use crate::specify::snapshot::LineSpan;

pub struct BraceScanner {
    tokens: Vec<Token>,
    pairs: Pairs,
    dialect: Dialect,
    out: Scanned,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    TopLevel,
    Container,
}

enum BodySearch {
    Body(usize),
    NoBody(usize),
}

enum Initializer {
    None,
    Value,
    ArrowFn,
}

impl BraceScanner {
    #[must_use]
    pub fn scan(text: &str, lines: Vec<u64>, dialect: Dialect) -> Scanned {
        let tokens = Lexer::tokenize(text, dialect);
        let pairs = Pairs::of(&tokens);
        let len = tokens.len();
        let mut scanner = Self {
            tokens,
            pairs,
            dialect,
            out: Scanned::with_lines(lines),
        };
        scanner.walk_statements(0, len, Scope::TopLevel);
        scanner.out.finalise();
        scanner.out
    }

    fn kind(&self, idx: usize) -> Option<TokenKind> {
        self.tokens.get(idx).map(|tok| tok.kind)
    }

    fn skip_paired(&self, idx: usize, end: usize) -> usize {
        self.pairs.close(idx).map_or(idx + 1, |close| (close + 1).min(end))
    }

    fn walk_statements(&mut self, start: usize, end: usize, scope: Scope) {
        let mut i = start;
        while i < end {
            if matches!(self.tokens[i].kind, TokenKind::Semi | TokenKind::Comma) {
                i += 1;
                continue;
            }
            let next = self.parse_statement(i, end, scope);
            i = next.max(i + 1);
        }
    }

    fn parse_statement(&mut self, start: usize, end: usize, scope: Scope) -> usize {
        let stmt_start_line = self.tokens[start].line;
        let k = self.skip_leading_modifiers(start, end);
        if k >= end {
            return end;
        }

        match self.tokens[k].kind {
            // top-level imports (`import ...` in JS/TS, `use ...` in Rust)
            TokenKind::Keyword(Keyword::Import)
                if self.dialect == Dialect::JsTs
                    && !matches!(self.kind(k + 1), Some(TokenKind::LParen | TokenKind::Dot)) =>
            {
                let next = self.end_of_js_import(k, end);
                if scope == Scope::TopLevel {
                    let last_line = self.tokens[next.saturating_sub(1)].line;
                    self.out.imports.extend(stmt_start_line..=last_line);
                }
                next
            }
            TokenKind::Keyword(Keyword::Use) if self.dialect == Dialect::Rust => {
                let next = self.end_of_delimited_stmt(k, end);
                if scope == Scope::TopLevel {
                    let last_line = self.tokens[next.saturating_sub(1)].line;
                    self.out.imports.extend(stmt_start_line..=last_line);
                }
                next
            }

            // JS/TS re-exports (`export { ... }` / `export * from "..."`)
            TokenKind::LBrace | TokenKind::Star
                if self.dialect == Dialect::JsTs
                    && self.has_modifier(start, k, Keyword::Export) =>
            {
                self.end_of_delimited_stmt(k, end)
            }

            // type aliases (`type Foo = ...`)
            TokenKind::Keyword(Keyword::Type) => self.end_of_delimited_stmt(k, end),

            // JS/TS `class` container
            TokenKind::Keyword(Keyword::Class) if self.dialect == Dialect::JsTs => {
                self.parse_container(k, end, true)
            }

            // other container blocks
            TokenKind::Keyword(
                Keyword::Interface | Keyword::Enum | Keyword::Namespace | Keyword::Module,
            ) if self.dialect == Dialect::JsTs => self.parse_container(k, end, false),

            TokenKind::Keyword(
                Keyword::Struct
                | Keyword::Enum
                | Keyword::Union
                | Keyword::Trait
                | Keyword::Impl
                | Keyword::Mod,
            ) if self.dialect == Dialect::Rust => self.parse_container(k, end, false),

            // named or default function declarations
            TokenKind::Keyword(Keyword::Function) if self.dialect == Dialect::JsTs => {
                self.parse_fn_item(stmt_start_line, k, end)
            }
            TokenKind::Keyword(Keyword::Fn)
                if self.dialect == Dialect::Rust
                    && self.kind(k + 1).is_some_and(TokenKind::is_word) =>
            {
                self.parse_fn_item(stmt_start_line, k, end)
            }

            // value and constant bindings (`const`, `let`, `var`, `static`)
            TokenKind::Keyword(Keyword::Const | Keyword::Let | Keyword::Var)
                if self.dialect == Dialect::JsTs =>
            {
                self.parse_value_stmt(start, stmt_start_line, k, end, scope)
            }
            TokenKind::Keyword(Keyword::Const | Keyword::Static)
                if self.dialect == Dialect::Rust =>
            {
                self.parse_value_stmt(start, stmt_start_line, k, end, scope)
            }

            // expression or other statement
            _ => {
                let next = self.end_of_delimited_stmt(start, end);
                self.walk_routine_tokens(start, next, None);
                next
            }
        }
    }

    fn skip_leading_modifiers(&self, start: usize, end: usize) -> usize {
        let mut i = start;
        while i < end {
            // Rust `#[...]` attribute
            if self.dialect == Dialect::Rust
                && self.tokens[i].kind == TokenKind::Hash
                && self.kind(i + 1) == Some(TokenKind::LBracket)
                && let Some(close) = self.pairs.close(i + 1)
            {
                i = (close + 1).min(end);
                continue;
            }
            // JS/TS `@decorator` or `@decorator(...)`
            if self.dialect == Dialect::JsTs && self.tokens[i].kind == TokenKind::At {
                i = self.skip_decorators(i, end);
                continue;
            }
            // `pub` or `pub(...)`
            if self.dialect == Dialect::Rust
                && self.tokens[i].kind == TokenKind::Keyword(Keyword::Pub)
            {
                i += 1;
                if self.kind(i) == Some(TokenKind::LParen) {
                    i = self.skip_paired(i, end);
                }
                continue;
            }
            // `const fn` / `const enum` vs `const X = ...`
            if self.tokens[i].kind == TokenKind::Keyword(Keyword::Const) {
                if matches!(
                    self.kind(i + 1),
                    Some(TokenKind::Keyword(
                        Keyword::Fn | Keyword::Enum | Keyword::Async | Keyword::Unsafe
                    ))
                ) {
                    i += 1;
                    continue;
                }
                break;
            }
            // `export type { ... }` / `export type *` vs `export type Foo = ...`
            if self.dialect == Dialect::JsTs
                && self.tokens[i].kind == TokenKind::Keyword(Keyword::Type)
                && matches!(self.kind(i + 1), Some(TokenKind::LBrace | TokenKind::Star))
                && self.has_modifier(start, i, Keyword::Export)
            {
                i += 1;
                break;
            }
            if matches!(
                self.tokens[i].kind,
                TokenKind::Keyword(
                    Keyword::Export
                        | Keyword::Default
                        | Keyword::Declare
                        | Keyword::Abstract
                        | Keyword::Async
                        | Keyword::Unsafe
                )
            ) {
                i += 1;
                continue;
            }
            // Rust `extern "C" fn`
            if self.dialect == Dialect::Rust
                && self.tokens[i].kind == TokenKind::Keyword(Keyword::Extern)
            {
                i += 1;
                if self.kind(i) == Some(TokenKind::Literal) {
                    i += 1;
                }
                continue;
            }
            break;
        }
        i
    }

    fn has_modifier(&self, start: usize, end: usize, target: Keyword) -> bool {
        self.tokens[start..end].iter().any(|tok| tok.kind == TokenKind::Keyword(target))
    }

    fn skip_decorators(&self, mut i: usize, end: usize) -> usize {
        while i < end && self.tokens[i].kind == TokenKind::At {
            i += 1;
            while i < end
                && (self.tokens[i].kind.is_word() || self.tokens[i].kind == TokenKind::Dot)
            {
                i += 1;
            }
            if self.kind(i) == Some(TokenKind::LParen) {
                i = self.skip_paired(i, end);
            }
        }
        i
    }

    fn end_of_js_import(&self, start: usize, end: usize) -> usize {
        let mut i = start + 1;
        if self.kind(i) == Some(TokenKind::Literal) {
            return self.consume_optional_import_tail(i + 1, end);
        }
        while i < end {
            match self.tokens[i].kind {
                TokenKind::Semi => return i + 1,
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => {
                    i = self.skip_paired(i, end);
                }
                TokenKind::Keyword(Keyword::From)
                    if self.kind(i + 1) == Some(TokenKind::Literal) =>
                {
                    return self.consume_optional_import_tail(i + 2, end);
                }
                _ => i += 1,
            }
        }
        end
    }

    fn consume_optional_import_tail(&self, mut i: usize, end: usize) -> usize {
        let prev_line = self.tokens[i.saturating_sub(1)].line;
        if i < end
            && self.tokens[i].line == prev_line
            && matches!(self.tokens[i].kind, TokenKind::Keyword(Keyword::With | Keyword::Assert))
            && self.kind(i + 1) == Some(TokenKind::LBrace)
            && let Some(close) = self.pairs.close(i + 1)
        {
            i = (close + 1).min(end);
        }
        if self.kind(i) == Some(TokenKind::Semi) {
            i += 1;
        }
        i
    }

    fn parse_container(&mut self, start: usize, end: usize, is_js_class: bool) -> usize {
        let mut i = start + 1;
        while i < end {
            match self.tokens[i].kind {
                TokenKind::Semi => return i + 1,
                TokenKind::LParen | TokenKind::LBracket => {
                    i = self.skip_paired(i, end);
                }
                TokenKind::LBrace => {
                    // const-generic `{ N }` inside `Foo<{ N }>` is not the container body
                    if self.dialect == Dialect::Rust
                        && i > start
                        && self.tokens[i - 1].kind == TokenKind::Lt
                        && let Some(close) = self.pairs.close(i)
                    {
                        i = (close + 1).min(end);
                        continue;
                    }
                    let Some(close) = self.pairs.close(i) else {
                        return end;
                    };
                    let close = close.min(end);
                    if is_js_class {
                        self.walk_class_body(i + 1, close);
                    } else {
                        self.walk_statements(i + 1, close, Scope::Container);
                    }
                    return (close + 1).min(end);
                }
                _ => i += 1,
            }
        }
        end
    }

    fn parse_fn_item(&mut self, head_line: usize, fn_idx: usize, end: usize) -> usize {
        let mut i = fn_idx + 1;
        while i < end {
            match self.tokens[i].kind {
                TokenKind::Semi => return i + 1,
                TokenKind::LParen => break,
                TokenKind::LBrace => return i,
                _ => i += 1,
            }
        }
        if self.kind(i) != Some(TokenKind::LParen) {
            return i.max(fn_idx + 1);
        }
        let Some(rparen) = self.pairs.close(i) else {
            return end;
        };
        let after_params = (rparen + 1).min(end);
        match self.find_body_after_params(after_params, end) {
            BodySearch::Body(body_open) => {
                let Some(body_close) = self.pairs.close(body_open) else {
                    return end;
                };
                let body_close = body_close.min(end);
                let nested = self.walk_routine_tokens(body_open + 1, body_close, None);
                if !nested {
                    self.out.routines.push(LineSpan::new(head_line, self.tokens[body_close].line));
                }
                (body_close + 1).min(end)
            }
            BodySearch::NoBody(next) => next.max(after_params),
        }
    }

    fn find_body_after_params(&self, mut i: usize, end: usize) -> BodySearch {
        if i >= end {
            return BodySearch::NoBody(end);
        }
        if self.tokens[i].kind == TokenKind::Semi {
            return BodySearch::NoBody(i + 1);
        }
        if self.tokens[i].kind == TokenKind::LBrace {
            return BodySearch::Body(i);
        }

        if self.dialect == Dialect::Rust {
            while i < end {
                match self.tokens[i].kind {
                    TokenKind::Semi => return BodySearch::NoBody(i + 1),
                    TokenKind::LParen | TokenKind::LBracket => {
                        i = self.skip_paired(i, end);
                    }
                    TokenKind::LBrace => {
                        if i > 0
                            && self.tokens[i - 1].kind == TokenKind::Lt
                            && let Some(close) = self.pairs.close(i)
                        {
                            i = (close + 1).min(end);
                            continue;
                        }
                        return BodySearch::Body(i);
                    }
                    _ => i += 1,
                }
            }
            return BodySearch::NoBody(end);
        }

        // JS/TS optional `: ReturnType` before `{`
        if self.tokens[i].kind != TokenKind::Colon {
            return BodySearch::NoBody(i);
        }
        i += 1;
        let mut expects_type_atom = true;
        while i < end {
            if self.tokens[i].kind == TokenKind::Semi {
                return BodySearch::NoBody(i + 1);
            }
            if expects_type_atom {
                match self.tokens[i].kind {
                    TokenKind::Pipe
                    | TokenKind::Amp
                    | TokenKind::Keyword(
                        Keyword::Keyof
                        | Keyword::Typeof
                        | Keyword::Readonly
                        | Keyword::Infer
                        | Keyword::Unique
                        | Keyword::New,
                    ) => {
                        i += 1;
                    }
                    TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => {
                        let Some(close) = self.pairs.close(i) else {
                            return BodySearch::NoBody(end);
                        };
                        i = (close + 1).min(end);
                        expects_type_atom = false;
                    }
                    TokenKind::Lt => {
                        i = self.skip_angle_brackets(i, end);
                    }
                    kind if kind.is_word() || kind == TokenKind::Literal => {
                        i += 1;
                        expects_type_atom = false;
                    }
                    _ => return BodySearch::NoBody(i),
                }
            } else {
                match self.tokens[i].kind {
                    TokenKind::LBrace => return BodySearch::Body(i),
                    TokenKind::Lt => {
                        i = self.skip_angle_brackets(i, end);
                    }
                    TokenKind::LBracket => {
                        let Some(close) = self.pairs.close(i) else {
                            return BodySearch::NoBody(end);
                        };
                        i = (close + 1).min(end);
                    }
                    TokenKind::Dot
                    | TokenKind::Pipe
                    | TokenKind::Amp
                    | TokenKind::FatArrow
                    | TokenKind::Question
                    | TokenKind::Colon
                    | TokenKind::Keyword(Keyword::Extends | Keyword::Is | Keyword::Asserts) => {
                        i += 1;
                        expects_type_atom = true;
                    }
                    _ => return BodySearch::NoBody(i),
                }
            }
        }
        BodySearch::NoBody(end)
    }

    fn skip_angle_brackets(&self, start: usize, end: usize) -> usize {
        let mut depth = 0usize;
        let mut i = start;
        while i < end {
            match self.tokens[i].kind {
                TokenKind::Lt => {
                    depth += 1;
                    i += 1;
                }
                TokenKind::Gt => {
                    depth = depth.saturating_sub(1);
                    i += 1;
                    if depth == 0 {
                        return i;
                    }
                }
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    i = self.skip_paired(i, end);
                }
                TokenKind::Semi => return i,
                _ => i += 1,
            }
        }
        end
    }

    fn parse_value_stmt(
        &mut self, stmt_start: usize, stmt_start_line: usize, kw_idx: usize, end: usize,
        scope: Scope,
    ) -> usize {
        let stmt_end = self.end_of_delimited_stmt(kw_idx, end);
        let has_routine = self.walk_routine_tokens(stmt_start, stmt_end, Some(stmt_start_line));
        if !has_routine {
            let last_line = self.tokens[stmt_end.saturating_sub(1)].line;
            match self.classify_initializer(kw_idx, stmt_end) {
                Initializer::ArrowFn => {
                    self.out.routines.push(LineSpan::new(stmt_start_line, last_line));
                }
                Initializer::Value if scope == Scope::TopLevel => {
                    self.out.values.extend(stmt_start_line..=last_line);
                }
                Initializer::Value | Initializer::None => {}
            }
        }
        stmt_end
    }

    fn classify_initializer(&self, kw_idx: usize, stmt_end: usize) -> Initializer {
        let mut i = kw_idx + 1;
        let mut seen_eq = false;
        while i < stmt_end {
            match self.tokens[i].kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    i = self.skip_paired(i, stmt_end);
                }
                TokenKind::Eq if !seen_eq => {
                    seen_eq = true;
                    i += 1;
                }
                TokenKind::FatArrow if seen_eq && self.dialect == Dialect::JsTs => {
                    return Initializer::ArrowFn;
                }
                _ => i += 1,
            }
        }
        if seen_eq { Initializer::Value } else { Initializer::None }
    }

    fn end_of_delimited_stmt(&self, start: usize, end: usize) -> usize {
        let mut i = start;
        let mut seen_eq_or_call = false;
        while i < end {
            let kind = self.tokens[i].kind;
            if kind == TokenKind::Semi {
                return i + 1;
            }
            if matches!(kind, TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace) {
                if kind == TokenKind::LParen {
                    seen_eq_or_call = true;
                }
                let next = self.skip_paired(i, end);
                if self.dialect == Dialect::JsTs && next < end {
                    let prev = self.tokens[next - 1];
                    let after = self.tokens[next];
                    if after.line > prev.line
                        && asi_can_end_between(prev.kind, after.kind, seen_eq_or_call)
                    {
                        return next;
                    }
                }
                i = next;
                continue;
            }
            if kind == TokenKind::Eq {
                seen_eq_or_call = true;
            }
            let next = i + 1;
            if self.dialect == Dialect::JsTs && next < end {
                let prev = self.tokens[i];
                let after = self.tokens[next];
                if after.line > prev.line
                    && asi_can_end_between(prev.kind, after.kind, seen_eq_or_call)
                {
                    return next;
                }
            }
            i = next;
        }
        end
    }

    fn walk_class_body(&mut self, start: usize, end: usize) {
        let mut i = start;
        while i < end {
            if matches!(self.tokens[i].kind, TokenKind::Semi | TokenKind::Comma) {
                i += 1;
                continue;
            }
            let next = self.parse_class_member(i, end);
            i = next.max(i + 1);
        }
    }

    fn parse_class_member(&mut self, start: usize, end: usize) -> usize {
        let member_start_line = self.tokens[start].line;
        let mut i = self.skip_decorators(start, end);
        if i >= end {
            return end;
        }

        // `static { ... }` block
        if self.tokens[i].kind == TokenKind::Keyword(Keyword::Static)
            && self.kind(i + 1) == Some(TokenKind::LBrace)
            && let Some(close) = self.pairs.close(i + 1)
        {
            let close = close.min(end);
            let nested = self.walk_routine_tokens(i + 2, close, None);
            if !nested {
                self.out.routines.push(LineSpan::new(member_start_line, self.tokens[close].line));
            }
            return (close + 1).min(end);
        }

        let mut is_abstract = false;
        while i < end {
            match self.tokens[i].kind {
                TokenKind::Keyword(Keyword::Abstract) => {
                    is_abstract = true;
                    i += 1;
                }
                TokenKind::Keyword(
                    Keyword::Public
                    | Keyword::Private
                    | Keyword::Protected
                    | Keyword::Readonly
                    | Keyword::Static
                    | Keyword::Async
                    | Keyword::Override
                    | Keyword::Declare
                    | Keyword::Accessor,
                ) => {
                    i += 1;
                }
                TokenKind::Keyword(Keyword::Get | Keyword::Set)
                    if !matches!(
                        self.kind(i + 1),
                        Some(
                            TokenKind::LParen
                                | TokenKind::Lt
                                | TokenKind::Colon
                                | TokenKind::Eq
                                | TokenKind::Semi
                        )
                    ) =>
                {
                    i += 1;
                }
                TokenKind::Star => {
                    i += 1;
                    break;
                }
                _ => break,
            }
        }
        if i >= end {
            return end;
        }

        // member name: `#name`, `[expr]`, identifier, or literal
        if self.tokens[i].kind == TokenKind::Hash {
            i += 1;
        }
        if self.kind(i) == Some(TokenKind::LBracket) {
            i = self.skip_paired(i, end);
        } else if self.kind(i).is_some_and(|kind| kind.is_word() || kind == TokenKind::Literal) {
            i += 1;
        }

        // optional `?` / `!` and `<T>` type parameters
        while matches!(self.kind(i), Some(TokenKind::Question | TokenKind::Bang)) {
            i += 1;
        }
        if self.kind(i) == Some(TokenKind::Lt) {
            i = self.skip_angle_brackets(i, end);
        }

        // method, constructor, getter, or setter when immediately followed by `(...)`
        if !is_abstract
            && self.kind(i) == Some(TokenKind::LParen)
            && let Some(rparen) = self.pairs.close(i)
        {
            let after_params = (rparen + 1).min(end);
            if let BodySearch::Body(body_open) = self.find_body_after_params(after_params, end)
                && let Some(body_close) = self.pairs.close(body_open)
            {
                let body_close = body_close.min(end);
                let nested = self.walk_routine_tokens(body_open + 1, body_close, None);
                if !nested {
                    self.out
                        .routines
                        .push(LineSpan::new(member_start_line, self.tokens[body_close].line));
                }
                return (body_close + 1).min(end);
            }
        }

        // field or abstract/overload signature
        let field_end = self.end_of_delimited_stmt(start, end);
        self.walk_routine_tokens(start, field_end, Some(member_start_line));
        field_end
    }

    // Scans a routine body or expression slice for nested routines, returning true if any was found.
    fn walk_routine_tokens(
        &mut self, start: usize, end: usize, binding_head_line: Option<usize>,
    ) -> bool {
        let mut found_routine = false;
        let mut i = start;
        let mut top_paren_depth = 0usize;

        while i < end {
            match self.tokens[i].kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    top_paren_depth += 1;
                    i += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    top_paren_depth = top_paren_depth.saturating_sub(1);
                    i += 1;
                }
                TokenKind::Keyword(Keyword::Function) if self.dialect == Dialect::JsTs => {
                    let head_line = if top_paren_depth == 0 {
                        binding_head_line.unwrap_or_else(|| self.fn_expr_head_line(start, i))
                    } else {
                        self.fn_expr_head_line(start, i)
                    };
                    if let Some(after) = self.parse_nested_fn(head_line, i, end) {
                        found_routine = true;
                        i = after;
                    } else {
                        i += 1;
                    }
                }
                TokenKind::Keyword(Keyword::Fn)
                    if self.dialect == Dialect::Rust
                        && self.kind(i + 1).is_some_and(TokenKind::is_word) =>
                {
                    let head_line = self.fn_expr_head_line(start, i);
                    if let Some(after) = self.parse_nested_fn(head_line, i, end) {
                        found_routine = true;
                        i = after;
                    } else {
                        i += 1;
                    }
                }
                TokenKind::FatArrow
                    if self.dialect == Dialect::JsTs
                        && self.kind(i + 1) == Some(TokenKind::LBrace) =>
                {
                    let Some(body_close) = self.pairs.close(i + 1) else {
                        i += 1;
                        continue;
                    };
                    let body_close = body_close.min(end);
                    let head_line = if top_paren_depth == 0 {
                        binding_head_line.unwrap_or_else(|| self.arrow_head_line(start, i))
                    } else {
                        self.arrow_head_line(start, i)
                    };
                    let nested = self.walk_routine_tokens(i + 2, body_close, None);
                    if !nested {
                        self.out
                            .routines
                            .push(LineSpan::new(head_line, self.tokens[body_close].line));
                    }
                    found_routine = true;
                    i = (body_close + 1).min(end);
                }
                TokenKind::Keyword(Keyword::Class) if self.dialect == Dialect::JsTs => {
                    let before_count = self.out.routines.len();
                    let next = self.parse_container(i, end, true);
                    if self.out.routines.len() > before_count {
                        found_routine = true;
                    }
                    i = next.max(i + 1);
                }
                _ => i += 1,
            }
        }

        found_routine
    }

    fn parse_nested_fn(&mut self, head_line: usize, fn_idx: usize, end: usize) -> Option<usize> {
        let mut i = fn_idx + 1;
        while i < end {
            match self.tokens[i].kind {
                TokenKind::LParen => break,
                TokenKind::Semi | TokenKind::LBrace => return None,
                _ => i += 1,
            }
        }
        if self.kind(i) != Some(TokenKind::LParen) {
            return None;
        }
        let rparen = self.pairs.close(i)?;
        let after_params = (rparen + 1).min(end);
        let BodySearch::Body(body_open) = self.find_body_after_params(after_params, end) else {
            return None;
        };
        let body_close = self.pairs.close(body_open)?.min(end);
        let nested = self.walk_routine_tokens(body_open + 1, body_close, None);
        if !nested {
            self.out.routines.push(LineSpan::new(head_line, self.tokens[body_close].line));
        }
        Some((body_close + 1).min(end))
    }

    fn fn_expr_head_line(&self, start: usize, fn_idx: usize) -> usize {
        let mut k = fn_idx;
        while k > start
            && matches!(
                self.tokens[k - 1].kind,
                TokenKind::Keyword(
                    Keyword::Async
                        | Keyword::Export
                        | Keyword::Pub
                        | Keyword::Unsafe
                        | Keyword::Const
                )
            )
        {
            k -= 1;
        }
        if k >= start + 3
            && self.tokens[k - 1].kind == TokenKind::Eq
            && self.tokens[k - 2].kind.is_word()
            && matches!(
                self.tokens[k - 3].kind,
                TokenKind::Keyword(Keyword::Const | Keyword::Let | Keyword::Var)
            )
        {
            return self.tokens[k - 3].line;
        }
        self.tokens[k].line
    }

    fn arrow_head_line(&self, start: usize, arrow_idx: usize) -> usize {
        if arrow_idx <= start {
            return self.tokens[arrow_idx].line;
        }
        let prev = arrow_idx - 1;
        let param_start = match self.tokens[prev].kind {
            kind if kind.is_word() => prev,
            TokenKind::RParen => self.pairs.open(prev).unwrap_or(prev),
            _ => self.find_params_before_return_type(start, prev).unwrap_or(arrow_idx),
        };
        let mut head = param_start;
        // step back over `<T>` generics and `async`
        if head > start && self.tokens[head - 1].kind == TokenKind::Gt {
            let mut depth = 0usize;
            let mut g = head - 1;
            loop {
                match self.tokens[g].kind {
                    TokenKind::Gt => depth += 1,
                    TokenKind::Lt => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            head = g;
                            break;
                        }
                    }
                    _ => {}
                }
                if g == start {
                    break;
                }
                g -= 1;
            }
        }
        if head > start && self.tokens[head - 1].kind == TokenKind::Keyword(Keyword::Async) {
            head -= 1;
        }
        // local variable `const shutdown = async (signal) => { ... }`
        if head >= start + 3
            && self.tokens[head - 1].kind == TokenKind::Eq
            && self.tokens[head - 2].kind.is_word()
            && matches!(
                self.tokens[head - 3].kind,
                TokenKind::Keyword(Keyword::Const | Keyword::Let | Keyword::Var)
            )
        {
            head -= 3;
        }
        self.tokens[head].line
    }

    fn find_params_before_return_type(&self, start: usize, mut i: usize) -> Option<usize> {
        loop {
            match self.tokens[i].kind {
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    i = self.pairs.open(i)?;
                }
                TokenKind::Gt => {
                    let mut depth = 1usize;
                    while depth > 0 {
                        if i == start {
                            return None;
                        }
                        i -= 1;
                        match self.tokens[i].kind {
                            TokenKind::Gt => depth += 1,
                            TokenKind::Lt => depth -= 1,
                            _ => {}
                        }
                    }
                }
                TokenKind::Colon if i > start && self.tokens[i - 1].kind == TokenKind::RParen => {
                    return self.pairs.open(i - 1);
                }
                TokenKind::Semi | TokenKind::Comma | TokenKind::Eq => return None,
                _ => {}
            }
            if i == start {
                return None;
            }
            i -= 1;
        }
    }
}

const fn asi_can_end_between(prev: TokenKind, next: TokenKind, seen_body: bool) -> bool {
    let prev_ends_atom = (prev.is_word()
        || matches!(
            prev,
            TokenKind::Literal
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Gt
                | TokenKind::Bang
        ))
        && !matches!(
            prev,
            TokenKind::Keyword(
                Keyword::Export
                    | Keyword::Default
                    | Keyword::Declare
                    | Keyword::Const
                    | Keyword::Let
                    | Keyword::Var
                    | Keyword::Type
                    | Keyword::Async
                    | Keyword::Await
                    | Keyword::New
                    | Keyword::Return
                    | Keyword::Throw
                    | Keyword::Typeof
                    | Keyword::Keyof
                    | Keyword::Extends
                    | Keyword::Implements
                    | Keyword::From
            )
        );
    if !prev_ends_atom {
        return false;
    }
    if !seen_body
        && matches!(next, TokenKind::Eq | TokenKind::Colon | TokenKind::LParen | TokenKind::Lt)
    {
        return false;
    }
    !matches!(
        next,
        TokenKind::Semi
            | TokenKind::Dot
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Eq
            | TokenKind::FatArrow
            | TokenKind::Question
            | TokenKind::Pipe
            | TokenKind::Amp
            | TokenKind::LParen
            | TokenKind::LBracket
    )
}
