# Spec format

Hard-coded conventions of the rendered `spec.md`. These are not configurable and none of them is yours to write.

- **Front matter**: `emery: <grammar>` and `revision: <id>` between `---` fences, stamped by the engine on every projection.
- **Title**: `# Specification`, then your `preamble` paragraphs.
- **Requirement heading**: `### Requirement: <subject>[ <tag>]`
- **Provenance lines**: `ID: REQ-NNN`, `Sources: [<source>:<claim>, …]`, `Status: <status>` — each source cited with the claim it contributed.
- **Trigger line**: `Route: <METHOD /path>` where a source's code registered the requirement's handler at a route, or `Message: <name>` where it registered it for an inbound message; one at most, rendered by the engine from the sources, absent otherwise, and read typed by the build.
- **Scenario heading**: `#### Scenario: <name>`, then `- **GIVEN**` / `- **WHEN**` / `- **THEN**` bullets, with `- **AND**` bullets for each follow-on outcome.

Your `preamble` opens the document: a short overview of what was bound and what the requirements say, as paragraphs. It names what the system reaches by its role — a message stream, a store, a secret store, an upstream service — never the product, protocol, framework, or host behind it, which `design.md` holds; the engine refuses a paragraph naming one. One flat document: no delta sections, no per-domain splits — `spec.md` is the whole reviewable set. Requirements are rendered in id order, whatever order the request lists them.
