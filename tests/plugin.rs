//! Cursor plugin drift
//!
//! Checks that the Cursor plugin's rule text only names verbs, flags, and
//! skills the shipped `emery` command actually has.
//!
//! The plugin is prose an agent follows, so nothing else would catch it
//! describing a verb that has since been deleted or renamed. Tying it to the
//! live `--help` surface turns that drift into a failing test.

#![cfg(not(target_arch = "wasm32"))]

#[path = "support/provider.rs"]
mod provider;
#[path = "support/verbs.rs"]
mod verbs;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use emery_source::is_kebab;
use omnia_guest::api::command::Response;
use provider::Provider;

#[derive(Debug)]
enum Mention<'a> {
    Cli(&'a str),
    Skill { name: &'a str, rest: &'a str },
}

// Global flags do not appear in verb-specific help.
const GLOBAL_FLAGS: &[&str] = &["--debug", "--quiet", "--format", "--help", "--version"];

// Each skill's flags validate against its single wrapped verb.
const SKILL_VERBS: &[(&str, &str)] = &[("specify", "specify")];

// Runs `argv` through the live grammar; no capability is dispatched, so an
// idle provider serves.
async fn grammar(argv: &[&str]) -> Response {
    provider::cli(&Provider::idle(), argv).await
}

// Plugin-rule CLI mentions must resolve to live verbs and flags.
#[tokio::test]
async fn rule_matches() {
    let rule = plugin_dir().join("rules/emery.mdc");
    let doc = std::fs::read_to_string(&rule)
        .unwrap_or_else(|err| panic!("reading {}: {err}", rule.display()));
    let help = grammar(&["emery", "--help"]).await;
    assert_eq!(help.exit, 0, "`emery --help` must succeed");
    let help_text = String::from_utf8_lossy(&help.stdout);
    let verbs: BTreeSet<&str> = verbs::verbs(&help_text).into_iter().collect();

    let mentions = mentions(&doc);
    assert!(
        mentions.iter().any(|mention| matches!(mention, Mention::Cli(_))),
        "the rule mentions no `emery` command at all — the extractor regressed"
    );

    for mention in mentions {
        match mention {
            Mention::Cli(text) => {
                let mut segments = text.split('|');
                let first = segments.next().expect("split yields at least one segment");
                let (verb, rest) = walk_verb(first.split_whitespace().skip(1), &verbs);
                if verb.is_none() {
                    assert!(
                        rest.is_none_or(|token| !is_kebab(token)),
                        "rule names `emery {}`, which is not a verb (in `{text}`)",
                        rest.unwrap_or_default(),
                    );
                }
                assert_flags(verb.unwrap_or(""), first).await;
                for segment in segments {
                    let (alt, _rest) = walk_verb(segment.split_whitespace(), &verbs);
                    assert!(
                        alt.is_some(),
                        "rule alternative `{segment}` does not resolve to a verb (in `{text}`)",
                    );
                }
            }
            Mention::Skill { name, rest } => {
                let skill = plugin_dir().join("skills").join(name).join("SKILL.md");
                assert!(skill.is_file(), "rule names `/emery:{name}`, but {skill:?} is missing");
                let verb = SKILL_VERBS
                    .iter()
                    .find_map(|(skill, verb)| (*skill == name).then_some(*verb))
                    .unwrap_or_else(|| {
                        panic!("skill `{name}` has no CLI verb mapping in this test — add it")
                    });
                assert_flags(verb, rest).await;
            }
        }
    }
}

// Every shipped skill is named by the always-applied rule.
#[test]
fn every_skill() {
    let doc = std::fs::read_to_string(plugin_dir().join("rules/emery.mdc")).expect("rule");
    let skills = std::fs::read_dir(plugin_dir().join("skills")).expect("skills dir");
    for entry in skills {
        let entry = entry.expect("skill entry");
        if !entry.path().join("SKILL.md").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            doc.contains(&format!("/emery:{name}")) || doc.contains(&format!("skills/{name}/")),
            "shipped skill `{name}` is not mentioned by the always-applied rule"
        );
    }
}

fn plugin_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/emery")
}

// Collects every standalone `emery` mention in `text` — a `/emery:<skill>`
// reference or a CLI invocation — skipping dotted or slashed paths and
// `emery-adapters`.
fn mentions_in(text: &str) -> Vec<Mention<'_>> {
    let bytes = text.as_bytes();
    let mut mentions = Vec::new();
    let mut i = 0;
    while let Some(found) = text[i..].find("emery") {
        let start = i + found;
        let end = start + "emery".len();
        let before = start.checked_sub(1).map(|b| bytes[b] as char);
        let after = bytes.get(end).map(|b| *b as char);
        i = end;
        if before == Some('/') && after == Some(':') {
            let rest = &text[end + 1..];
            let name_end =
                rest.find(|ch: char| !(ch.is_ascii_lowercase() || ch == '-')).unwrap_or(rest.len());
            let name = &rest[..name_end];
            let tail = &rest[name_end..];
            if !name.is_empty() {
                mentions.push(Mention::Skill { name, rest: tail });
            }
            continue;
        }
        let boundary_before =
            before.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '/' | '-')));
        let boundary_after = after.is_none_or(char::is_whitespace);
        if boundary_before && boundary_after {
            mentions.push(Mention::Cli(&text[start..]));
        }
    }
    mentions
}

fn mentions(doc: &str) -> Vec<Mention<'_>> {
    let mut in_fence = false;
    let mut mentions = Vec::new();
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            mentions.extend(mentions_in(line));
            continue;
        }
        mentions.extend(
            line.split('`')
                .enumerate()
                .filter(|(index, _)| index % 2 == 1)
                .flat_map(|(_, part)| mentions_in(part)),
        );
    }
    mentions
}

// Returns the first live verb among `tokens` and the first token it could
// not consume.
fn walk_verb<'a>(
    tokens: impl IntoIterator<Item = &'a str>, verbs: &BTreeSet<&str>,
) -> (Option<&'a str>, Option<&'a str>) {
    let mut rest = None;
    let mut verb = None;
    for token in tokens {
        if !is_kebab(token) {
            rest = Some(token);
            break;
        }
        if verb.is_none() && verbs.contains(token) {
            verb = Some(token);
            continue;
        }
        rest = Some(token);
        break;
    }
    (verb, rest)
}

fn flags_of(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .map(|token| {
            token.trim_matches(|ch: char| {
                matches!(ch, '[' | ']' | '(' | ')' | '"' | '\'' | ',' | ';' | '.')
            })
        })
        .filter(|token| token.starts_with("--"))
        .map(|token| {
            token
                .split_once('=')
                .map_or(token, |(flag, _value)| flag)
                .trim_end_matches(|ch: char| !(ch.is_ascii_alphanumeric()))
        })
}

async fn assert_flags(verb: &str, text: &str) {
    let mut help: Option<String> = None;
    for flag in flags_of(text) {
        if GLOBAL_FLAGS.contains(&flag) {
            continue;
        }
        assert!(
            !verb.is_empty(),
            "flag `{flag}` mentioned with no verb to validate against (in `{text}`)"
        );
        if help.is_none() {
            let response = grammar(&["emery", verb, "--help"]).await;
            assert_eq!(response.exit, 0, "`emery {verb} --help` must succeed");
            help = Some(String::from_utf8_lossy(&response.stdout).into_owned());
        }
        let help = help.as_deref().expect("help rendered above");
        assert!(
            help.contains(flag),
            "rule names `{flag}` on `emery {verb}`, but the grammar has no such flag"
        );
    }
}
