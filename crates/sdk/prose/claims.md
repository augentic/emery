# Claim ids, anchors, and the fail-closed gate

The three rules every extract answer is checked against before the engine reads it, and the one every `requirement` is written under. Adapter prompts link this document and keep only their own kind table and worked examples; the SDK's answer tail and the engine's load gate both enforce exactly what is written here.

## `id` grammar

An `id` is dotted-kebab: one or more segments joined by `.`, each segment lowercase ASCII letters and digits joined by `-`.

```text
^[a-z0-9]+(-[a-z0-9]+)*(\.[a-z0-9]+(-[a-z0-9]+)*)*$
```

- `password-reset.expiry`, `session.timeout`, `user-list.search-filter` are valid; `Not.Valid`, `req-007`, `session_timeout`, and a trailing `.` are not.
- `id` is **required** on `requirement`, `criterion`, and `example` claims — deterministic reconciliation keys off it. It is optional on every other kind; carry it there only when the claim backs a specific requirement.
- Derive ids from the domain concept the claim describes, using the source's own noun phrases — never from file names, heading positions, line numbers, or invented counters. Byte-equal ids across sources always merge into one requirement ([reconciliation.md](reconciliation.md)). The first segment is the surface the requirement belongs to — `session` in `session.timeout` — and the build plan slices the specification by it: requirements sharing a first segment are always built together, so lead each id with the noun of the surface it is built under.
- A `criterion` id must equal its requirement's id or extend it with a dotted suffix (`password-reset.expiry` or `password-reset.expiry.window`). A criterion with an unrelated id leaves its requirement uncovered, and an uncovered requirement renders as an `[unknown]` acceptance gap.

## `path` anchors

Every claim mined from a `$SOURCE_DIR` tree carries a `path` rooted relative to `$SOURCE_DIR`; claims from an inline value omit it. The grammar matches GitHub-style anchors:

- `<path>` — whole-file claim.
- `<path>#L<n>` — single line.
- `<path>#L<start>-L<end>` — line range.

Line numbers are 1-indexed against the file at extract time, a range ends no earlier than it starts, and neither line exceeds the file's length. The path is relative (no leading `/`, no `..`), names a regular file the tree holds, and is never under a skip root. When the call lists or lays out the files it mines, the path names one of them. Choose the tightest anchor that bounds the cited text: the anchor is the citation, the body field carries short context, and stable spans at named boundaries keep re-runs byte-stable.

## Skip roots

The engine's own files live in the project the sources are bound from, and they are output, never input; the checkout's own directory is no input either. Every adapter skips them wherever they appear under `$SOURCE_DIR` — never read them, never anchor a claim in them:

- `spec.md`, `design.md`, and `plan.md` — the Markdown projections of the current revision, rendered by `emery show`.
- `.emery/` — the engine's root, where the committed revision lives.
- `.git/` — the checkout's history and metadata.

Mining a projection back into claims would make the engine's last answer look like evidence for its next one, and every requirement it re-derived that way would read as `agreed` with itself. Adapter prompts add their own language- or format-specific skip roots (`node_modules`, `target`, test trees, …) beside this list, never instead of it.

## Behaviour, not technology

A `requirement`'s `statement` and `id`, and a `criterion`, say what the system does for its caller, in the domain's words. The specification is built by a target that chooses its own platform, so a requirement that names the source's — a product, a protocol, a library, a framework, a host service — binds the build to how the source was made rather than to what it does, and the target either copies the mechanism or has to guess the behaviour behind it. The technology is the design's, and the `call` and `excerpt` kinds carry it there.

- Name what the system reaches by its role — a message stream, a store, a cache, an upstream service, a secret, a setting, a token, a monitor — never by the product, protocol, library, or host that plays the role (a broker, a web framework, a vault, a cache server, an APM agent, an HTTP client). `Each converted event is published to the outbound stream`, not `.. sent to the Kafka producer`.
- Wiring is no requirement: a client constructed, a connection opened, a port bound, an agent initialised, a subscription made, a credential loaded. The behaviour a caller observes through the wiring is: the message consumed, the request answered, the token obtained before the first upstream call, the setting the program refuses to start without, the failure that ends the program. Claim that, where the code decides it, and let the construction be the `excerpt`'s to describe.
- A `call`'s `callee` and `synopsis` spell the technology exactly as the source does — the package, the topic or queue as constructed, the URL template, the entity and its keys — and an `excerpt` carries the connection's shape: the key names, defaults, hostnames, group and client names, SDK options. A deployment default — a hostname, a port, a consumer group — is an excerpt's and never a criterion's.
- A setting's key is spelled as the source spells it in a `criterion` and an `excerpt`, where a target reads it; a `statement` names a setting by what it configures (`the inbound stream its configuration names`, `the delay bound`), or by its key where the key is the domain's (`OVERDUE_DAYS`) rather than the technology's (`KAFKA_HOSTS`).
- An `id`'s first segment is the stem the build is sliced by, and a slice is named for it, so the rule binds ids whole: `invoices.reject-stale`, not `kafka-consumer.reject-stale`; `secrets.refuse-empty`, not `key-vault.refuse-empty`. Where the source's own name for a surface is the technology's, lead with what the surface handles or does instead.

## The fail-closed gate

- Required body fields are a closed table: `requirement` → `statement`, `criterion` → `criterion`, `example` → `replay-digest`. A claim missing its required field, carrying an `id` outside the grammar, or carrying a `path` outside the grammar or the tree, fails the **whole run** closed as a typed `bad_request` naming the source, claim, and key. There is no partial acceptance and no fallback to `synopsis`.
- When the call names the stem its `requirement` and `criterion` ids lead with, an id under another first segment is a finding too.
- The caller checks the answer first: a failing answer is returned with the findings and a bounded number of repairs is asked for. Correct the named claims; do not drop them.
- `claims: []` is valid output when the source genuinely has nothing to say. Never pad with speculative claims — the engine preserves gaps as `[unknown]` rather than guessing.
- Never write Evidence to disk; return the JSON body and the caller persists it.
