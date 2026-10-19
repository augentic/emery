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

A `requirement`'s `statement` and `id`, and a `criterion`, say what the system does for those it serves, in the domain's words. The specification is built by a target that chooses its own platform, so a requirement that names the source's — a product, a protocol, a library, a framework, a host service — binds the build to how the source was made rather than to what it does, and the target either copies the mechanism or has to guess the behaviour behind it. The technology is the design's, and the `call`, `excerpt`, and `decision` kinds carry it there.

- Name what the system reaches by its role — a message stream, a store, a cache, an upstream service, a secret, a setting, a token, a monitor — never by the product, protocol, library, or host that plays the role (a broker, a web framework, a vault, a cache server, an APM agent, an HTTP client). `Each converted event is published to the outbound stream`, not `.. sent to the Kafka producer`.
- A `requirement` states what the system does for those it serves, or what they observe when it cannot. How the program readies itself to do so, reaches another system, or recovers is the design's, carried by `excerpt` and `call` claims and never by a `requirement`: a credential or a token loaded, cached, or rotated; a reference table prefetched, or refreshed on a timer; a client, a pool, or a connection constructed; a consumer subscribed; a library's readiness helper wired over the program's own clients with no route or answer of the source's own; a periodic status or heartbeat log; a signal or uncaught-error handler; a graceful drain or an exit delay; a switch over whether a listener starts in this deployment; a retry rule, a backoff, or a timeout; a refresh interval; a delivery acknowledged or advanced; a transaction or a span opened; an error caught and logged and nothing more. What those the program serves observe survives, under `start` as under any surface: the rule applied to every request or command (an authentication guard's 401, a body limit, an error-to-status mapping, a request log's line), the health or status route whose path or answer the program itself states, the setting it refuses to run without, and the caller-visible face of an upstream call — `An order whose items cannot be allocated is rejected with a 409 response` — while the URL template, the retry rule, and the token are the `call`'s and the `excerpt`'s.
- A mechanism's internals are no requirement either. A class the design would label a mechanism — a client, a secret retriever, a settings carrier, a bootstrap — has no requirements of its own: how it caches, parses, schedules, retries, and what its fields are called are `excerpt`s. What the bootstrap obtains through the mechanism is readiness, the design's by the rule above, and what remains under `start` is what that rule lists as surviving.
- A helper is a requirement where a build must reproduce its arms, its formula, or the shape it constructs, and folds into its caller's decision where it is only that decision's condition or a lookup one caller makes. A mapping function (`mapStatus`: codes `2, 3 → confirmed`, `4, 5 → shipped`, `9 → cancelled`), a pure arithmetic or date formula, or a builder that constructs an outbound record is a requirement at its head, named for the rule it applies; a thin predicate (`isEligibleOrder`) or a single-caller lookup (`findRateByCode`) that only feeds one caller's branch is that caller's requirement, with any table or constant it reads carried by the backing `criterion` or `excerpt`.
- A statement and a scenario use the domain's words; an implementation's identifiers go to the `excerpt` and the `criterion`. The test is the one settings keys are read by below: a name is the domain's where a documentation source would spell it — `OVERDUE_DAYS`, `MAX_ITEMS`, `YYYY-MM-DD`, `Europe/London`, `invalid-email`, `an order`, `an Order`, `a shipment notice` — and the implementation's where only the code does — `Config.useTls`, `fetchRates`, `orderRow.createdAt`, `req.body.customerId`, `RETRY_DELAY_MS`. The first belongs in a statement; the second never does.
- The source's own identifiers are not exempt. A function, class, or key named for its technology is read for what it does: `useConfluentKafkaConfig` is the switch over the managed-broker configuration, spelled in a `criterion` or `excerpt` by the configuration rather than the function; `KeyVault` is the secret store, so the statement says `the secret store`. A setting's key is spelled as the source spells it in a `criterion` and an `excerpt`, where a target reads it; a `statement` names a setting by what it configures (`the inbound stream its configuration names`, `the delay bound`), or by its key where the key is the domain's (`OVERDUE_DAYS`) rather than the technology's (`KAFKA_HOSTS`, `KEYVAULT_HOST`).
- A `call`'s `callee` and `synopsis` spell the technology exactly as the source does — the package, the topic or queue as constructed, the URL template, the entity and its keys — and an `excerpt` carries the connection's shape: the key names, defaults, hostnames, group and client names, SDK options. A deployment default — a hostname, a port, a consumer group — is an excerpt's and never a criterion's.
- A prose source that prescribes a technology — a brief, a decision record, a page that says the events go through one broker and the secrets live in one vault — states a `decision`, with the prescription verbatim, and the behaviour it governs a `requirement` by role. The design lands the decision; the specification keeps the behaviour.
- An `id`'s first segment is the stem the build is sliced by, and a slice is named for it, so the rule binds ids whole: `invoices.reject-stale`, not `kafka-consumer.reject-stale`; `secrets.refuse-empty`, not `key-vault.refuse-empty`; `start.require-brokers`, not `start.require-kafka-brokers`. Where the source's own name for a surface or a behaviour is the technology's, lead with what it handles or does instead.
- The gate holds the rule: a `requirement` or `criterion` whose `id`, `statement`, or `criterion` spells a technology name — a vendor, a product, a protocol, a framework, a host, or a package the source imports — is returned as a finding naming it, and the call corrects the claim by stating the role and carrying the name in a `call` or `excerpt`.

## The fail-closed gate

- Required body fields are a closed table: `requirement` → `statement`, `criterion` → `criterion`, `example` → `replay-digest`. A claim missing its required field, carrying an `id` outside the grammar, or carrying a `path` outside the grammar or the tree, fails the **whole run** closed as a typed `bad_request` naming the source, claim, and key. There is no partial acceptance and no fallback to `synopsis`.
- When the call names the stem its `requirement` and `criterion` ids lead with, an id under another first segment is a finding too.
- A `requirement` or `criterion` naming a technology in its `id` or its body is a finding ([Behaviour, not technology](#behaviour-not-technology)).
- A `contract` claim carries what reaches the surface, where it does, as one of two extras: a `route`, `METHOD /path` — the method one of `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`, the path led by `/` with each parameter spelled `{name}` — or a `message`, the name of the inbound message its handler consumes, one word (`orders.cancel-requested`). A `contract` whose `route` or `message` is outside its grammar, or that carries both, is a finding. A code adapter's survey states the routes and messages its registrations spell by itself and puts them in place of any the call answers, so the call need not state one.
- The caller checks the answer first: a failing answer is returned with the findings and a bounded number of repairs is asked for. Correct the named claims; do not drop them.
- `claims: []` is valid output when the source genuinely has nothing to say. Never pad with speculative claims — the engine preserves gaps as `[unknown]` rather than guessing.
- Never write Evidence to disk; return the JSON body and the caller persists it.
