---
theme: default
title: Authority stays outside the model
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<style>
.slidev-layout .svg-kicker { font-size: 11px; }
.slidev-layout .svg-pill { font-size: 10.5px; }
.slidev-layout .svg-h2 { font-size: 24px; }
.slidev-layout .svg-h3 { font-size: 17.5px; }
.slidev-layout .svg-label { font-size: 14.5px; }
.slidev-layout .svg-body { font-size: 13px; }
.slidev-layout .svg-caption { font-size: 12px; }
.slidev-layout .hero-pillar { font-size: 15px; }
.slidev-layout .slide-eyebrow { position: relative; padding-right: 72px; box-sizing: border-box; }
.slidev-layout .eyebrow-num { position: absolute; top: 0; right: 0; white-space: nowrap; }
.slidev-layout.slide-split .eyebrow-num { display: none; }
</style>

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · TECHNICAL REFERENCE" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Authority stays<br>outside the model.

<p class="section-subtitle">The Trust Model, delivery controls,<br>and runtime boundaries.</p>

</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Authoring · Bound the agent and the work</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Delivery · Preserve evidence, intent, and verification</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Runtime · Grant application code nothing by default</span></div>
</div>

<!--
Use this deck selectively:
- It is a technical reference for architecture, engineering, security, platform, and governance conversations.
- The client deck carries the proposition. This deck answers how the system works and what it does not claim.
- The Trust Model classifies required controls by reach and dependence. It is not a maturity score.
-->

---
title: One system, four boundaries
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="SYSTEM VIEW · CONTROL PLANE, DELIVERY PATH, AND OPERATION" />

# One system. Four boundaries.

<p class="slide-lead">The model may work inside the path. It cannot grant itself authority, rewrite the record, or authorise release.</p>

<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 400" aria-hidden="true">
  <rect x="122" y="4" width="896" height="38" rx="19" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.26)" />
  <text x="570" y="28" text-anchor="middle" class="svg-kicker" fill="#ffffff">PROTECTED CONTROL PLANE · PEOPLE SET OUTCOME, SOURCE PRECEDENCE, ACCEPTANCE, RELEASE</text>

  <rect x="18" y="72" width="250" height="206" rx="15" fill="rgba(10,7,16,0.74)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="42" y="102" class="svg-kicker" fill="#78d8f2">BOUNDARY 01 · AUTHORING</text>
  <text x="42" y="134" class="svg-h3" fill="#ffffff">Rules-based toolchain</text>
  <line x1="42" y1="150" x2="244" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="42" y="180" class="svg-body" fill="rgba(255,255,255,0.82)">Approved tools and models</text>
  <text x="42" y="204" class="svg-body" fill="rgba(255,255,255,0.82)">Worker, source, data grants</text>
  <text x="42" y="228" class="svg-body" fill="rgba(255,255,255,0.82)">Guidance and deterministic gates</text>
  <text x="42" y="260" class="svg-caption" fill="#78d8f2">Variable author · controlled workspace</text>

  <line x1="268" y1="175" x2="300" y2="175" stroke="#ffffff" stroke-width="1.8" />
  <polygon points="294,170 304,175 294,180" fill="#ffffff" />

  <rect x="306" y="72" width="250" height="206" rx="15" fill="rgba(10,7,16,0.74)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="330" y="102" class="svg-kicker" fill="#f59ef5">BOUNDARY 02 · DELIVERY RECORD</text>
  <text x="330" y="134" class="svg-h3" fill="#ffffff">Emery</text>
  <line x1="330" y1="150" x2="532" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="330" y="180" class="svg-body" fill="rgba(255,255,255,0.82)">Typed evidence and authority</text>
  <text x="330" y="204" class="svg-body" fill="rgba(255,255,255,0.82)">Versioned spec, design, plan</text>
  <text x="330" y="228" class="svg-body" fill="rgba(255,255,255,0.82)">Isolated build and verification</text>
  <text x="330" y="260" class="svg-caption" fill="#f59ef5">Variable model · durable revision</text>

  <line x1="556" y1="175" x2="588" y2="175" stroke="#ffffff" stroke-width="1.8" />
  <polygon points="582,170 592,175 582,180" fill="#ffffff" />

  <rect x="594" y="72" width="250" height="206" rx="15" fill="rgba(10,7,16,0.74)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="618" y="102" class="svg-kicker" fill="#78d8f2">BOUNDARY 03 · APPLICATION</text>
  <text x="618" y="134" class="svg-h3" fill="#ffffff">Omnia</text>
  <line x1="618" y1="150" x2="820" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="618" y="180" class="svg-body" fill="rgba(255,255,255,0.82)">Sandboxed WASM component</text>
  <text x="618" y="204" class="svg-body" fill="rgba(255,255,255,0.82)">Typed capability contracts</text>
  <text x="618" y="228" class="svg-body" fill="rgba(255,255,255,0.82)">Host-held credentials and policy</text>
  <text x="618" y="260" class="svg-caption" fill="#78d8f2">Variable code · explicit runtime reach</text>

  <line x1="844" y1="175" x2="876" y2="175" stroke="#ffffff" stroke-width="1.8" />
  <polygon points="870,170 880,175 870,180" fill="#ffffff" />

  <rect x="882" y="72" width="240" height="206" rx="15" fill="rgba(10,7,16,0.74)" stroke="rgba(255,255,255,0.42)" stroke-width="1.6" />
  <text x="906" y="102" class="svg-kicker" fill="rgba(255,255,255,0.72)">BOUNDARY 04 · OPERATION</text>
  <text x="906" y="134" class="svg-h3" fill="#ffffff">Production rail</text>
  <line x1="906" y1="150" x2="1098" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="906" y="180" class="svg-body" fill="rgba(255,255,255,0.82)">Provision and deploy assets</text>
  <text x="906" y="204" class="svg-body" fill="rgba(255,255,255,0.82)">Release decision and telemetry</text>
  <text x="906" y="228" class="svg-body" fill="rgba(255,255,255,0.82)">Recovery and service ownership</text>
  <text x="906" y="260" class="svg-caption" fill="rgba(255,255,255,0.72)">Verified result · authorised operation</text>

  <line x1="143" y1="278" x2="143" y2="316" stroke="#78d8f2" stroke-width="1.5" />
  <line x1="431" y1="278" x2="431" y2="316" stroke="#f59ef5" stroke-width="1.5" />
  <line x1="719" y1="278" x2="719" y2="316" stroke="#78d8f2" stroke-width="1.5" />
  <line x1="1002" y1="278" x2="1002" y2="316" stroke="rgba(255,255,255,0.5)" stroke-width="1.5" />

  <rect x="18" y="318" width="1104" height="52" rx="12" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.24)" />
  <text x="42" y="350" class="svg-kicker" fill="#78d8f2">TRUSTED INPUTS</text>
  <text x="176" y="350" class="svg-label" fill="#ffffff">Rules and grants</text>
  <text x="360" y="350" class="svg-label" fill="#ffffff">Evidence and reviewed intent</text>
  <text x="656" y="350" class="svg-label" fill="#ffffff">Capabilities and host policy</text>
  <text x="944" y="350" class="svg-label" fill="#ffffff">Release and ownership</text>
</svg>
</div>

<!--
System view:
- Boundary 1 constrains the coding agent and its working environment.
- Boundary 2 keeps authoritative evidence and the delivery revision outside conversation.
- Boundary 3 constrains the application produced by the agent.
- Boundary 4 carries a verified revision into operation under an explicit release decision.
- Reusable infrastructure supports the four boundaries. It is not a substitute for them.
-->

---
title: Authority is split on purpose
class: slide-light
---

<Eyebrow text="CONTROL PLANE · FIVE AUTHORITIES THE MODEL NEVER OWNS" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 320" aria-hidden="true">
  <rect x="18" y="18" width="1104" height="278" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <rect x="18" y="18" width="1104" height="48" rx="14" fill="#0d1030" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">AUTHORITY</text>
  <text x="274" y="48" class="svg-kicker" fill="#78d8f2">HELD BY</text>
  <text x="544" y="48" class="svg-kicker" fill="#78d8f2">TECHNICAL EXPRESSION</text>
  <text x="920" y="48" class="svg-kicker" fill="#78d8f2">MODEL'S ROLE</text>

  <line x1="248" y1="66" x2="248" y2="296" stroke="rgba(13,16,48,0.12)" />
  <line x1="518" y1="66" x2="518" y2="296" stroke="rgba(13,16,48,0.12)" />
  <line x1="894" y1="66" x2="894" y2="296" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="112" x2="1122" y2="112" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="158" x2="1122" y2="158" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="204" x2="1122" y2="204" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="250" x2="1122" y2="250" stroke="rgba(13,16,48,0.12)" />

  <text x="42" y="95" class="svg-label" fill="#0d1030">Outcome and trade-offs</text>
  <text x="274" y="95" class="svg-body" fill="#0d1030">Accountable people</text>
  <text x="544" y="95" class="svg-body" fill="#0d1030">Accepted outcome, scope, scenarios</text>
  <text x="920" y="95" class="svg-caption" fill="#0c7fa3">Propose and expose gaps</text>

  <text x="42" y="141" class="svg-label" fill="#0d1030">Source precedence</text>
  <text x="274" y="141" class="svg-body" fill="#0d1030">Configured policy and owners</text>
  <text x="544" y="141" class="svg-body" fill="#0d1030">Ranked evidence; equal authority conflicts</text>
  <text x="920" y="141" class="svg-caption" fill="#a4238f">Reconcile; never silently choose</text>

  <text x="42" y="187" class="svg-label" fill="#0d1030">Authoring access</text>
  <text x="274" y="187" class="svg-body" fill="#0d1030">Toolchain and adapter policy</text>
  <text x="544" y="187" class="svg-body" fill="#0d1030">Lends, worker grants, merge and quality gates</text>
  <text x="920" y="187" class="svg-caption" fill="#0c7fa3">Use only what is granted</text>

  <text x="42" y="233" class="svg-label" fill="#0d1030">Runtime reach</text>
  <text x="274" y="233" class="svg-body" fill="#0d1030">Capability contract and host</text>
  <text x="544" y="233" class="svg-body" fill="#0d1030">Typed interfaces, linked backends, credentials</text>
  <text x="920" y="233" class="svg-caption" fill="#0c7fa3">Call only linked capabilities</text>

  <text x="42" y="279" class="svg-label" fill="#0d1030">Release and operation</text>
  <text x="274" y="279" class="svg-body" fill="#0d1030">Named release and service owners</text>
  <text x="544" y="279" class="svg-body" fill="#0d1030">Approval, deployment, observation, recovery</text>
  <text x="920" y="279" class="svg-caption" fill="#a4238f">Supply evidence; never authorise</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Authority is split<br>on purpose.

</div>
<div>
<p class="ed-prose"><strong>No model turn is the control plane.</strong> Decision rights, evidence precedence, access grants, runtime capabilities, and release authority have separate owners and separate technical representations.</p>
<p class="ed-prose"><strong>That separation is the trust model’s foundation.</strong> A strong result in one boundary cannot grant authority in another.</p>
</div>
</div>
</div>

<!--
Key distinction:
- A model can extract, reconcile, propose, implement, test, and repair.
- It cannot make itself authoritative, widen its own grants, add a runtime capability, or authorise release.
- Human authority is not a manual review of every line. It is explicit ownership of the decisions that change consequence.
-->

---
title: Trust is contextual, not a maturity score
class: slide-light
---

<Eyebrow text="TRUST MODEL · REQUIRED CONTROLS FOLLOW REACH AND DEPENDENCE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Trust is contextual,<br>not a maturity score.

</div>
<p class="ed-band-prose">Classify the application by what it reaches, who depends on it, and what happens when it fails. <strong>The highest-consequence answer sets the required state.</strong> There is no virtue in moving every system to the right.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="62" width="258" height="232" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.26)" stroke-width="1.5" stroke-dasharray="5 4" />
  <text x="42" y="92" class="svg-kicker" fill="rgba(13,16,48,0.56)">STATE 0 · EXPLORE</text>
  <text x="42" y="124" class="svg-h3" fill="#0d1030">Local discovery</text>
  <line x1="42" y1="140" x2="252" y2="140" stroke="rgba(13,16,48,0.12)" />
  <text x="42" y="168" class="svg-label" fill="#0d1030">One builder</text>
  <text x="42" y="194" class="svg-label" fill="#0d1030">Synthetic or local inputs</text>
  <text x="42" y="220" class="svg-label" fill="#0d1030">No shared dependence</text>
  <text x="42" y="266" class="svg-caption" fill="rgba(13,16,48,0.56)">Control: isolate and keep it local</text>

  <rect x="300" y="62" width="258" height="232" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="324" y="92" class="svg-kicker" fill="#0c7fa3">STATE 1 · GOVERN</text>
  <text x="324" y="124" class="svg-h3" fill="#0d1030">Shared work</text>
  <line x1="324" y1="140" x2="534" y2="140" stroke="rgba(13,16,48,0.12)" />
  <text x="324" y="168" class="svg-label" fill="#0d1030">Named owner</text>
  <text x="324" y="194" class="svg-label" fill="#0d1030">Approved workspace and tools</text>
  <text x="324" y="220" class="svg-label" fill="#0d1030">Protected changes and checks</text>
  <text x="324" y="266" class="svg-caption" fill="#0c7fa3">Control: authoring boundary</text>

  <rect x="582" y="62" width="258" height="232" rx="14" fill="#0d1030" />
  <text x="606" y="92" class="svg-kicker" fill="#78d8f2">STATE 2 · CONNECT</text>
  <text x="606" y="124" class="svg-h3" fill="#ffffff">Live capability</text>
  <line x1="606" y1="140" x2="816" y2="140" stroke="rgba(255,255,255,0.14)" />
  <text x="606" y="168" class="svg-label" fill="#ffffff">Explicit purpose and owner</text>
  <text x="606" y="194" class="svg-label" fill="#ffffff">Typed live-system contracts</text>
  <text x="606" y="220" class="svg-label" fill="#ffffff">Host policy and observability</text>
  <text x="606" y="266" class="svg-caption" fill="#78d8f2">Control: runtime boundary</text>

  <rect x="864" y="62" width="258" height="232" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="888" y="92" class="svg-kicker" fill="#a4238f">STATE 3 · SUPPORT</text>
  <text x="888" y="124" class="svg-h3" fill="#0d1030">Business dependence</text>
  <line x1="888" y1="140" x2="1098" y2="140" stroke="rgba(164,35,143,0.16)" />
  <text x="888" y="168" class="svg-label" fill="#0d1030">Durable reviewed intent</text>
  <text x="888" y="194" class="svg-label" fill="#0d1030">Release and service ownership</text>
  <text x="888" y="220" class="svg-label" fill="#0d1030">Recovery evidence</text>
  <text x="888" y="266" class="svg-caption" fill="#a4238f">Control: full delivery system</text>

  <text x="570" y="328" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.56)">Assess reach · data · dependence · continuity. Reassess when any one changes.</text>
</svg>
</div>
</div>

<!--
Trust Model:
- These are operational states, not levels of organisational maturity.
- A system should remain in the least costly state consistent with its actual consequence.
- The state follows the highest-consequence dimension: reach, data, dependence, or continuity.
- States can change in either direction as use and authority change.
- Avoid fixed user-count or estate-percentage thresholds unless the client adopts them as local policy.
-->

---
title: Crossing a boundary requires evidence
class: slide-light
---

<Eyebrow text="TRUST MODEL · THREE CLEARANCES, EACH WITH A DIFFERENT OWNER" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 680 440" aria-hidden="true">
  <rect x="10" y="42" width="208" height="366" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="28" y="70" class="svg-kicker" fill="#0c7fa3">01 · CLEARED TO SHARE</text>
  <text x="28" y="100" class="svg-h3" fill="#0d1030">Explore → Govern</text>
  <line x1="28" y1="116" x2="200" y2="116" stroke="rgba(13,16,48,0.12)" />
  <text x="28" y="146" class="svg-kicker" fill="rgba(13,16,48,0.5)">TRIGGER</text>
  <text x="28" y="168" class="svg-body" fill="#0d1030">Another person relies on it</text>
  <text x="28" y="198" class="svg-kicker" fill="rgba(13,16,48,0.5)">REQUIRED EVIDENCE</text>
  <text x="28" y="220" class="svg-body" fill="#0d1030">Named owner and workspace</text>
  <text x="28" y="242" class="svg-body" fill="#0d1030">Required checks pass</text>
  <text x="28" y="272" class="svg-kicker" fill="rgba(13,16,48,0.5)">DECISION</text>
  <text x="28" y="294" class="svg-body" fill="#0d1030">Sharing policy admits it</text>
  <rect x="28" y="334" width="172" height="42" rx="9" fill="#0c7fa3" />
  <text x="114" y="360" text-anchor="middle" class="svg-label" fill="#ffffff">Authoring boundary</text>

  <rect x="236" y="42" width="208" height="366" rx="14" fill="#0d1030" />
  <text x="254" y="70" class="svg-kicker" fill="#78d8f2">02 · CLEARED TO CONNECT</text>
  <text x="254" y="100" class="svg-h3" fill="#ffffff">Govern → Connect</text>
  <line x1="254" y1="116" x2="426" y2="116" stroke="rgba(255,255,255,0.14)" />
  <text x="254" y="146" class="svg-kicker" fill="rgba(255,255,255,0.5)">TRIGGER</text>
  <text x="254" y="168" class="svg-body" fill="#ffffff">Live system reach is needed</text>
  <text x="254" y="198" class="svg-kicker" fill="rgba(255,255,255,0.5)">REQUIRED EVIDENCE</text>
  <text x="254" y="220" class="svg-caption" fill="#ffffff">Purpose and data owner</text>
  <text x="254" y="242" class="svg-caption" fill="#ffffff">Capability contract</text>
  <text x="254" y="272" class="svg-kicker" fill="rgba(255,255,255,0.5)">DECISION</text>
  <text x="254" y="292" class="svg-caption" fill="#ffffff">Owners accept purpose</text>
  <text x="254" y="310" class="svg-caption" fill="#ffffff">and reach</text>
  <rect x="254" y="334" width="172" height="42" rx="9" fill="#ffffff" />
  <text x="340" y="360" text-anchor="middle" class="svg-label" fill="#0d1030">Runtime boundary</text>

  <rect x="462" y="42" width="208" height="366" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="480" y="70" class="svg-kicker" fill="#a4238f">03 · CLEARED TO DEPEND</text>
  <text x="480" y="100" class="svg-h3" fill="#0d1030">Connect → Support</text>
  <line x1="480" y1="116" x2="652" y2="116" stroke="rgba(164,35,143,0.16)" />
  <text x="480" y="146" class="svg-kicker" fill="rgba(13,16,48,0.5)">TRIGGER</text>
  <text x="480" y="168" class="svg-body" fill="#0d1030">Failure now interrupts the business</text>
  <text x="480" y="198" class="svg-kicker" fill="rgba(13,16,48,0.5)">REQUIRED EVIDENCE</text>
  <text x="480" y="220" class="svg-caption" fill="#0d1030">Reviewed durable intent</text>
  <text x="480" y="242" class="svg-caption" fill="#0d1030">Verification + recovery</text>
  <text x="480" y="272" class="svg-kicker" fill="rgba(13,16,48,0.5)">DECISION</text>
  <text x="480" y="292" class="svg-caption" fill="#0d1030">Owners accept release</text>
  <text x="480" y="310" class="svg-caption" fill="#0d1030">and service</text>
  <rect x="480" y="334" width="172" height="42" rx="9" fill="#a4238f" />
  <text x="566" y="360" text-anchor="middle" class="svg-label" fill="#ffffff">Full delivery path</text>
</svg>
</div>
<div class="ed-prose-col">

# Crossing a boundary<br>requires evidence.

<p class="ed-prose">A clearance is a change in permitted consequence. It names the trigger, the evidence required, and the authority that accepts the new reach or dependence.</p>
<p class="ed-prose"><strong>Automation produces evidence; it does not erase ownership.</strong> Local policy may add approvers without changing the three decisions.</p>

</div>
</div>

<!--
Clearances:
- Cleared to share: the work moves into a governed authoring environment.
- Cleared to connect: accountable owners accept a live capability for a stated purpose.
- Cleared to depend: release and service owners accept ongoing business dependence.
- There are no universal time or user-count thresholds. Clients calibrate local triggers; the evidence shape remains.
-->

---
title: Guidance shapes; gates decide
class: slide-light
---

<Eyebrow text="AUTHORING BOUNDARY · RULES LIVE WHERE THE WORK HAPPENS" />

<div class="ed-split">
<div class="ed-prose-col">

# Guidance shapes.<br>Gates decide.

<p class="ed-prose">Agent instructions, examples, and skeletons make the intended path easy to follow. Explicit grants and deterministic checks make the non-negotiable path hard to escape.</p>
<p class="ed-prose"><strong>The worker is a bounded execution environment of its own.</strong> Its filesystem, tools, network reach, and environment are grants—not assumptions inherited from the operator.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="16" y="24" width="608" height="384" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="40" y="54" class="svg-kicker" fill="#0c7fa3">CODING AGENT · BOUNDED WORKER</text>

  <rect x="40" y="82" width="250" height="224" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="60" y="110" class="svg-kicker" fill="#0c7fa3">GUIDANCE · ADVISORY</text>
  <text x="60" y="140" class="svg-label" fill="#0d1030">Agent and editor instructions</text>
  <text x="60" y="168" class="svg-label" fill="#0d1030">Approved references and examples</text>
  <text x="60" y="196" class="svg-label" fill="#0d1030">Known project skeletons</text>
  <text x="60" y="224" class="svg-label" fill="#0d1030">Design and slice context</text>
  <line x1="60" y1="248" x2="270" y2="248" stroke="rgba(13,16,48,0.12)" />
  <text x="60" y="276" class="svg-caption" fill="#0c7fa3">Makes the safe path the shortest path</text>

  <line x1="290" y1="194" x2="340" y2="194" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="334,189 344,194 334,199" fill="#0d1030" />
  <text x="316" y="180" text-anchor="middle" class="svg-caption" fill="#0d1030">Proposal</text>

  <rect x="346" y="82" width="254" height="224" rx="12" fill="#0d1030" />
  <text x="366" y="110" class="svg-kicker" fill="#78d8f2">ENFORCEMENT · REQUIRED</text>
  <text x="366" y="140" class="svg-label" fill="#ffffff">Lent workspace and shell roots</text>
  <text x="366" y="168" class="svg-label" fill="#ffffff">Explicit read, env, egress grants</text>
  <text x="366" y="196" class="svg-label" fill="#ffffff">Secret, dependency, test gates</text>
  <text x="366" y="224" class="svg-label" fill="#ffffff">Protected publication</text>
  <line x1="366" y1="248" x2="580" y2="248" stroke="rgba(255,255,255,0.14)" />
  <text x="366" y="276" class="svg-caption" fill="#78d8f2">Blocks what cannot be discretionary</text>

  <rect x="72" y="330" width="496" height="42" rx="21" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.4)" />
  <text x="320" y="356" text-anchor="middle" class="svg-label" fill="#a4238f">The model can use a grant. It cannot infer, create, or widen one.</text>

  <text x="320" y="426" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">Guidance improves proposals. Enforcement defines what may advance.</text>
</svg>
</div>
</div>

<!--
Authoring controls:
- Guidance is model-visible context. It influences a candidate but is not a security boundary.
- Enforcement lives in the worker sandbox, tool schemas, VCS policy, and deterministic gates.
- In Emery builds, a target adapter declares the worker reach its toolchain needs; the host applies that grant.
- A source extraction lend is read-only. Build and verify turns receive writable working copies, not the operator's checkout.
-->

---
title: Emery keeps the record outside the chat
class: slide-light
---

<Eyebrow text="EMERY · EVIDENCE, AUTHORITY, VERSIONED INTENT, BOUNDED BUILD" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Emery keeps the record<br>outside the chat.

</div>
<p class="ed-band-prose">Emery uses models inside a typed delivery process. Source adapters extract evidence; configured authority resolves agreement and divergence; reviewed intent is sealed as a content-addressed revision; isolated work is verified before the revision advances.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <text x="18" y="26" class="svg-kicker" fill="#0c7fa3">AUTHORITATIVE INPUTS</text>
  <rect x="18" y="46" width="208" height="78" rx="11" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.4" />
  <text x="36" y="74" class="svg-label" fill="#0d1030">Intent and documentation</text>
  <text x="36" y="98" class="svg-caption" fill="rgba(13,16,48,0.62)">Briefs · directives · decisions</text>
  <rect x="18" y="136" width="208" height="78" rx="11" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.4" />
  <text x="36" y="164" class="svg-label" fill="#0d1030">Observed behaviour</text>
  <text x="36" y="188" class="svg-caption" fill="rgba(13,16,48,0.62)">Code · tests · runtime facts</text>
  <rect x="18" y="226" width="208" height="78" rx="11" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.4" />
  <text x="36" y="254" class="svg-label" fill="#0d1030">Change evidence</text>
  <text x="36" y="278" class="svg-caption" fill="rgba(13,16,48,0.62)">New intent · retirements · constraints</text>

  <path d="M 226 85 C 264 85, 264 175, 298 175" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <path d="M 226 175 L 298 175" fill="none" stroke="#0d1030" stroke-width="1.6" />
  <path d="M 226 265 C 264 265, 264 175, 298 175" fill="none" stroke="#a4238f" stroke-width="1.6" />
  <polygon points="292,170 302,175 292,180" fill="#0d1030" />

  <rect x="304" y="76" width="214" height="198" rx="14" fill="#0d1030" />
  <text x="326" y="106" class="svg-kicker" fill="#78d8f2">EVIDENCE + AUTHORITY</text>
  <text x="326" y="140" class="svg-label" fill="#ffffff">Typed claims and anchors</text>
  <text x="326" y="170" class="svg-label" fill="#ffffff">Configured authority rank</text>
  <text x="326" y="200" class="svg-label" fill="#ffffff">Conflicts kept visible</text>
  <text x="326" y="230" class="svg-label" fill="#ffffff">[unknown] stays unknown</text>
  <text x="326" y="258" class="svg-caption" fill="#f59ef5">Model proposes · gates verify</text>

  <line x1="518" y1="175" x2="568" y2="175" stroke="#a4238f" stroke-width="2" />
  <polygon points="562,170 572,175 562,180" fill="#a4238f" />

  <rect x="574" y="76" width="248" height="198" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="596" y="106" class="svg-kicker" fill="#a4238f">VERSIONED DELIVERY REVISION</text>
  <rect x="596" y="126" width="204" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.28)" />
  <text x="612" y="149" class="svg-label" fill="#0d1030">Specification · behaviour</text>
  <rect x="596" y="172" width="204" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.28)" />
  <text x="612" y="195" class="svg-label" fill="#0d1030">Design · technology</text>
  <rect x="596" y="218" width="204" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.28)" />
  <text x="612" y="241" class="svg-label" fill="#0d1030">Plan · dependencies</text>

  <line x1="822" y1="175" x2="872" y2="175" stroke="#0d1030" stroke-width="2" />
  <polygon points="866,170 876,175 866,180" fill="#0d1030" />

  <rect x="878" y="76" width="244" height="198" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="900" y="106" class="svg-kicker" fill="#0c7fa3">BOUNDED BUILD</text>
  <text x="900" y="140" class="svg-label" fill="#0d1030">Dependency-ready waves</text>
  <text x="900" y="170" class="svg-label" fill="#0d1030">Isolated slice working copies</text>
  <text x="900" y="200" class="svg-label" fill="#0d1030">Deterministic merge policy</text>
  <text x="900" y="230" class="svg-label" fill="#0d1030">Integrated verification</text>
  <text x="900" y="258" class="svg-caption" fill="#0c7fa3">Labelled, resumable revision</text>

  <rect x="188" y="320" width="764" height="38" rx="19" fill="#0d1030" />
  <text x="570" y="344" text-anchor="middle" class="svg-label" fill="#ffffff">Conversation is replaceable. Evidence, authority, revision, and verification persist.</text>
</svg>
</div>
</div>

<!--
Emery mechanics:
- Source adapters turn source-specific material into typed, anchored evidence.
- Authority rank is configured or follows a source-kind default. Equal winning authority remains a conflict.
- Models answer typed questions; deterministic review gates reject candidates that contradict engine facts.
- A revision contains a specification, design, and dependency-aware plan and is content-addressed.
- Build slices run in isolated working copies. Ready slices build concurrently; integration and verification advance in waves.
- Emery labels the verified repository state. It does not push or confer release authority.
-->

---
title: Omnia gives code no ambient authority
class: slide-sandbox
---

<Eyebrow text="OMNIA · SANDBOXED COMPONENTS AND HOST-MEDIATED CAPABILITIES" />

<div class="ed-split">
<div class="ed-prose-col">

# Omnia gives code<br>no ambient authority.

<p class="ed-prose" style="color: rgba(255,255,255,0.76);">Business logic runs as a WebAssembly component. It has no ambient filesystem, network, process, credential, or environment access. The host supplies only the typed capabilities linked for that deployment.</p>
<p class="ed-prose" style="color: rgba(255,255,255,0.76);"><strong style="color: #ffffff;">No capability means no path.</strong> Credentials, concrete backends, egress, configuration, and policy remain in the host boundary.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="18" y="24" width="604" height="382" rx="16" fill="#0d1030" />
  <text x="42" y="54" class="svg-kicker" fill="#78d8f2">OMNIA HOST · POLICY AND AUTHORITY</text>
  <text x="598" y="54" text-anchor="end" class="svg-caption" fill="rgba(255,255,255,0.56)">Rust host on Wasmtime</text>

  <rect x="74" y="84" width="492" height="110" rx="14" fill="rgba(255,255,255,0.08)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="320" y="112" text-anchor="middle" class="svg-kicker" fill="#78d8f2">SANDBOXED WASI APPLICATION COMPONENT</text>
  <text x="320" y="146" text-anchor="middle" class="svg-h2" fill="#ffffff">Differentiating business behaviour</text>
  <text x="320" y="174" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.64)">No provider SDKs · No credentials · No ambient access</text>

  <rect x="204" y="210" width="232" height="32" rx="16" fill="rgba(164,35,143,0.22)" stroke="#f59ef5" />
  <text x="320" y="231" text-anchor="middle" class="svg-kicker" fill="#f59ef5">TYPED CAPABILITY CONTRACTS</text>

  <line x1="158" y1="194" x2="158" y2="258" stroke="#78d8f2" stroke-width="1.7" />
  <polygon points="153,252 158,262 163,252" fill="#78d8f2" />
  <line x1="320" y1="194" x2="320" y2="258" stroke="#78d8f2" stroke-width="1.7" />
  <polygon points="315,252 320,262 325,252" fill="#78d8f2" />
  <line x1="482" y1="194" x2="482" y2="258" stroke="#78d8f2" stroke-width="1.7" />
  <polygon points="477,252 482,262 487,252" fill="#78d8f2" />

  <rect x="74" y="264" width="168" height="68" rx="10" fill="rgba(255,255,255,0.09)" />
  <text x="158" y="292" text-anchor="middle" class="svg-label" fill="#ffffff">Data and state</text>
  <text x="158" y="316" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.58)">SQL · key-value · blob</text>
  <rect x="236" y="264" width="168" height="68" rx="10" fill="rgba(255,255,255,0.09)" />
  <text x="320" y="292" text-anchor="middle" class="svg-label" fill="#ffffff">Flow and calls</text>
  <text x="320" y="316" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.58)">Messaging · HTTP</text>
  <rect x="398" y="264" width="168" height="68" rx="10" fill="rgba(255,255,255,0.09)" />
  <text x="482" y="292" text-anchor="middle" class="svg-label" fill="#ffffff">Trust and insight</text>
  <text x="482" y="316" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.58)">Identity · telemetry</text>

  <rect x="74" y="352" width="492" height="40" rx="10" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.18)" />
  <text x="94" y="377" class="svg-kicker" fill="#78d8f2">HOST-SUPPLIED</text>
  <text x="216" y="377" class="svg-caption" fill="#ffffff">Backends · credentials · egress · configuration · observability</text>

  <text x="320" y="426" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.56)">The sandbox complements infrastructure and credential policy; it does not replace them.</text>
</svg>
</div>
</div>

<!--
Omnia mechanics:
- A component imports typed capabilities rather than provider-specific SDKs or credentials.
- The host decides which implementations are linked and carries concrete infrastructure policy.
- The component boundary removes ambient authority, but every linked capability is still a real grant.
- A compromised host, an overbroad capability implementation, or unsafe infrastructure policy remains outside this boundary.
-->

---
title: The two sandboxes are different
class: slide-light
---

<Eyebrow text="BOUNDARIES · CODING AGENT VERSUS RUNNING APPLICATION" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 320" aria-hidden="true">
  <rect x="18" y="18" width="536" height="276" rx="15" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="42" y="50" class="svg-kicker" fill="#0c7fa3">AUTHORING SANDBOX · CODING AGENT</text>
  <text x="42" y="84" class="svg-h2" fill="#0d1030">Protect the work from the worker</text>
  <line x1="42" y1="102" x2="530" y2="102" stroke="rgba(13,16,48,0.12)" />
  <text x="42" y="134" class="svg-kicker" fill="rgba(13,16,48,0.5)">PRINCIPAL</text>
  <text x="180" y="134" class="svg-label" fill="#0d1030">Model completion and its shell</text>
  <text x="42" y="166" class="svg-kicker" fill="rgba(13,16,48,0.5)">OBJECT</text>
  <text x="180" y="166" class="svg-label" fill="#0d1030">Lent source or isolated working copy</text>
  <text x="42" y="198" class="svg-kicker" fill="rgba(13,16,48,0.5)">AUTHORITY</text>
  <text x="180" y="198" class="svg-label" fill="#0d1030">Tools, paths, environment, egress</text>
  <text x="42" y="230" class="svg-kicker" fill="rgba(13,16,48,0.5)">OUTPUT</text>
  <text x="180" y="230" class="svg-label" fill="#0d1030">Proposed file changes and reports</text>
  <text x="42" y="270" class="svg-caption" fill="#0c7fa3">Enforced by worker boundary, tools, VCS, and gates</text>

  <rect x="586" y="18" width="536" height="276" rx="15" fill="#0d1030" />
  <text x="610" y="50" class="svg-kicker" fill="#78d8f2">RUNTIME SANDBOX · APPLICATION COMPONENT</text>
  <text x="610" y="84" class="svg-h2" fill="#ffffff">Protect operation from the code</text>
  <line x1="610" y1="102" x2="1098" y2="102" stroke="rgba(255,255,255,0.14)" />
  <text x="610" y="134" class="svg-kicker" fill="rgba(255,255,255,0.5)">PRINCIPAL</text>
  <text x="748" y="134" class="svg-label" fill="#ffffff">WebAssembly business component</text>
  <text x="610" y="166" class="svg-kicker" fill="rgba(255,255,255,0.5)">OBJECT</text>
  <text x="748" y="166" class="svg-label" fill="#ffffff">Live data, services, and effects</text>
  <text x="610" y="198" class="svg-kicker" fill="rgba(255,255,255,0.5)">AUTHORITY</text>
  <text x="748" y="198" class="svg-label" fill="#ffffff">Linked typed capabilities</text>
  <text x="610" y="230" class="svg-kicker" fill="rgba(255,255,255,0.5)">OUTPUT</text>
  <text x="748" y="230" class="svg-label" fill="#ffffff">Bounded business behaviour</text>
  <text x="610" y="270" class="svg-caption" fill="#78d8f2">Enforced by component sandbox, contracts, and host</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# The two sandboxes<br>are different.

</div>
<div>
<p class="ed-prose"><strong>The authoring sandbox contains the agent while it changes code.</strong> The runtime sandbox contains the resulting code while it handles real work.</p>
<p class="ed-prose"><strong>Neither implies the other.</strong> A tightly constrained agent can produce an ambiently privileged application; sandboxed application code can still be authored through an unsafe worker.</p>
</div>
</div>
</div>

<!--
Boundary comparison:
- They have different principals, protected objects, grant vocabularies, and enforcement mechanisms.
- The authoring boundary is temporary and work-oriented.
- The runtime boundary is deployed and behaviour-oriented.
- The framework needs both because generated code crosses from one trust domain into another.
-->

---
title: Verification and release are different decisions
class: slide-light
---

<Eyebrow text="PRODUCTION RAIL · FROM VERIFIED REVISION TO AUTHORISED OPERATION" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Verification and release<br>are different decisions.

</div>
<p class="ed-band-prose">Emery can prove that a repository state satisfies its declared build and verification process. It cannot decide that the organisation should release it. Reusable delivery assets carry the verified result through a separately owned operational path.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="72" width="184" height="144" rx="13" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="38" y="102" class="svg-kicker" fill="#a4238f">EMERY</text>
  <text x="38" y="132" class="svg-h3" fill="#0d1030">Verified revision</text>
  <text x="38" y="160" class="svg-caption" fill="rgba(13,16,48,0.62)">Specification · design · plan</text>
  <text x="38" y="182" class="svg-caption" fill="rgba(13,16,48,0.62)">Built and verified repository</text>
  <text x="38" y="204" class="svg-caption" fill="#a4238f">Content-addressed label</text>

  <line x1="202" y1="144" x2="250" y2="144" stroke="#0d1030" stroke-width="2" />
  <polygon points="244,139 254,144 244,149" fill="#0d1030" />

  <rect x="256" y="72" width="184" height="144" rx="13" fill="#0d1030" />
  <text x="276" y="102" class="svg-kicker" fill="#78d8f2">RELEASE AUTHORITY</text>
  <text x="276" y="132" class="svg-h3" fill="#ffffff">Release decision</text>
  <text x="276" y="160" class="svg-caption" fill="rgba(255,255,255,0.7)">Risk and timing accepted</text>
  <text x="276" y="182" class="svg-caption" fill="rgba(255,255,255,0.7)">Change authority recorded</text>
  <text x="276" y="204" class="svg-caption" fill="#f59ef5">Named release owner</text>

  <line x1="440" y1="144" x2="488" y2="144" stroke="#0d1030" stroke-width="2" />
  <polygon points="482,139 492,144 482,149" fill="#0d1030" />

  <rect x="494" y="72" width="184" height="144" rx="13" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="514" y="102" class="svg-kicker" fill="#0c7fa3">REUSABLE ASSETS</text>
  <text x="514" y="132" class="svg-h3" fill="#0d1030">Provision and deploy</text>
  <text x="514" y="160" class="svg-caption" fill="rgba(13,16,48,0.62)">Versioned infrastructure</text>
  <text x="514" y="182" class="svg-caption" fill="rgba(13,16,48,0.62)">Environment policy</text>
  <text x="514" y="204" class="svg-caption" fill="#0c7fa3">Repeatable promotion path</text>

  <line x1="678" y1="144" x2="726" y2="144" stroke="#0d1030" stroke-width="2" />
  <polygon points="720,139 730,144 720,149" fill="#0d1030" />

  <rect x="732" y="72" width="184" height="144" rx="13" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="752" y="102" class="svg-kicker" fill="#0c7fa3">OMNIA HOST</text>
  <text x="752" y="132" class="svg-h3" fill="#0d1030">Run with policy</text>
  <text x="752" y="160" class="svg-caption" fill="rgba(13,16,48,0.62)">Capabilities linked</text>
  <text x="752" y="182" class="svg-caption" fill="rgba(13,16,48,0.62)">Credentials held outside</text>
  <text x="752" y="204" class="svg-caption" fill="#0c7fa3">Observable runtime boundary</text>

  <line x1="916" y1="144" x2="964" y2="144" stroke="#a4238f" stroke-width="2" />
  <polygon points="958,139 968,144 958,149" fill="#a4238f" />

  <rect x="970" y="72" width="152" height="144" rx="13" fill="#a4238f" />
  <text x="1046" y="102" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.82)">SERVICE OWNER</text>
  <text x="1046" y="132" text-anchor="middle" class="svg-h3" fill="#ffffff">Observe</text>
  <text x="1046" y="156" text-anchor="middle" class="svg-h3" fill="#ffffff">and recover</text>
  <text x="1046" y="184" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">Operational evidence</text>
  <text x="1046" y="204" text-anchor="middle" class="svg-caption" fill="#ffffff">Ongoing ownership</text>

  <rect x="170" y="264" width="800" height="44" rx="22" fill="#0d1030" />
  <text x="570" y="292" text-anchor="middle" class="svg-label" fill="#ffffff">Verified means the declared checks passed. Released means an accountable owner accepted operation.</text>

  <text x="570" y="338" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">Emery labels repository state; it does not push. Omnia enforces runtime reach; it does not set release policy.</text>
</svg>
</div>
</div>

<!--
Production rail:
- Verification is technical evidence against a declared process.
- Release is an organisational decision about risk, timing, and ownership.
- Reusable infrastructure and pipeline assets reduce repeated work between those decisions.
- Omnia starts enforcing application authority when the deployed component runs.
- Observation, recovery, and service ownership continue after release.
-->

---
title: The boundary is explicit; the limitations are too
class: slide-light
---

<Eyebrow text="THREAT MODEL · WHAT THE FRAMEWORK CONTROLS AND WHAT REMAINS TRUSTED" />

<div class="ed-split">
<div class="ed-prose-col">

# The boundary is explicit.<br>The limitations are too.

<p class="ed-prose">The framework reduces reliance on model behaviour by moving authority into typed records, grants, sandboxes, and gates. It does not turn a bad requirement, a bad grant, or a compromised host into a safe system.</p>
<p class="ed-prose"><strong>Security review starts with the trust boundary, not with the word “AI”.</strong></p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="16" y="24" width="292" height="382" rx="15" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="38" y="54" class="svg-kicker" fill="#0c7fa3">CONTROLS PROVIDED</text>
  <text x="38" y="88" class="svg-label" fill="#0d1030">Guidance is not authority</text>
  <text x="38" y="122" class="svg-label" fill="#0d1030">Worker and tool reach is granted</text>
  <text x="38" y="156" class="svg-label" fill="#0d1030">Source conflicts stay visible</text>
  <text x="38" y="190" class="svg-label" fill="#0d1030">Revisions are traceable</text>
  <text x="38" y="224" class="svg-label" fill="#0d1030">Build work is isolated</text>
  <text x="38" y="258" class="svg-label" fill="#0d1030">Secret and spec rewrites are gated</text>
  <text x="38" y="292" class="svg-label" fill="#0d1030">Runtime reach is capability-bound</text>
  <line x1="38" y1="326" x2="286" y2="326" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="356" class="svg-caption" fill="#0c7fa3">Inspect: rules · grants · revisions · contracts</text>
  <text x="38" y="378" class="svg-caption" fill="#0c7fa3">labels · evidence · host configuration</text>

  <rect x="332" y="24" width="292" height="382" rx="15" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="354" y="54" class="svg-kicker" fill="#a4238f">REMAINS TRUSTED OR EXTERNAL</text>
  <text x="354" y="88" class="svg-label" fill="#0d1030">Intent and acceptance criteria</text>
  <text x="354" y="122" class="svg-label" fill="#0d1030">People setting authority and release</text>
  <text x="354" y="156" class="svg-label" fill="#0d1030">Adapter and toolchain code</text>
  <text x="354" y="190" class="svg-label" fill="#0d1030">Host, runtime, capability providers</text>
  <text x="354" y="224" class="svg-label" fill="#0d1030">Scope of every grant</text>
  <text x="354" y="258" class="svg-label" fill="#0d1030">Infrastructure, IAM, network policy</text>
  <text x="354" y="292" class="svg-label" fill="#0d1030">Defects beyond the checks</text>
  <line x1="354" y1="326" x2="602" y2="326" stroke="rgba(164,35,143,0.16)" />
  <text x="354" y="356" class="svg-caption" fill="#a4238f">Review: trusted code · grant scope · host policy</text>
  <text x="354" y="378" class="svg-caption" fill="#a4238f">release controls · operational ownership</text>
</svg>
</div>
</div>

<!--
Threat-model scope:
- The framework makes authority concrete and inspectable; it does not make every trusted component trustworthy.
- Guidance can be ignored by a model. Grants, filesystem lends, VCS checks, typed contracts, and runtime sandboxes are enforcement.
- A capability is authority by design. The host must still implement and scope it correctly.
- Evidence and tests bound what has been demonstrated, never all possible defects.
- Omnia complements infrastructure security and isolation. Emery complements release governance.
-->

---
title: Inspect the boundary
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>Open contracts, inspectable grants, reproducible evidence, and limitations stated alongside the claims.</p>
</div>
</div>

<div class="split-content-pane">
<div>
<Eyebrow text="OPEN SOURCE · EVIDENCE OVER ASSURANCE" />

# Inspect the boundary.<br>Replay the evidence.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Read the contracts and implementation</h4>
<p>Omnia: github.com/augentic/omnia · Emery: github.com/augentic/emery</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Review the authority artefacts</h4>
<p>Source precedence, worker grants, typed capabilities, revision labels, release ownership, and host policy.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Repeat the checks that support the claim</h4>
<p>Use the specification, fixed replay suite, build evidence, and one deliberately open failure—not a model demo.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">INTERNAL RAIL EVALUATION · OCTOBER 2026 · FIXED 22-CASE SUITE</div>
<div class="cta-card-title">21 / 22 · 12 / 12 recorded production messages</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>One open</span>
</div>
</div>
</div>

<!--
Close:
- Omnia and Emery are open source under MIT OR Apache-2.0.
- The rail result is an internal, anonymised evaluation against the same fixed 22-case replay suite used for the hand-written reference.
- The latest build passes 21 cases, including all 12 recorded production messages. One case remains open and reported.
- The score is not the product claim. The inspectable boundary and the compounding delivery system are.
-->
