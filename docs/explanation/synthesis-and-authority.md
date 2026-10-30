# Synthesis, authority, and the technology boundary

This page explains how Emery extracts claims from your sources, groups them into requirements, resolves disagreements using authority ranks, and keeps specifications technology-neutral while capturing concrete technologies in the design. It is for developers and architects who want to understand how `emery specify` and `emery amend` synthesize specifications.

## Contents

- [The synthesis pipeline at a glance](#the-synthesis-pipeline-at-a-glance)
- [1. Source survey and claim extraction](#1-source-survey-and-claim-extraction)
  - [Typed claim kinds](#typed-claim-kinds)
- [2. Grouping claims into requirements](#2-grouping-claims-into-requirements)
  - [How `emery amend` matches existing requirements](#how-emery-amend-matches-existing-requirements)
- [3. Authority ranks and reconciliation](#3-authority-ranks-and-reconciliation)
  - [Default and custom ranks](#default-and-custom-ranks)
  - [Requirement statuses: `agreed`, `divergence`, and `conflict`](#requirement-statuses-agreed-divergence-and-conflict)
- [4. The technology boundary: role vs. implementation](#4-the-technology-boundary-role-vs-implementation)
  - [Why specifications stay technology-neutral](#why-specifications-stay-technology-neutral)
  - [How the `Lexicon` gate works](#how-the-lexicon-gate-works)
  - [How concrete technologies reach the design](#how-concrete-technologies-reach-the-design)
  - [Data types vs. mechanism types](#data-types-vs-mechanism-types)
- [5. Deterministic triggers](#5-deterministic-triggers)
- [See also](#see-also)

---

## The synthesis pipeline at a glance

Model turns never author raw Markdown files in Emery. Instead, synthesis is a deterministic pipeline of schema-gated model questions (**briefs**) verified against code-derived facts:

1. **Extract:** Source adapters inspect each configured source in parallel and return a document of typed **claims**.
2. **Group:** Claims describing the same behaviour are merged into requirement groups, and code-derived **triggers** (HTTP routes or messages) are attached.
3. **Rank and reconcile:** Each requirement group is evaluated by source **authority rank** to determine its winning statement and status (`agreed`, `divergence`, or `conflict`).
4. **Draft:** Schema-gated model turns draft acceptance scenarios for `spec.json` and architectural prose for `design.json`.
5. **Slice:** Requirements and data types are partitioned into a dependency-ordered build plan (`plan.json`).

The resulting JSON documents (`spec.json`, `design.json`, `plan.json`) are the sole source of truth. Markdown views are read-only projections rendered by [`emery show`](../reference/cli/show.md).

---

## 1. Source survey and claim extraction

When you run `emery specify` or `emery amend`, Emery loads every configured source adapter (WebAssembly components implementing the `emery:adapter/source` WebAssembly Interface Types (WIT) contract) and dispatches extraction across all sources concurrently.

For multi-file workspaces, an adapter first runs a **survey** pass to divide the input into bounded extraction units called **seams**. Each seam constrains what the extraction turn may inspect, which requirement stems (`<stem>.<behaviour>`) it may emit, and which source line spans (`path#Lstart-Lend`) claims may anchor to.

### Typed claim kinds

Every source adapter returns an `Evidence` document containing a list of typed claims:

| Claim kind | Emitted by | What it captures | Where it lands |
| --- | --- | --- | --- |
| `requirement` | All source adapters | Observable system behaviour, stated by role (`statement`) under a dotted subject ID (`orders.create`). | `spec.json` requirements |
| `criterion` | All source adapters | Concrete acceptance condition (`criterion`) paired with a `statement` under a requirement subject. | `spec.json` scenarios |
| `retire` | Prose adapters (`intent`, `documentation`) | Explicit withdrawal of an existing behaviour (`statement` required; only read by `emery amend`). | Retires a requirement in `amend` |
| `contract` | Code adapters (survey pass) | Deterministic inbound trigger (`route` for HTTP or `message` for topic subscriptions). | Requirement `trigger` (`Route:` / `Message:`) and `design.json` APIs section |
| `decision` | Prose adapters (`intent`, `documentation`) | Architectural or technology prescription (such as an ADR (Architecture Decision Record) or operator note naming a database or broker). | `design.json` (`## APIs and integrations` and informed sections) |
| `type` | Code adapters (AST parser) | Exported data structure, class, or table-initialized constant copied verbatim from source code. | `design.json` (`## Domain model`) |
| `call` | Code adapters | Outbound call to an external package or upstream dependency (`callee`, `synopsis`). | `design.json` (`## APIs and integrations`) |
| `excerpt` | Code adapters | Significant implementation mechanism or algorithm excerpt (`synopsis`). | `design.json` (`## Technical logic`) |

---

## 2. Grouping claims into requirements

Once all sources finish extracting, Emery groups `requirement` claims that describe the same behaviour:

- **Exact ID match:** Claims from different sources with byte-identical subject IDs (for example, `orders.cancel`) are pre-merged automatically by code.
- **Cross-source semantic grouping:** When two or more sources are present, a schema-gated grouping turn matches claims that describe the same behaviour using different wording. During grouping, source authority ranks are withheld from the model so grouping decisions are purely semantic.
- **Within-source separation:** Two distinct `requirement` claims from the *same* source are never merged unless they share the exact same ID.

Within each group, the grouping step also partitions claims into **agreement classes**—sets of claims whose statements agree with one another versus those that state something different.

### How `emery amend` matches existing requirements

During [`emery amend`](../reference/cli/amend.md), Emery includes the existing requirements from the currently sealed revision under the reserved source name `baseline`:

- Each existing requirement enters as a single baseline contributor with its current `REQ-NNN` identifier, subject, statement, and source citations.
- Incoming change claims that match an existing requirement keep its `REQ-NNN` ID and classify it as `changed`, `retired`, `conflict`, or `unchanged`.
- Incoming change claims that match no existing requirement become `added` requirements, numbered sequentially starting above the highest `ReqId` in the current specification.
- Two existing requirements can never be merged into one group during an amendment, and a `retire` claim that fails to match any existing requirement is refused with `retire-unmatched`.

---

## 3. Authority ranks and reconciliation

When multiple sources describe the same requirement differently, Emery resolves the disagreement deterministically using **authority rank** (`rank`), a positive integer where **`1` is the highest authority**.

### Default and custom ranks

Every source has a default rank determined by its adapter's declared kind, which you can override per `[[source]]` entry in `emery.toml`:

| Source kind | Example adapters | Default `rank` | Typical role |
| --- | --- | --- | --- |
| `intent` | `emery:intent` | `1` (highest) | Operator instructions, prompt descriptions, and product briefs. |
| `documentation` | `emery:documentation` | `2` | Markdown specifications, runbooks, and ADRs. |
| `behaviour` | `emery:typescript`, `emery:python` | `3` | Existing application source code and test suites. |
| `baseline` *(amend only)* | *(Engine internal)* | Lowest change rank + 1 | Existing requirements in the sealed revision, ranked below all change sources so new changes always win over the baseline. |

For example, if you want existing production code (`rank = 3`) to take precedence over an outdated internal wiki (`documentation`, default `rank = 2`), set `rank = 4` on the wiki source in `emery.toml`.

### Requirement statuses: `agreed`, `divergence`, and `conflict`

After grouping claims into agreement classes, Emery looks at the highest authority rank (lowest number) present in each class:

| Status | When it occurs | What lands in `spec.json` |
| --- | --- | --- |
| `agreed` | All contributing sources belong to a single agreement class. | The winning statement becomes the requirement body; `losers` is empty. |
| `divergence` | Sources disagree, but one agreement class holds the unique highest authority rank (for example, `intent` at rank 1 overrides `behaviour` at rank 3). | The highest-ranked statement becomes the requirement body. Lower-ranked statements are preserved in `losers` (rendered as `Note:` lines in Markdown) for traceability. |
| `conflict` | Two or more disagreeing classes tie at the top authority rank (for example, two `documentation` sources at rank 2 contradict each other). | The requirement is marked `[conflict]`, its body is left empty, and all competing statements are recorded in `losers` followed by `Note: Operator reconciliation required.` |

A `[conflict]` does not block `emery specify` or `emery apply`—you can seal a conflicted specification to inspect it in Git, and resolve it in a subsequent run or amendment by adjusting source ranks or correcting the contradicting source.

---

## 4. The technology boundary: role vs. implementation

A core architectural invariant of Emery is the strict separation between **what** the system does (`spec.json`) and **how** it is built (`design.json`):

- **The specification (`spec.json`) describes behaviour by role:** It speaks in domain terms and abstract architectural roles—such as *message stream*, *state store*, *relational store*, *secret store*, or *upstream payment service*.
- **The design (`design.json`) holds concrete technologies:** It records specific vendors, products, protocols, frameworks, hosts, and packages—such as *Kafka*, *PostgreSQL*, *Redis*, *AWS Secrets Manager*, *Express*, or *FastAPI*.

### Why specifications stay technology-neutral

Keeping `spec.json` free of source-platform technology names allows Emery to extract behaviour from a legacy Python or TypeScript service and rebuild that exact behaviour on a completely different target platform (such as a Rust WebAssembly component on `emery:omnia`) without leaking Express middleware, SQLAlchemy sessions, or Kafka client settings into the behavioural requirements.

### How the `Lexicon` gate works

Emery enforces this boundary mechanically at every stage that authors specification text—during the SDK's survey stem check, the SDK's claim extraction gate, and the engine's `spec-draft` verification:

1. **Built-in lexicon:** The `emery-lexicon` crate maintains a fixed list of vendor, product, protocol, framework, and host names (while excluding plain domain words like `express` or `vault`).
2. **Dynamic package lexicon:** For each run, Emery extends the lexicon with package terms derived from the source's imported packages (`call` claim callees)—splitting camelCase, snake_case, and CONSTANT_CASE identifiers into words so `useConfluentKafkaConfig` or `KEYVAULT_HOST` is caught automatically.
3. **Self-correction:** If a survey stem, a `requirement` or `criterion` claim, a specification preamble paragraph, or a scenario line spells a forbidden technology name, the gate rejects the candidate answer with a specific finding instructing the model to restate the behaviour by role and move the technology name to a `call`, `excerpt`, or `decision` claim.

### How concrete technologies reach the design

Instead of being lost, concrete technologies flow directly into `design.json`—specifically into `## APIs and integrations` and `## Technical logic`:

- **From code sources (`typescript`, `python`):** `call` claims record the exact package callees and synopses, and `excerpt` claims capture implementation machinery.
- **From prose sources (`intent`, `documentation`):** When a prompt or document prescribes a technology (for example, *"Publish each order event to Kafka"*), the adapter emits two claims:
  1. A technology-neutral `requirement` claim stating the behaviour by role (*"When an order is created, publish an order event to the outbound message stream"*).
  2. A verbatim `decision` claim (*"Publish each order event to Kafka"*).

In the engine, `decision`, `call`, and `contract` claims all inform `## APIs and integrations`, ensuring the design section is generated whenever any source prescribes or uses a concrete integration technology. During [`emery build`](../reference/cli/build.md), the target adapter reads `## APIs and integrations` to map each abstract role in the specification to its concrete implementation.

### Data types vs. mechanism types

In `design.json`, every `type` block under `## Domain model` is classified by the design turn into one of two roles:

- **`data`:** A domain payload or record that the application reads, stores, or returns (for example, `Order`, `LineItem`). Every `data` type is assigned to and owned by exactly one build slice in `plan.json`.
- **`mechanism`:** Internal machinery of the source codebase (such as an HTTP client wrapper, a configuration loader, or a framework bootstrap class). Rendered as `Type: <key> (mechanism)` in Markdown, mechanism types are owned by **no slice** so target adapters do not scaffold source-platform boilerplate.

---

## 5. Deterministic triggers

A requirement's inbound **trigger**—how external callers reach the behaviour—is derived deterministically by code during the source adapter's survey pass, never invented by the language model:

- **HTTP routes (`Trigger::Http`):** Normalized to uppercase HTTP method and `{param}` path syntax (for example, `POST /orders/{id}/cancel`), rendered in `spec.md` as `Route: POST /orders/{id}/cancel`.
- **Messages (`Trigger::Message`):** A single trimmed topic or event name subscribed by a handler (for example, `orders.placed`), rendered in `spec.md` as `Message: orders.placed`.

The source adapter's parser extracts these registrations into `contract` claims. During requirement grouping, the engine binds each `contract` claim to matching requirements by ID prefix (or by overlapping file and line anchor). Target adapters read the typed trigger directly from each slice requirement to scaffold route tables and message consumers automatically.

---

## See also

- [Revision lifecycle](revision-lifecycle.md) — how staged specifications and amendments are sealed and tracked in Git.
- [Wave-based builds and verification](wave-builds.md) — how requirements, triggers, and data types are sliced and built.
- [`emery specify` CLI reference](../reference/cli/specify.md) — command flags and configuration for generating specifications.
- [`emery amend` CLI reference](../reference/cli/amend.md) — command flags and configuration for incremental amendments.
