---
theme: default
title: Omnia & Emery · Spec-Driven Wasm Delivery
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · OMNIA & EMERY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Specs are the truth.<br>Wasm is the boundary.

<p class="section-subtitle">How <strong>Omnia</strong> and <strong>Emery</strong> emerged from our Trust Model to power Tier 2 connectivity and Tier 3 production delivery.</p>
</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Born from the Trust Model (Tiers 2 &amp; 3)</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Omnia · Capability-scoped WebAssembly platform</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Emery · Spec-driven migrator &amp; wave builder</span></div>
</div>

<!--
Opening framing:
- In our Trust Model decks, we deliberately kept technology secondary to governance.
- This deck flips the camera onto the two technologies we built to make that model work at scale: Omnia and Emery.
- Together they answer the two hardest engineering questions in the AI era: how do you safely run code written with AI, and how do you graduate a prototype or legacy service into production without inheriting technical debt?
-->

---
title: Born from the Trust Model
class: slide-light
---

<Eyebrow text="THE ORIGIN · FROM WORLDVIEW TO ARCHITECTURE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Two technologies born<br>from our Trust Model.

</div>
<p class="ed-band-prose">Our Trust Model states that most citizen-built apps stay at <strong>Tier 1</strong>, ~20% connect to live data at <strong>Tier 2</strong>, and ~5% graduate to critical production at <strong>Tier 3</strong>. When we helped clients implement that model, conventional containers and manual rewrites made the final two clearances too expensive — so we built <strong>Omnia</strong> and <strong>Emery</strong>.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 365" aria-hidden="true">
  <!-- Top Tier Progression Spine -->
  <line x1="90" y1="54" x2="1050" y2="54" stroke="rgba(13,16,48,0.16)" stroke-width="2" />

  <circle cx="90" cy="54" r="22" fill="#ffffff" stroke="#b4234e" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="90" y="59" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0</text>
  <text x="90" y="94" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">1 laptop · Unmanaged</text>

  <circle cx="370" cy="54" r="22" fill="#ffffff" stroke="#0c7fa3" stroke-width="2" />
  <text x="370" y="59" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T1</text>
  <text x="370" y="94" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.65)">The Codex · Governed (~75%)</text>

  <circle cx="690" cy="54" r="26" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="2.5" />
  <text x="690" y="59" text-anchor="middle" class="svg-label" fill="#0c7fa3">T2</text>
  <text x="690" y="94" text-anchor="middle" class="svg-label" fill="#0d1030">Connected (~20%)</text>

  <circle cx="1030" cy="54" r="28" fill="#a4238f" />
  <text x="1030" y="59" text-anchor="middle" class="svg-label" fill="#ffffff">T3</text>
  <text x="1030" y="94" text-anchor="middle" class="svg-label" fill="#a4238f">Supported (~5%)</text>

  <!-- Vertical Connectors down to Omnia and Emery -->
  <line x1="690" y1="108" x2="690" y2="152" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="1030" y1="108" x2="1030" y2="152" stroke="#a4238f" stroke-width="1.5" stroke-dasharray="4 4" />

  <!-- Technology 01: Omnia (Spans Tier 2 and Tier 3) -->
  <rect x="520" y="156" width="600" height="88" rx="14" fill="#0d1030" />
  <text x="546" y="184" class="svg-kicker" fill="#78d8f2">OMNIA · RUNTIME &amp; INFRASTRUCTURE ABSTRACTION (TIERS 2 &amp; 3)</text>
  <text x="546" y="208" class="svg-h3" fill="#ffffff">How can agentic code touch live systems safely — and run anywhere?</text>
  <text x="546" y="228" class="svg-caption" fill="rgba(255,255,255,0.72)">WebAssembly capability sandbox · Zero ambient credentials · Portable across clouds &amp; self-hosted stacks</text>

  <!-- Technology 02: Emery (Drives the T2 -> T3 Graduation & Legacy Modernisation) -->
  <path d="M 690 296 L 780 296" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="774,291 784,296 774,301" fill="#a4238f" />
  <text x="676" y="292" text-anchor="end" class="svg-kicker" fill="#a4238f">T2 PROTOTYPE OR LEGACY REPO</text>
  <text x="676" y="310" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.58)">Working rules trapped in brittle code</text>

  <rect x="790" y="256" width="330" height="88" rx="14" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="814" y="284" class="svg-kicker" fill="#a4238f">EMERY · SPEC-DRIVEN ENGINE (TIER 3)</text>
  <text x="814" y="308" class="svg-h3" fill="#0d1030">Specs as truth → Verified Wasm</text>
  <text x="814" y="328" class="svg-caption" fill="rgba(13,16,48,0.68)">Extracts specs &amp; compiles Omnia-ready guests</text>
</svg>
</div>
</div>

<!--
How Omnia and Emery relate to the Trust Model:
- At Tier 1, GitHub + Cursor rules + CI gates (The Codex) are enough because nothing touches live data.
- At Tier 2 (the 3rd tier, Connected) and Tier 3 (the 4th tier, Supported), two hard problems appear:
  1. Runtime safety & portability: answered by Omnia across Tiers 2 and 3.
  2. Graduation without inheriting messy code: answered by Emery at Tier 3.
-->

---
title: Omnia
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART ONE · OMNIA RUNTIME PLATFORM" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Omnia

<p class="section-subtitle"><strong>The WebAssembly runtime for Tiers 2 and 3</strong> — sandboxed agentic execution, cloud portability, and recipe-driven hosting.</p>
</div>
<div class="section-big-glyph">WASM</div>
</div>

<TrustLadder variant="strip" :current="3" />

<!--
Part One: Three slides on Omnia.
1. WebAssembly for safer, sandboxed agentic code.
2. Omnia as an infrastructure abstraction layer for platform portability.
3. Omnia + recipes to assemble a complete Tier 3 production hosting platform.
-->

---
title: Omnia · Sandboxed Agentic Code
class: slide-light
---

<Eyebrow text="OMNIA 01 / 03 · SAFER AGENTIC EXECUTION" />

<div class="ed-split">
<div class="ed-prose-col">

# Sandboxed by construction,<br>not by policy.

<p class="ed-prose">When code is authored by AI assistants or domain builders, running it in a standard container is a gamble: a single imported package or hallucinated call inherits the container's environment variables, filesystem, and network sockets.</p>
<p class="ed-prose"><strong>Omnia compiles application logic to WebAssembly (WASI) components.</strong> Every request or message spins up a fresh, isolated micro-instance in microseconds with <strong>zero ambient authority</strong>. The guest holds no connection strings and no keys; it can only invoke the exact typed capability interfaces the host links to it.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Left Comparison: Traditional Container (Ambient Access) -->
  <rect x="16" y="44" width="232" height="352" rx="14" fill="rgba(255,59,107,0.04)" stroke="rgba(180,35,78,0.35)" stroke-width="1.5" stroke-dasharray="5 4" />
  <text x="132" y="74" text-anchor="middle" class="svg-kicker" fill="#b4234e">TRADITIONAL CONTAINER</text>
  <text x="132" y="96" text-anchor="middle" class="svg-h3" fill="#0d1030">Ambient Authority</text>
  <line x1="40" y1="112" x2="224" y2="112" stroke="rgba(180,35,78,0.2)" />

  <circle cx="44" cy="148" r="4" fill="#b4234e" />
  <text x="58" y="152" class="svg-label" fill="#0d1030">Raw env vars &amp; secrets</text>
  <text x="58" y="168" class="svg-caption" fill="rgba(13,16,48,0.58)">Any dependency can read process.env</text>

  <circle cx="44" cy="212" r="4" fill="#b4234e" />
  <text x="58" y="216" class="svg-label" fill="#0d1030">Unrestricted TCP sockets</text>
  <text x="58" y="232" class="svg-caption" fill="rgba(13,16,48,0.58)">Can phone home or scan internal net</text>

  <circle cx="44" cy="276" r="4" fill="#b4234e" />
  <text x="58" y="280" class="svg-label" fill="#0d1030">Full DB driver access</text>
  <text x="58" y="296" class="svg-caption" fill="rgba(13,16,48,0.58)">Connection string grants whole schema</text>

  <text x="132" y="366" text-anchor="middle" class="svg-caption" fill="#b4234e">1 prompt mistake = full exposure</text>

  <!-- Right Architecture: Omnia Wasm Sandbox -->
  <rect x="284" y="44" width="340" height="352" rx="14" fill="#0d1030" />
  <text x="454" y="74" text-anchor="middle" class="svg-kicker" fill="#78d8f2">OMNIA HOST · CAPABILITY SANDBOX</text>

  <!-- Inner Wasm Guest Box -->
  <rect x="312" y="96" width="284" height="112" rx="10" fill="rgba(255,255,255,0.08)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="454" y="124" text-anchor="middle" class="svg-kicker" fill="#78d8f2">ISOLATED GUEST (app.wasm)</text>
  <text x="454" y="148" text-anchor="middle" class="svg-h3" fill="#ffffff">Pure Domain Logic</text>
  <text x="454" y="170" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">No OS · No filesystem · No raw sockets · No keys</text>
  <text x="454" y="190" text-anchor="middle" class="svg-caption" fill="#78d8f2">Fresh instance per request / message</text>

  <!-- Explicit WIT Capability Links -->
  <line x1="368" y1="208" x2="368" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="363,252 373,252 368,260" fill="#78d8f2" />
  <line x1="454" y1="208" x2="454" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="449,252 459,252 454,260" fill="#78d8f2" />
  <line x1="540" y1="208" x2="540" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="535,252 545,252 540,260" fill="#78d8f2" />
  <text x="454" y="240" text-anchor="middle" class="svg-pill" fill="#f59ef5">EXPLICIT TYPED WASI GRANTS ONLY</text>

  <!-- Host-Managed Capability Backends -->
  <rect x="312" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="354" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:sql</text>
  <text x="354" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Scoped rows</text>

  <rect x="412" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="454" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:msg</text>
  <text x="454" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Named topics</text>

  <rect x="512" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="554" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:http</text>
  <text x="554" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Allowed hosts</text>

  <text x="454" y="366" text-anchor="middle" class="svg-caption" fill="#78d8f2">Host retains all credentials, pools &amp; audit logs</text>
</svg>
</div>
</div>

<!--
Omnia Slide 1 of 3 — WebAssembly for safer, sandboxed agentic code:
- Why Wasm? Because when AI writes or modifies code, you cannot trust what is inside the binary to police itself.
- In a Docker container, any library has ambient access to env vars, the filesystem, and raw network sockets.
- In Omnia, guest logic is compiled to a WASI component. It has zero ambient access: no filesystem, no env vars, no network sockets, no credentials.
- It can only call the typed WASI capabilities linked by the host (wasi:sql, wasi:messaging, wasi:keyvalue, wasi:http), and the host holds the credentials and audits every call.
-->

---
title: Omnia · Infrastructure Portability
class: slide-light
---

<Eyebrow text="OMNIA 02 / 03 · INFRASTRUCTURE ABSTRACTION & PORTABILITY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Change the host,<br>never the application.

</div>
<p class="ed-band-prose">When application code imports cloud SDKs directly, every infrastructure shift — moving from cloud to on-prem, swapping Redis for NATS, or tightening data residency — becomes an application rewrite. In <strong>Omnia</strong>, the guest WebAssembly component speaks only WASI interfaces; swapping the backing infrastructure is a one-line change in the host runtime with <strong>zero guest recompilation</strong>.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 355" aria-hidden="true">
  <!-- Top Layer: Portable Guest Component (Compiled Once) -->
  <rect x="290" y="12" width="560" height="72" rx="12" fill="#0d1030" />
  <text x="570" y="38" text-anchor="middle" class="svg-kicker" fill="#78d8f2">PORTABLE APPLICATION COMPONENT (service.wasm · COMPILED ONCE)</text>
  <text x="570" y="62" text-anchor="middle" class="svg-h3" fill="#ffffff">Pure Domain Logic — Zero Cloud SDKs or Driver Imports</text>

  <!-- Middle Layer: The 10 WASI Capability Contracts -->
  <line x1="570" y1="84" x2="570" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <line x1="80" y1="124" x2="1060" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <rect x="380" y="108" width="380" height="28" rx="14" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="570" y="126" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">TYPED WASI CAPABILITY BOUNDARY (HTTP · SQL · KV · MSG · BLOB · VAULT · ID · OTEL)</text>

  <!-- Three Vertical Branches to Interchangeable Host Environments -->
  <line x1="195" y1="124" x2="195" y2="174" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="190,168 200,168 195,176" fill="#0c7fa3" />
  <line x1="570" y1="136" x2="570" y2="174" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="565,168 575,168 570,176" fill="#0c7fa3" />
  <line x1="945" y1="124" x2="945" y2="174" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="940,168 950,168 945,176" fill="#a4238f" />

  <!-- Environment 1: Local / Tier 1 Sandbox -->
  <rect x="30" y="180" width="330" height="156" rx="14" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.3)" stroke-width="1.5" />
  <text x="54" y="208" class="svg-kicker" fill="#0c7fa3">01 · LOCAL &amp; TIER 1 SANDBOX</text>
  <text x="54" y="232" class="svg-h3" fill="#0d1030">Zero-Config Defaults</text>
  <text x="54" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• In-memory Key-Value, SQL &amp; DocStore</text>
  <text x="54" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• In-memory Pub/Sub &amp; WebSockets</text>
  <text x="54" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Deterministic omnia-test doubles</text>
  <text x="54" y="322" class="svg-caption" fill="#0c7fa3">Runs in milliseconds on any laptop or CI runner</text>

  <!-- Environment 2: Hyperscale Cloud (Azure / AWS / GCP) -->
  <rect x="405" y="180" width="330" height="156" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.5" />
  <text x="429" y="208" class="svg-kicker" fill="#0d1030">02 · HYPERSCALE CLOUD (PAAS)</text>
  <text x="429" y="232" class="svg-h3" fill="#0d1030">Managed Cloud Services</text>
  <text x="429" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• Azure Blob, Table Storage &amp; Key Vault</text>
  <text x="429" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• Azure Managed Identity / OAuth</text>
  <text x="429" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Managed PostgreSQL, Redis &amp; Kafka</text>
  <text x="429" y="322" class="svg-caption" fill="rgba(13,16,48,0.55)">Swap clouds without touching guest code</text>

  <!-- Environment 3: Self-Hosted / Sovereign / Air-Gapped -->
  <rect x="780" y="180" width="330" height="156" rx="14" fill="rgba(164,35,143,0.06)" stroke="rgba(164,35,143,0.35)" stroke-width="1.5" />
  <text x="804" y="208" class="svg-kicker" fill="#a4238f">03 · SELF-HOSTED / SOVEREIGN</text>
  <text x="804" y="232" class="svg-h3" fill="#0d1030">On-Premises &amp; Edge</text>
  <text x="804" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• Self-hosted PostgreSQL &amp; MongoDB</text>
  <text x="804" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• NATS JetStream, Apache Kafka &amp; Redis</text>
  <text x="804" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Local OTLP Collector &amp; local LLMs</text>
  <text x="804" y="322" class="svg-caption" fill="#a4238f">Full data residency &amp; zero cloud lock-in</text>
</svg>
</div>
</div>

<!--
Omnia Slide 2 of 3 — Infrastructure abstraction for platform portability:
- The exact same .wasm binary that runs against in-memory defaults on a developer's laptop runs unchanged against Azure managed services or a self-hosted sovereign stack (Postgres, NATS, Kafka, Redis).
- Clients retain real bargaining power and deployment optionality over a 10-year system lifecycle.
-->

---
title: Omnia · Complete Hosting Platform
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="OMNIA 03 / 03 · OMNIA + RECIPES FOR TIER 3 HOSTING" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Omnia + Recipes · Production Platform</span>

# One runtime and a recipe build a complete Tier 3 platform.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>runtime! + omnia.toml</span><em>Declarative composition</em></div>
<p>An Omnia host recipe pairs WASI interfaces with production backends in one macro call, while <strong>omnia.toml</strong> routes HTTP paths, messaging topics, and WebSockets across many Wasm guests.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>Zero-hop guest linking &amp; AOT</span><em>Built for SLA operations</em></div>
<p>Guests call each other in-memory across typed WIT interfaces with no network serialization, pre-compile to native artifacts for sub-millisecond cold starts, and export unified OpenTelemetry traces.</p>
</div>
</div>
</div>

<div class="ed-diagram-pane">
<svg viewBox="0 0 700 420" aria-hidden="true">
  <!-- Outer Platform Frame -->
  <rect x="16" y="20" width="668" height="380" rx="16" fill="rgba(10,7,16,0.58)" stroke="rgba(120,216,242,0.35)" stroke-width="1.5" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">TIER 3 HOSTING RECIPE · omnia.toml + omnia::runtime!</text>

  <!-- Left: Unified Ingress Router -->
  <rect x="42" y="74" width="144" height="224" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.2)" />
  <text x="114" y="102" text-anchor="middle" class="svg-kicker" fill="#78d8f2">INGRESS ROUTER</text>
  <text x="114" y="136" text-anchor="middle" class="svg-label" fill="#ffffff">HTTP Prefixes</text>
  <text x="114" y="154" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">/orders, /roster</text>
  <text x="114" y="196" text-anchor="middle" class="svg-label" fill="#ffffff">Topic Patterns</text>
  <text x="114" y="214" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">orders.&gt;, shifts.*</text>
  <text x="114" y="256" text-anchor="middle" class="svg-label" fill="#ffffff">WebSockets</text>
  <text x="114" y="274" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">Live client streams</text>

  <!-- Arrows from Router to Multi-Guest Fleet -->
  <line x1="186" y1="112" x2="236" y2="112" stroke="#78d8f2" stroke-width="1.6" />
  <line x1="186" y1="186" x2="236" y2="186" stroke="#78d8f2" stroke-width="1.6" />
  <line x1="186" y1="260" x2="236" y2="260" stroke="#78d8f2" stroke-width="1.6" />

  <!-- Center: Multi-Guest Wasm Fleet (with in-memory WIT link) -->
  <rect x="240" y="74" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="346" y="100" text-anchor="middle" class="svg-label" fill="#ffffff">Guest A · roster.wasm</text>
  <text x="346" y="118" text-anchor="middle" class="svg-caption" fill="#78d8f2">AOT pre-compiled · On-demand</text>

  <rect x="240" y="157" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="346" y="183" text-anchor="middle" class="svg-label" fill="#ffffff">Guest B · orders.wasm</text>
  <text x="346" y="201" text-anchor="middle" class="svg-caption" fill="#78d8f2">Calls Guest C via in-memory WIT</text>

  <line x1="346" y1="215" x2="346" y2="240" stroke="#f59ef5" stroke-width="1.8" stroke-dasharray="3 3" />

  <rect x="240" y="240" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="346" y="266" text-anchor="middle" class="svg-label" fill="#ffffff">Guest C · pricing.wasm</text>
  <text x="346" y="284" text-anchor="middle" class="svg-caption" fill="#f59ef5">Exported WIT service · Zero network hop</text>

  <!-- Right: Production Recipe Backends -->
  <line x1="452" y1="112" x2="498" y2="112" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />
  <line x1="452" y1="186" x2="498" y2="186" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />
  <line x1="452" y1="260" x2="498" y2="260" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />

  <rect x="502" y="74" width="156" height="224" rx="10" fill="rgba(214,58,214,0.14)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="580" y="102" text-anchor="middle" class="svg-kicker" fill="#f59ef5">RECIPE BACKENDS</text>
  <text x="580" y="136" text-anchor="middle" class="svg-label" fill="#ffffff">Connection Pools</text>
  <text x="580" y="154" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Postgres · Redis · Blob</text>
  <text x="580" y="196" text-anchor="middle" class="svg-label" fill="#ffffff">Event Brokers</text>
  <text x="580" y="214" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Kafka · NATS JetStream</text>
  <text x="580" y="256" text-anchor="middle" class="svg-label" fill="#ffffff">Vault &amp; Identity</text>
  <text x="580" y="274" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Key Vault · Managed ID</text>

  <!-- Bottom Observability & Packaging Bar -->
  <rect x="42" y="320" width="616" height="54" rx="10" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.16)" />
  <text x="64" y="352" class="svg-kicker" fill="#78d8f2">PLATFORM OPS</text>
  <text x="190" y="352" class="svg-body" fill="#ffffff">Built-in OTLP traces &amp; pool gauges · Non-root Alpine image · Hot-mount .wasm updates without host rebuild</text>
</svg>
</div>
</div>

<!--
Omnia Slide 3 of 3 — Omnia + recipes to build a complete hosting platform:
- Omnia is not just a library; paired with a host recipe (omnia::runtime! + omnia.toml), it is a complete multi-tenant, multi-guest hosting platform.
- One runtime process can host dozens of .wasm components, route HTTP/Messaging/WebSocket traffic to them, link guests to guests in-memory, and ship guest updates as volume mounts without rebuilding the host container.
-->

---
title: Emery
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART TWO · EMERY SPECIFICATION & BUILD ENGINE" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Emery

<p class="section-subtitle"><strong>Spec-driven development for Tier 3</strong> — specifications as the source of truth, a migrator for citizen and legacy apps, and verified Omnia guest builds.</p>
</div>
<div class="section-big-glyph">SPEC</div>
</div>

<TrustLadder variant="strip" :current="3" />

<!--
Part Two: Three slides on Emery.
1. Specs, not code, as the durable source of truth.
2. Citizen-coded app & legacy migrator (emery specify).
3. Wave-by-wave verified builder of Omnia-ready Wasm guests (emery build).
-->

---
title: Emery · Specs Not Code as Truth
class: slide-light
---

<Eyebrow text="EMERY 01 / 03 · SPECIFICATIONS AS THE SOURCE OF TRUTH" />

<div class="ed-split">
<div class="ed-prose-col">

# Specs are the asset.<br>Code is a projection.

<p class="ed-prose">In both vibe-coded prototypes and twenty-year-old legacy systems, business intent is trapped inside code. Prompts vanish when the chat window closes, and code tangles <em>what the business requires</em> with <em>how a framework happened to wire it</em>.</p>
<p class="ed-prose"><strong>Emery flips the hierarchy:</strong> the content-addressed revision — a pure, technology-free <strong>Specification</strong>, a <strong>Design</strong>, and a sliced <strong>Build Plan</strong> — is the permanent source of truth. Code is a replaceable artifact compiled from that specification for your target runtime.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Center Durable Core: Content-Addressed Revision -->
  <rect x="44" y="64" width="288" height="312" rx="16" fill="#0d1030" />
  <text x="188" y="96" text-anchor="middle" class="svg-kicker" fill="#78d8f2">DURABLE SOURCE OF TRUTH (emery/&lt;rev&gt;)</text>
  <text x="188" y="120" text-anchor="middle" class="svg-h3" fill="#ffffff">Content-Addressed Revision</text>

  <!-- Spec Box -->
  <rect x="68" y="140" width="240" height="64" rx="10" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="86" y="164" class="svg-kicker" fill="#78d8f2">01 · SPECIFICATION (spec.md)</text>
  <text x="86" y="184" class="svg-caption" fill="#ffffff">Requirements, Given/When/Then scenarios</text>
  <text x="86" y="198" class="svg-caption" fill="rgba(255,255,255,0.65)">Lexicon-gated: zero vendor/tech names</text>

  <!-- Design Box -->
  <rect x="68" y="216" width="240" height="64" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.18)" />
  <text x="86" y="240" class="svg-kicker" fill="#f59ef5">02 · DESIGN (design.md)</text>
  <text x="86" y="260" class="svg-caption" fill="#ffffff">Data types, mechanisms &amp; integrations</text>
  <text x="86" y="274" class="svg-caption" fill="rgba(255,255,255,0.65)">Holds the technology &amp; wire shapes</text>

  <!-- Plan Box -->
  <rect x="68" y="292" width="240" height="64" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.18)" />
  <text x="86" y="316" class="svg-kicker" fill="#78d8f2">03 · BUILD PLAN (plan.md)</text>
  <text x="86" y="336" class="svg-caption" fill="#ffffff">Dependency-ordered domain slices</text>
  <text x="86" y="350" class="svg-caption" fill="rgba(255,255,255,0.65)">One stem or trigger per slice</text>

  <!-- Projection Arrows to Replaceable Target Codebases -->
  <path d="M 332 180 C 378 180, 384 134, 428 134" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="422,129 432,134 422,139" fill="#a4238f" />
  <path d="M 332 260 C 378 260, 384 306, 428 306" fill="none" stroke="#0c7fa3" stroke-width="1.8" stroke-dasharray="4 4" />
  <polygon points="422,301 432,306 422,311" fill="#0c7fa3" />

  <!-- Primary Target: Omnia Rust/Wasm -->
  <rect x="436" y="78" width="188" height="116" rx="12" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="530" y="106" text-anchor="middle" class="svg-kicker" fill="#a4238f">TARGET · emery:omnia</text>
  <text x="530" y="130" text-anchor="middle" class="svg-h3" fill="#0d1030">Omnia Wasm Guest</text>
  <text x="530" y="152" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Rust + WASI capabilities</text>
  <text x="530" y="170" text-anchor="middle" class="svg-caption" fill="#a4238f">Tier 3 Production Standard</text>

  <!-- Secondary Target: Node / Other Targets -->
  <rect x="436" y="248" width="188" height="116" rx="12" fill="#f5f6fa" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="530" y="276" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">TARGET · emery:node</text>
  <text x="530" y="300" text-anchor="middle" class="svg-h3" fill="#0d1030">Other Runtimes</text>
  <text x="530" y="322" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Strict TypeScript / Node</text>
  <text x="530" y="340" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.52)">Same spec, new target adapter</text>
</svg>
</div>
</div>

<!--
Emery Slide 1 of 3 — Specifications, not code, as the source of truth:
- Why is a specification better than code as the long-term asset?
- Because Emery's lexicon gate strips vendor and framework names out of the Specification (spec.md), keeping requirements expressed purely by domain role and Given/When/Then scenarios.
- That means the Specification survives framework churn: you can compile it to an Omnia Rust/Wasm guest today, or re-target it tomorrow without re-discovering a single business rule.
-->

---
title: Emery · Citizen & Legacy Migrator
class: slide-light
---

<Eyebrow text="EMERY 02 / 03 · THE CITIZEN-CODED APP & LEGACY MIGRATOR" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Mine the behaviour.<br>Expose every conflict.

</div>
<p class="ed-band-prose">Whether you are graduating a nine-month citizen-coded prototype from Tier 2 or retiring a twenty-year-old legacy service, <strong>emery specify</strong> acts as an automated migrator. Source adapters parse code, docs, and intent into typed claims, and reconcile them under <strong>authority rank</strong> — surfacing conflicts for humans rather than letting an AI guess.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <!-- Left Column: Multi-Source Inputs with Authority Ranks -->
  <text x="16" y="26" class="svg-kicker" fill="#0c7fa3">01 · SOURCE ADAPTERS (WASM)</text>

  <rect x="16" y="44" width="256" height="64" rx="10" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="34" y="68" class="svg-kicker" fill="#0c7fa3">RANK 1 · emery:intent</text>
  <text x="34" y="90" class="svg-label" fill="#0d1030">Product Briefs &amp; Directives</text>

  <rect x="16" y="124" width="256" height="64" rx="10" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.5" />
  <text x="34" y="148" class="svg-kicker" fill="#0d1030">RANK 2 · emery:documentation</text>
  <text x="34" y="170" class="svg-label" fill="#0d1030">ADRs, Specs &amp; Feature Files</text>

  <rect x="16" y="204" width="256" height="96" rx="10" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="34" y="228" class="svg-kicker" fill="#a4238f">RANK 3 · typescript / python</text>
  <text x="34" y="250" class="svg-label" fill="#0d1030">T2 Prototype or Legacy Code</text>
  <text x="34" y="270" class="svg-caption" fill="rgba(13,16,48,0.65)">AST parser extracts routes, topics,</text>
  <text x="34" y="286" class="svg-caption" fill="rgba(13,16,48,0.65)">types, decision points &amp; test cases</text>

  <!-- Connectors to Reconciliation Engine -->
  <path d="M 272 76 C 330 76, 340 165, 396 165" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <line x1="272" y1="156" x2="396" y2="165" stroke="#0d1030" stroke-width="1.6" />
  <path d="M 272 252 C 330 252, 340 165, 396 165" fill="none" stroke="#a4238f" stroke-width="1.6" />

  <!-- Middle Column: Evidence & Authority Reconciliation -->
  <rect x="400" y="54" width="320" height="236" rx="14" fill="#0d1030" />
  <text x="560" y="84" text-anchor="middle" class="svg-kicker" fill="#78d8f2">02 · EVIDENCE &amp; AUTHORITY RECONCILIATION</text>
  <line x1="428" y1="100" x2="692" y2="100" stroke="rgba(255,255,255,0.14)" />

  <circle cx="436" cy="130" r="4.5" fill="#78d8f2" />
  <text x="452" y="134" class="svg-label" fill="#ffffff">Corroborated &amp; Single-Source</text>
  <text x="452" y="152" class="svg-caption" fill="rgba(255,255,255,0.68)">Anchored to exact file &amp; #L line spans</text>

  <circle cx="436" cy="186" r="4.5" fill="#f59ef5" />
  <text x="452" y="190" class="svg-label" fill="#ffffff">Divergence (Higher Rank Wins)</text>
  <text x="452" y="208" class="svg-caption" fill="rgba(255,255,255,0.68)">Intent overrides accidental prototype bugs</text>

  <circle cx="436" cy="242" r="4.5" fill="#ff7597" />
  <text x="452" y="246" class="svg-label" fill="#ffffff">Conflict &amp; [unknown] Preserved</text>
  <text x="452" y="264" class="svg-caption" fill="rgba(255,255,255,0.68)">Equal-rank ties surfaced for human sign-off</text>

  <!-- Connector to Output -->
  <line x1="720" y1="172" x2="796" y2="172" stroke="#a4238f" stroke-width="2.2" />
  <polygon points="790,167 800,172 790,177" fill="#a4238f" />

  <!-- Right Column: Clean Migration Blueprint -->
  <rect x="804" y="54" width="316" height="236" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="962" y="84" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">03 · REVIEWABLE MIGRATION BASELINE</text>
  <text x="962" y="114" text-anchor="middle" class="svg-h3" fill="#0d1030">Keep the criteria.</text>
  <text x="962" y="136" text-anchor="middle" class="svg-h3" fill="#0d1030">Drop the code.</text>
  <line x1="836" y1="154" x2="1088" y2="154" stroke="rgba(13,16,48,0.12)" />
  <text x="836" y="182" class="svg-body" fill="#0d1030">• Every HTTP route &amp; message trigger typed</text>
  <text x="836" y="208" class="svg-body" fill="#0d1030">• Domain data shapes separated from plumbing</text>
  <text x="836" y="234" class="svg-body" fill="#0d1030">• Product Owner &amp; engineers review before build</text>
  <text x="836" y="264" class="svg-caption" fill="#0c7fa3">Zero hallucinated requirements</text>
</svg>
</div>
</div>

<!--
Emery Slide 2 of 3 — Citizen-coded app & legacy migrator:
- Why naive AI translation fails: if you tell an LLM "port this prototype to Rust", it ports every bug and framework hack along with the business logic.
- emery specify parses the prototype or legacy repo with deterministic AST surveys (routes, message subscriptions, data types, test assertions), combines it with docs and product intent under authority rank, and produces a human-reviewable specification before a single line of replacement code is written.
-->

---
title: Emery · Verified Omnia Guest Builder
class: slide-light
---

<Eyebrow text="EMERY 03 / 03 · DELIVERING OMNIA-READY WASM GUESTS" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 660 440" aria-hidden="true">
  <!-- Step 1: Wave Slicing & Deterministic Scaffold -->
  <rect x="20" y="32" width="276" height="104" rx="12" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="40" y="58" class="svg-kicker" fill="#0c7fa3">01 · emery:omnia SCAFFOLD</text>
  <text x="40" y="82" class="svg-h3" fill="#0d1030">Typed Rust Skeleton</text>
  <text x="40" y="102" class="svg-caption" fill="rgba(13,16,48,0.65)">Exemplar guest template, typed routes,</text>
  <text x="40" y="118" class="svg-caption" fill="rgba(13,16,48,0.65)">wire structs &amp; scenario test stubs laid by code</text>

  <line x1="296" y1="84" x2="356" y2="84" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="350,79 360,84 350,89" fill="#0c7fa3" />

  <!-- Step 2: Parallel Slice Worktrees in Kernel Sandbox -->
  <rect x="364" y="32" width="276" height="104" rx="12" fill="#0d1030" />
  <text x="384" y="58" class="svg-kicker" fill="#78d8f2">02 · PARALLEL WAVE BUILD</text>
  <text x="384" y="82" class="svg-h3" fill="#ffffff">Isolated Slice Worktrees</text>
  <text x="384" y="102" class="svg-caption" fill="rgba(255,255,255,0.7)">Each slice built in .emery/vcs/worktrees/</text>
  <text x="384" y="118" class="svg-caption" fill="rgba(255,255,255,0.7)">inside a kernel-confined worker grant</text>

  <!-- Downward connector to Verification Gate -->
  <line x1="502" y1="136" x2="502" y2="180" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="497,174 507,174 502,182" fill="#0d1030" />

  <!-- Step 3: Deterministic AST & Wave Verification -->
  <rect x="20" y="186" width="620" height="118" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="44" y="214" class="svg-kicker" fill="#a4238f">03 · DETERMINISTIC WAVE VERIFICATION (BEFORE ANY SLICE MERGES OR LABELS)</text>

  <circle cx="52" cy="244" r="4" fill="#a4238f" />
  <text x="68" y="248" class="svg-label" fill="#0d1030">Host-Boundary Lints (clippy.toml)</text>
  <text x="68" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Forbids std::env, std::fs, raw HTTP &amp; timers</text>

  <circle cx="356" cy="244" r="4" fill="#a4238f" />
  <text x="372" y="248" class="svg-label" fill="#0d1030">Scenario &amp; AST Gate</text>
  <text x="372" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Every REQ-* scenario tested over omnia-test</text>

  <circle cx="52" cy="288" r="4" fill="#a4238f" />
  <text x="68" y="292" class="svg-label" fill="#0d1030">wasm32-wasip2 Compile Check</text>

  <circle cx="356" cy="288" r="4" fill="#a4238f" />
  <text x="372" y="292" class="svg-label" fill="#0d1030">Secret Gate on Every Commit</text>

  <!-- Downward connector to Sealed Output -->
  <line x1="330" y1="304" x2="330" y2="344" stroke="#a4238f" stroke-width="2" />
  <polygon points="325,338 335,338 330,346" fill="#a4238f" />

  <!-- Step 4: Output -->
  <rect x="130" y="350" width="400" height="68" rx="12" fill="#a4238f" />
  <text x="330" y="376" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.88)">SEALED &amp; LABELLED (emery/&lt;revision&gt;)</text>
  <text x="330" y="400" text-anchor="middle" class="svg-h3" fill="#ffffff">Verified, Omnia-Ready guest.wasm</text>
</svg>
</div>
<div class="ed-prose-col">

# Built in waves.<br>Verified before merge.

<p class="ed-prose">Once the specification is reviewed, <strong>emery build</strong> compiles it into an Omnia WebAssembly guest wave by wave. Rather than asking a model to invent a repository from scratch, the <strong>emery:omnia</strong> adapter scaffolds the exemplar Rust workspace, typed route and topic handlers, wire structs, and test stubs by code before the model turn begins.</p>
<p class="ed-prose">Every slice is checked by syntax-tree rules, forbidden-API lints that enforce the Omnia capability boundary, scenario tests over <strong>omnia-test</strong>, and a secret gate before the wave is sealed and labelled <strong>emery/&lt;revision&gt;</strong>.</p>
</div>
</div>

<!--
Emery Slide 3 of 3 — Delivering Omnia-ready Wasm guests:
- emery build doesn't let an LLM freehand an entire codebase.
- The emery:omnia target adapter deterministically scaffolds the exemplar Wasm guest layout and typed signatures first, then gates the model's implementation against clippy.toml host-boundary lints, syn syntax checks, omnia-test scenario tests, and wasm32-wasip2 compilation.
-->

---
title: Omnia + Emery Together
class: slide-light
---

<Eyebrow text="THE COMPLETE SYSTEM · OMNIA + EMERY" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <!-- Left Plane: EMERY (Build-Time Assurance) -->
  <rect x="20" y="24" width="490" height="236" rx="16" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="48" y="54" class="svg-kicker" fill="#a4238f">EMERY · BUILD-TIME ASSURANCE PLANE</text>
  <text x="48" y="80" class="svg-h2" fill="#0d1030">Preserves why it was built</text>

  <circle cx="60" cy="120" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="125" text-anchor="middle" class="svg-kicker" fill="#a4238f">01</text>
  <text x="92" y="116" class="svg-label" fill="#0d1030">Extracts &amp; reconciles intent (emery specify)</text>
  <text x="92" y="134" class="svg-caption" fill="rgba(13,16,48,0.6)">From T2 prototypes, legacy repos, docs &amp; briefs</text>

  <circle cx="60" cy="174" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="179" text-anchor="middle" class="svg-kicker" fill="#a4238f">02</text>
  <text x="92" y="170" class="svg-label" fill="#0d1030">Holds the technology-free specification</text>
  <text x="92" y="188" class="svg-caption" fill="rgba(13,16,48,0.6)">Human-reviewed scenarios &amp; explicit conflict resolution</text>

  <circle cx="60" cy="228" r="18" fill="#a4238f" />
  <text x="60" y="233" text-anchor="middle" class="svg-kicker" fill="#ffffff">03</text>
  <text x="92" y="224" class="svg-label" fill="#0d1030">Compiles verified waves (emery build)</text>
  <text x="92" y="242" class="svg-caption" fill="rgba(13,16,48,0.6)">Enforces capability lints, scenario tests &amp; secret gates</text>

  <!-- Handoff Bridge: Verified .wasm + Capability Manifest -->
  <line x1="510" y1="142" x2="630" y2="142" stroke="#0d1030" stroke-width="2.5" />
  <polygon points="622,136 634,142 622,148" fill="#0d1030" />
  <rect x="524" y="102" width="92" height="28" rx="14" fill="#0d1030" />
  <text x="570" y="120" text-anchor="middle" class="svg-pill" fill="#78d8f2">guest.wasm</text>
  <text x="570" y="166" text-anchor="middle" class="svg-caption" fill="#0d1030">Verified artifact</text>

  <!-- Right Plane: OMNIA (Run-Time Execution) -->
  <rect x="634" y="24" width="486" height="236" rx="16" fill="#0d1030" />
  <text x="662" y="54" class="svg-kicker" fill="#78d8f2">OMNIA · RUN-TIME EXECUTION PLANE</text>
  <text x="662" y="80" class="svg-h2" fill="#ffffff">Decouples where it runs</text>

  <circle cx="674" cy="120" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="125" text-anchor="middle" class="svg-kicker" fill="#78d8f2">04</text>
  <text x="706" y="116" class="svg-label" fill="#ffffff">Executes inside a WASI capability sandbox</text>
  <text x="706" y="134" class="svg-caption" fill="rgba(255,255,255,0.68)">Zero ambient authority · Audited caller identity</text>

  <circle cx="674" cy="174" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="179" text-anchor="middle" class="svg-kicker" fill="#78d8f2">05</text>
  <text x="706" y="170" class="svg-label" fill="#ffffff">Abstracts cloud &amp; on-prem infrastructure</text>
  <text x="706" y="188" class="svg-caption" fill="rgba(255,255,255,0.68)">Swap Azure, AWS, Postgres, Kafka, NATS or Redis in the host</text>

  <circle cx="674" cy="228" r="18" fill="#78d8f2" />
  <text x="674" y="233" text-anchor="middle" class="svg-kicker" fill="#0d1030">06</text>
  <text x="706" y="224" class="svg-label" fill="#ffffff">Scales via multi-guest hosting recipes</text>
  <text x="706" y="242" class="svg-caption" fill="rgba(255,255,255,0.68)">Declarative routing, zero-hop WIT links &amp; OTLP telemetry</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Emery preserves the why.<br>Omnia decouples the where.

</div>
<div>
<p class="ed-prose">Together, the two technologies close the loop between fast domain discovery and long-term production ownership. <strong>Emery</strong> ensures you never lose business intent to a chat log or a legacy codebase; <strong>Omnia</strong> ensures that what you build is sandboxed by construction and never locked into a single cloud provider.</p>
<p class="ed-prose">When business rules change six months later, you update the specification, run a verified build wave, and mount the new <strong>.wasm</strong> guest into the running Omnia host — with zero infrastructure churn.</p>
</div>
<div></div>
</div>
</div>

<!--
Synthesis slide:
- Emery is the build-time assurance plane: it preserves *why* the system was built and compiles verified .wasm guests.
- Omnia is the run-time execution plane: it decouples *where* the system runs from *what* it does.
-->

---
title: Start small. Prove the loop.
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>25 years of building critical software across Aotearoa and the world — combining AI speed with senior engineering judgment.</p>
</div>
</div>

<div class="split-content-pane">
<div>
<Eyebrow text="PROPELLERHEAD · NEXT STEPS" />

# Start small.<br>Prove the loop.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Pick one Tier 2 prototype or legacy service</h4>
<p>Choose a candidate with clear business value — either a citizen-coded tool ready to graduate to Tier 3 or a bounded legacy component.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Extract the specification with Emery in days</h4>
<p>We mine the working rules, contracts, and test scenarios into a versioned specification your domain owners and engineers can review together.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Deploy the verified Wasm guest on Omnia</h4>
<p>You finish with a capability-sandboxed production service, portable across cloud or self-hosted recipes, and a living specification your team owns.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">READY TO PROVE THE LOOP?</div>
<div class="cta-card-title">Let's pick a first candidate together.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>propellerhead.co.nz</span>
</div>
</div>
</div>

<!--
Closing call to action:
- Pick one candidate — either a citizen-coded prototype that has outgrown Tier 2, or a bounded legacy service that needs modernising.
- In days we extract the Emery specification, compile and verify the Wasm guest, and deploy it on an Omnia host recipe alongside your team.
-->
