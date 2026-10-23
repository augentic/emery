---
theme: default
title: Explicit Intent · Bounded Authority · Repeatable Infrastructure
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · DELIVERY ARCHITECTURE" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Explicit intent.<br>Bounded authority.<br>Repeatable infrastructure.

<p class="section-subtitle">The Trust Model, expressed in architecture: a WASI runtime bounds what code may touch, service recipes standardise how platforms run, and spec-driven development keeps business intent durable.</p>
</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>WASI runtime · What application code may touch</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Service recipes · How the platform is provisioned and run</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Spec-driven development · What must remain true</span></div>
</div>

<!--
Opening framing:
- The Trust Model says when an application earns more authority and ownership. This deck explains how the architecture enforces that worldview.
- The WASI runtime separates application logic from infrastructure authority.
- Service recipes combine the runtime with Terraform so the customer chooses an outcome and constraints, not a bespoke topology.
- Spec-driven development separates durable intent from replaceable implementation.
-->

---
title: Born from the Trust Model
class: slide-light
---

<Eyebrow text="THE ORIGIN · FROM WORLDVIEW TO ARCHITECTURE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Three questions,<br>three layers.

</div>
<p class="ed-band-prose">As an application grows beyond a team trial in <strong>The Codex</strong>, engineering must answer three questions: <strong>what may this code touch</strong>, <strong>why rebuild the same platform for every app</strong>, and <strong>what must remain true when it becomes critical?</strong> One architectural layer answers each.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 375" aria-hidden="true">
  <!-- Top Tier Progression Spine -->
  <line x1="90" y1="46" x2="1030" y2="46" stroke="rgba(13,16,48,0.16)" stroke-width="2" />

  <circle cx="90" cy="46" r="20" fill="#ffffff" stroke="#b4234e" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="90" y="50" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0</text>
  <text x="90" y="84" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">1 laptop · Unmanaged</text>

  <circle cx="360" cy="46" r="22" fill="#ffffff" stroke="#0c7fa3" stroke-width="2" />
  <text x="360" y="50" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T1</text>
  <text x="360" y="84" text-anchor="middle" class="svg-caption" fill="#0c7fa3">The Codex · Governed (~75%)</text>

  <circle cx="670" cy="46" r="24" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="2.5" />
  <text x="670" y="51" text-anchor="middle" class="svg-label" fill="#0c7fa3">T2</text>
  <text x="670" y="84" text-anchor="middle" class="svg-label" fill="#0d1030">Connected (~20%)</text>

  <circle cx="1030" cy="46" r="26" fill="#a4238f" />
  <text x="1030" y="51" text-anchor="middle" class="svg-label" fill="#ffffff">T3</text>
  <text x="1030" y="84" text-anchor="middle" class="svg-label" fill="#a4238f">Supported (~5%)</text>

  <!-- Vertical Connectors down to the architectural layers -->
  <line x1="360" y1="96" x2="360" y2="126" stroke="rgba(12,127,163,0.4)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="670" y1="96" x2="670" y2="306" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="1030" y1="96" x2="1030" y2="278" stroke="#a4238f" stroke-width="1.5" stroke-dasharray="4 4" />

  <!-- Foundation at T1: The Codex -->
  <rect x="210" y="126" width="280" height="66" rx="12" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.35)" stroke-width="1.5" />
  <text x="232" y="150" class="svg-kicker" fill="#0c7fa3">WORKSPACE · TIER 1+</text>
  <text x="232" y="172" class="svg-label" fill="#0d1030">The Codex: template, git &amp; CI gates</text>

  <!-- Layer 01: WASI Runtime (Spans Tier 2 and Tier 3) -->
  <rect x="520" y="114" width="600" height="72" rx="12" fill="#0d1030" />
  <text x="544" y="138" class="svg-kicker" fill="#78d8f2">01 · WASI RUNTIME · TIERS 2 &amp; 3</text>
  <text x="544" y="160" class="svg-h3" fill="#ffffff">What may application code touch?</text>
  <text x="850" y="160" class="svg-caption" fill="rgba(255,255,255,0.72)">Explicit capabilities · Credentials stay outside</text>

  <!-- Layer 02: Service Recipes (Spans Tier 2 and Tier 3) -->
  <rect x="520" y="198" width="600" height="72" rx="12" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="544" y="222" class="svg-kicker" fill="#0c7fa3">02 · SERVICE RECIPES · TIERS 2 &amp; 3</text>
  <text x="544" y="244" class="svg-h3" fill="#0d1030">Why rebuild the same platform per app?</text>
  <text x="850" y="244" class="svg-caption" fill="rgba(13,16,48,0.68)">Terraform + runtime · S / M / L service envelopes</text>

  <!-- Layer 03: Spec-Driven Development (Drives T2 -> T3 graduation) -->
  <path d="M 670 322 L 778 322" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="772,317 782,322 772,327" fill="#a4238f" />
  <circle cx="670" cy="322" r="4.5" fill="#a4238f" />
  <text x="654" y="316" text-anchor="end" class="svg-kicker" fill="#a4238f">T2 PROTOTYPE OR LEGACY REPO</text>
  <text x="654" y="334" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.58)">Rules trapped in brittle code</text>

  <rect x="788" y="284" width="332" height="78" rx="12" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="812" y="308" class="svg-kicker" fill="#a4238f">03 · SPEC-DRIVEN DEVELOPMENT · TIER 3</text>
  <text x="812" y="330" class="svg-h3" fill="#0d1030">What must remain true?</text>
  <text x="812" y="348" class="svg-caption" fill="rgba(13,16,48,0.68)">Recovered spec → verified wave rebuild</text>
</svg>
</div>
</div>

<!--
How the architecture follows from the Trust Model:
- Tier 1 needs a governed workspace (The Codex) because nothing live is reachable.
- Tier 2 needs a structural boundary (WASI runtime) between variable application code and live systems.
- Tiers 2 and 3 share repeatable Terraform + runtime service recipes instead of bespoke cloud builds.
- Tier 3 needs durable intent (spec-driven development), explicit promotion, and accountable operation.
-->

---
title: WebAssembly Runtime
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART ONE · BOUNDED RUNTIME AUTHORITY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# WebAssembly runtime

<p class="section-subtitle">Portable WASI components, explicit capability contracts, and infrastructure policy kept outside application code.</p>
</div>
<div class="section-big-glyph">WASI</div>
</div>

<TrustLadder variant="strip" :current="[2, 3]" />

<!--
Part One: Three slides on bounded runtime authority and repeatable recipes (Tiers 2 and 3).
1. WebAssembly (WASI) as a structural boundary for variable code.
2. Capability interfaces separate applications from provider plumbing.
3. The runtime and Terraform-backed recipes turn infrastructure into a repeatable service.
-->

---
title: WebAssembly · Bounded Execution
class: slide-light
---

<Eyebrow text="RUNTIME 01 / 03 · BOUNDED EXECUTION" />

<div class="ed-split">
<div class="ed-prose-col">

# Policy lives outside<br>the application.

<p class="ed-prose">When code is written by engineers, domain experts, and AI, assurance cannot depend on the code policing itself.</p>
<p class="ed-prose"><strong>The WASI runtime hosts logic as an isolated component.</strong> The component declares the capability it needs; the host holds credentials, selects the backend, and enforces network and data policy outside the code.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Left Comparison: Application-Owned Policy -->
  <rect x="16" y="44" width="232" height="352" rx="14" fill="rgba(255,59,107,0.04)" stroke="rgba(180,35,78,0.35)" stroke-width="1.5" stroke-dasharray="5 4" />
  <text x="132" y="74" text-anchor="middle" class="svg-kicker" fill="#b4234e">POLICY IN EACH APPLICATION</text>
  <text x="132" y="96" text-anchor="middle" class="svg-h3" fill="#0d1030">Repeated Trust Logic</text>
  <line x1="40" y1="112" x2="224" y2="112" stroke="rgba(180,35,78,0.2)" />

  <circle cx="44" cy="148" r="4" fill="#b4234e" />
  <text x="58" y="152" class="svg-label" fill="#0d1030">Credentials in process</text>
  <text x="58" y="168" class="svg-caption" fill="rgba(13,16,48,0.58)">Each app handles raw secrets</text>

  <circle cx="44" cy="212" r="4" fill="#b4234e" />
  <text x="58" y="216" class="svg-label" fill="#0d1030">Provider-specific SDKs</text>
  <text x="58" y="232" class="svg-caption" fill="rgba(13,16,48,0.58)">Logic absorbs cloud plumbing</text>

  <circle cx="44" cy="276" r="4" fill="#b4234e" />
  <text x="58" y="280" class="svg-label" fill="#0d1030">Local enforcement</text>
  <text x="58" y="296" class="svg-caption" fill="rgba(13,16,48,0.58)">Boundaries rebuilt per team</text>

  <text x="132" y="366" text-anchor="middle" class="svg-caption" fill="#b4234e">Authorship is mistaken for assurance</text>

  <!-- Right Architecture: WebAssembly Capability Boundary -->
  <rect x="284" y="44" width="340" height="352" rx="14" fill="#0d1030" />
  <text x="454" y="74" text-anchor="middle" class="svg-kicker" fill="#78d8f2">WASI RUNTIME HOST · CAPABILITY BOUNDARY</text>

  <!-- Inner Wasm Guest Box -->
  <rect x="312" y="96" width="284" height="112" rx="10" fill="rgba(255,255,255,0.08)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="454" y="124" text-anchor="middle" class="svg-kicker" fill="#78d8f2">WASI APPLICATION COMPONENT</text>
  <text x="454" y="148" text-anchor="middle" class="svg-h3" fill="#ffffff">Differentiating Logic</text>
  <text x="454" y="170" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">Portable sandbox · Explicit WIT interfaces</text>
  <text x="454" y="190" text-anchor="middle" class="svg-caption" fill="#78d8f2">Zero infrastructure credentials in code</text>

  <!-- Explicit WIT Capability Links -->
  <line x1="368" y1="208" x2="368" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="363,252 373,252 368,260" fill="#78d8f2" />
  <line x1="454" y1="208" x2="454" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="449,252 459,252 454,260" fill="#78d8f2" />
  <line x1="540" y1="208" x2="540" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="535,252 545,252 540,260" fill="#78d8f2" />
  <text x="454" y="240" text-anchor="middle" class="svg-pill" fill="#f59ef5">HOST-MEDIATED CAPABILITY INTERFACES</text>

  <!-- Host-Managed Capability Backends -->
  <rect x="312" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="354" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">DATA</text>
  <text x="354" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Chosen backend</text>

  <rect x="412" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="454" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">MESSAGING</text>
  <text x="454" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Managed routing</text>

  <rect x="512" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="554" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">HTTP</text>
  <text x="554" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Network policy</text>

  <text x="454" y="366" text-anchor="middle" class="svg-caption" fill="#78d8f2">Host retains credentials, infrastructure choices &amp; policy</text>
</svg>
</div>
</div>

<!--
Runtime slide 1 of 3 — a structural boundary for variable code:
- Generated and citizen-authored code is treated as untrusted input.
- Authorship does not create authority; the host does.
- The component names the capability it needs, while the host chooses the implementation and holds infrastructure credentials.
-->

---
title: WebAssembly · Infrastructure Portability
class: slide-light
---

<Eyebrow text="RUNTIME 02 / 03 · INFRASTRUCTURE ABSTRACTION & PORTABILITY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Name the role.<br>Bind it later.

</div>
<p class="ed-band-prose">Application code asks for durable roles — state, messaging, identity, secrets, outbound calls — through stable WASI capability interfaces. The environment recipe binds those interfaces to local doubles, managed cloud services, or sovereign infrastructure without changing the component.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 355" aria-hidden="true">
  <!-- Top Layer: Portable Guest Component -->
  <rect x="290" y="12" width="560" height="72" rx="12" fill="#0d1030" />
  <text x="570" y="38" text-anchor="middle" class="svg-kicker" fill="#78d8f2">PORTABLE WASI COMPONENT</text>
  <text x="570" y="62" text-anchor="middle" class="svg-h3" fill="#ffffff">Business Behaviour — Zero Provider Plumbing</text>

  <!-- Middle Layer: Capability Contracts -->
  <line x1="570" y1="84" x2="570" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <line x1="80" y1="124" x2="1060" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <rect x="380" y="108" width="380" height="28" rx="14" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="570" y="126" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">STABLE WASI CAPABILITY INTERFACES</text>

  <!-- Three Vertical Branches to Interchangeable Host Environments -->
  <line x1="195" y1="124" x2="195" y2="174" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="190,168 200,168 195,176" fill="#0c7fa3" />
  <line x1="570" y1="136" x2="570" y2="174" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="565,168 575,168 570,176" fill="#0c7fa3" />
  <line x1="945" y1="124" x2="945" y2="174" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="940,168 950,168 945,176" fill="#a4238f" />

  <!-- Environment 1: Development Recipe -->
  <rect x="30" y="180" width="330" height="156" rx="14" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.3)" stroke-width="1.5" />
  <text x="54" y="208" class="svg-kicker" fill="#0c7fa3">01 · DEVELOPMENT RECIPE</text>
  <text x="54" y="232" class="svg-h3" fill="#0d1030">Fast Local Feedback</text>
  <text x="54" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• In-memory data and state</text>
  <text x="54" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• Local messaging and test doubles</text>
  <text x="54" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Identical capability interfaces</text>
  <text x="54" y="322" text-anchor="start" class="svg-caption" fill="#0c7fa3">Optimised for iteration and testing</text>

  <!-- Environment 2: Managed Cloud Recipe -->
  <rect x="405" y="180" width="330" height="156" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.5" />
  <text x="429" y="208" class="svg-kicker" fill="#0d1030">02 · MANAGED CLOUD RECIPE</text>
  <text x="429" y="232" class="svg-h3" fill="#0d1030">Operated Cloud Services</text>
  <text x="429" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• Managed data, cache and messaging</text>
  <text x="429" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• Identity, secrets and private networking</text>
  <text x="429" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Capacity and recovery from the plan</text>
  <text x="429" y="322" class="svg-caption" fill="rgba(13,16,48,0.55)">Optimised for service outcomes</text>

  <!-- Environment 3: Sovereign Recipe -->
  <rect x="780" y="180" width="330" height="156" rx="14" fill="rgba(164,35,143,0.06)" stroke="rgba(164,35,143,0.35)" stroke-width="1.5" />
  <text x="804" y="208" class="svg-kicker" fill="#a4238f">03 · SOVEREIGN RECIPE</text>
  <text x="804" y="232" class="svg-h3" fill="#0d1030">On-Premises &amp; Edge</text>
  <text x="804" y="258" class="svg-body" fill="rgba(13,16,48,0.72)">• Approved data and messaging services</text>
  <text x="804" y="280" class="svg-body" fill="rgba(13,16,48,0.72)">• Local identity and telemetry</text>
  <text x="804" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Residency and air-gapped constraints</text>
  <text x="804" y="322" class="svg-caption" fill="#a4238f">Optimised for control and locality</text>
</svg>
</div>
</div>

<!--
Runtime slide 2 of 3 — infrastructure as a replaceable implementation:
- The application expresses stable capability needs instead of importing a provider architecture.
- The recipe layer selects the concrete services for the environment.
- This separates the long-lived business contract from infrastructure choices that will change.
-->

---
title: WebAssembly + Terraform · Service Recipes
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="RUNTIME 03 / 03 · VERSIONED SERVICE RECIPES" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Service recipes · WASI + Terraform</span>

# Choose a plan.<br>We provision it.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Plan S · M · L</span><em>Outcome contract</em></div>
<p>The customer selects capacity, resilience, recovery, residency, and connectivity — a <strong>service envelope, not a bespoke cloud diagram or VM size</strong>.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>One versioned recipe</span><em>Repeatable delivery</em></div>
<p>Terraform provisions the environment, the WASI runtime binds capabilities to it, and Propellerhead operates the result against the plan.</p>
</div>
</div>
</div>

<div class="ed-diagram-pane">
<svg viewBox="0 0 700 420" aria-hidden="true">
  <!-- Outer Service Recipe Frame -->
  <rect x="16" y="20" width="668" height="380" rx="16" fill="rgba(10,7,16,0.58)" stroke="rgba(120,216,242,0.35)" stroke-width="1.5" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">VERSIONED TERRAFORM + WASI RUNTIME RECIPE</text>

  <!-- Left: Service Envelope -->
  <rect x="42" y="74" width="144" height="224" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.2)" />
  <text x="114" y="102" text-anchor="middle" class="svg-kicker" fill="#78d8f2">SERVICE ENVELOPE</text>
  <text x="114" y="136" text-anchor="middle" class="svg-label" fill="#ffffff">Demand</text>
  <text x="114" y="154" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">Capacity · latency</text>
  <text x="114" y="196" text-anchor="middle" class="svg-label" fill="#ffffff">Continuity</text>
  <text x="114" y="214" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">Availability · recovery</text>
  <text x="114" y="256" text-anchor="middle" class="svg-label" fill="#ffffff">Constraints</text>
  <text x="114" y="274" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.6)">Residency · connectivity</text>

  <!-- Arrows from Envelope to Recipe -->
  <line x1="186" y1="112" x2="236" y2="112" stroke="#78d8f2" stroke-width="1.6" />
  <line x1="186" y1="186" x2="236" y2="186" stroke="#78d8f2" stroke-width="1.6" />
  <line x1="186" y1="260" x2="236" y2="260" stroke="#78d8f2" stroke-width="1.6" />

  <!-- Center: Application and Platform -->
  <rect x="240" y="74" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="346" y="100" text-anchor="middle" class="svg-label" fill="#ffffff">WASI component</text>
  <text x="346" y="118" text-anchor="middle" class="svg-caption" fill="#78d8f2">Differentiating behaviour</text>

  <rect x="240" y="157" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="346" y="183" text-anchor="middle" class="svg-label" fill="#ffffff">Runtime capability boundary</text>
  <text x="346" y="201" text-anchor="middle" class="svg-caption" fill="#78d8f2">Capabilities · routing · policy</text>

  <line x1="346" y1="215" x2="346" y2="240" stroke="#f59ef5" stroke-width="1.8" stroke-dasharray="3 3" />

  <rect x="240" y="240" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="346" y="266" text-anchor="middle" class="svg-label" fill="#ffffff">Terraform infrastructure recipe</text>
  <text x="346" y="284" text-anchor="middle" class="svg-caption" fill="#f59ef5">Provisioned to the plan</text>

  <!-- Right: Provisioned Resources -->
  <line x1="452" y1="112" x2="498" y2="112" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />
  <line x1="452" y1="186" x2="498" y2="186" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />
  <line x1="452" y1="260" x2="498" y2="260" stroke="rgba(255,255,255,0.35)" stroke-width="1.5" />

  <rect x="502" y="74" width="156" height="224" rx="10" fill="rgba(214,58,214,0.14)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="580" y="102" text-anchor="middle" class="svg-kicker" fill="#f59ef5">PROVISIONED STACK</text>
  <text x="580" y="136" text-anchor="middle" class="svg-label" fill="#ffffff">Data &amp; State</text>
  <text x="580" y="154" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">SQL · cache · object store</text>
  <text x="580" y="196" text-anchor="middle" class="svg-label" fill="#ffffff">Flow &amp; Network</text>
  <text x="580" y="214" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Messaging · ingress · egress</text>
  <text x="580" y="256" text-anchor="middle" class="svg-label" fill="#ffffff">Trust &amp; Operations</text>
  <text x="580" y="274" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Identity · secrets · telemetry</text>

  <!-- Bottom Operations Bar -->
  <rect x="42" y="320" width="616" height="54" rx="10" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.16)" />
  <text x="64" y="352" class="svg-kicker" fill="#78d8f2">EVERY PLAN</text>
  <text x="168" y="352" class="svg-body" fill="#ffffff">One security baseline · One versioned recipe · One accountable operator</text>
</svg>
</div>
</div>

<!--
Runtime slide 3 of 3 — repeatable service envelopes:
- Customers buy a service outcome, not a one-off infrastructure architecture.
- Small, medium, and large plans express measurable demand and constraints. Security remains the same baseline across all three.
- A versioned recipe combines Terraform-provisioned resources with the WASI runtime host and application component.
-->

---
title: Spec-Driven Development
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART TWO · DURABLE INTENT & EVIDENCE-BASED PROMOTION" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Spec-driven development

<p class="section-subtitle">Durable specifications, bounded AI implementation, explicit human review, and evidence before promotion.</p>
</div>
<div class="section-big-glyph">SPEC</div>
</div>

<TrustLadder variant="strip" :current="3" />

<!--
Part Two: Three slides on spec-driven development (Tier 3).
1. Specifications, not code, as the durable source of truth.
2. Recovering intent from citizen-built applications and legacy systems.
3. Bounded, wave-by-wave implementation and verification.
-->

---
title: Specifications · Durable Intent
class: slide-light
---

<Eyebrow text="SPECIFICATION 01 / 03 · INTENT AS THE SOURCE OF TRUTH" />

<div class="ed-split">
<div class="ed-prose-col">

# Intent is the asset.<br>Code is replaceable.

<p class="ed-prose">In both vibe-coded prototypes and legacy systems, business rules are tangled inside framework plumbing. Prompts vanish when the chat window closes.</p>
<p class="ed-prose"><strong>Spec-driven development flips the hierarchy:</strong> a versioned contract holds a technology-neutral <strong>Specification</strong>, a technology-aware <strong>Design</strong>, and a dependency-ordered <strong>Build Plan</strong> — so implementation can be cleanly rebuilt for any target.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Left Durable Core: Versioned Delivery Contract with concrete artifact lines -->
  <rect x="20" y="34" width="328" height="372" rx="16" fill="#0d1030" />
  <text x="44" y="64" class="svg-kicker" fill="#78d8f2">DURABLE SOURCE OF TRUTH · SEALED CONTRACT</text>
  <text x="44" y="88" class="svg-h3" fill="#ffffff">Versioned Delivery Revision</text>

  <!-- 01 Specification Artifact -->
  <rect x="40" y="106" width="288" height="104" rx="10" fill="rgba(255,255,255,0.09)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="56" y="128" class="svg-kicker" fill="#78d8f2">01 · SPECIFICATION (ROLE-BASED)</text>
  <text x="56" y="148" class="svg-label" fill="#ffffff">REQ-014 shifts.assign-cover</text>
  <text x="56" y="165" class="svg-caption" fill="rgba(255,255,255,0.72)">Route: POST /shifts · Given an open shift</text>
  <text x="56" y="180" class="svg-caption" fill="rgba(255,255,255,0.72)">When the lead assigns cover → Then cap at 40h</text>
  <text x="56" y="196" class="svg-caption" fill="#ff7597">Status: divergence · brief outranks prototype</text>

  <!-- 02 Design Artifact -->
  <rect x="40" y="222" width="288" height="84" rx="10" fill="rgba(255,255,255,0.07)" stroke="rgba(255,255,255,0.18)" />
  <text x="56" y="244" class="svg-kicker" fill="#f59ef5">02 · DESIGN (TECHNOLOGY &amp; SHAPES)</text>
  <text x="56" y="264" class="svg-label" fill="#ffffff">Type: ShiftAssignment (data)</text>
  <text x="56" y="281" class="svg-caption" fill="rgba(255,255,255,0.68)">Wire schemas, data types &amp; mechanisms</text>
  <text x="56" y="296" class="svg-caption" fill="rgba(255,255,255,0.68)">Reaches: roster upstream · state store · publish</text>

  <!-- 03 Build Plan Artifact -->
  <rect x="40" y="318" width="288" height="80" rx="10" fill="rgba(255,255,255,0.07)" stroke="rgba(255,255,255,0.18)" />
  <text x="56" y="340" class="svg-kicker" fill="#78d8f2">03 · BUILD PLAN (DEPENDENCY WAVES)</text>
  <text x="56" y="360" class="svg-label" fill="#ffffff">Wave 1 roster · Wave 2 shifts</text>
  <text x="56" y="378" class="svg-caption" fill="rgba(255,255,255,0.68)">Each slice owns its requirements, types &amp; tests</text>

  <!-- Projection Arrows to Replaceable Target Codebases -->
  <path d="M 348 160 C 388 160, 396 134, 432 134" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="426,129 436,134 426,139" fill="#a4238f" />
  <path d="M 348 300 C 388 300, 396 306, 432 306" fill="none" stroke="#0c7fa3" stroke-width="1.8" stroke-dasharray="4 4" />
  <polygon points="426,301 436,306 426,311" fill="#0c7fa3" />

  <!-- Primary Target: WebAssembly -->
  <rect x="440" y="76" width="184" height="118" rx="12" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="532" y="104" text-anchor="middle" class="svg-kicker" fill="#a4238f">PRIMARY TARGET · WASI</text>
  <text x="532" y="128" text-anchor="middle" class="svg-h3" fill="#0d1030">WASI Component</text>
  <text x="532" y="150" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Capability-bound guest</text>
  <text x="532" y="168" text-anchor="middle" class="svg-caption" fill="#a4238f">Preferred production target</text>

  <!-- Secondary Target: Conventional Application Stack -->
  <rect x="440" y="246" width="184" height="118" rx="12" fill="#f5f6fa" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="532" y="274" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">ALTERNATIVE TARGET</text>
  <text x="532" y="298" text-anchor="middle" class="svg-h3" fill="#0d1030">Conventional Stack</text>
  <text x="532" y="320" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Node / enterprise service</text>
  <text x="532" y="338" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.52)">Same spec, different adapter</text>
</svg>
</div>
</div>

<!--
Specification slide 1 of 3 — durable intent:
- A specification is reviewable by the people accountable for the business outcome.
- Behaviour is kept separate from technology choices; the design carries mechanisms and integrations.
- Each delivery target builds from the reviewed contract, so the model receives bounded work and explicit acceptance criteria.
-->

---
title: Specifications · Recovery
class: slide-light
---

<Eyebrow text="SPECIFICATION 02 / 03 · RECOVER INTENT FROM CITIZEN & LEGACY SYSTEMS" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Recover intent.<br>Expose the gaps.

</div>
<p class="ed-band-prose">Graduating a prototype or replacing a legacy system usually forces a bad choice: inherit brittle code nobody understands, or rewrite from a blank page and lose hard-won rules. <strong>Specification recovery is the third option</strong> — reconciling intent, docs, tests, and code under explicit source authority while keeping every conflict visible.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 355" aria-hidden="true">
  <!-- Left Column: Authoritative Inputs -->
  <text x="16" y="24" class="svg-kicker" fill="#0c7fa3">01 · AUTHORITATIVE INPUTS</text>

  <rect x="16" y="40" width="256" height="62" rx="10" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="34" y="64" class="svg-kicker" fill="#0c7fa3">RANK 1 · PRODUCT INTENT</text>
  <text x="34" y="84" class="svg-label" fill="#0d1030">Product Briefs &amp; Directives</text>

  <rect x="16" y="116" width="256" height="62" rx="10" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.5" />
  <text x="34" y="140" class="svg-kicker" fill="#0d1030">RANK 2 · DOCUMENTATION</text>
  <text x="34" y="160" class="svg-label" fill="#0d1030">Design Records, Specs &amp; Tests</text>

  <rect x="16" y="192" width="256" height="92" rx="10" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="34" y="216" class="svg-kicker" fill="#a4238f">RANK 3 · OBSERVED BEHAVIOUR</text>
  <text x="34" y="236" class="svg-label" fill="#0d1030">T2 Prototype or Legacy Code</text>
  <text x="34" y="256" class="svg-caption" fill="rgba(13,16,48,0.65)">Routes, decisions, data shapes,</text>
  <text x="34" y="272" class="svg-caption" fill="rgba(13,16,48,0.65)">and tests extracted as claims</text>

  <!-- Connectors to Reconciliation Engine -->
  <path d="M 272 71 C 330 71, 340 162, 396 162" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <line x1="272" y1="147" x2="396" y2="162" stroke="#0d1030" stroke-width="1.6" />
  <path d="M 272 238 C 330 238, 340 162, 396 162" fill="none" stroke="#a4238f" stroke-width="1.6" />

  <!-- Middle Column: Evidence & Authority Review -->
  <rect x="400" y="48" width="320" height="236" rx="14" fill="#0d1030" />
  <text x="560" y="78" text-anchor="middle" class="svg-kicker" fill="#78d8f2">02 · AUTHORITY RECONCILIATION</text>
  <line x1="428" y1="94" x2="692" y2="94" stroke="rgba(255,255,255,0.14)" />

  <circle cx="436" cy="124" r="4.5" fill="#78d8f2" />
  <text x="452" y="128" class="svg-label" fill="#ffffff">Corroborated &amp; Single-Source</text>
  <text x="452" y="146" class="svg-caption" fill="rgba(255,255,255,0.68)">Anchored to exact source lines</text>

  <circle cx="436" cy="180" r="4.5" fill="#f59ef5" />
  <text x="452" y="184" class="svg-label" fill="#ffffff">Divergence Resolved by Rank</text>
  <text x="452" y="202" class="svg-caption" fill="rgba(255,255,255,0.68)">Intent outranks accidental code habits</text>

  <circle cx="436" cy="236" r="4.5" fill="#ff7597" />
  <text x="452" y="240" class="svg-label" fill="#ffffff">Conflicts &amp; [unknown] Preserved</text>
  <text x="452" y="258" class="svg-caption" fill="rgba(255,255,255,0.68)">Never guessed or silently flattened</text>

  <!-- Connector to Output -->
  <line x1="720" y1="166" x2="796" y2="166" stroke="#a4238f" stroke-width="2.2" />
  <polygon points="790,161 800,166 790,171" fill="#a4238f" />

  <!-- Right Column: Reviewable Delivery Baseline -->
  <rect x="804" y="48" width="316" height="236" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="962" y="78" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">03 · REVIEWABLE CANDIDATE</text>
  <text x="962" y="108" text-anchor="middle" class="svg-h3" fill="#0d1030">Keep the criteria.</text>
  <text x="962" y="130" text-anchor="middle" class="svg-h3" fill="#0d1030">Drop the code.</text>
  <line x1="836" y1="148" x2="1088" y2="148" stroke="rgba(13,16,48,0.12)" />
  <text x="836" y="176" class="svg-body" fill="#0d1030">• Routes and triggers bound from code</text>
  <text x="836" y="202" class="svg-body" fill="#0d1030">• Domain data separated from plumbing</text>
  <text x="836" y="228" class="svg-body" fill="#0d1030">• Staged for human review before sealing</text>
  <text x="836" y="258" class="svg-caption" fill="#0c7fa3">Zero silent assumptions</text>

  <!-- Bottom Comparison Strip: Avoiding the Two Graduation Traps -->
  <rect x="16" y="302" width="540" height="40" rx="8" fill="rgba(255,59,107,0.05)" stroke="rgba(180,35,78,0.28)" stroke-dasharray="4 4" />
  <text x="34" y="326" class="svg-kicker" fill="#b4234e">TRAP 01 · HARDEN IN PLACE</text>
  <text x="230" y="326" class="svg-caption" fill="rgba(13,16,48,0.68)">Engineers inherit months of opaque generated code</text>

  <rect x="580" y="302" width="540" height="40" rx="8" fill="rgba(255,59,107,0.05)" stroke="rgba(180,35,78,0.28)" stroke-dasharray="4 4" />
  <text x="598" y="326" class="svg-kicker" fill="#b4234e">TRAP 02 · BLANK-PAGE REWRITE</text>
  <text x="812" y="326" class="svg-caption" fill="rgba(13,16,48,0.68)">Throws away the rules the prototype paid to discover</text>
</svg>
</div>
</div>

<!--
Specification slide 2 of 3 — specification recovery:
- A direct AI rewrite confuses historical implementation with intended behaviour.
- Evidence adapters recover structured claims from each authoritative input.
- Authority is explicit and configurable; divergence, conflict, and unknowns remain visible for human review.
-->

---
title: Specifications · Bounded Delivery
class: slide-light
---

<Eyebrow text="SPECIFICATION 03 / 03 · BOUNDED AI WORK & EVIDENCE-BASED PROMOTION" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 660 440" aria-hidden="true">
  <!-- Step 1: Wave Slicing & Deterministic Scaffold -->
  <rect x="20" y="32" width="276" height="104" rx="12" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="40" y="58" class="svg-kicker" fill="#0c7fa3">01 · DETERMINISTIC SCAFFOLD</text>
  <text x="40" y="82" class="svg-h3" fill="#0d1030">Typed Skeleton First</text>
  <text x="40" y="102" class="svg-caption" fill="rgba(13,16,48,0.65)">Approved layout, typed interfaces,</text>
  <text x="40" y="118" class="svg-caption" fill="rgba(13,16,48,0.65)">and test markers laid down by code</text>

  <line x1="296" y1="84" x2="356" y2="84" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="350,79 360,84 350,89" fill="#0c7fa3" />

  <!-- Step 2: Parallel Slice Working Copies -->
  <rect x="364" y="32" width="276" height="104" rx="12" fill="#0d1030" />
  <text x="384" y="58" class="svg-kicker" fill="#78d8f2">02 · PARALLEL WAVE BUILD</text>
  <text x="384" y="82" class="svg-h3" fill="#ffffff">Isolated Slice Worktrees</text>
  <text x="384" y="102" class="svg-caption" fill="rgba(255,255,255,0.7)">Each slice sees only its cut of the spec</text>
  <text x="384" y="118" class="svg-caption" fill="rgba(255,255,255,0.7)">inside a kernel-bounded worker</text>

  <!-- Downward connector to Verification Gate -->
  <line x1="502" y1="136" x2="502" y2="180" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="497,174 507,174 502,182" fill="#0d1030" />

  <!-- Step 3: Integrated Wave Verification -->
  <rect x="20" y="186" width="620" height="118" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="44" y="214" class="svg-kicker" fill="#a4238f">03 · INTEGRATE · VERIFY THE WAVE · PROMOTE THE REVISION</text>

  <circle cx="52" cy="244" r="4" fill="#a4238f" />
  <text x="68" y="248" class="svg-label" fill="#0d1030">Boundary &amp; Lint Gates</text>
  <text x="68" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Host-bypass and unsafe patterns refused</text>

  <circle cx="356" cy="244" r="4" fill="#a4238f" />
  <text x="372" y="248" class="svg-label" fill="#0d1030">Scenario &amp; Structure Checks</text>
  <text x="372" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Every covered scenario tested &amp; wired</text>

  <circle cx="52" cy="288" r="4" fill="#a4238f" />
  <text x="68" y="292" class="svg-label" fill="#0d1030">Compile &amp; WASI Target Build</text>

  <circle cx="356" cy="288" r="4" fill="#a4238f" />
  <text x="372" y="292" class="svg-label" fill="#0d1030">Secret &amp; Spec Integrity Guards</text>

  <!-- Downward connector to Verified Output -->
  <line x1="330" y1="304" x2="330" y2="344" stroke="#a4238f" stroke-width="2" />
  <polygon points="325,338 335,338 330,346" fill="#a4238f" />

  <!-- Step 4: Output -->
  <rect x="130" y="350" width="400" height="68" rx="12" fill="#a4238f" />
  <text x="330" y="376" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.88)">VERIFIED &amp; PROMOTED REVISION</text>
  <text x="330" y="400" text-anchor="middle" class="svg-h3" fill="#ffffff">Labelled only after every check passes</text>
</svg>
</div>
<div class="ed-prose-col">

# Built in waves.<br>Promoted on proof.

<p class="ed-prose">Once a specification is sealed, the build engine delivers its plan in dependency-ordered waves. Deterministic code scaffolds the crate layout, typed interfaces, and test markers first; the model fills only the bounded slice.</p>
<p class="ed-prose">Slices merge in order and the integrated wave is verified as a whole. <strong>The model may implement and repair, but only passing verification advances the production label.</strong></p>
</div>
</div>

<!--
Specification slide 3 of 3 — bounded delivery:
- The model receives one slice: its requirements, relevant design, owned types, and plan brief.
- Code lays down the target structure and markers first; the model fills bounded gaps.
- Promotion follows evidence. The model may implement and repair, but it cannot approve its own work.
-->

---
title: One Operating Model
class: slide-light
---

<Eyebrow text="THE OPERATING MODEL · SPECIFICATIONS + WEBASSEMBLY + RECIPES" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <!-- Left Plane: Specification-Driven Delivery -->
  <rect x="20" y="24" width="490" height="236" rx="16" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="48" y="54" class="svg-kicker" fill="#a4238f">SPEC-DRIVEN DEVELOPMENT · BUILD-TIME ASSURANCE</text>
  <text x="48" y="80" class="svg-h2" fill="#0d1030">Governs what must remain true</text>

  <circle cx="60" cy="120" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="125" text-anchor="middle" class="svg-kicker" fill="#a4238f">01</text>
  <text x="92" y="116" class="svg-label" fill="#0d1030">Recovers &amp; reconciles intent</text>
  <text x="92" y="134" class="svg-caption" fill="rgba(13,16,48,0.6)">From T2 prototypes, legacy repos, docs &amp; briefs</text>

  <circle cx="60" cy="174" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="179" text-anchor="middle" class="svg-kicker" fill="#a4238f">02</text>
  <text x="92" y="170" class="svg-label" fill="#0d1030">Seals the role-based specification</text>
  <text x="92" y="188" class="svg-caption" fill="rgba(13,16,48,0.6)">Reviewable scenarios, authority &amp; visible conflicts</text>

  <circle cx="60" cy="228" r="18" fill="#a4238f" />
  <text x="60" y="233" text-anchor="middle" class="svg-kicker" fill="#ffffff">03</text>
  <text x="92" y="224" class="svg-label" fill="#0d1030">Builds and verifies bounded waves</text>
  <text x="92" y="242" class="svg-caption" fill="rgba(13,16,48,0.6)">Promotes only after integrated checks pass</text>

  <!-- Handoff Bridge: Verified Implementation -->
  <line x1="510" y1="142" x2="630" y2="142" stroke="#0d1030" stroke-width="2.5" />
  <polygon points="622,136 634,142 622,148" fill="#0d1030" />
  <rect x="524" y="102" width="92" height="28" rx="14" fill="#0d1030" />
  <text x="570" y="120" text-anchor="middle" class="svg-pill" fill="#78d8f2">REVISION</text>
  <text x="570" y="166" text-anchor="middle" class="svg-caption" fill="#0d1030">Verified WASI</text>

  <!-- Right Plane: WebAssembly and Recipes -->
  <rect x="634" y="24" width="486" height="236" rx="16" fill="#0d1030" />
  <text x="662" y="54" class="svg-kicker" fill="#78d8f2">WASI RUNTIME + RECIPES · RUN-TIME AUTHORITY</text>
  <text x="662" y="80" class="svg-h2" fill="#ffffff">Bounds authority. Standardises how.</text>

  <circle cx="674" cy="120" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="125" text-anchor="middle" class="svg-kicker" fill="#78d8f2">04</text>
  <text x="706" y="116" class="svg-label" fill="#ffffff">WASI runtime bounds application effects</text>
  <text x="706" y="134" class="svg-caption" fill="rgba(255,255,255,0.68)">Host-selected capabilities · Credentials outside code</text>

  <circle cx="674" cy="174" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="179" text-anchor="middle" class="svg-kicker" fill="#78d8f2">05</text>
  <text x="706" y="170" class="svg-label" fill="#ffffff">Terraform provisions the service recipe</text>
  <text x="706" y="188" class="svg-caption" fill="rgba(255,255,255,0.68)">Data · messaging · identity · network · telemetry</text>

  <circle cx="674" cy="228" r="18" fill="#78d8f2" />
  <text x="674" y="233" text-anchor="middle" class="svg-kicker" fill="#0d1030">06</text>
  <text x="706" y="224" class="svg-label" fill="#ffffff">Propellerhead operates the service envelope</text>
  <text x="706" y="242" class="svg-caption" fill="rgba(255,255,255,0.68)">Performance · cost · recovery · continuous assurance</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Two independent loops.<br>One operating model.

</div>
<div>
<p class="ed-prose"><strong>Business intent and infrastructure evolve on separate clocks.</strong> When business rules change, domain owners update the authoritative sources and seal a new specification. When scale, residency, or cloud backends change, the environment recipe updates — without rewriting application logic.</p>
</div>
</div>
</div>

<!--
Synthesis slide:
- Specifications govern what must remain true.
- WebAssembly governs what application code may do.
- Terraform recipes govern how the service runs in a particular environment.
- Propellerhead owns the coherence of all three planes and operates the resulting service.
-->

---
title: Prove One Complete Service
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

# Start with one<br>proven winner.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Choose an application that already matters</h4>
<p>Bring a citizen-built tool ready to graduate or a bounded legacy service whose business value and pain are already visible.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Recover and review its durable contract</h4>
<p>We reconcile intent, documentation, tests, and behaviour into a versioned specification your domain owners and engineers can challenge together.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Prove the boundary and service envelope</h4>
<p>We rebuild the verified WASI component, bind its capabilities through the runtime, and prove a recipe sized to your operating outcome.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">READY TO PROVE THE FULL TRUST LOOP?</div>
<div class="cta-card-title">Bring one application that already matters.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>propellerhead.co.nz</span>
</div>
</div>
</div>

<!--
Closing call to action:
- Pick one application with known value and a bounded surface.
- Recover its specification, prove bounded delivery, and select a service envelope.
-->
