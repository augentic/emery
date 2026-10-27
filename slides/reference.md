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

<p class="section-subtitle">Three mechanisms, the trust radius,<br>and the boundaries they enforce.</p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Standardise delivery · Codex, compiler-enforced rules, automated gates</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Specify behaviour · Typed evidence, sealed specification, verified build</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Sandbox execution · WASI components, capability contracts, runtime recipes</span></div>
</div>

<!--
Use this deck selectively:
- It is a technical reference for architecture, engineering, security, platform, and governance conversations.
- The approach deck carries the proposition as three mechanisms: standardise delivery, specify behaviour, sandbox execution. This deck answers how each works, what a reviewer can inspect, and what the system does not claim.
- The trust radius sets the required state from the highest-consequence dimension: reach, data, dependence, or continuity. It is not a maturity score.
-->

---
title: One system, three mechanisms
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="SYSTEM VIEW · CONTROL PLANE, THREE MECHANISMS, ONE PATH" />

# One system. Three mechanisms.

<p class="slide-lead">The model may work inside the path. It cannot grant itself authority, rewrite the specification, or authorise release.</p>

<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 400" aria-hidden="true">
  <rect x="122" y="4" width="896" height="38" rx="19" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.26)" />
  <text x="570" y="28" text-anchor="middle" class="svg-kicker" fill="#ffffff">PROTECTED CONTROL PLANE · PEOPLE SET OUTCOME, SOURCE PRECEDENCE, ACCEPTANCE, RELEASE</text>

  <rect x="18" y="72" width="348" height="192" rx="15" fill="rgba(10,7,16,0.74)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="42" y="102" class="svg-kicker" fill="#78d8f2">01 · STANDARDISE DELIVERY</text>
  <text x="42" y="134" class="svg-h3" fill="#ffffff">Toolchain and pipeline</text>
  <line x1="42" y1="150" x2="342" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="42" y="178" class="svg-body" fill="rgba(255,255,255,0.82)">Governed workspaces · approved tools and models</text>
  <text x="42" y="202" class="svg-body" fill="rgba(255,255,255,0.82)">Engineering codex · compiler-enforced standards</text>
  <text x="42" y="226" class="svg-body" fill="rgba(255,255,255,0.82)">Build once · test · gate · promote</text>
  <text x="42" y="252" class="svg-caption" fill="#78d8f2">How work is created, checked, shipped, and operated</text>

  <line x1="366" y1="168" x2="390" y2="168" stroke="#ffffff" stroke-width="1.8" />
  <polygon points="384,163 394,168 384,173" fill="#ffffff" />

  <rect x="396" y="72" width="348" height="192" rx="15" fill="rgba(10,7,16,0.74)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="420" y="102" class="svg-kicker" fill="#f59ef5">02 · SPECIFY BEHAVIOUR</text>
  <text x="420" y="134" class="svg-h3" fill="#ffffff">Emery</text>
  <line x1="420" y1="150" x2="720" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="178" class="svg-body" fill="rgba(255,255,255,0.82)">Typed evidence under configured authority</text>
  <text x="420" y="202" class="svg-body" fill="rgba(255,255,255,0.82)">Sealed spec.md and design.md · derived plan</text>
  <text x="420" y="226" class="svg-body" fill="rgba(255,255,255,0.82)">Isolated build · integration · verification</text>
  <text x="420" y="252" class="svg-caption" fill="#f59ef5">What must remain true and how acceptance is proven</text>

  <line x1="744" y1="168" x2="768" y2="168" stroke="#ffffff" stroke-width="1.8" />
  <polygon points="762,163 772,168 762,173" fill="#ffffff" />

  <rect x="774" y="72" width="348" height="192" rx="15" fill="rgba(10,7,16,0.74)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="798" y="102" class="svg-kicker" fill="#78d8f2">03 · SANDBOX EXECUTION</text>
  <text x="798" y="134" class="svg-h3" fill="#ffffff">Omnia</text>
  <line x1="798" y1="150" x2="1098" y2="150" stroke="rgba(255,255,255,0.14)" />
  <text x="798" y="178" class="svg-body" fill="rgba(255,255,255,0.82)">Sandboxed WASI component on Wasmtime</text>
  <text x="798" y="202" class="svg-body" fill="rgba(255,255,255,0.82)">Typed capability contracts</text>
  <text x="798" y="226" class="svg-body" fill="rgba(255,255,255,0.82)">Runtime recipe · bindings, policy, credentials</text>
  <text x="798" y="252" class="svg-caption" fill="#78d8f2">What running code may access</text>

  <line x1="192" y1="264" x2="192" y2="290" stroke="#78d8f2" stroke-width="1.5" />
  <line x1="570" y1="264" x2="570" y2="290" stroke="#f59ef5" stroke-width="1.5" />
  <line x1="948" y1="264" x2="948" y2="290" stroke="#78d8f2" stroke-width="1.5" />

  <rect x="18" y="292" width="1104" height="34" rx="10" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.18)" />
  <text x="42" y="314" class="svg-kicker" fill="#78d8f2">ONE PATH</text>
  <text x="176" y="314" class="svg-label" fill="#ffffff">Specify</text>
  <text x="246" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="276" y="314" class="svg-label" fill="#ffffff">Build</text>
  <text x="330" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="360" y="314" class="svg-label" fill="#ffffff">Test</text>
  <text x="406" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="436" y="314" class="svg-label" fill="#ffffff">Analyse</text>
  <text x="508" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="538" y="314" class="svg-label" fill="#ffffff">Deploy</text>
  <text x="600" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="630" y="314" class="svg-label" fill="#ffffff">Observe</text>
  <text x="700" y="314" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="730" y="314" class="svg-label" fill="#ffffff">Recover</text>
  <text x="1098" y="314" text-anchor="end" class="svg-caption" fill="rgba(255,255,255,0.64)">People decide release on the path's evidence</text>

  <rect x="18" y="338" width="1104" height="52" rx="12" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.24)" />
  <text x="42" y="370" class="svg-kicker" fill="#78d8f2">TRUSTED INPUTS</text>
  <text x="176" y="370" class="svg-label" fill="#ffffff">Codex, rules, and grants</text>
  <text x="420" y="370" class="svg-label" fill="#ffffff">Evidence, precedence, sealed specification</text>
  <text x="798" y="370" class="svg-label" fill="#ffffff">Capability contracts and runtime recipe</text>
</svg>
</div>

<!--
System view:
- Standardise delivery governs how work is created, checked, shipped, and operated: the governed workspace, the engineering codex, the compiler-enforced rules, and the automated path from build to production.
- Specify behaviour governs what must remain true and what evidence accepts the result: Emery keeps typed evidence, configured authority, and the sealed specification outside conversation.
- Sandbox execution governs what the running application may access: Omnia runs the result as a sandboxed component over typed capabilities the host binds through a runtime recipe.
- Operation is the end of the path, not a fourth mechanism. Release is a decision people make on the evidence the path produces.
- The coding agent's working environment and the application's runtime sandbox are separate trust boundaries; a later slide compares them.
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
  <text x="544" y="187" class="svg-body" fill="#0d1030">Compiler lints, lends, worker grants, merge gates</text>
  <text x="920" y="187" class="svg-caption" fill="#0c7fa3">Use only what is granted</text>

  <text x="42" y="233" class="svg-label" fill="#0d1030">Runtime reach</text>
  <text x="274" y="233" class="svg-body" fill="#0d1030">Capability contract and runtime recipe</text>
  <text x="544" y="233" class="svg-body" fill="#0d1030">Typed interfaces, bound backends, host credentials</text>
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

<Eyebrow text="TRUST MODEL · REQUIRED SAFEGUARDS FOLLOW CONSEQUENCE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Trust is contextual,<br>not a maturity score.

</div>
<p class="ed-band-prose">Explore, Govern, Connect, and Support are situations to match, not levels to climb. <strong>The highest-consequence answer across reach, data, dependence, and continuity sets the required state.</strong> Choose the least costly state that matches the actual consequence.</p>
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

  <text x="570" y="328" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.56)">Assess reach · data · dependence · continuity. The furthest answer is the trust radius. Reassess when any one changes.</text>
</svg>
</div>
</div>

<!--
Trust Model:
- These are operational states, not levels of organisational maturity.
- A system should remain in the least costly state consistent with its actual consequence.
- The state follows the highest-consequence dimension: reach, data, dependence, or continuity. The next slide is the assessment.
- States can change in either direction as use and authority change.
- Avoid fixed user-count or estate-percentage thresholds unless the client adopts them as local policy.
-->

---
title: The highest consequence sets the trust radius
class: slide-light
---

<Eyebrow text="TRUST RADIUS · FOUR DIMENSIONS ON ONE OPERATIONAL SCALE" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 320" aria-hidden="true">
  <rect x="18" y="18" width="1104" height="236" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <rect x="18" y="18" width="1104" height="48" rx="14" fill="#0d1030" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">DIMENSION</text>
  <text x="196" y="48" class="svg-kicker" fill="#78d8f2">QUESTION</text>
  <text x="456" y="48" class="svg-kicker" fill="#78d8f2">EXPLORE → GOVERN → CONNECT → SUPPORT</text>
  <text x="864" y="48" class="svg-kicker" fill="#78d8f2">REVIEWER INSPECTS</text>

  <line x1="170" y1="66" x2="170" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="430" y1="66" x2="430" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="838" y1="66" x2="838" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="113" x2="1122" y2="113" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="160" x2="1122" y2="160" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="207" x2="1122" y2="207" stroke="rgba(13,16,48,0.12)" />

  <text x="42" y="95" class="svg-label" fill="#0d1030">Reach</text>
  <text x="196" y="95" class="svg-body" fill="#0d1030">What can it access?</text>
  <text x="456" y="95" class="svg-body" fill="#0d1030">Local environment · shared systems · live capabilities</text>
  <text x="864" y="95" class="svg-caption" fill="#0c7fa3">Grants, capability contracts, recipe</text>

  <text x="42" y="142" class="svg-label" fill="#0d1030">Data</text>
  <text x="196" y="142" class="svg-body" fill="#0d1030">What can it read or change?</text>
  <text x="456" y="142" class="svg-body" fill="#0d1030">Synthetic data · project data · live or regulated data</text>
  <text x="864" y="142" class="svg-caption" fill="#0c7fa3">Bound backends, data owner, retention</text>

  <text x="42" y="189" class="svg-label" fill="#0d1030">Dependence</text>
  <text x="196" y="189" class="svg-body" fill="#0d1030">Who relies on it?</text>
  <text x="456" y="189" class="svg-body" fill="#0d1030">One builder · a team · customers or operations</text>
  <text x="864" y="189" class="svg-caption" fill="#a4238f">Owner, consumers, service ownership</text>

  <text x="42" y="236" class="svg-label" fill="#0d1030">Continuity</text>
  <text x="196" y="236" class="svg-body" fill="#0d1030">What must survive failure?</text>
  <text x="456" y="236" class="svg-body" fill="#0d1030">Discard · restore · recover within an obligation</text>
  <text x="864" y="236" class="svg-caption" fill="#a4238f">Recovery evidence, restore tests</text>

  <rect x="170" y="270" width="800" height="40" rx="20" fill="#0d1030" />
  <text x="570" y="295" text-anchor="middle" class="svg-label" fill="#ffffff">Take the furthest answer, not the average. The radius names the state; the state names the boundary.</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# The furthest answer<br>sets the radius.

</div>
<div>
<p class="ed-prose"><strong>Place each dimension on the same four-state scale and take the furthest answer, not the average.</strong> That is the application's trust radius: the widest consequence it can create or inherit.</p>
<p class="ed-prose">The radius can contract as well as expand. Reassess whenever reach, data, dependence, or continuity changes, and record what triggered the reassessment.</p>
</div>
</div>
</div>

<!--
Trust radius:
- Reach asks what systems, services, and capabilities the application can access.
- Data asks what it can read, retain, produce, or change, including the consequence of disclosure or corruption.
- Dependence asks who or what relies on it and what stops when it is unavailable or wrong.
- Continuity asks what must survive failure and what recovery obligation exists.
- Each answer is placed independently in Explore, Govern, Connect, or Support. The highest-consequence answer sets the required state and therefore its safeguards.
- The right-hand column names the artefacts a reviewer reads to check each answer rather than taking it on assertion.
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
  <text x="566" y="360" text-anchor="middle" class="svg-label" fill="#ffffff">Full delivery system</text>
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

<Eyebrow text="STANDARDISE DELIVERY · AUTHORING CONTROLS · ISOLATE, GUIDE, ENFORCE" />

<div class="ed-split">
<div class="ed-prose-col">

# Guidance shapes.<br>Gates decide.

<p class="ed-prose">Isolate work in governed, non-production workspaces. Guide people and agents with one versioned engineering codex of standards, patterns, examples, and templates. Enforce the non-negotiable rules where they cannot be skipped: critical rules fail at compile time, before review.</p>
<p class="ed-prose"><strong>The worker is a bounded execution environment of its own.</strong> Its filesystem, tools, network reach, and environment are grants—not assumptions inherited from the operator.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="16" y="24" width="608" height="384" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="40" y="54" class="svg-kicker" fill="#0c7fa3">CODING AGENT · GOVERNED WORKSPACE, BOUNDED WORKER</text>

  <rect x="40" y="82" width="250" height="224" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="60" y="110" class="svg-kicker" fill="#0c7fa3">GUIDE · ADVISORY</text>
  <text x="60" y="140" class="svg-label" fill="#0d1030">Shared engineering codex</text>
  <text x="60" y="168" class="svg-label" fill="#0d1030">Standards, patterns, examples</text>
  <text x="60" y="196" class="svg-label" fill="#0d1030">Templates and skeletons</text>
  <text x="60" y="224" class="svg-label" fill="#0d1030">Design and slice context</text>
  <line x1="60" y1="248" x2="270" y2="248" stroke="rgba(13,16,48,0.12)" />
  <text x="60" y="276" class="svg-caption" fill="#0c7fa3">Makes the safe path the shortest path</text>

  <line x1="290" y1="194" x2="340" y2="194" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="334,189 344,194 334,199" fill="#0d1030" />
  <text x="316" y="180" text-anchor="middle" class="svg-caption" fill="#0d1030">Proposal</text>

  <rect x="346" y="82" width="254" height="224" rx="12" fill="#0d1030" />
  <text x="366" y="110" class="svg-kicker" fill="#78d8f2">ENFORCE · REQUIRED</text>
  <text x="366" y="140" class="svg-label" fill="#ffffff">Compiler-enforced standards</text>
  <text x="366" y="168" class="svg-label" fill="#ffffff">Lent workspace and shell roots</text>
  <text x="366" y="196" class="svg-label" fill="#ffffff">Explicit read, env, egress grants</text>
  <text x="366" y="224" class="svg-label" fill="#ffffff">Secret, dependency, test gates</text>
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
- Isolate: work happens in governed, non-production workspaces for people and agents, never against production.
- Guide: the codex is model-visible context. It influences a candidate but is not a security boundary.
- Enforce: critical coding rules are compiler lints that fail the build. In the Omnia layout the host boundary is carried as disallowed methods, types, and macros, each with a reason naming the capability to reach instead, so a direct filesystem, network, or environment call does not compile. The same gates then apply to every change: secrets, dependencies, tests, protected publication.
- Enforcement also lives in the worker sandbox, tool schemas, and VCS policy. In Emery builds, a target adapter declares the worker reach its toolchain needs; the host applies that grant.
- A source extraction lend is read-only. Build and verify turns receive writable working copies, not the operator's checkout.
-->

---
title: Automate the path from code to production
class: slide-light
---

<Eyebrow text="STANDARDISE DELIVERY · DELIVERY AUTOMATION · ONE ARTIFACT, EVERY GATE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Automate the path<br>from code to production.

</div>
<p class="ed-band-prose"><strong>Faster implementation matters only when evidence and delivery move at the same speed.</strong> The pipeline builds one reproducible artifact, tests behaviour at every level, gates every change, and promotes the same artifact forward with its evidence. Automation removes mechanical delay; it does not remove the decision to accept a change.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="36" width="252" height="222" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="40" y="66" class="svg-kicker" fill="#0c7fa3">01 · BUILD</text>
  <text x="40" y="98" class="svg-h3" fill="#0d1030">Build once</text>
  <line x1="40" y1="114" x2="248" y2="114" stroke="rgba(12,127,163,0.18)" />
  <text x="40" y="144" class="svg-body" fill="#0d1030">Reproducible artifact</text>
  <text x="40" y="168" class="svg-body" fill="#0d1030">Promoted unchanged</text>
  <text x="40" y="206" class="svg-kicker" fill="rgba(13,16,48,0.5)">INSPECT</text>
  <text x="40" y="230" class="svg-caption" fill="#0c7fa3">Build record and artifact identity</text>

  <line x1="270" y1="147" x2="298" y2="147" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="292,142 302,147 292,152" fill="#0d1030" />

  <rect x="304" y="36" width="252" height="222" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.5" />
  <text x="326" y="66" class="svg-kicker" fill="#0d1030">02 · TEST</text>
  <text x="326" y="98" class="svg-h3" fill="#0d1030">Test behaviour</text>
  <line x1="326" y1="114" x2="534" y2="114" stroke="rgba(13,16,48,0.12)" />
  <text x="326" y="144" class="svg-body" fill="#0d1030">Unit · integration · end-to-end</text>
  <text x="326" y="168" class="svg-body" fill="#0d1030">Replay of recorded cases</text>
  <text x="326" y="206" class="svg-kicker" fill="rgba(13,16,48,0.5)">INSPECT</text>
  <text x="326" y="230" class="svg-caption" fill="#0c7fa3">Test reports; the open failures</text>

  <line x1="556" y1="147" x2="584" y2="147" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="578,142 588,147 578,152" fill="#0d1030" />

  <rect x="590" y="36" width="252" height="222" rx="14" fill="#0d1030" />
  <text x="612" y="66" class="svg-kicker" fill="#78d8f2">03 · ANALYSE</text>
  <text x="612" y="98" class="svg-h3" fill="#ffffff">Gate every change</text>
  <line x1="612" y1="114" x2="820" y2="114" stroke="rgba(255,255,255,0.14)" />
  <text x="612" y="144" class="svg-body" fill="rgba(255,255,255,0.86)">Compiler · security · dependencies</text>
  <text x="612" y="168" class="svg-body" fill="rgba(255,255,255,0.86)">Secrets · automated review</text>
  <text x="612" y="206" class="svg-kicker" fill="rgba(255,255,255,0.5)">INSPECT</text>
  <text x="612" y="230" class="svg-caption" fill="#78d8f2">Gate results on the change itself</text>

  <line x1="842" y1="147" x2="870" y2="147" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="864,142 874,147 864,152" fill="#a4238f" />

  <rect x="876" y="36" width="246" height="222" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="898" y="66" class="svg-kicker" fill="#a4238f">04 · DEPLOY</text>
  <text x="898" y="98" class="svg-h3" fill="#0d1030">Promote the artifact</text>
  <line x1="898" y1="114" x2="1100" y2="114" stroke="rgba(164,35,143,0.18)" />
  <text x="898" y="144" class="svg-body" fill="#0d1030">Provision · deploy · release</text>
  <text x="898" y="168" class="svg-body" fill="#0d1030">Policy as code</text>
  <text x="898" y="206" class="svg-kicker" fill="rgba(13,16,48,0.5)">INSPECT</text>
  <text x="898" y="230" class="svg-caption" fill="#a4238f">Promotion record and release owner</text>

  <rect x="170" y="290" width="800" height="44" rx="22" fill="#0d1030" />
  <text x="570" y="318" text-anchor="middle" class="svg-label" fill="#ffffff">The same accepted artifact moves forward with its evidence. Release stays a human decision.</text>
</svg>
</div>
</div>

<!--
Delivery automation:
- These changes predate the current AI wave. They are what makes AI-assisted implementation operationally useful: evidence and delivery have to move as fast as the code does.
- Build produces one reproducible artifact; the same artifact is promoted through every environment rather than rebuilt per stage.
- Test automation runs unit, integration, and end-to-end evidence. Replay suites run recorded production cases against the build; an open failure is reported, not hidden.
- Analysis makes compiler, dependency, secret, security, and automated review controls part of every change.
- Pipeline automation carries an accepted change through provision, deployment, and release under policy as code.
- Prerequisites: testable systems, reliable suites, standard environments, and ownership of pipeline health.
-->

---
title: Emery keeps the specification outside the chat
class: slide-light
---

<Eyebrow text="SPECIFY BEHAVIOUR · EMERY · EVIDENCE, AUTHORITY, SEALED SPECIFICATION, BOUNDED BUILD" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Emery keeps the specification<br>outside the chat.

</div>
<p class="ed-band-prose">Emery runs spec-driven development as a typed process. Source adapters extract evidence; configured authority resolves agreement and divergence; the specification—spec.md and design.md—is reviewed and sealed in the repository as a content-addressed revision; the plan is derived from it; isolated work is built, integrated, and verified against it before the revision's label advances.</p>
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
  <text x="36" y="278" class="svg-caption" fill="rgba(13,16,48,0.62)">New intent · retirements</text>

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
  <text x="596" y="106" class="svg-kicker" fill="#a4238f">REVIEWED · SEALED · IN REPO</text>
  <rect x="596" y="126" width="204" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.28)" />
  <text x="612" y="149" class="svg-label" fill="#0d1030">spec.md · behaviour</text>
  <rect x="596" y="172" width="204" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.28)" />
  <text x="612" y="195" class="svg-label" fill="#0d1030">design.md · technology</text>
  <rect x="596" y="218" width="204" height="36" rx="8" fill="rgba(13,16,48,0.03)" stroke="rgba(13,16,48,0.3)" stroke-dasharray="5 4" />
  <text x="612" y="241" class="svg-label" fill="rgba(13,16,48,0.7)">Plan · derived, regenerable</text>

  <line x1="822" y1="175" x2="872" y2="175" stroke="#0d1030" stroke-width="2" />
  <polygon points="866,170 876,175 866,180" fill="#0d1030" />

  <rect x="878" y="76" width="244" height="198" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="900" y="106" class="svg-kicker" fill="#0c7fa3">BOUNDED BUILD</text>
  <text x="900" y="140" class="svg-label" fill="#0d1030">Dependency-ready waves</text>
  <text x="900" y="170" class="svg-label" fill="#0d1030">Isolated slice working copies</text>
  <text x="900" y="200" class="svg-label" fill="#0d1030">Deterministic merge policy</text>
  <text x="900" y="230" class="svg-label" fill="#0d1030">Verified against the spec</text>
  <text x="900" y="258" class="svg-caption" fill="#0c7fa3">Labelled, resumable revision</text>

  <rect x="188" y="320" width="764" height="38" rx="19" fill="#0d1030" />
  <text x="570" y="344" text-anchor="middle" class="svg-label" fill="#ffffff">The model, the plan, and the code are replaceable. The sealed specification and its evidence persist.</text>
</svg>
</div>
</div>

<!--
Emery mechanics:
- Source adapters turn source-specific material into typed, anchored evidence. In modernisation, legacy source code is one of those sources.
- Authority rank is configured or follows a source-kind default. Equal winning authority remains a conflict; a gap stays [unknown].
- Models answer typed questions; deterministic review gates reject candidates that contradict engine facts. No model turn authors a document.
- `emery specify` stages the specification, design, and plan as one content-addressed candidate; a person reviews it and `emery apply` seals it under `spec/` in the target repository at a commit labelled `emery/<id>`. `spec.md`, `design.md`, and the plan are Markdown projections of that typed revision through `emery show`.
- The plan is derived from the sealed specification by slicing requirements into dependency-ordered units of work. Re-running specify regenerates it; the specification is what is reviewed.
- Build slices run in isolated working copies cut from the integration head. Ready slices build concurrently; integration and verification advance in waves, and only a verified wave moves the revision's label.
- Emery labels the verified repository state. It does not push or confer release authority.
-->

---
title: Omnia gives code no ambient authority
class: slide-sandbox
---

<Eyebrow text="SANDBOX EXECUTION · OMNIA · SANDBOXED COMPONENTS AND HOST-MEDIATED CAPABILITIES" />

<div class="ed-split">
<div class="ed-prose-col">

# Omnia gives code<br>no ambient authority.

<p class="ed-prose" style="color: rgba(255,255,255,0.76);">Generated code has less freedom to surprise. Business logic runs as a WASI component in a Wasmtime sandbox with no ambient filesystem, network, process, credential, or environment access. The host supplies only the typed capabilities linked for that deployment.</p>
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
- The host decides which implementations are linked and carries concrete infrastructure policy. That binding is the runtime recipe on the next slide.
- The component boundary removes ambient authority, but every linked capability is still a real grant.
- A compromised host, an overbroad capability implementation, or unsafe infrastructure policy remains outside this boundary.
-->

---
title: Different infrastructure, the same application
class: slide-light
---

<Eyebrow text="SANDBOX EXECUTION · PORTABILITY · THE RUNTIME RECIPE" />

<div class="ed-split">
<div class="ed-prose-col">

# Different infrastructure.<br>Same application.

<p class="ed-prose">The component depends on stable capability contracts, not on infrastructure products. A runtime recipe—the runtime, the capability bindings, policy and identity, configuration and observability—binds those contracts to approved services in each environment.</p>
<p class="ed-prose"><strong>Where the capability semantics hold, change the recipe, not the application.</strong> The recipe is a versioned deployment asset, so the next application starts with a known operating shape.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="16" y="24" width="608" height="384" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />

  <rect x="200" y="46" width="240" height="44" rx="10" fill="rgba(124,92,205,0.12)" stroke="rgba(124,92,205,0.7)" stroke-width="1.8" />
  <text x="320" y="64" text-anchor="middle" class="svg-kicker" fill="#7557c7">APPLICATION UNCHANGED</text>
  <text x="320" y="82" text-anchor="middle" class="svg-label" fill="#0d1030">WASI Component</text>

  <line x1="320" y1="90" x2="320" y2="104" stroke="rgba(13,16,48,0.46)" stroke-width="1.6" />
  <polygon points="315,100 325,100 320,110" fill="#0d1030" />
  <text x="320" y="124" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">STABLE CAPABILITY CONTRACT</text>

  <rect x="40" y="132" width="132" height="30" rx="7" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.4" />
  <text x="106" y="152" text-anchor="middle" class="svg-caption" fill="#0d1030">HTTP</text>
  <rect x="182" y="132" width="132" height="30" rx="7" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.4" />
  <text x="248" y="152" text-anchor="middle" class="svg-caption" fill="#0d1030">Messaging</text>
  <rect x="324" y="132" width="132" height="30" rx="7" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.4" />
  <text x="390" y="152" text-anchor="middle" class="svg-caption" fill="#0d1030">KeyValue</text>
  <rect x="466" y="132" width="134" height="30" rx="7" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.4" />
  <text x="533" y="152" text-anchor="middle" class="svg-caption" fill="#0d1030">BlobStore · SQL · …</text>

  <line x1="320" y1="162" x2="320" y2="176" stroke="rgba(13,16,48,0.46)" stroke-width="1.6" />
  <polygon points="315,172 325,172 320,182" fill="#0d1030" />
  <text x="320" y="196" text-anchor="middle" class="svg-kicker" fill="#a4238f">RUNTIME RECIPE · ONE PER ENVIRONMENT</text>

  <rect x="40" y="206" width="560" height="134" rx="12" fill="rgba(164,35,143,0.04)" stroke="rgba(164,35,143,0.4)" stroke-width="1.5" />
  <text x="60" y="232" class="svg-label" fill="#0d1030">Runtime</text>
  <text x="250" y="232" class="svg-caption" fill="rgba(13,16,48,0.66)">Host build and version; which capabilities are linked</text>
  <text x="60" y="262" class="svg-label" fill="#0d1030">Capability bindings</text>
  <text x="250" y="262" class="svg-caption" fill="rgba(13,16,48,0.66)">Which approved service answers each contract</text>
  <text x="60" y="292" class="svg-label" fill="#0d1030">Policy + identity</text>
  <text x="250" y="292" class="svg-caption" fill="rgba(13,16,48,0.66)">Credentials held by the host; identity; egress policy</text>
  <text x="60" y="322" class="svg-label" fill="#0d1030">Config + observability</text>
  <text x="250" y="322" class="svg-caption" fill="rgba(13,16,48,0.66)">Settings, telemetry sinks, and what is logged</text>

  <rect x="40" y="354" width="176" height="40" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.4" />
  <text x="128" y="371" text-anchor="middle" class="svg-kicker" fill="rgba(13,16,48,0.58)">LOCAL / TRIAL</text>
  <text x="128" y="386" text-anchor="middle" class="svg-caption" fill="#0d1030">In-memory + local services</text>
  <rect x="232" y="354" width="176" height="40" rx="9" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.5)" stroke-width="1.4" />
  <text x="320" y="371" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">CLOUD</text>
  <text x="320" y="386" text-anchor="middle" class="svg-caption" fill="#0d1030">Managed services</text>
  <rect x="424" y="354" width="176" height="40" rx="9" fill="rgba(164,35,143,0.05)" stroke="rgba(164,35,143,0.44)" stroke-width="1.4" />
  <text x="512" y="371" text-anchor="middle" class="svg-kicker" fill="#a4238f">ON-PREMISE</text>
  <text x="512" y="386" text-anchor="middle" class="svg-caption" fill="#0d1030">Approved internal services</text>
</svg>
</div>
</div>

<!--
Runtime recipe:
- The application component depends on stable capability contracts rather than on infrastructure products; the contracts are the WIT interfaces the component imports.
- Each environment supplies a reviewed recipe: the runtime, the capability bindings, identity and policy, configuration, and observability. A local or trial recipe may bind in-memory and local services; cloud and on-premise recipes bind the same contracts to approved managed or internal services.
- The infrastructure is not literally identical. Its application-facing contract and deployment grammar are.
- The recipe is what a platform or security reviewer inspects for runtime reach: which backend answers each capability, which credentials the host holds, what the egress policy permits. None of that is in the component.
- Retaining the recipe as versioned infrastructure-as-code is what makes the path to production reusable.
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
<p class="ed-prose"><strong>The authoring sandbox contains the agent while it changes code</strong>—the Standardise delivery mechanism. The runtime sandbox contains the resulting code while it handles real work—the Sandbox execution mechanism.</p>
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

<Eyebrow text="ONE PATH · FROM VERIFIED REVISION TO AUTHORISED OPERATION" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Verification and release<br>are different decisions.

</div>
<p class="ed-band-prose">Emery can prove that a repository state satisfies its declared build and verification process. It cannot decide that the organisation should release it. Delivery automation carries the verified artifact, with its evidence, through a separately owned operational path.</p>
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
  <text x="514" y="102" class="svg-kicker" fill="#0c7fa3">DELIVERY AUTOMATION</text>
  <text x="514" y="132" class="svg-h3" fill="#0d1030">Promote the artifact</text>
  <text x="514" y="160" class="svg-caption" fill="rgba(13,16,48,0.62)">Built once · tested · gated</text>
  <text x="514" y="182" class="svg-caption" fill="rgba(13,16,48,0.62)">Provision · deploy</text>
  <text x="514" y="204" class="svg-caption" fill="#0c7fa3">Same artifact, with its evidence</text>

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
One path:
- Verification is technical evidence against a declared process.
- Release is an organisational decision about risk, timing, and ownership.
- Delivery automation, with the runtime recipe kept as versioned infrastructure-as-code, reduces repeated work between those decisions.
- Omnia starts enforcing application authority when the deployed component runs.
- Observation, recovery, and service ownership continue after release.
-->

---
title: The path becomes a loop
class: slide-light
---

<Eyebrow text="THE FUTURE · WHAT IS BUILT TODAY AND WHAT COMES NEXT" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Feedback drives<br>the next change.

</div>
<p class="ed-band-prose"><strong>Nothing bypasses the path.</strong> A production signal is investigated and a fix proposed; the fix is reviewed, specified, built, and merged like any other change, in a code lane and an infrastructure lane, then observed. The path exists today. The loop around it is the direction, not the claim.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="40" width="252" height="196" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="40" y="70" class="svg-kicker" fill="#0c7fa3">01 · SENSE</text>
  <text x="40" y="102" class="svg-h3" fill="#0d1030">Signals from production</text>
  <line x1="40" y1="118" x2="248" y2="118" stroke="rgba(12,127,163,0.18)" />
  <text x="40" y="148" class="svg-body" fill="#0d1030">Infrastructure monitoring</text>
  <text x="40" y="172" class="svg-body" fill="#0d1030">Security feeds · dependency audit</text>
  <text x="40" y="196" class="svg-body" fill="#0d1030">Incidents</text>
  <rect x="40" y="208" width="72" height="18" rx="9" fill="rgba(12,127,163,0.14)" />
  <text x="76" y="221" text-anchor="middle" class="svg-pill" fill="#0c7fa3">TODAY</text>

  <line x1="270" y1="138" x2="298" y2="138" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="292,133 302,138 292,143" fill="#0d1030" />

  <rect x="304" y="40" width="252" height="196" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="6 5" />
  <text x="326" y="70" class="svg-kicker" fill="#0d1030">02 · DECIDE</text>
  <text x="326" y="102" class="svg-h3" fill="#0d1030">Incident manager</text>
  <line x1="326" y1="118" x2="534" y2="118" stroke="rgba(13,16,48,0.12)" />
  <text x="326" y="148" class="svg-body" fill="#0d1030">Investigate the signal</text>
  <text x="326" y="172" class="svg-body" fill="#0d1030">Propose a fix</text>
  <text x="326" y="196" class="svg-body" fill="#0d1030">A proposal is not a shipment</text>
  <rect x="326" y="208" width="60" height="18" rx="9" fill="rgba(164,35,143,0.12)" />
  <text x="356" y="221" text-anchor="middle" class="svg-pill" fill="#a4238f">NEXT</text>

  <line x1="556" y1="138" x2="584" y2="138" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="578,133 588,138 578,143" fill="#0d1030" />

  <rect x="590" y="40" width="252" height="196" rx="14" fill="#0d1030" />
  <text x="612" y="70" class="svg-kicker" fill="#f59ef5">03 · CHANGE</text>
  <text x="612" y="102" class="svg-h3" fill="#ffffff">Review, seal, build, merge</text>
  <line x1="612" y1="118" x2="820" y2="118" stroke="rgba(255,255,255,0.14)" />
  <text x="612" y="148" class="svg-body" fill="rgba(255,255,255,0.86)">Code lane · spec.md, design.md</text>
  <text x="612" y="172" class="svg-body" fill="rgba(255,255,255,0.86)">Infrastructure lane · runtime recipe</text>
  <text x="612" y="196" class="svg-body" fill="rgba(255,255,255,0.86)">Same gates, same promotion</text>
  <rect x="612" y="208" width="72" height="18" rx="9" fill="rgba(120,216,242,0.18)" />
  <text x="648" y="221" text-anchor="middle" class="svg-pill" fill="#78d8f2">TODAY</text>

  <line x1="842" y1="138" x2="870" y2="138" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="864,133 874,138 864,143" fill="#a4238f" />

  <rect x="876" y="40" width="246" height="196" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="898" y="70" class="svg-kicker" fill="#a4238f">04 · OBSERVE</text>
  <text x="898" y="102" class="svg-h3" fill="#0d1030">New evidence</text>
  <line x1="898" y1="118" x2="1100" y2="118" stroke="rgba(164,35,143,0.18)" />
  <text x="898" y="148" class="svg-body" fill="#0d1030">Deployed and updated</text>
  <text x="898" y="172" class="svg-body" fill="#0d1030">Monitoring raises the next signal</text>
  <text x="898" y="196" class="svg-body" fill="#0d1030">Safeguards set by the trust radius</text>
  <rect x="898" y="208" width="72" height="18" rx="9" fill="rgba(164,35,143,0.12)" />
  <text x="934" y="221" text-anchor="middle" class="svg-pill" fill="#a4238f">TODAY</text>

  <path d="M 1000 236 C 1000 300, 140 300, 140 236" fill="none" stroke="rgba(13,16,48,0.4)" stroke-width="1.6" stroke-dasharray="5 4" />
  <polygon points="135,246 140,236 145,246" fill="rgba(13,16,48,0.6)" />
  <text x="570" y="304" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">A signal becomes a change; a change becomes new evidence</text>

  <rect x="222" y="312" width="696" height="32" rx="16" fill="rgba(164,35,143,0.06)" stroke="rgba(164,35,143,0.4)" />
  <text x="570" y="333" text-anchor="middle" class="svg-caption" fill="#a4238f">The incident manager proposes. People review and seal. The path ships. Nothing bypasses it.</text>
</svg>
</div>
</div>

<!--
The loop:
- Sense, change, and observe are the path as it stands: monitoring, audits, and incidents on one side; specify, build, merge, deploy, observe on the other.
- Decide is the part that is next: an incident manager that investigates a signal and proposes a fix. It proposes; it does not ship. The proposal enters at the review gate like any other change.
- The fix runs in two lanes with the same gates: a code lane over the sealed specification, and an infrastructure lane over the runtime recipe as versioned infrastructure-as-code.
- The safeguards on each pass are the ones the trust radius requires, so the loop tightens for a Support-state system and stays cheap for an Explore-state one.
- For a governance reviewer: the claim today is the path. The loop is the direction and is labelled as such.
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
  <text x="38" y="86" class="svg-label" fill="#0d1030">Guidance is not authority</text>
  <text x="38" y="116" class="svg-label" fill="#0d1030">Critical rules fail the build</text>
  <text x="38" y="146" class="svg-label" fill="#0d1030">Worker and tool reach is granted</text>
  <text x="38" y="176" class="svg-label" fill="#0d1030">Source conflicts stay visible</text>
  <text x="38" y="206" class="svg-label" fill="#0d1030">Revisions are traceable</text>
  <text x="38" y="236" class="svg-label" fill="#0d1030">Build work is isolated</text>
  <text x="38" y="266" class="svg-label" fill="#0d1030">Secret and spec rewrites are gated</text>
  <text x="38" y="296" class="svg-label" fill="#0d1030">Runtime reach is capability-bound</text>
  <line x1="38" y1="326" x2="286" y2="326" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="356" class="svg-caption" fill="#0c7fa3">Inspect: codex · lints · grants · revisions</text>
  <text x="38" y="378" class="svg-caption" fill="#0c7fa3">labels · contracts · runtime recipe</text>

  <rect x="332" y="24" width="292" height="382" rx="15" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="354" y="54" class="svg-kicker" fill="#a4238f">REMAINS TRUSTED OR EXTERNAL</text>
  <text x="354" y="86" class="svg-label" fill="#0d1030">Intent and acceptance criteria</text>
  <text x="354" y="116" class="svg-label" fill="#0d1030">People setting authority and release</text>
  <text x="354" y="146" class="svg-label" fill="#0d1030">The trust radius assessment itself</text>
  <text x="354" y="176" class="svg-label" fill="#0d1030">Adapter and toolchain code</text>
  <text x="354" y="206" class="svg-label" fill="#0d1030">Host, runtime, capability providers</text>
  <text x="354" y="236" class="svg-label" fill="#0d1030">Scope of every grant</text>
  <text x="354" y="266" class="svg-label" fill="#0d1030">Infrastructure, IAM, network policy</text>
  <text x="354" y="296" class="svg-label" fill="#0d1030">Defects beyond the checks</text>
  <line x1="354" y1="326" x2="602" y2="326" stroke="rgba(164,35,143,0.16)" />
  <text x="354" y="356" class="svg-caption" fill="#a4238f">Review: trusted code · grants · recipe · host</text>
  <text x="354" y="378" class="svg-caption" fill="#a4238f">release controls · operational ownership</text>
</svg>
</div>
</div>

<!--
Threat-model scope:
- The framework makes authority concrete and inspectable; it does not make every trusted component trustworthy.
- Guidance can be ignored by a model. Compiler lints, grants, filesystem lends, VCS checks, typed contracts, and runtime sandboxes are enforcement.
- The trust radius is an assessment people make. A wrong answer on reach, data, dependence, or continuity under-sizes the safeguards; the framework cannot detect that.
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

<div class="split-content-pane" style="justify-content: center;">
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
<p>Engineering codex and compiler lints, source precedence, worker grants, sealed revisions and labels, capability contracts, runtime recipes, release ownership.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Repeat the checks yourself</h4>
<p>Seal a specification, build against it, and run the tests, gates, and replays in your own environment—evidence from the path, not a model demo.</p>
</div>
</div>
</div>
</div>
</div>

<!--
Close:
- Omnia and Emery are open source under MIT OR Apache-2.0.
- Every control on the preceding slides is a contract, a grant, a lint, or a commit a reviewer can read. Nothing depends on a demo.
- The claim is the inspectable boundary and the compounding delivery system, not a score.
-->
