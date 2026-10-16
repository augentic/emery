# Requirement block

Every requirement in `spec.md` is one block the engine renders from the requirement and your draft: heading, three provenance lines, the body, templated notes, and at least one scenario. You draft the scenarios for each listed subject; everything else — including the body — is rendered.

## What the engine renders

```markdown
### Requirement: <subject>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>:<claim>, <source>:<claim>, …]
Status: <agreed|unknown|conflict|divergence>

<the winning claim's statement, verbatim; none for a requirement in conflict>

Note: <templated loser and gap lines, where the requirement has them>

#### Scenario: <your scenario name>

- **GIVEN** <your context, optional>
- **WHEN** <your trigger or input>
- **THEN** <your expected behaviour>
- **AND** <your follow-on outcome, optional>
```

## What you draft

One entry per requirement listed under *Requirements (draft one entry per subject)*, keyed by its `subject` exactly as listed:

- **`scenarios`** — at least one, each with a `name`, optional `given` lines, a `when`, a `then`, and optional `and` lines that follow the `then`, all single lines. Draft from the `criterion` claims covering the requirement. For an uncovered requirement, a scenario that states what is checked, with `then` the outcome the requirement's own statements name — the response, the status, the record, the value a behaviour source observed (`a 400 response with error invalid-email`) — when they name one, and `[unknown]` when they do not; never an outcome no statement or criterion evidences. For a requirement in `conflict` (see [authority.md](authority.md)), the scenario must not pick a side.

The engine refuses a draft that omits a listed requirement, drafts a subject that is not listed, drafts a subject twice, omits a scenario, opens a preamble paragraph line with `#`, `ID:`, `Sources:`, `Status:`, `Note:`, or `Type:`, names a technology in a preamble paragraph or a scenario line — a vendor, a product, a protocol, a framework, a host, or a package the `call` claims spell, however the line spells it (`Kafka`, `KEYVAULT_HOST`, `useConfluentKafkaConfig`) — or drafts boilerplate: a `when` or a `then` that is the requirement's statement (case and punctuation aside), or an `[unknown]` `then` for a covered requirement. A large specification is drafted in chunks of requirements, each its own call over the same claims: the requirements listed are this call's, the preamble is drafted with the first chunk alone, and a call told to leave it empty answers an empty `preamble`.

## Scenario conventions

The body is the winning claim's statement verbatim and is never yours; these conventions govern the scenarios alone.

- **The domain's words, never the code's.** A scenario's `name`, `given`, `when`, `then`, and `and` are written in the words of the domain the requirement serves — the order, the invoice, the customer, the overdue threshold — and never spell an accessor, a field path, a method, a parameter, or a constant that only the implementation knows (`Config.useTls`, `fetchRates`, `orderRow.createdAt`, `req.body.customerId`). A name is the domain's where a documentation source would spell it — `OVERDUE_DAYS`, `MAX_ITEMS`, `YYYY-MM-DD`, `Europe/London`, `invalid-email`, an `Order` — and the implementation's where only the code does; a setting is named by what it configures, or by its key where the key is the domain's. Draw the behaviour from the `criterion` and `requirement` claims and do not paraphrase it into a different behaviour; the identifiers a `criterion` or `excerpt` carries stay in the design.
- **A trigger, not a restatement.** The `when` names the event or input that starts the scenario. The engine refuses the requirement's statement itself as a `when`; a paraphrase of it passes the gate and is no better, so do not draft one.
- **An outcome, not a refrain.** The `then` states what this scenario observes once the trigger has run — the response, the record, the error. The engine refuses the statement itself as a `then`; a paraphrase of the statement passes the gate and is no better, and so does one fixed phrase repeated across requirements, which the gate lets through and the reader cannot use — two requirements share a `then` only when they observe one outcome.
- **No commentary about provenance.** Winners, losers, and gaps are the engine's notes; a scenario never restates them.
- **The role, never the product.** A scenario names what the system reaches as the claims do — the inbound stream, the store, the upstream service, the secret — and never the product, library, or host behind it, which the design holds; a deployment default a claim carries (a hostname, a group name) is the design's too, not a `then`. The source's own identifiers are not exempt: a function, class, or key named for its technology is named in a scenario for what it does — `the message clients are configured from the credentials obtained`, not `useConfluentKafkaConfig runs`; `the secret store`, not `KeyVault` — and a key is spelled only where it is the domain's (`OVERDUE_DAYS`), never the technology's (`KAFKA_HOSTS`). The engine refuses a line naming one, and the correction states the role.
- **Literal values verbatim.** Where a `criterion` or `excerpt` states the values a behaviour reads — the entries of a lookup mapping and what each maps to, a numeric threshold, a format string, a timezone or locale identifier, a default — the `given` or `then` carries those values as written (`GIVEN the region code EU maps to zone 3`, `THEN the timestamp is formatted as YYYY-MM-DD HH:mm in the Europe/London zone`), never a reference to the constant that held them (`the configured mapping`, `the zone constant`): the builder reads the scenario and never the source, so a value the scenario summarises away is a value the build invents.
- **No invented outcomes.** Where neither a criterion nor the requirement's statements evidence the outcome, state what is checked and let `then` be `[unknown]`. A statement mined from code names what the code does — the status it returns, the record it writes, the delay it waits — and that outcome is evidenced, so `then` states it, in the scenario's own words rather than the statement's.
