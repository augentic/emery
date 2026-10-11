//! The secret gate: names every changed path that carries a credential.
//!
//! What a build or a verification left changed is held to two rules before
//! it is sealed. A path is a credential carrier by name — a private key file,
//! a `.npmrc`, a `.env` — whatever it holds. Any other path is read as a
//! commit would carry it, a file's bytes or a symbolic link's target, and
//! searched for the shapes well-known credentials take: a PEM private-key
//! block, and the fixed prefix and run of a provider's token. A deleted path
//! carries nothing and is passed over; one that cannot be read fails the
//! gate rather than passing it.

use omnia_sdk::Error;
use omnia_sdk::vcs::{Change, ChangeKind};

use crate::Files;

// Files that carry a credential by name, matched on the final path segment.
const CARRIERS: [&str; 10] = [
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    ".npmrc",
    ".netrc",
    ".git-credentials",
    ".pgpass",
    ".pypirc",
    ".env",
];

const PEM_HEAD: &[u8] = b"-----BEGIN ";
const PEM_DASHES: &[u8] = b"-----";
const PEM_PRIVATE: &[u8] = b"PRIVATE KEY";

// The bytes a token's run may hold after its prefix.
#[derive(Clone, Copy)]
enum Run {
    // `[A-Z0-9]`
    Upper,
    // `[A-Za-z0-9]`
    Alnum,
    // `[A-Za-z0-9_-]`
    Token,
}

impl Run {
    const fn admits(self, byte: u8) -> bool {
        match self {
            Self::Upper => byte.is_ascii_uppercase() || byte.is_ascii_digit(),
            Self::Alnum => byte.is_ascii_alphanumeric(),
            Self::Token => byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-',
        }
    }
}

// One credential shape: a fixed prefix, then at least `least` bytes of `run`.
struct Shape {
    prefix: &'static [u8],
    run: Run,
    least: usize,
    what: &'static str,
}

const SHAPES: [Shape; 17] = [
    Shape {
        prefix: b"AKIA",
        run: Run::Upper,
        least: 16,
        what: "an AWS access key id",
    },
    Shape {
        prefix: b"ghp_",
        run: Run::Alnum,
        least: 36,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"gho_",
        run: Run::Alnum,
        least: 36,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"ghu_",
        run: Run::Alnum,
        least: 36,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"ghs_",
        run: Run::Alnum,
        least: 36,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"ghr_",
        run: Run::Alnum,
        least: 36,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"github_pat_",
        run: Run::Token,
        least: 22,
        what: "a GitHub token",
    },
    Shape {
        prefix: b"npm_",
        run: Run::Alnum,
        least: 36,
        what: "an npm token",
    },
    Shape {
        prefix: b"xoxa-",
        run: Run::Token,
        least: 10,
        what: "a Slack token",
    },
    Shape {
        prefix: b"xoxb-",
        run: Run::Token,
        least: 10,
        what: "a Slack token",
    },
    Shape {
        prefix: b"xoxp-",
        run: Run::Token,
        least: 10,
        what: "a Slack token",
    },
    Shape {
        prefix: b"xoxr-",
        run: Run::Token,
        least: 10,
        what: "a Slack token",
    },
    Shape {
        prefix: b"xoxs-",
        run: Run::Token,
        least: 10,
        what: "a Slack token",
    },
    Shape {
        prefix: b"sk_live_",
        run: Run::Alnum,
        least: 24,
        what: "a Stripe live key",
    },
    Shape {
        prefix: b"rk_live_",
        run: Run::Alnum,
        least: 24,
        what: "a Stripe live key",
    },
    Shape {
        prefix: b"AIza",
        run: Run::Token,
        least: 35,
        what: "a Google API key",
    },
    Shape {
        prefix: b"glpat-",
        run: Run::Token,
        least: 20,
        what: "a GitLab token",
    },
];

// The `.npmrc` line that carries a registry token, in any file: the run
// excludes `$`, so `_authToken=${NPM_TOKEN}` carries nothing.
const AUTH_TOKEN: Shape = Shape {
    prefix: b"_authToken=",
    run: Run::Token,
    least: 20,
    what: "an npm `_authToken` line",
};

// Names every change under `at` that carries a credential, `<path>: <what>`,
// reading only the paths the name rule passes.
pub(super) async fn findings<F: Files>(
    files: &F, at: &str, changes: &[Change],
) -> Result<Vec<String>, Error> {
    let mut findings = Vec::new();
    for change in changes {
        if change.kind == ChangeKind::Deleted {
            continue;
        }
        let what = match named(&change.path) {
            Some(what) => Some(what),
            None => carried(&files.read(at, &change.path).await?),
        };
        if let Some(what) = what {
            findings.push(format!("{}: {what}", change.path));
        }
    }
    Ok(findings)
}

fn named(path: &str) -> Option<&'static str> {
    let mut segments = path.rsplit('/');
    let name = segments.next()?;
    if CARRIERS.contains(&name) || (name == "credentials" && segments.next() == Some(".aws")) {
        return Some("a file that carries a credential by name");
    }
    None
}

fn carried(bytes: &[u8]) -> Option<&'static str> {
    if pem_private_key(bytes) {
        return Some("a PEM private key block");
    }
    SHAPES.iter().chain([&AUTH_TOKEN]).find(|shape| shape.matches(bytes)).map(|shape| shape.what)
}

// A `-----BEGIN <label>-----` header whose label ends in `PRIVATE KEY`,
// wherever on its line it stands: a key pasted into a string literal keeps
// the header and the escaped newline after it on one line.
fn pem_private_key(bytes: &[u8]) -> bool {
    occurrences(bytes, PEM_HEAD).any(|start| {
        let rest = &bytes[start + PEM_HEAD.len()..];
        occurrences(rest, PEM_DASHES).next().is_some_and(|end| {
            let label = &rest[..end];
            label.ends_with(PEM_PRIVATE) && !label.contains(&b'\n')
        })
    })
}

impl Shape {
    fn matches(&self, bytes: &[u8]) -> bool {
        occurrences(bytes, self.prefix).any(|start| {
            let run = bytes[start + self.prefix.len()..]
                .iter()
                .take_while(|byte| self.run.admits(**byte))
                .count();
            run >= self.least
        })
    }
}

fn occurrences<'a>(bytes: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    (0..bytes.len()).filter(move |start| bytes[*start..].starts_with(needle))
}
