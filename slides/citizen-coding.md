---
theme: default
title: Code is cheap. Delivery is hard.
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<h6 class="slide-eyebrow">PROPELLERHEAD · SPEC-DRIVEN DELIVERY <span class="eyebrow-num">01 / 07</span></h6>

# Code is cheap.<br>Delivery is hard.

<div class="hero-body">
<p class="hero-subheadline">How <strong>Emery</strong> and <strong>Omnia</strong> turn citizen-coded prototypes and legacy systems into verified production software.</p>
<div class="phl-tagline-box">
<p>Every team now has access to tools that generate working code in minutes. <strong>Writing code is no longer the bottleneck.</strong></p>
<p>The hard problem is what comes next: graduating the prototypes your business comes to rely on — and modernising the legacy systems you already own — <strong>without inheriting opaque code or losing business rules.</strong></p>
</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>The Codex · Tier 1 governed sandbox</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Emery · Durable specifications &amp; verified waves</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Omnia · Capability-scoped WebAssembly runtime</span></div>
</div>

<!--
Opening framing:
- Start with the shift everyone in the room has already felt: generating code used to be expensive and slow; today anyone with a prompt can produce a working screen or script in minutes.
- That hasn't made software delivery easy — it has moved the bottleneck from writing code to governing, integrating, and owning what gets built.
- This deck is the technical companion to the Trust Model: how we use The Codex, Emery, and Omnia to keep experiments safe at Tiers 1 and 2, and rebuild proven winners and legacy services cleanly at Tier 3.
-->

---
title: Give your domain experts a safe lane
class: slide-light
---

<h6 class="slide-eyebrow">PROPELLERHEAD · THE TRUST MODEL IN PRACTICE <span class="eyebrow-num">02 / 07</span></h6>

<div class="ed-split">
<div class="ed-prose-col">

# Give domain experts<br>a safe lane.

<p class="ed-prose">Operations, finance, and product teams already use AI to solve real problems. Blocking them drives shadow IT onto personal laptops (<strong>Tier 0</strong>); waving everything into production creates unmaintainable risk.</p>
<p class="ed-prose">Our answer is a proportionate <strong>Trust Model</strong>: let frontline staff explore freely inside <strong>The Codex</strong> at <strong>Tier 1</strong> (~75% of apps), connect department tools to live data through <strong>Omnia</strong> contracts at <strong>Tier 2</strong> (~20%), and invest senior engineering only in the <strong>~5% of proven winners</strong> that graduate to <strong>Tier 3</strong>.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Upper Bad Path: Lock it down -->
  <path d="M 36 86 L 604 86" fill="none" stroke="rgba(180,35,78,0.35)" stroke-width="1.5" stroke-dasharray="4 4" />
  <circle cx="36" cy="86" r="4.5" fill="#b4234e" />
  <text x="52" y="70" class="svg-kicker" fill="#b4234e">FALSE CHOICE 01 · BAN OR QUEUE IT</text>
  <text x="52" y="110" class="svg-body" fill="rgba(13,16,48,0.65)">Builders hide on personal laptops at Tier 0 → Shadow IT grows unseen</text>
  <!-- Lower Bad Path: Wave it into production -->
  <path d="M 36 354 L 604 354" fill="none" stroke="rgba(180,35,78,0.35)" stroke-width="1.5" stroke-dasharray="4 4" />
  <circle cx="36" cy="354" r="4.5" fill="#b4234e" />
  <text x="52" y="338" class="svg-kicker" fill="#b4234e">FALSE CHOICE 02 · TRUST THE PROTOTYPE IN PRODUCTION</text>
  <text x="52" y="378" class="svg-body" fill="rgba(13,16,48,0.65)">Raw database keys &amp; brittle prompt code land in live operations</text>
  <!-- Center Safe Lane: The Propellerhead Trust Model -->
  <rect x="20" y="146" width="600" height="142" rx="16" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="44" y="174" class="svg-kicker" fill="#0c7fa3">THE PROPELLERHEAD SAFE LANE · EARNED TRUST</text>
  <line x1="96" y1="222" x2="544" y2="222" stroke="#0d1030" stroke-width="2" />
  <circle cx="96" cy="222" r="24" fill="#ffffff" stroke="#0c7fa3" stroke-width="2" />
  <text x="96" y="227" text-anchor="middle" class="svg-label" fill="#0c7fa3">T1</text>
  <text x="96" y="266" text-anchor="middle" class="svg-caption" fill="#0d1030">The Codex (~75%)</text>
  <circle cx="320" cy="222" r="24" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="320" y="227" text-anchor="middle" class="svg-label" fill="#0d1030">T2</text>
  <text x="320" y="266" text-anchor="middle" class="svg-caption" fill="#0d1030">Omnia Contracts (~20%)</text>
  <circle cx="544" cy="222" r="24" fill="#a4238f" />
  <text x="544" y="227" text-anchor="middle" class="svg-label" fill="#ffffff">T3</text>
  <text x="544" y="266" text-anchor="middle" class="svg-caption" fill="#a4238f">Emery + Omnia SLA (~5%)</text>
</svg>
</div>
</div>

<!--
Key message:
- Every organisation currently faces a false binary with citizen coding: either block it via policy (which drives it underground at Tier 0) or look the other way (which creates a wave of unmaintainable shadow IT).
- Our safe lane maps directly to the Trust Model: Tier 1 in The Codex for team trials, Tier 2 on Omnia for live data by contract, and Tier 3 with Emery + Omnia for the 5% of winners that become critical.
-->

---
title: One engineering problem, two disguises
class: slide-light
---

<h6 class="slide-eyebrow">PROPELLERHEAD · THE REWRITE TAX <span class="eyebrow-num">03 / 07</span></h6>

<div class="ed-band">
<div class="ed-band-top">
<div>

# One problem,<br>two disguises.

</div>
<p class="ed-band-prose">A nine-month vibe-coded prototype that operations now depend on and a twenty-year-old legacy service that nobody dares touch fail in the <strong>exact same four ways</strong>. In both cases, critical business behaviour is trapped inside code that engineering cannot safely maintain.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 355" aria-hidden="true">
  <!-- Two Left Sources converging on the same 4 failure modes -->
  <circle cx="96" cy="108" r="54" fill="rgba(255,59,107,0.06)" stroke="#b4234e" stroke-width="1.8" stroke-dasharray="5 4" />
  <text x="96" y="96" text-anchor="middle" class="svg-kicker" fill="#b4234e">SOURCE 01</text>
  <text x="96" y="116" text-anchor="middle" class="svg-label" fill="#0d1030">Succeeded</text>
  <text x="96" y="132" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">AI Prototype</text>
  <circle cx="96" cy="252" r="54" fill="rgba(13,16,48,0.04)" stroke="#0d1030" stroke-width="1.8" />
  <text x="96" y="240" text-anchor="middle" class="svg-kicker" fill="#0d1030">SOURCE 02</text>
  <text x="96" y="260" text-anchor="middle" class="svg-label" fill="#0d1030">20-Year-Old</text>
  <text x="96" y="276" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">Legacy Service</text>
  <!-- Converging node -->
  <path d="M 150 108 C 220 108, 230 180, 290 180" fill="none" stroke="#0d1030" stroke-width="1.8" />
  <path d="M 150 252 C 220 252, 230 180, 290 180" fill="none" stroke="#0d1030" stroke-width="1.8" />
  <circle cx="290" cy="180" r="6" fill="#b4234e" />
  <!-- Four Horizontal Failure Branches -->
  <path d="M 290 180 C 370 180, 380 48, 460 48" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" />
  <path d="M 290 180 C 370 180, 380 136, 460 136" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" />
  <path d="M 290 180 C 370 180, 380 224, 460 224" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" />
  <path d="M 290 180 C 370 180, 380 312, 460 312" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" />
  <circle cx="460" cy="48" r="4.5" fill="#b4234e" />
  <text x="480" y="42" class="svg-kicker" fill="#b4234e">01 · CONTINUITY · ORPHANED OWNERSHIP</text>
  <text x="480" y="62" class="svg-h3" fill="#0d1030">The author moved on — and nobody else knows how to fix it safely when it breaks</text>
  <circle cx="460" cy="136" r="4.5" fill="#b4234e" />
  <text x="480" y="130" class="svg-kicker" fill="#b4234e">02 · INTENT · EVAPORATED KNOWLEDGE</text>
  <text x="480" y="150" class="svg-h3" fill="#0d1030">Chat logs and old commits don't separate deliberate business rules from accidents</text>
  <circle cx="460" cy="224" r="4.5" fill="#b4234e" />
  <text x="480" y="218" class="svg-kicker" fill="#b4234e">03 · SECURITY · AMBIENT AUTHORITY</text>
  <text x="480" y="238" class="svg-h3" fill="#0d1030">Broad database credentials, unvetted packages, and un-audited network reach</text>
  <circle cx="460" cy="312" r="4.5" fill="#b4234e" />
  <text x="480" y="306" class="svg-kicker" fill="#b4234e">04 · ECONOMICS · THE REWRITE TAX</text>
  <text x="480" y="326" class="svg-h3" fill="#0d1030">Paying twice: once to discover what the business needs, and again for an emergency rescue</text>
</svg>
</div>
</div>

<!--
Key message:
- Why do we treat citizen-coding graduation (Tier 3) and legacy modernisation with the exact same platform?
- Because a 9-month prompt-built tool and a 20-year-old service fail in the exact same four ways: orphaned ownership, evaporated intent, ambient credentials, and the rewrite tax.
-->

---
title: Give people and agents clear boundaries
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<h6 class="slide-eyebrow">PROPELLERHEAD · TIERS 1 &amp; 2 ARCHITECTURE <span class="eyebrow-num">04 / 07</span></h6>

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Tiers 1 &amp; 2 · The Codex + Omnia</span>

# Design the boundary, not a review committee.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Tier 1 · The Codex</span><em>Cleared to share</em></div>
<p>Instead of policing prompts, we encode your architecture rules in the AI editor and enforce secret scans, dependency audits, and tests on every push.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>Tier 2 · Omnia Data Plane</span><em>Cleared for live data</em></div>
<p>When a team needs live data, they never receive a database credential. <strong>Omnia</strong> grants scoped, audited capabilities under the caller's identity. <span class="enabler"><img src="/images/omnia-icon.svg" alt="" />Omnia</span></p>
</div>
</div>
</div>

<div class="ed-diagram-pane">
<svg viewBox="0 0 700 410" aria-hidden="true">
  <!-- Left Input: Builder + AI -->
  <circle cx="78" cy="205" r="52" fill="rgba(10,7,16,0.55)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="78" y="192" text-anchor="middle" class="svg-kicker" fill="#78d8f2">BUILDER</text>
  <text x="78" y="212" text-anchor="middle" class="svg-label" fill="#ffffff">Domain Expert</text>
  <text x="78" y="228" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.62)">+ Approved AI</text>
  <line x1="130" y1="205" x2="204" y2="205" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="198,200 208,205 198,210" fill="#78d8f2" />
  <!-- Middle Plane: The Codex (T1) -->
  <rect x="212" y="52" width="216" height="306" rx="14" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.24)" stroke-width="1.5" />
  <text x="320" y="82" text-anchor="middle" class="svg-kicker" fill="#78d8f2">TIER 1 · THE CODEX</text>
  <line x1="236" y1="98" x2="404" y2="98" stroke="rgba(255,255,255,0.14)" />
  <text x="320" y="128" text-anchor="middle" class="svg-label" fill="#ffffff">Encoded Editor Rules</text>
  <text x="320" y="146" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Starter template &amp; harness</text>
  <text x="320" y="196" text-anchor="middle" class="svg-label" fill="#ffffff">Deterministic CI Gates</text>
  <text x="320" y="214" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Secrets, CVEs &amp; tests</text>
  <text x="320" y="264" text-anchor="middle" class="svg-label" fill="#ffffff">Isolated Team Sandbox</text>
  <text x="320" y="282" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">~75% of apps stay here</text>
  <!-- Connector T1 -> T2 -->
  <line x1="428" y1="205" x2="502" y2="205" stroke="#f59ef5" stroke-width="1.8" />
  <polygon points="496,200 506,205 496,210" fill="#f59ef5" />
  <text x="465" y="190" text-anchor="middle" class="svg-pill" fill="#f59ef5">CONTRACT</text>
  <!-- Right Plane: Omnia Capability Sandbox (T2) -->
  <rect x="510" y="88" width="174" height="234" rx="14" fill="rgba(255,255,255,0.12)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="597" y="118" text-anchor="middle" class="svg-kicker" fill="#f59ef5">TIER 2 · OMNIA</text>
  <line x1="532" y1="134" x2="662" y2="134" stroke="rgba(255,255,255,0.16)" />
  <text x="597" y="166" text-anchor="middle" class="svg-label" fill="#ffffff">Scoped Data Plane</text>
  <text x="597" y="184" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Exact rows &amp; actions</text>
  <text x="597" y="226" text-anchor="middle" class="svg-label" fill="#ffffff">Zero Raw Keys</text>
  <text x="597" y="244" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Audited caller identity</text>
  <text x="597" y="292" text-anchor="middle" class="svg-caption" fill="#78d8f2">~20% of apps</text>
</svg>
</div>
</div>

<!--
Key message:
- Governance doesn't work when it's a PDF policy document that people have to remember to follow. It works when it's built into the workspace and the runtime.
- At Tier 1, The Codex gives builders an approved AI editor, a pre-wired template, and automated CI gates running in an isolated sandbox.
- At Tier 2, Omnia connects proven department tools to live data through scoped capability contracts rather than raw database credentials.
-->

---
title: A repeatable path from prototype to production
class: slide-light
---

<h6 class="slide-eyebrow">PROPELLERHEAD · TIER 3 GRADUATION LOOP <span class="eyebrow-num">05 / 07</span></h6>

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Circular Loop Track -->
  <circle cx="320" cy="220" r="142" fill="none" stroke="rgba(13,16,48,0.14)" stroke-width="2" stroke-dasharray="5 5" />
  <!-- Curved directional arcs around the loop -->
  <path d="M 368 86 A 142 142 0 0 1 454 172" fill="none" stroke="#0c7fa3" stroke-width="2" />
  <polygon points="449,170 456,178 459,168" fill="#0c7fa3" />
  <path d="M 454 268 A 142 142 0 0 1 368 354" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="370,349 362,356 372,359" fill="#a4238f" />
  <path d="M 272 354 A 142 142 0 0 1 186 268" fill="none" stroke="#0d1030" stroke-width="2" />
  <polygon points="191,270 184,262 181,272" fill="#0d1030" />
  <path d="M 186 172 A 142 142 0 0 1 272 86" fill="none" stroke="rgba(13,16,48,0.35)" stroke-width="1.8" stroke-dasharray="4 4" />
  <!-- Center Core Thesis -->
  <text x="320" y="212" text-anchor="middle" class="svg-kicker" fill="#a4238f">TIER 3 · GRADUATION</text>
  <text x="320" y="234" text-anchor="middle" class="svg-h3" fill="#0d1030">Keep the criteria.</text>
  <text x="320" y="254" text-anchor="middle" class="svg-h3" fill="#0d1030">Drop the code.</text>
  <!-- Top Node (12 o'clock): 01 Explore -->
  <circle cx="320" cy="78" r="46" fill="#ffffff" stroke="#0c7fa3" stroke-width="2" />
  <text x="320" y="68" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · T1 / T2</text>
  <text x="320" y="88" text-anchor="middle" class="svg-h3" fill="#0d1030">Explore</text>
  <text x="320" y="18" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">Validate value in The Codex</text>
  <!-- Right Node (3 o'clock): 02 Specify (Emery) -->
  <circle cx="462" cy="220" r="48" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="462" y="210" text-anchor="middle" class="svg-kicker" fill="#a4238f">02 · EMERY</text>
  <text x="462" y="230" text-anchor="middle" class="svg-h3" fill="#0d1030">Specify</text>
  <text x="524" y="214" class="svg-label" fill="#0d1030">Extract intent</text>
  <text x="524" y="230" class="svg-caption" fill="rgba(13,16,48,0.58)">Rules, design &amp; scenarios</text>
  <!-- Bottom Node (6 o'clock): 03 Build (Emery + Engineers) -->
  <circle cx="320" cy="362" r="48" fill="#0d1030" />
  <text x="320" y="352" text-anchor="middle" class="svg-kicker" fill="#78d8f2">03 · EMERY</text>
  <text x="320" y="372" text-anchor="middle" class="svg-h3" fill="#ffffff">Build</text>
  <text x="320" y="428" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">Rebuilt in verified waves with senior engineers</text>
  <!-- Left Node (9 o'clock): 04 Run (Omnia) -->
  <circle cx="178" cy="220" r="48" fill="#a4238f" />
  <text x="178" y="210" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.85)">04 · OMNIA</text>
  <text x="178" y="230" text-anchor="middle" class="svg-h3" fill="#ffffff">Run</text>
  <text x="116" y="214" text-anchor="end" class="svg-label" fill="#0d1030">Wasm sandbox</text>
  <text x="116" y="230" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.58)">Explicit capability grants</text>
</svg>
</div>
<div class="ed-prose-col">

# How a proven winner<br>graduates to Tier 3.

<p class="ed-prose">When the one prototype in twenty becomes mission-critical, we neither harden its prompt-generated code in place nor start over from a blank page.</p>
<p class="ed-prose"><strong>Emery</strong> extracts the business rules, scenarios, and design from the working prototype into a versioned specification, then slices and rebuilds it in verified waves alongside senior engineers. <strong>Omnia</strong> runs the resulting WebAssembly component inside strict capability boundaries under an engineering SLA.</p>
</div>
</div>

<!--
Key message:
- How does the 5% of prototypes that reach Tier 3 actually become production software without starting from scratch?
- Follow the four-step loop clockwise:
  1. Explore at Tiers 1 & 2: validate the workflow with real users in The Codex.
  2. Specify with Emery: extract the requirements, edge cases, and contracts into a human-readable, versioned specification.
  3. Build with Emery + Senior Engineers: rebuild against that specification in verified waves.
  4. Run on Omnia: deploy as a capability-governed WebAssembly component with zero ambient authority.
-->

---
title: AI makes rewriting cheap enough to be dangerous
class: slide-light
---

<h6 class="slide-eyebrow">PROPELLERHEAD · LEGACY MODERNISATION <span class="eyebrow-num">06 / 07</span></h6>

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <!-- Left Node: Legacy System -->
  <circle cx="92" cy="145" r="54" fill="rgba(13,16,48,0.04)" stroke="#0d1030" stroke-width="2" />
  <text x="92" y="132" text-anchor="middle" class="svg-kicker" fill="#0d1030">LEGACY</text>
  <text x="92" y="152" text-anchor="middle" class="svg-label" fill="#0d1030">20-Year Code</text>
  <text x="92" y="168" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Rules + debt mixed</text>
  <!-- Top Trap Path: Naive 1:1 LLM Translation -->
  <path d="M 146 122 C 230 122, 250 42, 360 42 L 680 42" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="674" y1="34" x2="688" y2="50" stroke="#b4234e" stroke-width="2" />
  <line x1="688" y1="34" x2="674" y2="50" stroke="#b4234e" stroke-width="2" />
  <text x="370" y="26" class="svg-kicker" fill="#b4234e">THE TRAP · NAIVE 1:1 LLM TRANSLATION ("TRANSLATE THIS REPO")</text>
  <text x="706" y="46" class="svg-body" fill="rgba(13,16,48,0.62)">Copies 20 years of accidental plumbing into a new language &amp; silently drops edge cases</text>
  <!-- Lower Winning Path: 01 Recover Intent -> 02 Re-express -> 03 Verify & Replace -->
  <path d="M 146 162 C 210 162, 220 182, 286 182" fill="none" stroke="#0c7fa3" stroke-width="2.2" />
  <polygon points="280,177 290,182 280,187" fill="#0c7fa3" />
  <!-- Step 01: Recover Intent -->
  <circle cx="344" cy="182" r="52" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="2" />
  <text x="344" y="168" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · EMERY</text>
  <text x="344" y="188" text-anchor="middle" class="svg-label" fill="#0d1030">Recover Intent</text>
  <text x="344" y="254" text-anchor="middle" class="svg-body" fill="#0d1030">Platform-neutral spec</text>
  <text x="344" y="270" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Sheds old framework noise</text>
  <line x1="396" y1="182" x2="588" y2="182" stroke="#0d1030" stroke-width="2.2" />
  <polygon points="582,177 592,182 582,187" fill="#0d1030" />
  <text x="492" y="168" text-anchor="middle" class="svg-caption" fill="#0d1030">Pure domain rules</text>
  <!-- Step 02: Re-express in Waves -->
  <circle cx="646" cy="182" r="52" fill="#0d1030" />
  <text x="646" y="168" text-anchor="middle" class="svg-kicker" fill="#78d8f2">02 · EMERY</text>
  <text x="646" y="188" text-anchor="middle" class="svg-label" fill="#ffffff">Re-Express</text>
  <text x="646" y="254" text-anchor="middle" class="svg-body" fill="#0d1030">Build slice by slice</text>
  <text x="646" y="270" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Leave legacy lock-in behind</text>
  <line x1="698" y1="182" x2="890" y2="182" stroke="#a4238f" stroke-width="2.2" />
  <polygon points="884,177 894,182 884,187" fill="#a4238f" />
  <text x="794" y="168" text-anchor="middle" class="svg-caption" fill="#a4238f">Replay recorded traffic</text>
  <!-- Step 03: Replace Safely on Omnia -->
  <circle cx="952" cy="182" r="56" fill="#a4238f" />
  <text x="952" y="166" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.85)">03 · OMNIA</text>
  <text x="952" y="186" text-anchor="middle" class="svg-label" fill="#ffffff">Replace Safely</text>
  <text x="952" y="202" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.85)">Verified parity</text>
  <text x="952" y="254" text-anchor="middle" class="svg-body" fill="#0d1030">Proven against live fixtures</text>
  <text x="952" y="270" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Before retiring the old system</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# AI makes rewriting<br>cheap enough to be<br>dangerous.

</div>
<div>
<p class="ed-prose">When teams point an LLM directly at a legacy codebase and say <em>"translate this"</em>, they copy twenty years of accidental complexity into a brand-new language — and silently drop the edge cases that actually matter.</p>
<p class="ed-prose">The exact same <strong>Emery + Omnia</strong> discipline that graduates a Tier 3 prototype solves legacy modernisation: <strong>separate what the system must do from how the old platform happened to do it</strong>, and prove behavioural parity slice by slice before cutover.</p>
</div>
<div></div>
</div>
</div>

<!--
Key message:
- The exact same discipline that graduates a Tier 3 prototype also solves one of the biggest traps in enterprise AI right now: naive legacy translation.
- Pointing an LLM at an old codebase and asking for a 1:1 translation just ports technical debt into a new syntax while losing subtle business rules.
- With Emery and Omnia, we recover the platform-neutral specification first, re-express cleanly, and verify against recorded traffic before retiring the legacy service.
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
<h6 class="slide-eyebrow">PROPELLERHEAD · NEXT STEPS <span class="eyebrow-num">07 / 07</span></h6>

# Start small.<br>Prove the loop.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Pick one proven prototype or legacy service</h4>
<p>Choose a candidate with clear business value — either a Tier 2 citizen-coded tool ready to graduate or a bounded legacy component.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Extract the specification with Emery in days</h4>
<p>We mine the working rules, contracts, and test scenarios into a versioned specification your domain owners and engineers can read together.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Rebuild in verified waves on Omnia</h4>
<p>You finish with a clean, capability-sandboxed production service, verified requirement by requirement, and a repeatable pattern your team owns.</p>
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
- You don't need a six-month programme to test this.
- Pick one candidate — either a citizen-coded prototype that has outgrown Tier 2, or a bounded legacy service that needs modernising.
- In days we extract the Emery specification, run the verified build loop onto Omnia alongside your engineers, and leave you with working production software and a durable specification you own.
-->
