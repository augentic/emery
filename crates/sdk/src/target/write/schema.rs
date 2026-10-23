//! The shapes the write tools take and answer, read as a model spells them.

use std::fmt;

use schemars::JsonSchema;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::Wrote;

/// The files one `write_files` call writes and removes.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, JsonSchema)]
pub struct WriteFiles {
    /// The files to write, in order; a later entry naming an earlier one's
    /// path replaces it.
    #[serde(default)]
    pub files: Vec<WriteFile>,
    /// The paths of files to remove, each `/`-separated and relative to
    /// `$WORKSPACE`; one the tree lacks is already removed, and one `files`
    /// also names is replaced by that entry, not removed.
    #[serde(default)]
    pub delete: Vec<String>,
}

// The schema asks for `files` and `delete`; one entry spelled bare at the
// root of the call — `{path, content}`, as a model sends for one file — is
// read as `files` holding it alone, and never beside `files`.
impl<'de> Deserialize<'de> for WriteFiles {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Call;

        impl<'de> Visitor<'de> for Call {
            type Value = WriteFiles;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("`files` and `delete`, or one `{path, content}` entry")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut files = None;
                let mut delete = None;
                let mut bare = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "files" => {
                            if files.is_some() {
                                return Err(de::Error::duplicate_field("files"));
                            }
                            files = Some(map.next_value_seed(Files)?);
                        }
                        "delete" => {
                            if delete.is_some() {
                                return Err(de::Error::duplicate_field("delete"));
                            }
                            delete = Some(map.next_value()?);
                        }
                        "path" | "content" | "shrink" => bare.push((key, map.next_value()?)),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["files", "delete", "path", "content", "shrink"],
                            ));
                        }
                    }
                }
                let files = match (files, bare.is_empty()) {
                    (files, true) => files.unwrap_or_default(),
                    (None, false) => vec![single_file(bare)?],
                    (Some(_), false) => {
                        return Err(de::Error::custom(
                            "`files` and a bare `{path, content}` entry cannot both be given; \
                             put every entry under `files`",
                        ));
                    }
                };
                Ok(WriteFiles {
                    files,
                    delete: delete.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(Call)
    }
}

/// One file of a `write_files` call.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteFile {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`.
    pub path: String,
    /// The whole content the file holds afterwards.
    pub content: String,
    /// Set when the file is meant to become empty or less than half its
    /// current size; without it, such a write is refused, since it is
    /// usually a fragment sent in place of the whole file.
    #[serde(default)]
    pub shrink: bool,
}

/// The scaffolded files one `restore_files` call puts back.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RestoreFiles {
    /// The `/`-separated paths of the files to put back as they were
    /// scaffolded, each relative to `$WORKSPACE` and named by the brief as
    /// scaffolded before this turn.
    pub paths: Vec<String>,
}

/// The edits one `edit_files` call makes within files the tree holds.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, JsonSchema)]
pub struct EditFiles {
    /// The edits, in order; a later edit of a file meets it as the earlier
    /// ones left it.
    pub edits: Vec<Edit>,
}

// The schema asks for `edits`; one edit spelled bare at the root of the
// call — `{path, find, replace}`, as a model sends by analogy with a
// write's one file — is read as `edits` holding it alone, and never beside
// `edits`.
impl<'de> Deserialize<'de> for EditFiles {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Call;

        impl<'de> Visitor<'de> for Call {
            type Value = EditFiles;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("`edits`, or one `{path, find, replace}` edit")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut edits = None;
                let mut bare = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "edits" => {
                            if edits.is_some() {
                                return Err(de::Error::duplicate_field("edits"));
                            }
                            edits = Some(map.next_value_seed(Edits)?);
                        }
                        "path" | "find" | "replace" => bare.push((key, map.next_value()?)),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["edits", "path", "find", "replace"],
                            ));
                        }
                    }
                }
                let edits = match (edits, bare.is_empty()) {
                    (Some(edits), true) => edits,
                    (None, true) => return Err(de::Error::missing_field("edits")),
                    (None, false) => vec![single_edit(bare)?],
                    (Some(_), false) => {
                        return Err(de::Error::custom(
                            "`edits` and a bare `{path, find, replace}` edit cannot both be \
                             given; put every edit under `edits`",
                        ));
                    }
                };
                Ok(EditFiles { edits })
            }
        }

        deserializer.deserialize_map(Call)
    }
}

/// One edit: the one place `find` matches in a file, replaced.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Edit {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`, which
    /// the tree must hold.
    pub path: String,
    /// The text to replace, which must occur exactly once in the file.
    pub find: String,
    /// The text put in its place; empty removes the match.
    pub replace: String,
}

// The schema asks for a sequence of `{path, content}`; a map from path to
// content, which a model reaches for as readily, is read all the same, in
// the order the call spells it; one entry spelled bare — a map with a
// `path` key — is that entry, not files named `path` and `content`; and
// the shape sent as JSON text is the shape.
struct Files;

impl<'de> DeserializeSeed<'de> for Files {
    type Value = Vec<WriteFile>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Files {
    type Value = Vec<WriteFile>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a list of `{path, content}` entries, one such entry, or a map from path to content",
        )
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut files = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(file) = seq.next_element()? {
            files.push(file);
        }
        Ok(files)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let entries = entries(map)?;
        if entries.iter().any(|(key, _)| key == "path") {
            return single_file(entries).map(|file| vec![file]);
        }
        entries
            .into_iter()
            .map(|(path, content)| {
                let content = text(content, &path)?;
                Ok(WriteFile {
                    path,
                    content,
                    shrink: false,
                })
            })
            .collect()
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        spelled(text, self)
    }
}

// The schema asks for a sequence of `{path, find, replace}`; one edit
// spelled bare — a map in place of the list — is that edit; and the shape
// sent as JSON text is the shape.
struct Edits;

impl<'de> DeserializeSeed<'de> for Edits {
    type Value = Vec<Edit>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Edits {
    type Value = Vec<Edit>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of `{path, find, replace}` edits, or one such edit")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut edits = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(edit) = seq.next_element()? {
            edits.push(edit);
        }
        Ok(edits)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        single_edit(entries(map)?).map(|edit| vec![edit])
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        spelled(text, self)
    }
}

// A model that serialises the list before it sends it puts the shape in a
// string; the string is read as the JSON it spells, through the same seed,
// and one that spells no JSON is refused as the seed expects.
fn spelled<'de, S, E>(text: &str, seed: S) -> Result<<S as DeserializeSeed<'de>>::Value, E>
where
    S: DeserializeSeed<'de> + Visitor<'de>,
    E: de::Error,
{
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => {
            return Err(E::custom(format!(
                "a string that is not JSON ({err}), where {} is expected",
                &seed as &dyn de::Expected
            )));
        }
    };
    DeserializeSeed::deserialize(seed, value).map_err(E::custom)
}

// A map's entries in the order the call spells them, which a
// `serde_json::Value` would not keep.
fn entries<'de, A: MapAccess<'de>>(
    mut map: A,
) -> Result<Vec<(String, serde_json::Value)>, A::Error> {
    let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
    while let Some(entry) = map.next_entry()? {
        entries.push(entry);
    }
    Ok(entries)
}

// The one edit a bare `{path, find, replace}` spells, held to those keys as
// the derived reading of the struct would hold a listed one.
fn single_edit<E: de::Error>(entries: Vec<(String, serde_json::Value)>) -> Result<Edit, E> {
    let (mut path, mut find, mut replace) = (None, None, None);
    for (key, value) in entries {
        match key.as_str() {
            "path" => path = Some(text(value, "path")?),
            "find" => find = Some(text(value, "find")?),
            "replace" => replace = Some(text(value, "replace")?),
            other => return Err(E::unknown_field(other, &["path", "find", "replace"])),
        }
    }
    Ok(Edit {
        path: path.ok_or_else(|| E::missing_field("path"))?,
        find: find.ok_or_else(|| E::missing_field("find"))?,
        replace: replace.ok_or_else(|| E::missing_field("replace"))?,
    })
}

// The one entry a bare `{path, content, shrink}` spells, held to those keys
// as the derived reading of the struct would hold a listed one.
fn single_file<E: de::Error>(entries: Vec<(String, serde_json::Value)>) -> Result<WriteFile, E> {
    let (mut path, mut content, mut shrink) = (None, None, false);
    for (key, value) in entries {
        match key.as_str() {
            "path" => path = Some(text(value, "path")?),
            "content" => content = Some(text(value, "content")?),
            "shrink" => {
                shrink = value
                    .as_bool()
                    .ok_or_else(|| E::custom("`shrink` holds a non-boolean, not true or false"))?;
            }
            other => return Err(E::unknown_field(other, &["path", "content", "shrink"])),
        }
    }
    Ok(WriteFile {
        path: path.ok_or_else(|| E::missing_field("path"))?,
        content: content.ok_or_else(|| E::missing_field("content"))?,
        shrink,
    })
}

fn text<E: de::Error>(value: serde_json::Value, at: &str) -> Result<String, E> {
    match value {
        serde_json::Value::String(text) => Ok(text),
        serde_json::Value::Null => Err(E::custom(format!("`{at}` holds null, not a string"))),
        serde_json::Value::Bool(_) => {
            Err(E::custom(format!("`{at}` holds a boolean, not a string")))
        }
        serde_json::Value::Number(_) => {
            Err(E::custom(format!("`{at}` holds a number, not a string")))
        }
        serde_json::Value::Array(_) => Err(E::custom(format!("`{at}` holds a list, not a string"))),
        serde_json::Value::Object(_) => Err(E::custom(format!("`{at}` holds a map, not a string"))),
    }
}

#[derive(Serialize)]
struct PathBytes<'a> {
    bytes: usize,
    path: &'a str,
}

#[derive(Serialize)]
pub struct WriteResponse {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deleted: Vec<String>,
    pub written: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct EditResponse {
    pub edited: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct RestoreResponse {
    pub restored: Vec<serde_json::Value>,
}

pub fn json_of(wrote: &[Wrote]) -> Vec<serde_json::Value> {
    wrote
        .iter()
        .map(|wrote| {
            serde_json::to_value(PathBytes {
                bytes: wrote.bytes,
                path: &wrote.file,
            })
            .expect("path and byte count serialise")
        })
        .collect()
}

// The schema the tools declare reaches no scripted model, which records
// the tools by name alone.
#[cfg(test)]
mod tests {
    use omnia_sdk::model::Function;
    use serde_json::{Value, json};

    use super::EditFiles;
    use crate::target::write::EDIT_FILES;

    // A provider holding a call to the schema refuses `{}` before it is
    // sent, since the one argument is required and its shape is the list.
    #[test]
    fn edit_files_schema_requires_edits() {
        let function = Function::of::<EditFiles>(EDIT_FILES, "");
        let schema: Value = serde_json::from_str(&function.parameters).expect("the schema is JSON");
        assert_eq!(schema["required"], json!(["edits"]), "{schema}");
        assert_eq!(schema["properties"]["edits"]["type"], "array", "{schema}");
        assert_eq!(
            schema["$defs"]["Edit"]["required"],
            json!(["path", "find", "replace"]),
            "{schema}"
        );
    }
}
