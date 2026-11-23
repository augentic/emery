//! Tokenises JS/TS and Rust source text into owned line-stamped tokens and delimiter pairs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    JsTs,
    Rust,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keyword {
    Abstract,
    Accessor,
    Assert,
    Asserts,
    Async,
    Await,
    Case,
    Class,
    Const,
    Declare,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    Extern,
    Fn,
    From,
    Function,
    Get,
    Impl,
    Implements,
    Import,
    In,
    Infer,
    Instanceof,
    Interface,
    Is,
    Keyof,
    Let,
    Mod,
    Module,
    Namespace,
    New,
    Of,
    Override,
    Private,
    Protected,
    Pub,
    Public,
    Readonly,
    Return,
    Set,
    Static,
    Struct,
    Throw,
    Trait,
    Type,
    Typeof,
    Union,
    Unique,
    Unsafe,
    Use,
    Var,
    Void,
    With,
    Yield,
}

impl Keyword {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "abstract" => Self::Abstract,
            "accessor" => Self::Accessor,
            "assert" => Self::Assert,
            "asserts" => Self::Asserts,
            "async" => Self::Async,
            "await" => Self::Await,
            "case" => Self::Case,
            "class" => Self::Class,
            "const" => Self::Const,
            "declare" => Self::Declare,
            "default" => Self::Default,
            "delete" => Self::Delete,
            "do" => Self::Do,
            "else" => Self::Else,
            "enum" => Self::Enum,
            "export" => Self::Export,
            "extends" => Self::Extends,
            "extern" => Self::Extern,
            "fn" => Self::Fn,
            "from" => Self::From,
            "function" => Self::Function,
            "get" => Self::Get,
            "impl" => Self::Impl,
            "implements" => Self::Implements,
            "import" => Self::Import,
            "in" => Self::In,
            "infer" => Self::Infer,
            "instanceof" => Self::Instanceof,
            "interface" => Self::Interface,
            "is" => Self::Is,
            "keyof" => Self::Keyof,
            "let" => Self::Let,
            "mod" => Self::Mod,
            "module" => Self::Module,
            "namespace" => Self::Namespace,
            "new" => Self::New,
            "of" => Self::Of,
            "override" => Self::Override,
            "private" => Self::Private,
            "protected" => Self::Protected,
            "pub" => Self::Pub,
            "public" => Self::Public,
            "readonly" => Self::Readonly,
            "return" => Self::Return,
            "set" => Self::Set,
            "static" => Self::Static,
            "struct" => Self::Struct,
            "throw" => Self::Throw,
            "trait" => Self::Trait,
            "type" => Self::Type,
            "typeof" => Self::Typeof,
            "union" => Self::Union,
            "unique" => Self::Unique,
            "unsafe" => Self::Unsafe,
            "use" => Self::Use,
            "var" => Self::Var,
            "void" => Self::Void,
            "with" => Self::With,
            "yield" => Self::Yield,
            _ => return None,
        })
    }

    const fn precedes_regex(self) -> bool {
        matches!(
            self,
            Self::Return
                | Self::Throw
                | Self::Case
                | Self::In
                | Self::Of
                | Self::Typeof
                | Self::Instanceof
                | Self::Void
                | Self::Delete
                | Self::New
                | Self::Await
                | Self::Yield
                | Self::Else
                | Self::Do
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Keyword(Keyword),
    Literal,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semi,
    Colon,
    Comma,
    Eq,
    FatArrow,
    ThinArrow,
    PathSep,
    At,
    Hash,
    Lt,
    Gt,
    Dot,
    Question,
    Bang,
    Star,
    Pipe,
    Amp,
    Other,
}

impl TokenKind {
    #[must_use]
    pub const fn is_word(self) -> bool {
        matches!(self, Self::Ident | Self::Keyword(_))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

pub struct Pairs {
    close_of: Vec<Option<usize>>,
    open_of: Vec<Option<usize>>,
}

impl Pairs {
    #[must_use]
    pub fn of(tokens: &[Token]) -> Self {
        let len = tokens.len();
        let mut close_of = vec![None; len];
        let mut open_of = vec![None; len];
        let mut braces = Vec::new();
        let mut parens = Vec::new();
        let mut brackets = Vec::new();

        for (idx, tok) in tokens.iter().enumerate() {
            match tok.kind {
                TokenKind::LBrace => braces.push(idx),
                TokenKind::RBrace => {
                    if let Some(open) = braces.pop() {
                        close_of[open] = Some(idx);
                        open_of[idx] = Some(open);
                    }
                }
                TokenKind::LParen => parens.push(idx),
                TokenKind::RParen => {
                    if let Some(open) = parens.pop() {
                        close_of[open] = Some(idx);
                        open_of[idx] = Some(open);
                    }
                }
                TokenKind::LBracket => brackets.push(idx),
                TokenKind::RBracket => {
                    if let Some(open) = brackets.pop() {
                        close_of[open] = Some(idx);
                        open_of[idx] = Some(open);
                    }
                }
                _ => {}
            }
        }

        Self { close_of, open_of }
    }

    #[must_use]
    pub fn close(&self, idx: usize) -> Option<usize> {
        self.close_of.get(idx).copied().flatten()
    }

    #[must_use]
    pub fn open(&self, idx: usize) -> Option<usize> {
        self.open_of.get(idx).copied().flatten()
    }
}

pub struct Lexer<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    brace_depth: usize,
    template_braces: Vec<usize>,
    can_start_regex: bool,
    dialect: Dialect,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub fn tokenize(text: &'a str, dialect: Dialect) -> Vec<Token> {
        let mut lexer = Self {
            text,
            bytes: text.as_bytes(),
            pos: 0,
            line: 1,
            brace_depth: 0,
            template_braces: Vec::new(),
            can_start_regex: true,
            dialect,
            tokens: Vec::new(),
        };
        lexer.run();
        lexer.tokens
    }

    fn run(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'\n' {
                self.line += 1;
                self.pos += 1;
            } else if matches!(b, b' ' | b'\t' | b'\r' | 0x0c) {
                self.pos += 1;
            } else if self.try_skip_comment()
                || self.try_scan_string_or_template()
                || self.try_scan_regex()
                || self.try_scan_word()
                || self.try_scan_number()
                || self.try_scan_multi_op()
                || self.try_resume_template()
            {
            } else {
                self.scan_single_punct(b);
            }
        }
    }

    fn push(&mut self, kind: TokenKind, line: usize, regex_next: bool) {
        self.tokens.push(Token { kind, line });
        self.can_start_regex = regex_next;
    }

    fn try_skip_comment(&mut self) -> bool {
        if self.bytes[self.pos] != b'/' {
            return false;
        }
        match self.bytes.get(self.pos + 1) {
            Some(b'/') => {
                self.pos += 2;
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                true
            }
            Some(b'*') => {
                self.pos += 2;
                let mut depth = 1usize;
                while self.pos < self.bytes.len() && depth > 0 {
                    if self.dialect == Dialect::Rust
                        && self.bytes[self.pos] == b'/'
                        && self.bytes.get(self.pos + 1) == Some(&b'*')
                    {
                        depth += 1;
                        self.pos += 2;
                    } else if self.bytes[self.pos] == b'*'
                        && self.bytes.get(self.pos + 1) == Some(&b'/')
                    {
                        depth -= 1;
                        self.pos += 2;
                    } else {
                        if self.bytes[self.pos] == b'\n' {
                            self.line += 1;
                        }
                        self.pos += 1;
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn try_scan_string_or_template(&mut self) -> bool {
        let b = self.bytes[self.pos];
        if self.dialect == Dialect::Rust {
            if let Some(after_raw) = scan_rust_raw_string(self.bytes, self.pos, &mut self.line) {
                self.push(TokenKind::Literal, self.line, false);
                self.pos = after_raw;
                return true;
            }
            if b == b'\'' {
                let tok_line = self.line;
                if let Some(after_char) = scan_rust_char_literal(self.text, self.bytes, self.pos) {
                    self.push(TokenKind::Literal, tok_line, false);
                    self.pos = after_char;
                } else {
                    self.pos += 1;
                    while self.pos < self.bytes.len()
                        && (self.bytes[self.pos].is_ascii_alphanumeric()
                            || self.bytes[self.pos] == b'_')
                    {
                        self.pos += 1;
                    }
                    self.push(TokenKind::Other, tok_line, false);
                }
                return true;
            }
        }

        if b == b'"' || (self.dialect == Dialect::JsTs && b == b'\'') {
            let tok_line = self.line;
            self.pos += 1;
            while self.pos < self.bytes.len() {
                if self.bytes[self.pos] == b'\\' {
                    self.pos += 1;
                    if self.bytes.get(self.pos) == Some(&b'\n') {
                        self.line += 1;
                    }
                    if self.pos < self.bytes.len() {
                        self.pos += 1;
                    }
                } else if self.bytes[self.pos] == b {
                    self.pos += 1;
                    break;
                } else {
                    if self.bytes[self.pos] == b'\n' {
                        self.line += 1;
                    }
                    self.pos += 1;
                }
            }
            self.push(TokenKind::Literal, tok_line, false);
            return true;
        }

        if self.dialect == Dialect::JsTs && b == b'`' {
            let tok_line = self.line;
            self.pos += 1;
            if scan_template_chunk(self.bytes, &mut self.pos, &mut self.line) {
                self.template_braces.push(self.brace_depth);
            }
            self.push(TokenKind::Literal, tok_line, true);
            return true;
        }

        false
    }

    fn try_scan_regex(&mut self) -> bool {
        if self.dialect == Dialect::JsTs
            && self.bytes[self.pos] == b'/'
            && self.can_start_regex
            && let Some(after) = scan_js_regex(self.bytes, self.pos)
        {
            self.push(TokenKind::Literal, self.line, false);
            self.pos = after;
            return true;
        }
        false
    }

    fn try_scan_word(&mut self) -> bool {
        if !is_ident_start(self.bytes[self.pos]) {
            return false;
        }
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.bytes.len() && is_ident_continue(self.bytes[self.pos]) {
            self.pos += 1;
        }
        let word = &self.text[start..self.pos];
        let (kind, regex_next) = Keyword::parse(word)
            .map_or((TokenKind::Ident, false), |kw| (TokenKind::Keyword(kw), kw.precedes_regex()));
        self.push(kind, self.line, regex_next);
        true
    }

    fn try_scan_number(&mut self) -> bool {
        if !self.bytes[self.pos].is_ascii_digit() {
            return false;
        }
        self.pos += 1;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_alphanumeric()
                || self.bytes[self.pos] == b'_'
                || self.bytes[self.pos] == b'.')
        {
            self.pos += 1;
        }
        self.push(TokenKind::Literal, self.line, false);
        true
    }

    fn try_scan_multi_op(&mut self) -> bool {
        if matches!(self.bytes.get(self.pos..self.pos + 3), Some(b"===" | b"!==")) {
            self.push(TokenKind::Other, self.line, true);
            self.pos += 3;
            return true;
        }
        let Some(pair) = self.bytes.get(self.pos..self.pos + 2) else {
            return false;
        };
        let multi = match pair {
            b"=>" => Some((TokenKind::FatArrow, true)),
            b"->" => Some((TokenKind::ThinArrow, false)),
            b"::" => Some((TokenKind::PathSep, false)),
            b"==" | b"!=" | b"<=" | b">=" | b"&&" | b"||" | b"??" | b"+=" | b"-=" | b"*="
            | b"/=" => Some((TokenKind::Other, true)),
            _ => None,
        };
        if let Some((kind, regex_next)) = multi {
            self.push(kind, self.line, regex_next);
            self.pos += 2;
            return true;
        }
        false
    }

    fn try_resume_template(&mut self) -> bool {
        if self.bytes[self.pos] != b'}' || self.template_braces.last() != Some(&self.brace_depth) {
            return false;
        }
        self.template_braces.pop();
        let tok_line = self.line;
        self.pos += 1;
        let regex_next = if scan_template_chunk(self.bytes, &mut self.pos, &mut self.line) {
            self.template_braces.push(self.brace_depth);
            true
        } else {
            false
        };
        self.push(TokenKind::Literal, tok_line, regex_next);
        true
    }

    fn scan_single_punct(&mut self, b: u8) {
        let (kind, regex_next) = match b {
            b'{' => {
                self.brace_depth += 1;
                (TokenKind::LBrace, true)
            }
            b'}' => {
                self.brace_depth = self.brace_depth.saturating_sub(1);
                (TokenKind::RBrace, false)
            }
            b'(' => (TokenKind::LParen, true),
            b')' => (TokenKind::RParen, false),
            b'[' => (TokenKind::LBracket, true),
            b']' => (TokenKind::RBracket, false),
            b';' => (TokenKind::Semi, true),
            b':' => (TokenKind::Colon, true),
            b',' => (TokenKind::Comma, true),
            b'=' => (TokenKind::Eq, true),
            b'@' => (TokenKind::At, false),
            b'#' => (TokenKind::Hash, false),
            b'<' => (TokenKind::Lt, true),
            b'>' => (TokenKind::Gt, true),
            b'.' => (TokenKind::Dot, false),
            b'?' => (TokenKind::Question, true),
            b'!' => (TokenKind::Bang, true),
            b'*' => (TokenKind::Star, true),
            b'|' => (TokenKind::Pipe, true),
            b'&' => (TokenKind::Amp, true),
            _ => (TokenKind::Other, true),
        };
        self.push(kind, self.line, regex_next);
        self.pos += self.text[self.pos..].chars().next().map_or(1, char::len_utf8);
    }
}

const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

const fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

fn scan_template_chunk(bytes: &[u8], pos: &mut usize, line: &mut usize) -> bool {
    let len = bytes.len();
    while *pos < len {
        match bytes[*pos] {
            b'\\' => {
                *pos += 1;
                if bytes.get(*pos) == Some(&b'\n') {
                    *line += 1;
                }
                if *pos < len {
                    *pos += 1;
                }
            }
            b'`' => {
                *pos += 1;
                return false;
            }
            b'$' if bytes.get(*pos + 1) == Some(&b'{') => {
                *pos += 2;
                return true;
            }
            b'\n' => {
                *line += 1;
                *pos += 1;
            }
            _ => *pos += 1,
        }
    }
    false
}

fn scan_js_regex(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    let mut p = start + 1;
    let mut in_class = false;
    while p < len {
        match bytes[p] {
            b'\n' | b'\r' => return None,
            b'\\' => {
                p += 1;
                if matches!(bytes.get(p), None | Some(b'\n' | b'\r')) {
                    return None;
                }
                p += 1;
            }
            b'[' => {
                in_class = true;
                p += 1;
            }
            b']' => {
                in_class = false;
                p += 1;
            }
            b'/' if !in_class => {
                if p == start + 1 {
                    return None;
                }
                p += 1;
                while bytes.get(p).is_some_and(u8::is_ascii_alphabetic) {
                    p += 1;
                }
                return Some(p);
            }
            _ => p += 1,
        }
    }
    None
}

fn scan_rust_raw_string(bytes: &[u8], start: usize, line: &mut usize) -> Option<usize> {
    let mut p = match bytes.get(start)? {
        b'r' => start + 1,
        b'b' | b'c' if bytes.get(start + 1) == Some(&b'r') => start + 2,
        _ => return None,
    };
    let mut hashes = 0usize;
    while bytes.get(p) == Some(&b'#') {
        hashes += 1;
        p += 1;
    }
    if bytes.get(p) != Some(&b'"') {
        return None;
    }
    p += 1;
    let len = bytes.len();
    while p < len {
        if bytes[p] == b'\n' {
            *line += 1;
            p += 1;
        } else if bytes[p] == b'"'
            && bytes.get(p + 1..p + 1 + hashes).is_some_and(|tail| tail.iter().all(|&b| b == b'#'))
        {
            return Some(p + 1 + hashes);
        } else {
            p += 1;
        }
    }
    Some(len)
}

fn scan_rust_char_literal(text: &str, bytes: &[u8], start: usize) -> Option<usize> {
    let after = start + 1;
    if bytes.get(after) == Some(&b'\\') {
        let mut p = after + 1;
        let limit = (after + 12).min(bytes.len());
        while p < limit {
            match bytes[p] {
                b'\n' | b'\r' => return None,
                b'\'' => return Some(p + 1),
                _ => p += 1,
            }
        }
        return None;
    }
    let ch = text.get(after..)?.chars().next()?;
    if ch == '\'' || ch == '\n' || ch == '\r' {
        return None;
    }
    let close = after + ch.len_utf8();
    (bytes.get(close) == Some(&b'\'')).then_some(close + 1)
}
