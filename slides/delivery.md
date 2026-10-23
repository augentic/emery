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

<p class="section-subtitle">The Trust Model, expressed in architecture: specifications make intent explicit, WebAssembly bounds what code may do, and infrastructure recipes standardise how secure, scalable platforms are provisioned and operated.</p>
</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Spec-driven framework · What must remain true</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Wasm runtime · What application code may do</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Service recipes · How each environment is standardised</span></div>
</div>

<!--
Opening framing:
- The Trust Model says when an application earns more authority and ownership. This deck explains how the architecture enforces that worldview.
- The spec-driven framework (Emery) separates durable intent from replaceable implementation.
- The Wasm runtime (Omnia) separates application logic from infrastructure authority.
- Recipes combine the runtime with Terraform so the customer chooses an outcome and constraints, not a bespoke topology.
-->

---
title: Born from the Trust Model
class: slide-light
---

<Eyebrow text="THE ORIGIN · FROM WORLDVIEW TO ARCHITECTURE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Three questions changed<br>our architecture.

</div>
<p class="ed-band-prose">At Tier 2 the question becomes <strong>what may this code touch?</strong> At Tier 3 it becomes <strong>what must remain true, and who owns it?</strong> Across both sits a third question: <strong>why custom-build the same infrastructure around every application?</strong> A Wasm runtime, a spec-driven coding framework, and recipe-driven infrastructure answer one question each.</p>
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
  <text x="546" y="184" class="svg-kicker" fill="#78d8f2">WASM RUNTIME · OMNIA (TIERS 2 &amp; 3)</text>
  <text x="546" y="208" class="svg-h3" fill="#ffffff">What may application code do?</text>
  <text x="546" y="228" class="svg-caption" fill="rgba(255,255,255,0.72)">Constrained WebAssembly boundary · Host-mediated capabilities · Infrastructure credentials stay outside</text>

  <!-- Technology 02: Emery (Drives the T2 -> T3 Graduation & Legacy Modernisation) -->
  <path d="M 690 296 L 780 296" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="774,291 784,296 774,301" fill="#a4238f" />
  <text x="676" y="292" text-anchor="end" class="svg-kicker" fill="#a4238f">T2 PROTOTYPE OR LEGACY REPO</text>
  <text x="676" y="310" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.58)">Working rules trapped in brittle code</text>

  <rect x="790" y="256" width="330" height="88" rx="14" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="814" y="284" class="svg-kicker" fill="#a4238f">SPEC-DRIVEN FRAMEWORK · EMERY (TIER 3)</text>
  <text x="814" y="308" class="svg-h3" fill="#0d1030">What must remain true?</text>
  <text x="814" y="328" class="svg-caption" fill="rgba(13,16,48,0.68)">Reconciles evidence · Stages a reviewable spec · Builds a verified revision</text>
</svg>
</div>
</div>

<!--
How the architecture follows from the Trust Model:
- Tier 1 needs a governed workspace because nothing live is reachable.
- Tier 2 needs a structural boundary between variable application code and infrastructure authority.
- Tier 3 needs durable intent, explicit promotion, and accountable operation.
- Standard service recipes keep those controls repeatable rather than redesigning a platform for every app.
-->

---
title: WebAssembly runtime
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART ONE · A WASM RUNTIME · OMNIA" />

<div class="section-hero-grid">
<div class="section-hero-left">

# WebAssembly runtime

<p class="section-subtitle"><strong>The runtime boundary for Tiers 2 and 3</strong> — portable components, explicit capabilities, and infrastructure policy outside application code.</p>
</div>
<div class="section-big-glyph">OMNIA</div>
</div>

<TrustLadder variant="strip" :current="3" />

<!--
Part One: Three slides on Omnia.
1. WebAssembly as a structural boundary for variable code.
2. Capability interfaces separate applications from provider plumbing.
3. Omnia + Terraform recipes turn infrastructure into a repeatable service.
-->

---
title: WASI · Sandboxed Agentic Code
class: slide-light
---

<Eyebrow text="WASI 01 / 03 · SAFER AGENTIC EXECUTION" />

<div class="ed-split">
<div class="ed-prose-col">

# Policy lives outside<br>the application.

<p class="ed-prose">When implementation can be written by engineers, domain builders, or models, assurance cannot depend on the code policing itself. We assume any implementation can be wrong and put authority somewhere else.</p>
<p class="ed-prose"><strong>Omnia hosts application logic as WebAssembly (WASI) components.</strong> The component reaches infrastructure through explicit interfaces selected by the host. Credentials, backend choice, and operational policy stay outside the application.</p>
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
  <text x="58" y="168" class="svg-caption" fill="rgba(13,16,48,0.58)">Each app handles sensitive configuration</text>

  <circle cx="44" cy="212" r="4" fill="#b4234e" />
  <text x="58" y="216" class="svg-label" fill="#0d1030">Provider-specific SDKs</text>
  <text x="58" y="232" class="svg-caption" fill="rgba(13,16,48,0.58)">Business logic absorbs platform choices</text>

  <circle cx="44" cy="276" r="4" fill="#b4234e" />
  <text x="58" y="280" class="svg-label" fill="#0d1030">Local enforcement</text>
  <text x="58" y="296" class="svg-caption" fill="rgba(13,16,48,0.58)">Every team recreates the boundary</text>

  <text x="132" y="366" text-anchor="middle" class="svg-caption" fill="#b4234e">Authorship is mistaken for assurance</text>

  <!-- Right Architecture: Omnia Wasm Sandbox -->
  <rect x="284" y="44" width="340" height="352" rx="14" fill="#0d1030" />
  <text x="454" y="74" text-anchor="middle" class="svg-kicker" fill="#78d8f2">OMNIA HOST · CAPABILITY BOUNDARY</text>

  <!-- Inner Wasm Guest Box -->
  <rect x="312" y="96" width="284" height="112" rx="10" fill="rgba(255,255,255,0.08)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="454" y="124" text-anchor="middle" class="svg-kicker" fill="#78d8f2">APPLICATION COMPONENT</text>
  <text x="454" y="148" text-anchor="middle" class="svg-h3" fill="#ffffff">Differentiating Logic</text>
  <text x="454" y="170" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">Portable Wasm · Explicit imports · Credentials stay outside</text>
  <text x="454" y="190" text-anchor="middle" class="svg-caption" fill="#78d8f2">Same boundary for every author</text>

  <!-- Explicit WIT Capability Links -->
  <line x1="368" y1="208" x2="368" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="363,252 373,252 368,260" fill="#78d8f2" />
  <line x1="454" y1="208" x2="454" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="449,252 459,252 454,260" fill="#78d8f2" />
  <line x1="540" y1="208" x2="540" y2="256" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="535,252 545,252 540,260" fill="#78d8f2" />
  <text x="454" y="240" text-anchor="middle" class="svg-pill" fill="#f59ef5">HOST-SELECTED CAPABILITY INTERFACES</text>

  <!-- Host-Managed Capability Backends -->
  <rect x="312" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="354" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:sql</text>
  <text x="354" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Chosen backend</text>

  <rect x="412" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="454" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:msg</text>
  <text x="454" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Managed routing</text>

  <rect x="512" y="264" width="84" height="54" rx="8" fill="rgba(255,255,255,0.1)" />
  <text x="554" y="288" text-anchor="middle" class="svg-kicker" fill="#ffffff">wasi:http</text>
  <text x="554" y="304" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.65)">Network policy</text>

  <text x="454" y="366" text-anchor="middle" class="svg-caption" fill="#78d8f2">Host retains credentials, infrastructure choices &amp; policy</text>
</svg>
</div>
</div>

<!--
WASI Slide 1 of 3 — a structural boundary for variable code:
- Generated and citizen-authored code is treated as untrusted input.
- Authorship does not create authority; the host does.
- The component names the capability it needs, while the host chooses the implementation and holds infrastructure credentials.
- Deployment controls can tighten data and network reach without rewriting the application.
-->

---
title: WASI · Infrastructure Portability
class: slide-light
---

<Eyebrow text="WASI 02 / 03 · INFRASTRUCTURE ABSTRACTION & PORTABILITY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# The application names the need.<br>The recipe chooses how.

</div>
<p class="ed-band-prose">Application code asks for durable roles — state, messaging, identity, secrets, outbound calls — through stable capability interfaces. Our next recipe layer will choose Azure, another cloud, or on-premises resources for the environment. Where interfaces and semantics remain compatible, the component can stay unchanged.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 355" aria-hidden="true">
  <!-- Top Layer: Portable Guest Component -->
  <rect x="290" y="12" width="560" height="72" rx="12" fill="#0d1030" />
  <text x="570" y="38" text-anchor="middle" class="svg-kicker" fill="#78d8f2">PORTABLE APPLICATION COMPONENT</text>
  <text x="570" y="62" text-anchor="middle" class="svg-h3" fill="#ffffff">Business Behaviour — No Provider Plumbing</text>

  <!-- Middle Layer: Capability Contracts -->
  <line x1="570" y1="84" x2="570" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <line x1="80" y1="124" x2="1060" y2="124" stroke="#0c7fa3" stroke-width="2" />
  <rect x="380" y="108" width="380" height="28" rx="14" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="570" y="126" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">TYPED CAPABILITY BOUNDARY (STATE · DATA · MESSAGING · IDENTITY · HTTP · TELEMETRY)</text>

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
  <text x="54" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Same application interfaces</text>
  <text x="54" y="322" class="svg-caption" fill="#0c7fa3">Optimised for learning and iteration</text>

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
  <text x="804" y="302" class="svg-body" fill="rgba(13,16,48,0.72)">• Residency and connectivity constraints</text>
  <text x="804" y="322" class="svg-caption" fill="#a4238f">Optimised for control and locality</text>
</svg>
</div>
</div>

<!--
WASI Slide 2 of 3 — infrastructure as a replaceable implementation:
- The application expresses stable capability needs instead of importing a provider architecture.
- The planned Terraform recipe layer selects the concrete services for the environment.
- Portability is governed by compatible interfaces and semantics, not a claim that every backend is identical.
- This separates the long-lived business contract from infrastructure choices that will change.
-->

---
title: WASI + Terraform · Service Recipes
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="WASI 03 / 03 · NEXT PRODUCT LAYER: WASI + TERRAFORM RECIPES" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Omnia + Terraform · Service Envelope</span>

# Choose the outcome.<br>We provision the stack.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Plan S · M · L</span><em>Outcome contract</em></div>
<p>The plan captures capacity, resilience, recovery, residency, and connectivity. It is a service envelope — <strong>not a prescribed cloud diagram or a VM size</strong>.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>One versioned recipe</span><em>Repeatable delivery</em></div>
<p>Terraform will provision the environment. Omnia binds application capabilities to it. Propellerhead operates the result against the plan.</p>
</div>
</div>
</div>

<div class="ed-diagram-pane">
<svg viewBox="0 0 700 420" aria-hidden="true">
  <!-- Outer Service Recipe Frame -->
  <rect x="16" y="20" width="668" height="380" rx="16" fill="rgba(10,7,16,0.58)" stroke="rgba(120,216,242,0.35)" stroke-width="1.5" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">NEXT PRODUCT LAYER · VERSIONED TERRAFORM + OMNIA SERVICE RECIPE</text>

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
  <text x="346" y="100" text-anchor="middle" class="svg-label" fill="#ffffff">WASI application component</text>
  <text x="346" y="118" text-anchor="middle" class="svg-caption" fill="#78d8f2">Differentiating behaviour</text>

  <rect x="240" y="157" width="212" height="58" rx="9" fill="rgba(255,255,255,0.12)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="346" y="183" text-anchor="middle" class="svg-label" fill="#ffffff">Omnia host boundary</text>
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
  <text x="64" y="352" class="svg-kicker" fill="#78d8f2">ONE SERVICE</text>
  <text x="190" y="352" class="svg-body" fill="#ffffff">One safety baseline · Versioned recipe · Cost and performance measured · Propellerhead accountable</text>
</svg>
</div>
</div>

<!--
Omnia Slide 3 of 3 — the direction of the full service:
- Customers should buy a service outcome, not commission a one-off infrastructure architecture.
- Small, medium, and large plans express measurable demand and constraints. Security remains the same baseline.
- A versioned recipe combines Terraform-provisioned resources with an Omnia host and the application component.
- Architecture stays transparent and inspectable, but it stops being the bespoke product.
- This is the next step: productising the combined Omnia and Terraform stack.
-->

---
title: Spec-driven coding framework
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PART TWO · A SPEC-DRIVEN CODING FRAMEWORK · EMERY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Spec-driven coding framework

<p class="section-subtitle"><strong>Build-time trust for Tier 3</strong> — durable specifications, bounded model work, explicit review, and evidence before promotion.</p>
</div>
<div class="section-big-glyph">EMERY</div>
</div>

<TrustLadder variant="strip" :current="3" />

<!--
Part Two: Three slides on Emery.
1. Specs, not code, as the durable source of truth.
2. Citizen-coded app & legacy migrator (emery specify).
3. Bounded, wave-by-wave delivery for an Omnia-ready source revision (emery build).
-->

---
title: Emery · Specs Not Code as Truth
class: slide-light
---

<Eyebrow text="EMERY 01 / 03 · SPECIFICATIONS AS THE SOURCE OF TRUTH" />

<div class="ed-split">
<div class="ed-prose-col">

# Intent is the asset.<br>Code is replaceable.

<p class="ed-prose">In both vibe-coded prototypes and twenty-year-old legacy systems, business intent is trapped inside code. Prompts vanish when the chat window closes, and code tangles <em>what the business requires</em> with <em>how a framework happened to wire it</em>.</p>
<p class="ed-prose"><strong>Emery flips the hierarchy:</strong> a reviewed, content-addressed revision holds a technology-neutral <strong>Specification</strong>, a technology-aware <strong>Design</strong>, and a sliced <strong>Build Plan</strong>. The implementation can then be rebuilt for a target without asking a model to rediscover intent from scratch.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Center Durable Core: Content-Addressed Revision -->
  <rect x="44" y="64" width="288" height="312" rx="16" fill="#0d1030" />
  <text x="188" y="96" text-anchor="middle" class="svg-kicker" fill="#78d8f2">DURABLE SOURCE OF TRUTH (emery/&lt;rev&gt;)</text>
  <text x="188" y="120" text-anchor="middle" class="svg-h3" fill="#ffffff">Content-Addressed Revision</text>

  <!-- Spec Box -->
  <rect x="68" y="140" width="240" height="64" rx="10" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="86" y="164" class="svg-kicker" fill="#78d8f2">01 · SPECIFICATION</text>
  <text x="86" y="184" class="svg-caption" fill="#ffffff">Requirements, Given/When/Then scenarios</text>
  <text x="86" y="198" class="svg-caption" fill="rgba(255,255,255,0.65)">Behaviour expressed by domain role</text>

  <!-- Design Box -->
  <rect x="68" y="216" width="240" height="64" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.18)" />
  <text x="86" y="240" class="svg-kicker" fill="#f59ef5">02 · DESIGN</text>
  <text x="86" y="260" class="svg-caption" fill="#ffffff">Data types, mechanisms &amp; integrations</text>
  <text x="86" y="274" class="svg-caption" fill="rgba(255,255,255,0.65)">Holds the technology &amp; wire shapes</text>

  <!-- Plan Box -->
  <rect x="68" y="292" width="240" height="64" rx="10" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.18)" />
  <text x="86" y="316" class="svg-kicker" fill="#78d8f2">03 · BUILD PLAN</text>
  <text x="86" y="336" class="svg-caption" fill="#ffffff">Dependency-ordered domain slices</text>
  <text x="86" y="350" class="svg-caption" fill="rgba(255,255,255,0.65)">Behaviour grouped for bounded delivery</text>

  <!-- Projection Arrows to Replaceable Target Codebases -->
  <path d="M 332 180 C 378 180, 384 134, 428 134" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="422,129 432,134 422,139" fill="#a4238f" />
  <path d="M 332 260 C 378 260, 384 306, 428 306" fill="none" stroke="#0c7fa3" stroke-width="1.8" stroke-dasharray="4 4" />
  <polygon points="422,301 432,306 422,311" fill="#0c7fa3" />

  <!-- Primary Target: Omnia Rust/Wasm -->
  <rect x="436" y="78" width="188" height="116" rx="12" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="530" y="106" text-anchor="middle" class="svg-kicker" fill="#a4238f">TARGET · emery:omnia</text>
  <text x="530" y="130" text-anchor="middle" class="svg-h3" fill="#0d1030">Omnia Component</text>
  <text x="530" y="152" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Rust + WASI capabilities</text>
  <text x="530" y="170" text-anchor="middle" class="svg-caption" fill="#a4238f">Preferred production target</text>

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
Emery Slide 1 of 3 — durable intent:
- A specification is reviewable by the people accountable for the business outcome.
- Behaviour is kept separate from technology choices; the design carries mechanisms and integrations.
- The stored revision is typed data. Markdown is a projection for human review.
- A target adapter builds from the reviewed revision, so the model receives bounded work and explicit acceptance criteria.
-->

---
title: Emery · Specification Recovery
class: slide-light
---

<Eyebrow text="EMERY 02 / 03 · RECOVER INTENT FROM CITIZEN & LEGACY SYSTEMS" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Recover the contract.<br>Expose uncertainty.

</div>
<p class="ed-band-prose">Whether a citizen-built prototype is graduating or a legacy service is being replaced, <strong>emery specify</strong> recovers a contract from intent, documentation, tests, and observed code structure. Typed evidence is reconciled under configured <strong>authority</strong>; conflicts and unknowns stay visible in the candidate.</p>
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
  <text x="452" y="190" class="svg-label" fill="#ffffff">Divergence (Authority Resolves)</text>
  <text x="452" y="208" class="svg-caption" fill="rgba(255,255,255,0.68)">Configured authority selects the candidate</text>

  <circle cx="436" cy="242" r="4.5" fill="#ff7597" />
  <text x="452" y="246" class="svg-label" fill="#ffffff">Conflict &amp; [unknown] Preserved</text>
  <text x="452" y="264" class="svg-caption" fill="rgba(255,255,255,0.68)">Ambiguity remains visible to reviewers</text>

  <!-- Connector to Output -->
  <line x1="720" y1="172" x2="796" y2="172" stroke="#a4238f" stroke-width="2.2" />
  <polygon points="790,167 800,172 790,177" fill="#a4238f" />

  <!-- Right Column: Clean Migration Blueprint -->
  <rect x="804" y="54" width="316" height="236" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="962" y="84" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">03 · REVIEWABLE MIGRATION BASELINE</text>
  <text x="962" y="114" text-anchor="middle" class="svg-h3" fill="#0d1030">Keep the criteria.</text>
  <text x="962" y="136" text-anchor="middle" class="svg-h3" fill="#0d1030">Drop the code.</text>
  <line x1="836" y1="154" x2="1088" y2="154" stroke="rgba(13,16,48,0.12)" />
  <text x="836" y="182" class="svg-body" fill="#0d1030">• Recognised routes &amp; message triggers typed</text>
  <text x="836" y="208" class="svg-body" fill="#0d1030">• Domain data shapes separated from plumbing</text>
  <text x="836" y="234" class="svg-body" fill="#0d1030">• Candidate staged for explicit review and seal</text>
  <text x="836" y="264" class="svg-caption" fill="#0c7fa3">No silent assumptions</text>
</svg>
</div>
</div>

<!--
Emery Slide 2 of 3 — specification recovery:
- A direct AI rewrite confuses historical implementation with intended behaviour.
- Source adapters extract typed evidence from each authoritative input.
- Authority is explicit and configurable; divergence, conflict, and unknowns are represented rather than silently flattened.
- The result is staged as a reviewable candidate. A person or controlled workflow decides whether to seal it.
-->

---
title: Emery · Bounded Wave Builder
class: slide-light
---

<Eyebrow text="EMERY 03 / 03 · BOUNDED MODEL WORK & EVIDENCED PROMOTION" />

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

  <!-- Step 3: Integrated Wave Verification -->
  <rect x="20" y="186" width="620" height="118" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="44" y="214" class="svg-kicker" fill="#a4238f">03 · INTEGRATE · VERIFY THE WAVE · ADVANCE THE LABEL</text>

  <circle cx="52" cy="244" r="4" fill="#a4238f" />
  <text x="68" y="248" class="svg-label" fill="#0d1030">Host-Boundary Lints (clippy.toml)</text>
  <text x="68" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Forbids std::env, std::fs, raw HTTP &amp; timers</text>

  <circle cx="356" cy="244" r="4" fill="#a4238f" />
  <text x="372" y="248" class="svg-label" fill="#0d1030">Scenario &amp; Structure Gate</text>
  <text x="372" y="266" class="svg-caption" fill="rgba(13,16,48,0.62)">Covered behaviour checked against the integrated tree</text>

  <circle cx="52" cy="288" r="4" fill="#a4238f" />
  <text x="68" y="292" class="svg-label" fill="#0d1030">wasm32-wasip2 Compile Check</text>

  <circle cx="356" cy="288" r="4" fill="#a4238f" />
  <text x="372" y="292" class="svg-label" fill="#0d1030">Secret Gate at Seal Points</text>

  <!-- Downward connector to Verified Output -->
  <line x1="330" y1="304" x2="330" y2="344" stroke="#a4238f" stroke-width="2" />
  <polygon points="325,338 335,338 330,346" fill="#a4238f" />

  <!-- Step 4: Output -->
  <rect x="130" y="350" width="400" height="68" rx="12" fill="#a4238f" />
  <text x="330" y="376" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.88)">VERIFIED &amp; LABELLED (emery/&lt;revision&gt;)</text>
  <text x="330" y="400" text-anchor="middle" class="svg-h3" fill="#ffffff">Reviewable Omnia-ready source revision</text>
</svg>
</div>
<div class="ed-prose-col">

# Built in waves.<br>Promoted with evidence.

<p class="ed-prose">Once the specification is sealed, <strong>emery build</strong> delivers its plan in dependency-ordered waves. Rather than asking a model to invent a repository from scratch, the <strong>emery:omnia</strong> adapter scaffolds the approved Rust layout, typed handlers, wire structures, and test markers before each bounded turn.</p>
<p class="ed-prose">Slices build in isolated working copies and integrate in a controlled order. The combined wave is then checked and repaired; secret and specification guards run at seal points. Only a passing wave advances the <strong>emery/&lt;revision&gt;</strong> label.</p>
</div>
</div>

<!--
Emery Slide 3 of 3 — bounded delivery:
- The model receives one slice: its requirements, relevant design, owned types, and plan brief.
- Code lays down the target structure and markers first; the model fills bounded gaps.
- Slices merge before the integrated wave is verified. A failed verification leaves the label at the last known-good wave.
- Promotion follows evidence. The model may implement and repair, but it cannot label its own work as accepted.
-->

---
title: The Three Control Planes
class: slide-light
---

<Eyebrow text="THE OPERATING MODEL · EMERY + OMNIA + NEXT: TERRAFORM RECIPES" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <!-- Left Plane: EMERY (Build-Time Assurance) -->
  <rect x="20" y="24" width="490" height="236" rx="16" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="48" y="54" class="svg-kicker" fill="#a4238f">EMERY · BUILD-TIME ASSURANCE PLANE</text>
  <text x="48" y="80" class="svg-h2" fill="#0d1030">Preserves why it was built</text>

  <circle cx="60" cy="120" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="125" text-anchor="middle" class="svg-kicker" fill="#a4238f">01</text>
  <text x="92" y="116" class="svg-label" fill="#0d1030">Recovers &amp; reconciles intent (emery specify)</text>
  <text x="92" y="134" class="svg-caption" fill="rgba(13,16,48,0.6)">From T2 prototypes, legacy repos, docs &amp; briefs</text>

  <circle cx="60" cy="174" r="18" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="60" y="179" text-anchor="middle" class="svg-kicker" fill="#a4238f">02</text>
  <text x="92" y="170" class="svg-label" fill="#0d1030">Holds the role-based specification</text>
  <text x="92" y="188" class="svg-caption" fill="rgba(13,16,48,0.6)">Reviewable scenarios, authority and visible uncertainty</text>

  <circle cx="60" cy="228" r="18" fill="#a4238f" />
  <text x="60" y="233" text-anchor="middle" class="svg-kicker" fill="#ffffff">03</text>
  <text x="92" y="224" class="svg-label" fill="#0d1030">Builds and verifies bounded waves (emery build)</text>
  <text x="92" y="242" class="svg-caption" fill="rgba(13,16,48,0.6)">Promotes only after integrated checks pass</text>

  <!-- Handoff Bridge: Verified Implementation -->
  <line x1="510" y1="142" x2="630" y2="142" stroke="#0d1030" stroke-width="2.5" />
  <polygon points="622,136 634,142 622,148" fill="#0d1030" />
  <rect x="524" y="102" width="92" height="28" rx="14" fill="#0d1030" />
  <text x="570" y="120" text-anchor="middle" class="svg-pill" fill="#78d8f2">REVISION</text>
  <text x="570" y="166" text-anchor="middle" class="svg-caption" fill="#0d1030">Verified source</text>

  <!-- Right Plane: OMNIA and Recipes (Run-Time Authority) -->
  <rect x="634" y="24" width="486" height="236" rx="16" fill="#0d1030" />
  <text x="662" y="54" class="svg-kicker" fill="#78d8f2">OMNIA + RECIPES · RUN-TIME AUTHORITY</text>
  <text x="662" y="80" class="svg-h2" fill="#ffffff">Bounds authority. Standardises how.</text>

  <circle cx="674" cy="120" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="125" text-anchor="middle" class="svg-kicker" fill="#78d8f2">04</text>
  <text x="706" y="116" class="svg-label" fill="#ffffff">Omnia bounds application effects</text>
  <text x="706" y="134" class="svg-caption" fill="rgba(255,255,255,0.68)">Host-selected capabilities · Credentials outside code</text>

  <circle cx="674" cy="174" r="18" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="674" y="179" text-anchor="middle" class="svg-kicker" fill="#78d8f2">05</text>
  <text x="706" y="170" class="svg-label" fill="#ffffff">Terraform provisions a proven recipe</text>
  <text x="706" y="188" class="svg-caption" fill="rgba(255,255,255,0.68)">Data · messaging · identity · network · telemetry</text>

  <circle cx="674" cy="228" r="18" fill="#78d8f2" />
  <text x="674" y="233" text-anchor="middle" class="svg-kicker" fill="#0d1030">06</text>
  <text x="706" y="224" class="svg-label" fill="#ffffff">Propellerhead operates the service envelope</text>
  <text x="706" y="242" class="svg-caption" fill="rgba(255,255,255,0.68)">Performance · cost · recovery · improvement</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Emery governs what.<br>Omnia governs what may.<br>Recipes govern how.

</div>
<div>
<p class="ed-prose"><strong>Emery</strong> turns evidence into durable intent and bounded delivery. <strong>Omnia</strong> keeps application effects behind explicit host interfaces. The next layer of <strong>Terraform recipes</strong> will realise the chosen service envelope in the customer’s environment.</p>
<p class="ed-prose">When business rules change, the authoritative sources change and a new specification is reviewed and sealed. When scale or provider choices change, the recipe changes. The two concerns evolve independently, with evidence at every promotion boundary.</p>
</div>
<div></div>
</div>
</div>

<!--
Synthesis slide:
- Emery governs what must remain true.
- Omnia governs what application code may do.
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
<h4>Prove the plan, boundary, and service envelope</h4>
<p>We build the component, bind its capabilities through Omnia, and prove a recipe sized to the performance, cost, recovery, and residency outcome.</p>
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
- Demonstrate that business differentiation can stay custom while the safety boundary and infrastructure become repeatable.
-->
