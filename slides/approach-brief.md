---
theme: default
title: Code is cheap. Delivery is hard.
sync: opening
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
.slidev-layout .foundation-progress { position: absolute; top: 28px; right: 160px; z-index: 2; width: 460px; margin: 0; display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
.slidev-layout .foundation-progress span { padding: 4px 8px 3px; border-top: 2px solid rgba(13, 16, 48, 0.12); font-size: 9px; line-height: 1.15; font-weight: 600; letter-spacing: 0.07em; text-transform: uppercase; color: var(--phl-ink-muted); }
.slidev-layout .foundation-progress .is-active { border-color: var(--phl-cyan); background: rgba(22, 180, 226, 0.05); color: var(--phl-cyan-ink); }
.slidev-layout .foundation-progress .is-active.is-magenta { border-color: var(--phl-magenta); background: rgba(164, 35, 143, 0.05); color: #a4238f; }
</style>

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · ACCOUNTABLE SOFTWARE DELIVERY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Code is cheap.<br>Delivery is hard.

<p class="section-subtitle">Use a delivery system that lets AI<br>deliver software — not just code.</p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Standardise delivery · One path from change to production</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Specify behaviour · Preserve what must remain true</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Sandbox execution · Limit what running code may access</span></div>
</div>

<!--
Opening:
- Code generation is no longer the scarce part of software delivery.
- Accountability has not moved to the model. The organisation still owns the outcome, the access, the evidence, and the consequences.
- Our answer is not a better prompt. It is a delivery system whose rules, record, and runtime boundary sit outside the model.

Client question:
- Where has implementation become faster while getting a change safely into production has not?
-->

---
title: Coding is no longer the bottleneck.
sync: shift
class: slide-light
---
<Eyebrow text="THE SHIFT · ABUNDANT CODE IS NOT A COHERENT SOLUTION" />

<div class="ed-split">
<div class="ed-prose-col">

# Coding is no longer<br>the bottleneck.

<p class="ed-prose">AI can write code in minutes, but it cannot frame a problem, weigh trade-offs, or own consequences.</p>
<p class="ed-prose"><strong>More code ≠ a coherent solution.</strong> Coding agents smooth over ambiguity with plausible code, skipping the design conversations a team needed to have. The scarce work is comprehension, coherence, and evidence.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="14" y="22" width="612" height="108" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="38" y="52" class="svg-kicker" fill="#0c7fa3">GENERATION EFFORT</text>
  <text x="38" y="88" class="svg-h2" fill="#0d1030">Weeks</text>
  <line x1="130" y1="81" x2="488" y2="81" stroke="rgba(12,127,163,0.42)" stroke-width="2" />
  <polygon points="482,76 492,81 482,86" fill="#a4238f" />
  <text x="602" y="88" text-anchor="end" class="svg-h2" fill="#a4238f">Minutes</text>
  <text x="38" y="114" class="svg-caption" fill="rgba(13,16,48,0.58)">More plausible code, sooner</text>

  <rect x="14" y="148" width="612" height="224" rx="14" fill="#0d1030" />
  <text x="38" y="180" class="svg-kicker" fill="#78d8f2">MORE CODE ≠ A COHERENT SOLUTION</text>

  <rect x="38" y="200" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="56" y="226" class="svg-label" fill="#ffffff">Cognitive load</text>
  <text x="56" y="248" class="svg-caption" fill="rgba(255,255,255,0.68)">Overwhelms human review</text>
  <rect x="332" y="200" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="350" y="226" class="svg-label" fill="#ffffff">Hidden ambiguity</text>
  <text x="350" y="248" class="svg-caption" fill="rgba(255,255,255,0.68)">Plausible code skips needed debate</text>

  <rect x="38" y="282" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="56" y="308" class="svg-label" fill="#ffffff">Lost comprehension</text>
  <text x="56" y="330" class="svg-caption" fill="rgba(255,255,255,0.68)">No one understands it all</text>
  <rect x="332" y="282" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="350" y="308" class="svg-label" fill="#ffffff">Accumulated slop</text>
  <text x="350" y="330" class="svg-caption" fill="rgba(255,255,255,0.68)">Unmanageable over time</text>

  <rect x="70" y="394" width="500" height="36" rx="18" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.42)" />
  <text x="320" y="417" text-anchor="middle" class="svg-caption" fill="#a4238f">The risk: An unmanageable application and codebase.</text>
</svg>
</div>
</div>

<!--
Coding and test generation are no longer the bottlenecks they once were.
However, a lot of code does not equal a coherent, well-thought-through, useful business solution.
Generated tests can repeat the implementation's assumptions. Trusted evidence still has to reject plausible but wrong code.
It can exacerbate cognitive load and overwhelm, and lead to an unmanageable application and codebase.
-->

---
title: Build the right thing. Build it to last.
sync: pivot
class: slide-light
---
<Eyebrow text="THE RESPONSE · STRATEGY AND DELIVERY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Build the right thing.<br>Build it to last.

</div>
<p class="ed-band-prose"><strong>Designing great software is a human process.</strong> Good product strategy decides what is worth building. A reliable delivery system delivers it as maintainable software. While we can help with product design, <strong>our expertise is in delivering systems that last.</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="550" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · GOOD PRODUCT STRATEGY</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Building the right thing</text>
  <line x1="38" y1="122" x2="544" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Frame the problem · weigh trade-offs · test with prototypes</text>
  <text x="38" y="204" class="svg-caption" fill="rgba(13,16,48,0.62)">Decide what matters before committing to production</text>

  <rect x="574" y="36" width="550" height="214" rx="14" fill="#0d1030" />
  <text x="596" y="68" class="svg-kicker" fill="#f59ef5">02 · A RELIABLE DELIVERY SYSTEM</text>
  <text x="596" y="104" class="svg-h2" fill="#ffffff">Delivering maintainable software</text>
  <line x1="596" y1="122" x2="1102" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="596" y="158" class="svg-label" fill="#ffffff">Standardise delivery · specify behaviour · sandbox execution</text>
  <text x="596" y="204" class="svg-caption" fill="rgba(255,255,255,0.7)">Turn intent into coherent, verified software</text>

  <rect x="170" y="282" width="800" height="48" rx="24" fill="#0d1030" />
  <text x="570" y="312" text-anchor="middle" class="svg-label" fill="#ffffff">Decide what matters, deliver it safely, and keep it maintainable.</text>
</svg>
</div>
</div>

<!--
Two things are increasingly important:
1) Good product strategy—knowing what is worth building and using rapid prototypes to learn without mistaking them for production systems. A prototype informs the decision; it does not make it.
2) A reliable delivery system—turning intent into coherent, comprehensible, verified software without accumulating slop.
While we can help with both, our real expertise is in the second.
-->

---
title: A delivery system for AI-generated code.
sync: system
class: slide-sandbox
---
<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="THE DELIVERY SYSTEM · HUMAN-LED ENGINEERING" />

# A delivery system for AI-generated code.

<p class="slide-lead">Human-led engineering — with agents working inside the system, not in charge of it.</p>

<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 400" aria-hidden="true">
  <rect x="180" y="6" width="780" height="38" rx="19" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.24)" />
  <text x="570" y="30" text-anchor="middle" class="svg-kicker" fill="#ffffff">PROTECTED CONTROL PLANE · PEOPLE SET OUTCOME, SOURCE PRECEDENCE, ACCEPTANCE, RELEASE</text>

  <line x1="205" y1="44" x2="205" y2="78" stroke="#78d8f2" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="570" y1="44" x2="570" y2="78" stroke="#f59ef5" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="935" y1="44" x2="935" y2="78" stroke="#78d8f2" stroke-width="1.5" stroke-dasharray="4 4" />

  <rect x="12" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="38" y="112" class="svg-kicker" fill="#78d8f2">01 · CHANGE PATH</text>
  <text x="38" y="142" class="svg-h2" fill="#ffffff">Standardise delivery</text>
  <line x1="38" y1="156" x2="342" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="38" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Approved tools and models</text>
  <text x="38" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Data, source, and worker boundaries</text>
  <text x="38" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Agent instructions and pipeline gates</text>
  <text x="38" y="270" class="svg-caption" fill="#78d8f2">How work is created, checked, shipped, and operated.</text>

  <rect x="392" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="418" y="112" class="svg-kicker" fill="#f59ef5">02 · DURABLE AUTHORITY</text>
  <text x="418" y="142" class="svg-h2" fill="#ffffff">Specify behaviour</text>
  <line x1="418" y1="156" x2="722" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="418" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Evidence and explicit source authority</text>
  <text x="418" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Versioned spec.md and design.md</text>
  <text x="418" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Bounded implementation and verification</text>
  <text x="418" y="270" class="svg-caption" fill="#f59ef5">What must remain true and how acceptance is proven</text>

  <rect x="772" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="798" y="112" class="svg-kicker" fill="#78d8f2">03 · RUNTIME BOUNDARY</text>
  <text x="798" y="142" class="svg-h2" fill="#ffffff">Sandbox execution</text>
  <line x1="798" y1="156" x2="1102" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="798" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Sandboxed WebAssembly components</text>
  <text x="798" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Typed capabilities and explicit grants</text>
  <text x="798" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Credentials and infrastructure in the host</text>
  <text x="798" y="270" class="svg-caption" fill="#78d8f2">What running code may access</text>

  <line x1="190" y1="290" x2="190" y2="322" stroke="#78d8f2" stroke-width="1.5" />
  <line x1="570" y1="290" x2="570" y2="322" stroke="#f59ef5" stroke-width="1.5" />
  <line x1="950" y1="290" x2="950" y2="322" stroke="#78d8f2" stroke-width="1.5" />

  <rect x="12" y="326" width="1116" height="58" rx="14" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.22)" />
  <text x="38" y="351" class="svg-kicker" fill="#78d8f2">REUSABLE PATH TO PRODUCTION</text>
  <text x="258" y="351" class="svg-label" fill="#ffffff">Build</text>
  <text x="320" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="356" y="351" class="svg-label" fill="#ffffff">Test</text>
  <text x="412" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="448" y="351" class="svg-label" fill="#ffffff">Provision</text>
  <text x="534" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="570" y="351" class="svg-label" fill="#ffffff">Deploy</text>
  <text x="636" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="672" y="351" class="svg-label" fill="#ffffff">Release</text>
  <text x="744" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="780" y="351" class="svg-label" fill="#ffffff">Observe</text>
  <text x="854" y="351" class="svg-caption" fill="rgba(255,255,255,0.46)">→</text>
  <text x="890" y="351" class="svg-label" fill="#ffffff">Recover</text>
  <text x="38" y="372" class="svg-caption" fill="rgba(255,255,255,0.64)">Reusable infrastructure and pipeline assets carry the known shape into operation.</text>
  <text x="1102" y="372" text-anchor="end" class="svg-caption" fill="#f59ef5">The delivery engine and runtime are open source.</text>
</svg>
</div>

<!--
The complete system:
- This is the map for the next three sections. Take each mechanism in turn.
- These are three different mechanisms over three different questions.
- Standardise delivery governs how work is created, checked, shipped, and operated.
- Specify behaviour governs what must remain true and what evidence accepts the result.
- Sandbox execution governs what the running application may access.
- Reusable delivery assets carry the result to production; they are supporting infrastructure, not a substitute fourth foundation.

Boundary:
- The coding agent's working environment and the application's runtime sandbox are separate trust boundaries.
-->

---
title: Standardise delivery.
sync: section-standardise
class: slide-light
---
<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 01" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Standardise delivery.

</div>
<p class="ed-band-prose"><strong>One governed path from change to production—across how code is written and how it ships.</strong> Teams move faster when people and agents work inside governed workspaces, follow one engineering codex, fail critical rules at compile time, and promote one reproducible artifact through automated gates.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · ISOLATE &amp; GUIDE</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Isolate &amp; guide</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Governed workspaces and</text>
  <text x="38" y="184" class="svg-label" fill="#0d1030">shared engineering codex</text>
  <text x="38" y="218" class="svg-caption" fill="rgba(13,16,48,0.62)">One standard for people and agents</text>
  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">02 · ENFORCE AT COMPILE TIME</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Enforce at compile time</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="158" class="svg-label" fill="#ffffff">Fail critical rules before review</text>
  <text x="420" y="190" class="svg-caption" fill="rgba(255,255,255,0.7)">Review focuses on design and trade-offs</text>
  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#0c7fa3">03 · AUTOMATE PROMOTION</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Automate promotion</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="816" y="158" class="svg-label" fill="#0d1030">Build once, test behaviour, and</text>
  <text x="816" y="184" class="svg-label" fill="#0d1030">gate every change</text>
  <text x="816" y="218" class="svg-caption" fill="rgba(13,16,48,0.62)">Evidence moves with the artifact</text>
</svg>
</div>
</div>

<!--
Section 01 divider: Standardise delivery.
-->

---
title: Standardise how code is written.
sync: authoring
class: slide-light
---
<Eyebrow text="01 · STANDARDISE DELIVERY · TOOLCHAIN CONTROLS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="ed-split">
<div class="ed-prose-col">

# Standardise how code is written.

<p class="ed-prose">Give people and agents a safe workspace, shared standards, and compile-time guardrails.</p>
<p class="ed-prose"><strong>Guidance is shared. Non-negotiable rules are enforced.</strong> Critical rules fail at compile time, before review—so review focuses on design and trade-offs.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="14" y="24" width="612" height="102" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <circle cx="62" cy="75" r="22" fill="#0c7fa3" />
  <text x="62" y="80" text-anchor="middle" class="svg-label" fill="#ffffff">01</text>
  <text x="102" y="58" class="svg-kicker" fill="#0c7fa3">ISOLATE</text>
  <text x="102" y="84" class="svg-h3" fill="#0d1030">Governed workspaces</text>
  <text x="102" y="108" class="svg-caption" fill="rgba(13,16,48,0.62)">Non-production environments for people and agents</text>

  <rect x="14" y="144" width="612" height="102" rx="14" fill="rgba(164,35,143,0.05)" stroke="#a4238f" stroke-width="1.6" />
  <circle cx="62" cy="195" r="22" fill="#a4238f" />
  <text x="62" y="200" text-anchor="middle" class="svg-label" fill="#ffffff">02</text>
  <text x="102" y="178" class="svg-kicker" fill="#a4238f">GUIDE</text>
  <text x="102" y="204" class="svg-h3" fill="#0d1030">Shared engineering codex</text>
  <text x="102" y="228" class="svg-caption" fill="rgba(13,16,48,0.62)">Versioned standards, approved patterns, examples, and templates for people and agents</text>

  <rect x="14" y="264" width="612" height="102" rx="14" fill="#0d1030" />
  <circle cx="62" cy="315" r="22" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.4" />
  <text x="62" y="320" text-anchor="middle" class="svg-label" fill="#78d8f2">03</text>
  <text x="102" y="298" class="svg-kicker" fill="#78d8f2">ENFORCE</text>
  <text x="102" y="324" class="svg-h3" fill="#ffffff">Compiler-enforced standards</text>
  <text x="102" y="344" class="svg-caption" fill="rgba(255,255,255,0.7)">Critical rules fail at compile time, before review.</text>
  <text x="102" y="360" class="svg-caption" fill="rgba(255,255,255,0.7)"></text>

  <rect x="72" y="390" width="496" height="38" rx="19" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.4)" />
  <text x="320" y="414" text-anchor="middle" class="svg-caption" fill="#a4238f">Guidance is shared · Non-negotiable rules are enforced</text>
</svg>
</div>
</div>

<!--
Mechanism one, first half — how code is authored. Standardising delivery starts before the pipeline: where work is done, what guidance it follows, and which rules are enforced automatically.
- Isolate work from production while an idea is still changing.
- Publish one versioned engineering codex so people and agents work from the same standards, patterns, examples, and templates.
- Enforce critical coding rules as the code compiles, then apply the same security, dependency, quality, and test controls to every change.

Client question:
- Where does experimentation currently become inconsistent, risky, or difficult to audit?
-->

---
title: Automate the path to production.
sync: delivery-automation
class: slide-light
---
<Eyebrow text="01 · STANDARDISE DELIVERY · DELIVERY AUTOMATION" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Automate the path<br>to production.

</div>
<p class="ed-band-prose"><strong>Faster implementation matters only when evidence and delivery move at the same speed.</strong> We removed mechanical delay by automating the path around the code.</p>
</div>

<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · BUILD</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Build once</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Reproducible artifact</text>
  <text x="38" y="204" class="svg-caption" fill="rgba(13,16,48,0.62)">One artifact, built reproducibly</text>

  <line x1="346" y1="143" x2="392" y2="143" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="386,138 396,143 386,148" fill="#0d1030" />

  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#78d8f2">02 · TEST</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Test behaviour</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="156" class="svg-label" fill="#ffffff">Unit · integration ·</text>
  <text x="420" y="182" class="svg-label" fill="#ffffff">replay · end-to-end</text>
  <text x="420" y="222" class="svg-caption" fill="rgba(255,255,255,0.7)">Evidence at every level</text>

  <line x1="742" y1="143" x2="788" y2="143" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="782,138 792,143 782,148" fill="#a4238f" />

  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#a4238f">03 · GATE + PROMOTE</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Gate every change</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(164,35,143,0.18)" />
  <text x="816" y="156" class="svg-label" fill="#0d1030">Compiler · security ·</text>
  <text x="816" y="182" class="svg-label" fill="#0d1030">dependencies · secrets</text>
  <text x="816" y="222" class="svg-caption" fill="rgba(13,16,48,0.62)">Then promote the artifact</text>

  <rect x="170" y="282" width="800" height="48" rx="24" fill="#0d1030" />
  <text x="570" y="312" text-anchor="middle" class="svg-label" fill="#ffffff">The same accepted artifact moves forward with its evidence.</text>
</svg>
</div>
</div>

<!--
These changes predate the current AI wave and make AI-assisted implementation operationally useful.
- Full test automation runs unit, integration, and end-to-end evidence without manual coordination.
- Pipeline automation carries an accepted pull request through build, provision, deployment, and production.
- Automated analysis makes linting, dependency, security, and review controls part of every change. Current examples include Cursor Bugbot, GitHub CodeQL, and Snyk.
- This required testable systems, reliable suites, standard environments, policy as code, and ownership of pipeline health.
Automation removes mechanical delay. It does not remove the human decision to accept a change.

Client question:
- Where does a change still wait for a manual hand-off or bespoke decision?
-->

---
title: Specify behaviour.
sync: section-intent
class: slide-light
---
<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 02" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Specify behaviour.

</div>
<p class="ed-band-prose"><strong>People and agents share one source of truth: durable specifications, not code.</strong> Coding agents smooth over ambiguity with plausible-looking code. The specification forces that ambiguity into the open so people can resolve it—and if the code disagrees with it, the code is repaired or regenerated.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · SURFACE AMBIGUITY</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Surface ambiguity</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Expose conflicts and gaps before coding</text>
  <text x="38" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">Nothing buried in plausible code</text>
  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">02 · HUMAN REVIEW GATE</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Human review gate</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="158" class="svg-label" fill="#ffffff">Debate trade-offs and seal the spec</text>
  <text x="420" y="190" class="svg-caption" fill="rgba(255,255,255,0.7)">The specification stays in the loop</text>
  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#0c7fa3">03 · DURABLE TRUTH</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Durable truth</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="816" y="158" class="svg-label" fill="#0d1030">Code is disposable; specifications remain</text>
  <text x="816" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">The reviewed specification endures</text>
</svg>
</div>
</div>

<!--
Section 02 divider: Spec-driven development.
- Governed intent: move up from ad hoc prompting.
- In the loop: specify, review, plan, implement.
- Durable truth: code is disposable; specifications remain.
-->

---
title: AI coding moves right. Governance moves up.
sync: record-position
class: slide-light
---
<Eyebrow text="02 · SPECIFY BEHAVIOUR · SPEC-DRIVEN DEVELOPMENT" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# AI coding moves right.<br>Governance moves up.

</div>
<p class="ed-band-prose"><strong>AI coding changes implementation. It does not create governance.</strong> It does not establish authority, retain intent, or create an audit trail. Spec-driven development makes the separate upward move.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <rect x="476" y="8" width="188" height="36" rx="18" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.55)" />
  <text x="570" y="31" text-anchor="middle" class="svg-kicker" fill="#a4238f">GOVERNED</text>

  <rect x="170" y="52" width="800" height="248" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.34)" stroke-width="1.6" />
  <rect x="570" y="52" width="400" height="124" rx="0" fill="rgba(164,35,143,0.07)" />
  <rect x="170" y="176" width="800" height="124" rx="0" fill="rgba(13,16,48,0.025)" />
  <line x1="570" y1="52" x2="570" y2="300" stroke="rgba(13,16,48,0.34)" stroke-width="1.5" />
  <line x1="170" y1="176" x2="970" y2="176" stroke="rgba(13,16,48,0.34)" stroke-width="1.5" />
  <rect x="572" y="54" width="396" height="120" fill="none" stroke="#a4238f" stroke-width="2.5" />

  <text x="370" y="108" text-anchor="middle" class="svg-h2" fill="#0d1030">Traditional SDLC</text>
  <text x="370" y="134" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Governance around human implementation</text>

  <text x="770" y="108" text-anchor="middle" class="svg-h2" fill="#a4238f">Spec-driven development</text>
  <text x="770" y="134" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.64)">Durable intent around AI implementation</text>

  <text x="370" y="232" text-anchor="middle" class="svg-h2" fill="#0d1030">Ad hoc development</text>
  <text x="370" y="258" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Local choices and human memory</text>

  <text x="770" y="232" text-anchor="middle" class="svg-h2" fill="#0d1030">AI-assisted coding alone</text>
  <text x="770" y="258" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Conversation-led generation</text>

  <rect x="8" y="154" width="146" height="44" rx="22" fill="#ffffff" stroke="rgba(13,16,48,0.28)" />
  <text x="81" y="181" text-anchor="middle" class="svg-kicker" fill="#0d1030">MANUAL CODING</text>
  <rect x="986" y="154" width="146" height="44" rx="22" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.55)" />
  <text x="1059" y="181" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">AI-ASSISTED</text>

  <rect x="476" y="316" width="188" height="36" rx="18" fill="#ffffff" stroke="rgba(13,16,48,0.28)" />
  <text x="570" y="339" text-anchor="middle" class="svg-kicker" fill="#0d1030">UNGOVERNED</text>
</svg>
</div>
</div>

<!--
Foundation two — why spec-driven delivery:
- Most AI adoption begins as a horizontal move: introduce an assistant or agent and produce code faster.
- That does not create authority, an audit trail, or a durable statement of what must remain true.
- Spec-driven development is the separate upward move. It puts governed intent around AI-assisted implementation.

Client question:
- Has AI changed how code is produced without changing what governs its acceptance?
-->

---
title: Specifications drive the loop.
sync: record-development
class: slide-light
---
<Eyebrow text="02 · SPECIFY BEHAVIOUR · THE DEVELOPMENT LOOP" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Specifications drive<br>the loop.

</div>
<p class="ed-band-prose"><strong>The specification stays in the development loop</strong>—surface conflicts and gaps for human judgement instead of burying them in generated code. Each change runs the loop. If the specification changes, it is reviewed and sealed again.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <text x="16" y="24" class="svg-kicker" fill="#a4238f">DURABLE SPECIFICATION</text>
  <text x="472" y="24" class="svg-kicker" fill="#0c7fa3">EPHEMERAL DELIVERY RUN</text>

  <rect x="16" y="54" width="268" height="202" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="38" y="84" class="svg-kicker" fill="#0c7fa3">01 · WHAT + HOW</text>
  <text x="38" y="116" class="svg-h3" fill="#0d1030">Specify</text>
  <line x1="38" y1="132" x2="262" y2="132" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-kicker" fill="#0c7fa3">SOURCES</text>
  <text x="38" y="180" class="svg-label" fill="#0d1030">Intent · docs · tests</text>
  <text x="38" y="204" class="svg-label" fill="#0d1030">Legacy source code</text>
  <text x="38" y="228" class="svg-kicker" fill="#a4238f">SPECIFICATION</text>
  <text x="38" y="246" class="svg-caption" fill="rgba(13,16,48,0.68)">spec.md · design.md</text>

  <line x1="284" y1="155" x2="316" y2="155" stroke="#0d1030" stroke-width="1.7" />
  <polygon points="310,150 320,155 310,160" fill="#0d1030" />

  <rect x="320" y="42" width="120" height="226" rx="14" fill="#0d1030" />
  <text x="380" y="78" text-anchor="middle" class="svg-kicker" fill="#f59ef5">REVIEW GATE</text>
  <text x="380" y="118" text-anchor="middle" class="svg-h3" fill="#ffffff">Review</text>
  <text x="380" y="144" text-anchor="middle" class="svg-h3" fill="#ffffff">&amp; seal</text>
  <line x1="342" y1="166" x2="418" y2="166" stroke="rgba(255,255,255,0.16)" />
  <text x="380" y="196" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Human</text>
  <text x="380" y="216" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">decision gate</text>

  <line x1="440" y1="155" x2="468" y2="155" stroke="#a4238f" stroke-width="1.7" />
  <polygon points="462,150 472,155 462,160" fill="#a4238f" />

  <rect x="472" y="54" width="268" height="202" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="494" y="84" class="svg-kicker" fill="#a4238f">02 · DERIVE</text>
  <text x="494" y="116" class="svg-h3" fill="#0d1030">Plan</text>
  <line x1="494" y1="132" x2="718" y2="132" stroke="rgba(164,35,143,0.18)" />
  <text x="494" y="164" class="svg-label" fill="#0d1030">Units of work,</text>
  <text x="494" y="192" class="svg-label" fill="#0d1030">ordered by dependency</text>
  <text x="494" y="226" class="svg-caption" fill="rgba(13,16,48,0.6)">Discard or regenerate</text>

  <line x1="740" y1="155" x2="768" y2="155" stroke="#0d1030" stroke-width="1.7" />
  <polygon points="762,150 772,155 762,160" fill="#a4238f" />

  <rect x="772" y="54" width="352" height="202" rx="14" fill="#0d1030" />
  <text x="794" y="84" class="svg-kicker" fill="#78d8f2">03 · BUILD + PROVE</text>
  <text x="794" y="116" class="svg-h3" fill="#ffffff">Implement</text>
  <line x1="794" y1="132" x2="1102" y2="132" stroke="rgba(255,255,255,0.14)" />
  <text x="794" y="170" class="svg-label" fill="#ffffff">Build · integrate · verify</text>
  <text x="794" y="204" class="svg-label" fill="#ffffff">Against the specification</text>
  <text x="794" y="238" class="svg-caption" fill="rgba(255,255,255,0.68)">Complete only when verified</text>

  <rect x="148" y="294" width="844" height="48" rx="24" fill="#0d1030" />
  <text x="570" y="324" text-anchor="middle" class="svg-label" fill="#ffffff">Each change runs the loop.</text>
</svg>
</div>
</div>

<!--
This is the recognisable spec-driven development loop.
- Specify reconciles intent, documentation, tests, and, where it exists, legacy source code into `spec.md` and `design.md`. Together, those files are the specification.
- A person reviews and seals that specification in the repository.
- Plan derives units of work, ordered by dependency, from the reviewed specification. It can be discarded and generated again.
- Implement gives each unit of work the exact specification and plan entry it needs, builds and integrates it, and verifies the combined result against the specification.
- Verification is part of completing implementation, not a separate downstream activity; only a verified wave advances the revision's label.
- The coding agent and plan are replaceable. The reviewed specification is not.

This leads to the next boundary:
- The specification governs what must be built. The runtime governs what the resulting code may touch.
-->

---
title: Code is disposable. Specifications are durable.
sync: record-truth
class: slide-light
---
<Eyebrow text="02 · SPECIFY BEHAVIOUR · DURABLE SPECIFICATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Code is disposable.<br><span style="color:#a4238f">Specifications are durable.</span>

</div>
<p class="ed-band-prose"><strong>The implementation is a derived output.</strong> Regenerate the plan or rebuild the code. The reviewed specification—spec.md and design.md—preserves what the system must do and how it is designed.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <text x="16" y="24" class="svg-kicker" fill="#0c7fa3">DISPOSABLE IMPLEMENTATION</text>
  <rect x="16" y="42" width="360" height="226" rx="16" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.34)" stroke-width="1.6" stroke-dasharray="6 5" />
  <text x="196" y="140" text-anchor="middle" font-size="52" font-weight="600" fill="#0d1030">Code</text>
  <line x1="72" y1="166" x2="320" y2="166" stroke="rgba(13,16,48,0.12)" />
  <text x="196" y="202" text-anchor="middle" class="svg-label" fill="rgba(13,16,48,0.68)">Rebuild from the specification</text>
  <text x="196" y="236" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.52)">A derived output</text>

  <text x="408" y="24" class="svg-kicker" fill="#a4238f">DURABLE AUTHORITY</text>
  <rect x="408" y="42" width="716" height="226" rx="16" fill="#0d1030" stroke="#a4238f" stroke-width="2" />
  <text x="434" y="78" class="svg-h2" fill="#ffffff">The specification</text>
  <text x="1098" y="78" text-anchor="end" class="svg-kicker" fill="#f59ef5">REVIEWED · VERSIONED · IN REPO</text>
  <line x1="434" y1="96" x2="1098" y2="96" stroke="rgba(255,255,255,0.14)" />
  <rect x="434" y="116" width="664" height="48" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="454" y="146" class="svg-label" fill="#ffffff">spec.md</text>
  <text x="604" y="146" class="svg-caption" fill="rgba(255,255,255,0.72)">Requirements and scenarios</text>
  <rect x="434" y="180" width="664" height="48" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="454" y="210" class="svg-label" fill="#ffffff">design.md</text>
  <text x="604" y="210" class="svg-caption" fill="rgba(255,255,255,0.72)">Architecture and decisions</text>
  <text x="434" y="252" class="svg-caption" fill="#78d8f2">What must remain true—and why</text>

  <rect x="100" y="298" width="940" height="48" rx="24" fill="#0d1030" stroke="#a4238f" stroke-width="1.8" />
  <text x="570" y="328" text-anchor="middle" class="svg-h3" fill="#ffffff">Regenerate the plan. Rebuild the code. The specification remains.</text>
</svg>
</div>
</div>

<!--
This is the deliberate provocation at the end of the section.
- Disposable does not mean unimportant. It means the implementation is a derived output, not the authority over intent.
- In modernisation, legacy source code is still valuable input to specification recovery. That role belongs on the preceding slide.
- The plan can be regenerated and the implementation can be rebuilt. The coding model is interchangeable too, but it is tooling rather than an artefact, so it stays out of the diagram.
- Only the reviewed `spec.md` and `design.md` remain as the durable specification in the repository.
- Those two files preserve what the system must do and how it is designed.

Transition:
- The specification governs what must be built. The runtime governs what the resulting code may touch.
-->

---
title: Sandbox execution.
sync: section-constrain
class: slide-light
---
<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 03" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Sandbox execution.

</div>
<p class="ed-band-prose"><strong>Grant generated code nothing by default—it reaches infrastructure only through host-mediated capabilities.</strong> The host, not the generated code, decides what can be reached, and grants only the capabilities the work needs.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · WASMTIME SANDBOX</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Wasmtime sandbox</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">No direct route to infrastructure</text>
  <text x="38" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">Generated code runs inside the sandbox</text>
  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">02 · CAPABILITY CONTRACTS</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Capability contracts</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="158" class="svg-label" fill="#ffffff">HTTP, Messaging, KeyValue, BlobStore</text>
  <text x="420" y="190" class="svg-caption" fill="rgba(255,255,255,0.7)">Access only through typed contracts</text>
  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#0c7fa3">03 · RUNTIME RECIPES</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Runtime recipes</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="816" y="158" class="svg-label" fill="#0d1030">Local, cloud, and on-premise</text>
  <text x="816" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">The same application, bound per environment</text>
</svg>
</div>
</div>

<!--
Section 03 divider: Sandboxed execution.
- Grant generated code nothing by default.
- It reaches infrastructure only through host-mediated capabilities.
- Runtime recipes bind the same contracts in each environment.
-->

---
title: Generated code cannot reach past its sandbox.
sync: runtime
class: slide-light
---
<Eyebrow text="03 · SANDBOX EXECUTION · CAPABILITY BOUNDARY" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Generated code cannot reach<br>past its sandbox.

</div>
<p class="ed-band-prose"><strong>With the platform safe by construction, agents work within known bounds while engineers focus on the product.</strong> Code reaches infrastructure through host-provided capabilities; grant generated code nothing by default.</p>
</div>
<div class="ed-band-canvas" style="padding-top: 24px;">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <g transform="translate(114 12) scale(0.8)">
  <rect x="16" y="0" width="1108" height="178" rx="16" fill="rgba(65,145,203,0.08)" stroke="rgba(65,145,203,0.68)" stroke-width="2.4" />

  <rect x="100" y="26" width="390" height="70" rx="10" fill="rgba(124,92,205,0.1)" stroke="rgba(124,92,205,0.7)" stroke-width="2.4" />
  <text x="295" y="56" text-anchor="middle" class="svg-h3" fill="#0d1030">WASI Component</text>
  <text x="295" y="80" text-anchor="middle" class="svg-label" font-style="italic" fill="rgba(13,16,48,0.72)">Generated code</text>

  <rect x="650" y="26" width="390" height="70" rx="10" fill="rgba(124,92,205,0.1)" stroke="rgba(124,92,205,0.7)" stroke-width="2.4" />
  <text x="845" y="56" text-anchor="middle" class="svg-h3" fill="#0d1030">WASI Component</text>
  <text x="845" y="80" text-anchor="middle" class="svg-label" font-style="italic" fill="rgba(13,16,48,0.72)">Generated code</text>

  <text x="570" y="134" text-anchor="middle" class="svg-h2" fill="#0d1030">Wasm Runtime</text>
  <text x="570" y="159" text-anchor="middle" class="svg-label" font-style="italic" fill="rgba(13,16,48,0.72)">Wasmtime sandbox</text>

  <line x1="16" y1="191" x2="404" y2="191" stroke="rgba(13,16,48,0.18)" />
  <line x1="736" y1="191" x2="1124" y2="191" stroke="rgba(13,16,48,0.18)" />
  <text x="570" y="195" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">HOST-GRANTED CAPABILITIES</text>

  <rect x="16" y="207" width="259" height="46" rx="9" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="2.2" />
  <text x="145.5" y="236" text-anchor="middle" class="svg-label" fill="#0d1030">HTTP</text>
  <rect x="299" y="207" width="259" height="46" rx="9" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="2.2" />
  <text x="428.5" y="236" text-anchor="middle" class="svg-label" fill="#0d1030">Messaging</text>
  <rect x="582" y="207" width="259" height="46" rx="9" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="2.2" />
  <text x="711.5" y="236" text-anchor="middle" class="svg-label" fill="#0d1030">KeyValue</text>
  <rect x="865" y="207" width="259" height="46" rx="9" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="2.2" />
  <text x="994.5" y="236" text-anchor="middle" class="svg-label" fill="#0d1030">BlobStore</text>

  <line x1="16" y1="271" x2="404" y2="271" stroke="rgba(13,16,48,0.18)" />
  <line x1="736" y1="271" x2="1124" y2="271" stroke="rgba(13,16,48,0.18)" />
  <text x="570" y="275" text-anchor="middle" class="svg-kicker" fill="#a4238f">HOST-SELECTED INFRASTRUCTURE</text>

  <rect x="16" y="287" width="1108" height="62" rx="10" fill="#ffffff" stroke="rgba(13,16,48,0.42)" stroke-width="2.2" />
  <text x="570" y="316" text-anchor="middle" class="svg-h3" fill="#0d1030">Infrastructure</text>
  <text x="570" y="339" text-anchor="middle" class="svg-label" font-style="italic" fill="rgba(13,16,48,0.7)">Cloud/On-premise</text>
  </g>
</svg>
</div>
</div>

<!--
The headline and payoff use the language on the Propellerhead homepage.
- Generated application code is packaged as one or more WASI components inside a Wasmtime sandbox.
- The components have no direct route to infrastructure.
- They reach HTTP, messaging, key-value storage, and blob storage only through capability interfaces supplied by the host.
- The host chooses the cloud or on-premise implementation behind each capability and retains its credentials and policy.
- A generated mistake or poisoned dependency cannot reach a capability the host did not supply.
- This complements infrastructure security; it does not replace credential scope, network policy, or backend isolation.
- Put simply: the code gets exactly what the host grants—and nothing else.

Client question:
- If this code—or one of its dependencies—were compromised, what could it actually reach?
-->

---
title: Different infrastructure. Same application.
sync: runtime-portability
class: slide-light
---
<Eyebrow text="03 · SANDBOX EXECUTION · PORTABLE INFRASTRUCTURE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Different infrastructure.<br>Same application.

</div>
<p class="ed-band-prose"><strong>The component depends on capabilities, not products.</strong> A runtime recipe binds them to approved services in each environment.</p>
</div>
<div class="ed-band-canvas" style="padding-top: 8px;">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <rect x="430" y="2" width="280" height="58" rx="11" fill="rgba(124,92,205,0.12)" stroke="rgba(124,92,205,0.7)" stroke-width="2" />
  <text x="570" y="24" text-anchor="middle" class="svg-kicker" fill="#7557c7">APPLICATION UNCHANGED</text>
  <text x="570" y="49" text-anchor="middle" class="svg-h3" fill="#0d1030">WASI Component</text>

  <line x1="570" y1="60" x2="570" y2="76" stroke="rgba(13,16,48,0.46)" stroke-width="1.8" />
  <polygon points="565,72 575,72 570,82" fill="#0d1030" />
  <text x="570" y="94" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">STABLE CAPABILITY CONTRACT</text>

  <rect x="196" y="102" width="177" height="42" rx="8" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.8" />
  <text x="284.5" y="128" text-anchor="middle" class="svg-label" fill="#0d1030">HTTP</text>
  <rect x="386" y="102" width="177" height="42" rx="8" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.8" />
  <text x="474.5" y="128" text-anchor="middle" class="svg-label" fill="#0d1030">Messaging</text>
  <rect x="576" y="102" width="177" height="42" rx="8" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.8" />
  <text x="664.5" y="128" text-anchor="middle" class="svg-label" fill="#0d1030">KeyValue</text>
  <rect x="766" y="102" width="177" height="42" rx="8" fill="#f8f8f5" stroke="rgba(90,90,82,0.3)" stroke-width="1.8" />
  <text x="854.5" y="128" text-anchor="middle" class="svg-label" fill="#0d1030">BlobStore</text>

  <line x1="570" y1="144" x2="570" y2="158" stroke="rgba(13,16,48,0.46)" stroke-width="1.8" />
  <polygon points="565,154 575,154 570,164" fill="#0d1030" />
  <text x="570" y="181" text-anchor="middle" class="svg-kicker" fill="#a4238f">HOST-SELECTED RUNTIME RECIPE</text>

  <rect x="16" y="190" width="350" height="154" rx="13" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.6" />
  <text x="38" y="216" class="svg-kicker" fill="rgba(13,16,48,0.58)">LOCAL / TRIAL</text>
  <text x="38" y="245" class="svg-h3" fill="#0d1030">In-memory + local services</text>
  <line x1="38" y1="258" x2="344" y2="258" stroke="rgba(13,16,48,0.12)" />
  <rect x="38" y="271" width="145" height="25" rx="6" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.1)" />
  <text x="110.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Runtime</text>
  <rect x="199" y="271" width="145" height="25" rx="6" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.1)" />
  <text x="271.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Capability bindings</text>
  <rect x="38" y="307" width="145" height="25" rx="6" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.1)" />
  <text x="110.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Policy + identity</text>
  <rect x="199" y="307" width="145" height="25" rx="6" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.1)" />
  <text x="271.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Config + observability</text>

  <rect x="395" y="190" width="350" height="154" rx="13" fill="rgba(22,180,226,0.05)" stroke="rgba(12,127,163,0.5)" stroke-width="1.8" />
  <text x="417" y="216" class="svg-kicker" fill="#0c7fa3">CLOUD</text>
  <text x="417" y="245" class="svg-h3" fill="#0d1030">Managed services</text>
  <line x1="417" y1="258" x2="723" y2="258" stroke="rgba(12,127,163,0.16)" />
  <rect x="417" y="271" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="489.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Runtime</text>
  <rect x="578" y="271" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="650.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Capability bindings</text>
  <rect x="417" y="307" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="489.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Policy + identity</text>
  <rect x="578" y="307" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="650.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Config + observability</text>

  <rect x="774" y="190" width="350" height="154" rx="13" fill="rgba(164,35,143,0.045)" stroke="rgba(164,35,143,0.44)" stroke-width="1.8" />
  <text x="796" y="216" class="svg-kicker" fill="#a4238f">ON-PREMISE</text>
  <text x="796" y="245" class="svg-h3" fill="#0d1030">Approved internal services</text>
  <line x1="796" y1="258" x2="1102" y2="258" stroke="rgba(164,35,143,0.16)" />
  <rect x="796" y="271" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="868.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Runtime</text>
  <rect x="957" y="271" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="1029.5" y="287" text-anchor="middle" class="svg-pill" fill="#0d1030">Capability bindings</text>
  <rect x="796" y="307" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="868.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Policy + identity</text>
  <rect x="957" y="307" width="145" height="25" rx="6" fill="rgba(255,255,255,0.78)" stroke="rgba(13,16,48,0.1)" />
  <text x="1029.5" y="323" text-anchor="middle" class="svg-pill" fill="#0d1030">Config + observability</text>
</svg>
</div>
</div>

<!--
Different infrastructure, the same application.
- The application component depends on stable capability contracts rather than directly on infrastructure products.
- Each environment supplies a reviewed runtime recipe: the runtime, capability bindings, identity and policy, configuration, and observability.
- A local or trial recipe may use in-memory and local services. Cloud and on-premise recipes bind the same contracts to approved managed or internal services.
- The recipe can be retained as versioned deployment and infrastructure-as-code assets, so the next application starts with a known operating shape.
- The infrastructure is not literally identical. Its application-facing contract and deployment grammar are.
- Where the capability semantics remain the same, change the recipe—not the application.

Client question:
- How much application code currently exists only to accommodate a particular environment or infrastructure product?
-->

---
title: Match safeguards to the stakes.
sync: section-trust
class: slide-light
---
<Eyebrow text="DELIVERY SYSTEM · TRUST MODEL" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Match safeguards<br>to the stakes.

</div>
<p class="ed-band-prose"><strong>Give prototypes room to learn—strengthen safeguards as people come to depend on the system.</strong> Explore while you learn. Govern what becomes shared. Connect with explicit purpose. Support what the business depends on.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · EXPLORE &amp; GOVERN</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Explore &amp; Govern</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Keep discovery cheap</text>
  <text x="38" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">and shared work clean</text>
  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">02 · CONNECT &amp; SUPPORT</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Connect &amp; Support</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="158" class="svg-label" fill="#ffffff">Enforce runtime and</text>
  <text x="420" y="190" class="svg-caption" fill="rgba(255,255,255,0.7)">full delivery boundaries</text>
  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#0c7fa3">03 · TRUST RADIUS</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Trust radius</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="816" y="158" class="svg-label" fill="#0d1030">Assess reach, data,</text>
  <text x="816" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">dependence, and continuity</text>
</svg>
</div>
</div>

<!--
Section divider: Trust Model and Trust Radius.
- Match safeguards to the stakes.
- Explore, Govern, Connect, and Support are situations to match—not levels to climb.
-->

---
title: Trust is contextual, not a maturity score.
sync: trust-model
class: slide-light
---
<Eyebrow text="TRUST MODEL · REQUIRED SAFEGUARDS FOLLOW CONSEQUENCE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Trust is contextual,<br>not a maturity score.

</div>
<p class="ed-band-prose"><strong>Explore, Govern, Connect, and Support are situations to match—not levels to climb.</strong> Choose the least costly state that matches the actual consequence. Read each state as its situation, its minimum evidence and ownership, then its required boundary.</p>
</div>
<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="10" width="267" height="292" rx="13" fill="#ffffff" stroke="rgba(13,16,48,0.28)" stroke-width="1.5" stroke-dasharray="5 4" />
  <text x="34" y="35" class="svg-kicker" fill="rgba(13,16,48,0.58)">STATE 0 · EXPLORE</text>
  <line x1="18" y1="51" x2="285" y2="51" stroke="rgba(13,16,48,0.1)" />
  <text x="34" y="76" class="svg-kicker" fill="rgba(13,16,48,0.48)">CONTEXT</text>
  <text x="34" y="101" class="svg-h3" fill="#0d1030">Local discovery</text>
  <line x1="34" y1="117" x2="269" y2="117" stroke="rgba(13,16,48,0.1)" />
  <text x="34" y="142" class="svg-kicker" fill="rgba(13,16,48,0.48)">EVIDENCE + OWNERSHIP</text>
  <text x="34" y="168" class="svg-label" fill="#0d1030">One builder</text>
  <text x="34" y="191" class="svg-label" fill="#0d1030">Synthetic or local inputs</text>
  <text x="34" y="214" class="svg-label" fill="#0d1030">No shared dependence</text>
  <line x1="34" y1="232" x2="269" y2="232" stroke="rgba(13,16,48,0.1)" />
  <text x="34" y="256" class="svg-kicker" fill="rgba(13,16,48,0.48)">REQUIRED BOUNDARY</text>
  <text x="34" y="282" class="svg-label" fill="#0d1030">Isolate and keep local</text>

  <rect x="297" y="10" width="267" height="292" rx="13" fill="rgba(22,180,226,0.045)" stroke="rgba(12,127,163,0.58)" stroke-width="1.6" />
  <text x="313" y="35" class="svg-kicker" fill="#0c7fa3">STATE 1 · GOVERN</text>
  <line x1="297" y1="51" x2="564" y2="51" stroke="rgba(12,127,163,0.13)" />
  <text x="313" y="76" class="svg-kicker" fill="rgba(12,127,163,0.72)">CONTEXT</text>
  <text x="313" y="101" class="svg-h3" fill="#0d1030">Shared work</text>
  <line x1="313" y1="117" x2="548" y2="117" stroke="rgba(12,127,163,0.13)" />
  <text x="313" y="142" class="svg-kicker" fill="rgba(12,127,163,0.72)">EVIDENCE + OWNERSHIP</text>
  <text x="313" y="168" class="svg-label" fill="#0d1030">Named owner</text>
  <text x="313" y="191" class="svg-label" fill="#0d1030">Approved workspace and tools</text>
  <text x="313" y="214" class="svg-label" fill="#0d1030">Protected changes and checks</text>
  <line x1="313" y1="232" x2="548" y2="232" stroke="rgba(12,127,163,0.13)" />
  <text x="313" y="256" class="svg-kicker" fill="rgba(12,127,163,0.72)">REQUIRED BOUNDARY</text>
  <text x="313" y="282" class="svg-label" fill="#0d1030">Authoring boundary</text>

  <rect x="576" y="10" width="267" height="292" rx="13" fill="rgba(96,93,213,0.05)" stroke="rgba(96,93,213,0.58)" stroke-width="1.6" />
  <text x="592" y="35" class="svg-kicker" fill="#605dd5">STATE 2 · CONNECT</text>
  <line x1="576" y1="51" x2="843" y2="51" stroke="rgba(96,93,213,0.14)" />
  <text x="592" y="76" class="svg-kicker" fill="#605dd5">CONTEXT</text>
  <text x="592" y="101" class="svg-h3" fill="#0d1030">Live capability</text>
  <line x1="592" y1="117" x2="827" y2="117" stroke="rgba(96,93,213,0.14)" />
  <text x="592" y="142" class="svg-kicker" fill="#605dd5">EVIDENCE + OWNERSHIP</text>
  <text x="592" y="168" class="svg-label" fill="#0d1030">Explicit purpose and owner</text>
  <text x="592" y="191" class="svg-label" fill="#0d1030">Typed live-system contracts</text>
  <text x="592" y="214" class="svg-label" fill="#0d1030">Host policy and observability</text>
  <line x1="592" y1="232" x2="827" y2="232" stroke="rgba(96,93,213,0.14)" />
  <text x="592" y="256" class="svg-kicker" fill="#605dd5">REQUIRED BOUNDARY</text>
  <text x="592" y="282" class="svg-label" fill="#0d1030">Runtime boundary</text>

  <rect x="855" y="10" width="267" height="292" rx="13" fill="rgba(164,35,143,0.04)" stroke="rgba(164,35,143,0.55)" stroke-width="1.6" />
  <text x="871" y="35" class="svg-kicker" fill="#a4238f">STATE 3 · SUPPORT</text>
  <line x1="855" y1="51" x2="1122" y2="51" stroke="rgba(164,35,143,0.13)" />
  <text x="871" y="76" class="svg-kicker" fill="#a4238f">CONTEXT</text>
  <text x="871" y="101" class="svg-h3" fill="#0d1030">Business dependence</text>
  <line x1="871" y1="117" x2="1106" y2="117" stroke="rgba(164,35,143,0.13)" />
  <text x="871" y="142" class="svg-kicker" fill="#a4238f">EVIDENCE + OWNERSHIP</text>
  <text x="871" y="168" class="svg-label" fill="#0d1030">Durable reviewed intent</text>
  <text x="871" y="191" class="svg-label" fill="#0d1030">Release and service ownership</text>
  <text x="871" y="214" class="svg-label" fill="#0d1030">Recovery evidence</text>
  <line x1="871" y1="232" x2="1106" y2="232" stroke="rgba(164,35,143,0.13)" />
  <text x="871" y="256" class="svg-kicker" fill="#a4238f">REQUIRED BOUNDARY</text>
  <text x="871" y="282" class="svg-label" fill="#0d1030">Full delivery system</text>

  <text x="570" y="335" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Choose the least costly state that matches the actual consequence.</text>
</svg>
</div>
</div>

<!--
These are operational states, not levels of organisational maturity.
Explore is local discovery: one builder, local or synthetic inputs, and no shared dependence.
Govern begins when work is shared. It requires a named owner, an approved authoring environment, and protected changes and checks.
Connect begins when the application reaches a live capability. Purpose, ownership, typed contracts, host policy, and observability become explicit.
Support begins when the business depends on the application. Durable intent, release and service ownership, and recovery evidence are then required.
There is no virtue in moving every application to the right. Keep it in the least costly state consistent with its actual consequence.
An application can move in either direction as its use, reach, and dependence change.

Client question:
- Which applications are carrying controls for a state they are not actually in—and which are missing controls for the state they are?
-->

---
title: The highest consequence sets the trust radius.
sync: trust-radius
class: slide-light
---
<Eyebrow text="TRUST RADIUS · ASSESS FOUR DIMENSIONS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# The highest consequence<br>sets the trust radius.

</div>
<p class="ed-band-prose"><strong>Place each dimension on the same four-state scale.</strong> Take the furthest answer—not the average—and reassess whenever one changes.</p>
</div>
<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="10" width="267" height="126" rx="13" fill="rgba(22,180,226,0.045)" stroke="rgba(12,127,163,0.58)" stroke-width="1.6" />
  <text x="36" y="38" class="svg-kicker" fill="#0c7fa3">01 · REACH</text>
  <text x="36" y="68" class="svg-h3" fill="#0d1030">What can it access?</text>
  <text x="36" y="97" class="svg-caption" fill="rgba(13,16,48,0.66)">Local environment · shared systems</text>
  <text x="36" y="117" class="svg-caption" fill="rgba(13,16,48,0.66)">Live capabilities</text>

  <rect x="297" y="10" width="267" height="126" rx="13" fill="rgba(96,93,213,0.05)" stroke="rgba(96,93,213,0.58)" stroke-width="1.6" />
  <text x="315" y="38" class="svg-kicker" fill="#605dd5">02 · DATA</text>
  <text x="315" y="68" class="svg-h3" fill="#0d1030">What can it read or change?</text>
  <text x="315" y="97" class="svg-caption" fill="rgba(13,16,48,0.66)">Synthetic data · project data</text>
  <text x="315" y="117" class="svg-caption" fill="rgba(13,16,48,0.66)">Live or regulated data</text>

  <rect x="576" y="10" width="267" height="126" rx="13" fill="rgba(164,35,143,0.04)" stroke="rgba(164,35,143,0.55)" stroke-width="1.6" />
  <text x="594" y="38" class="svg-kicker" fill="#a4238f">03 · DEPENDENCE</text>
  <text x="594" y="68" class="svg-h3" fill="#0d1030">Who relies on it?</text>
  <text x="594" y="97" class="svg-caption" fill="rgba(13,16,48,0.66)">One builder · a team</text>
  <text x="594" y="117" class="svg-caption" fill="rgba(13,16,48,0.66)">Customers or operations</text>

  <rect x="855" y="10" width="267" height="126" rx="13" fill="rgba(13,16,48,0.025)" stroke="rgba(13,16,48,0.32)" stroke-width="1.5" />
  <text x="873" y="38" class="svg-kicker" fill="rgba(13,16,48,0.62)">04 · CONTINUITY</text>
  <text x="873" y="68" class="svg-h3" fill="#0d1030">What must survive failure?</text>
  <text x="873" y="97" class="svg-caption" fill="rgba(13,16,48,0.66)">Discard · restore</text>
  <text x="873" y="117" class="svg-caption" fill="rgba(13,16,48,0.66)">Recover within an obligation</text>

  <text x="570" y="165" text-anchor="middle" class="svg-kicker" fill="rgba(13,16,48,0.56)">PLACE EVERY ANSWER ON THE SAME OPERATIONAL SCALE</text>

  <rect x="126" y="184" width="222" height="58" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.4" />
  <text x="237" y="219" text-anchor="middle" class="svg-label" fill="#0d1030"><tspan font-weight="700">0</tspan> · Explore</text>
  <rect x="348" y="184" width="222" height="58" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.4)" stroke-width="1.4" />
  <text x="459" y="219" text-anchor="middle" class="svg-label" fill="#0d1030"><tspan font-weight="700">1</tspan> · Govern</text>
  <rect x="570" y="184" width="222" height="58" fill="rgba(96,93,213,0.07)" stroke="rgba(96,93,213,0.42)" stroke-width="1.4" />
  <text x="681" y="219" text-anchor="middle" class="svg-label" fill="#0d1030"><tspan font-weight="700">2</tspan> · Connect</text>
  <rect x="792" y="184" width="222" height="58" rx="9" fill="rgba(164,35,143,0.06)" stroke="rgba(164,35,143,0.42)" stroke-width="1.4" />
  <text x="903" y="219" text-anchor="middle" class="svg-label" fill="#0d1030"><tspan font-weight="700">3</tspan> · Support</text>

  <line x1="570" y1="242" x2="570" y2="261" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="565,256 575,256 570,266" fill="#a4238f" />
  <rect x="330" y="268" width="480" height="48" rx="10" fill="rgba(164,35,143,0.04)" stroke="#a4238f" stroke-width="1.5" />
  <text x="570" y="298" text-anchor="middle" class="svg-label" fill="#0d1030" style="font-weight: 600;">The furthest answer defines the trust radius—and the safeguards it requires.</text>

  <text x="570" y="342" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Reassess whenever reach, data, dependence, or continuity changes.</text>
</svg>
</div>
</div>

<!--
The Trust Model becomes an assessment method through four dimensions.
Reach asks what systems, services, and capabilities the application can access.
Data asks what it can read, retain, produce, or change, including the consequence of disclosure or corruption.
Dependence asks who or what relies on it and what work stops when it is unavailable or wrong.
Continuity asks what must survive failure and what recovery obligation exists.
Place each answer independently in Explore, Govern, Connect, or Support.
Do not average the answers. The highest-consequence answer sets the application's required state and therefore its safeguards.
This is the application's trust radius: the widest consequence it can create or inherit.
The radius can contract as well as expand. Reassess whenever one of the four dimensions changes.

Client question:
- Which dimension pushes your application furthest to the right today?
-->

---
title: One delivery system. Four use cases.
sync: use-cases
class: slide-light
---

<Eyebrow text="THE DELIVERY SYSTEM · WHERE IT APPLIES" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# One delivery system.<br>Four use cases.

</div>
<p class="ed-band-prose"><strong>Whether starting from a prototype, a legacy codebase, or a new service, the same path applies.</strong> The specification loop, compiler-enforced standards, and capability-based runtime solve four practical problems without bespoke process for each.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="18" width="542" height="148" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="48" class="svg-kicker" fill="#0c7fa3">01 · PROTOTYPE TO PRODUCTION</text>
  <text x="38" y="80" class="svg-h3" fill="#0d1030">Take vibe-coded apps to production</text>
  <line x1="38" y1="96" x2="534" y2="96" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="124" class="svg-label" fill="#0d1030">Extract intent from a rapid prototype and</text>
  <text x="38" y="146" class="svg-label" fill="#0d1030">rebuild it as a supported service.</text>

  <rect x="582" y="18" width="542" height="148" rx="14" fill="rgba(164,35,143,0.05)" stroke="#a4238f" stroke-width="1.7" />
  <text x="604" y="48" class="svg-kicker" fill="#a4238f">02 · LEGACY MODERNISATION</text>
  <text x="604" y="80" class="svg-h3" fill="#0d1030">Retire technical debt</text>
  <line x1="604" y1="96" x2="1100" y2="96" stroke="rgba(164,35,143,0.18)" />
  <text x="604" y="124" class="svg-label" fill="#0d1030">Recover durable specifications from legacy code</text>
  <text x="604" y="146" class="svg-label" fill="#0d1030">and regenerate a clean implementation.</text>

  <rect x="16" y="184" width="542" height="148" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.28)" stroke-width="1.6" />
  <text x="38" y="214" class="svg-kicker" fill="rgba(13,16,48,0.62)">03 · CLOUD PORTABILITY</text>
  <text x="38" y="246" class="svg-h3" fill="#0d1030">Move across environments</text>
  <line x1="38" y1="262" x2="534" y2="262" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="290" class="svg-label" fill="#0d1030">Build once against capability contracts and</text>
  <text x="38" y="312" class="svg-label" fill="#0d1030">swap the runtime recipe per environment.</text>

  <rect x="582" y="184" width="542" height="148" rx="14" fill="#0d1030" />
  <text x="604" y="214" class="svg-kicker" fill="#78d8f2">04 · HIGH-TRUST SYSTEMS</text>
  <text x="604" y="246" class="svg-h3" fill="#ffffff">Deliver in regulated environments</text>
  <line x1="604" y1="262" x2="1100" y2="262" stroke="rgba(255,255,255,0.14)" />
  <text x="604" y="290" class="svg-label" fill="#ffffff">Move at AI speed with an auditable specification,</text>
  <text x="604" y="312" class="svg-label" fill="#ffffff">compile-time rules, and sandboxed execution.</text>
</svg>
</div>
</div>

<!--
Where the delivery system applies in practice:
1. Prototype to production: extract the real intent from a vibe-coded or citizen-built prototype and rebuild it through the governed path as a maintainable service.
2. Legacy modernisation: recover durable specifications (`spec.md` and `design.md`) from existing source code, tests, and docs, then regenerate a clean implementation.
3. Cloud portability: build once against capability contracts and move across local, cloud, and on-premise environments by changing the runtime recipe.
4. High-trust and regulated systems: gain AI delivery speed while preserving an auditable specification, compiler-enforced rules, and a zero-trust runtime sandbox.
-->

---
title: Roadmap
sync: future-system
class: slide-light
---

<Eyebrow text="DELIVERY SYSTEM · THE FUTURE" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Roadmap

</div>
<p class="ed-band-prose"><strong>A constrained delivery system unlocks smaller models on a narrower target, and production signals returning as governed changes.</strong> Because the target is narrow and the path is governed, generation shrinks enough for smaller local models—and production signals can return through the same path as governed changes.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · NARROW THE TARGET</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Narrow the target</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Pre-scaffold before the turn</text>
  <text x="38" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">Crates, types, routes, and scenario tests</text>

  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">02 · RUN SMALLER MODELS</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Run smaller models</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="158" class="svg-label" fill="#ffffff">Local and open-weight models</text>
  <text x="420" y="190" class="svg-caption" fill="rgba(255,255,255,0.7)">Verified inside the same compiler and test gates</text>

  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#0c7fa3">03 · CLOSE THE LOOP</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Close the loop</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="816" y="158" class="svg-label" fill="#0d1030">Sense, decide, and change</text>
  <text x="816" y="190" class="svg-caption" fill="rgba(13,16,48,0.62)">Production signals return as governed changes</text>
</svg>
</div>
</div>

<!--
Section divider: What comes next.
- So far the argument has covered the delivery system as it runs today: standardise delivery, specify behaviour, sandbox execution, and match safeguards to the stakes.
- That constrained shape unlocks two active directions on the roadmap:
  1. Smaller, local models on a narrower target—because the build adapter pre-scaffolds structure and tests and the runtime fixes the capability boundary.
  2. The path becoming a loop—where production signals are investigated and fixes proposed, then specified, built, and merged like any other change.
-->

---
title: A narrower target unlocks smaller models.
sync: future-models
class: slide-light
---

<Eyebrow text="THE FUTURE · PRE-SCAFFOLDED SLICES" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# A narrower target<br>unlocks smaller models.

</div>
<p class="ed-band-prose"><strong>Frontier models are needed when an agent has to invent architecture—not when structure, contracts, and tests are already fixed.</strong> Pre-scaffolding the slice from the sealed specification shrinks generation to filling bounded logic verified by compiler and test gates.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="550" height="214" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.28)" stroke-width="1.6" />
  <text x="38" y="68" class="svg-kicker" fill="rgba(13,16,48,0.58)">BROAD TARGET · OPEN-ENDED CODING</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Requires frontier models</text>
  <line x1="38" y1="122" x2="544" y2="122" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Invent structure · choose libraries · hold the full codebase in context</text>
  <text x="38" y="204" class="svg-caption" fill="rgba(13,16,48,0.62)">The model carries architecture, conventions, and wiring at once</text>

  <rect x="574" y="36" width="550" height="214" rx="14" fill="#0d1030" />
  <text x="596" y="68" class="svg-kicker" fill="#78d8f2">NARROW TARGET · PRE-SCAFFOLDED SLICE</text>
  <text x="596" y="104" class="svg-h2" fill="#ffffff">Unlocks smaller, local models</text>
  <line x1="596" y1="122" x2="1102" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="596" y="158" class="svg-label" fill="#ffffff">Structure and tests pre-built · fill bounded logic · verify at compile time</text>
  <text x="596" y="204" class="svg-caption" fill="rgba(255,255,255,0.7)">Constraint as leverage: narrowing the target makes local models practical</text>

  <rect x="170" y="282" width="800" height="48" rx="24" fill="rgba(164,35,143,0.06)" stroke="rgba(164,35,143,0.42)" stroke-width="1.5" />
  <text x="570" y="312" text-anchor="middle" class="svg-label" fill="#0d1030">Today: frontier models in production · Next: open-weight and client-hosted models.</text>
</svg>
</div>
</div>

<!--
Smaller models on a narrower target:
- A frontier model is necessary when a coding agent has to choose libraries, invent project structure, and hold an entire codebase in context.
- Our target build adapter removes that work before the model turn begins: it scaffolds the crate, module layout, typed routes, message consumers, and scenario test skeletons from the sealed specification, then asks the model only to fill the behavioural markers and pass compiler, lint, and test checks.
- Today, that adapter runs with frontier LLMs in current engagements.
- In early proof-of-concept runs, the same scaffold-and-verify loop already builds working WebAssembly guest slices using an untuned open-weight model (`qwen3-coder-next`).
- Active roadmap focus: hardening small language model support so sensitive systems can be built and verified on local or client-hosted hardware without depending on a frontier API.
-->

---
title: Feedback drives the next change.
sync: future-loop
class: slide-light
---

<Eyebrow text="THE FUTURE · THE COMPLETE SYSTEM" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Feedback drives<br>the next change.

</div>
<p class="ed-band-prose"><strong>Nothing bypasses the path.</strong> A signal becomes a change; a change becomes new evidence. An incident manager investigates and proposes; people review and seal the change before it is built, merged, and deployed across code and infrastructure lanes.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 480" aria-hidden="true" style="max-height: 430px;">
  <defs>
    <marker id="loop-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="rgba(13,16,48,0.55)" />
    </marker>
    <marker id="loop-arrow-magenta" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="#7c5cc9" />
    </marker>
    <marker id="loop-arrow-red" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="#ff3b6b" />
    </marker>
    <linearGradient id="loop-gradient" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#16b4e2" />
      <stop offset="100%" stop-color="#d63ad6" />
    </linearGradient>
  </defs>

  <!-- sources -->
  <rect x="10" y="36" width="176" height="54" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="98" y="69" text-anchor="middle" class="svg-label" fill="#0d1030">Security feeds</text>
  <rect x="10" y="354" width="176" height="54" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="98" y="387" text-anchor="middle" class="svg-label" fill="#0d1030">Dependency audit</text>

  <rect x="10" y="195" width="176" height="54" rx="9" fill="rgba(255,59,107,0.06)" stroke="rgba(255,59,107,0.8)" stroke-width="1.6" />
  <text x="98" y="215" text-anchor="middle" class="svg-label" fill="#0d1030">Vulnerability</text>
  <text x="98" y="233" text-anchor="middle" class="svg-label" fill="#0d1030">monitoring</text>
  <text x="98" y="246" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8px;">Analyse · security, dependencies</text>

  <line x1="98" y1="90" x2="98" y2="191" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <line x1="98" y1="354" x2="98" y2="253" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <line x1="186" y1="222" x2="228" y2="222" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />

  <!-- incident manager -->
  <rect x="232" y="150" width="304" height="144" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.6" />
  <text x="252" y="178" class="svg-kicker" fill="rgba(13,16,48,0.58)">INCIDENT MANAGER</text>
  <rect x="252" y="196" width="122" height="58" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="313" y="231" text-anchor="middle" class="svg-label" fill="#0d1030">Investigate</text>
  <line x1="374" y1="225" x2="390" y2="225" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <rect x="394" y="196" width="122" height="58" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="455" y="231" text-anchor="middle" class="svg-label" fill="#0d1030">Propose Fix</text>
  <rect x="399" y="262" width="112" height="24" rx="12" fill="#0d1030" />
  <text x="455" y="278" text-anchor="middle" fill="#ffffff" style="font-size: 10.5px; font-weight: 600;">Review &amp; seal</text>

  <!-- forks to the two lanes -->
  <path d="M500 150 C 560 110, 560 92, 592 92" fill="none" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <path d="M500 294 C 560 334, 560 368, 592 368" fill="none" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <text x="512" y="118" class="svg-caption" fill="rgba(13,16,48,0.58)">investigate/fix</text>
  <text x="512" y="352" class="svg-caption" fill="rgba(13,16,48,0.58)">investigate/fix</text>

  <!-- infrastructure lane -->
  <rect x="596" y="28" width="528" height="142" rx="14" fill="rgba(22,180,226,0.04)" stroke="rgba(12,127,163,0.4)" stroke-width="1.5" />
  <text x="616" y="54" class="svg-kicker" fill="#0c7fa3">INFRASTRUCTURE</text>
  <rect x="616" y="59" width="168" height="4" rx="2" fill="url(#loop-gradient)" />
  <rect x="616" y="66" width="168" height="62" rx="9" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="700" y="96" text-anchor="middle" class="svg-label" fill="#0d1030">Specify–Build–Merge</text>
  <text x="700" y="116" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)" style="font-size: 8.5px;">Spec → Plan → Build → Merge → Verify</text>
  <line x1="784" y1="97" x2="808" y2="97" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-start="url(#loop-arrow)" marker-end="url(#loop-arrow)" />
  <path d="M812 76 a60 10 0 0 0 120 0 v42 a60 10 0 0 1 -120 0 z" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <ellipse cx="872" cy="76" rx="60" ry="10" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <text x="872" y="102" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="872" y="119" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8.5px;">spec.md · design.md</text>
  <rect x="972" y="66" width="136" height="62" rx="9" fill="rgba(124,92,205,0.1)" stroke="rgba(124,92,205,0.7)" stroke-width="1.6" />
  <text x="1040" y="99" text-anchor="middle" class="svg-label" fill="#0d1030">Terraform</text>
  <text x="1040" y="117" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">runtime recipe</text>
  <line x1="972" y1="97" x2="936" y2="97" stroke="rgba(124,92,205,0.8)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow-magenta)" />
  <line x1="1040" y1="128" x2="1040" y2="198" stroke="rgba(124,92,205,0.9)" stroke-width="2" marker-end="url(#loop-arrow-magenta)" />
  <text x="1052" y="168" class="svg-caption" fill="rgba(13,16,48,0.58)">update</text>

  <!-- shared infrastructure + monitoring -->
  <rect x="928" y="202" width="196" height="62" rx="10" fill="rgba(22,180,226,0.1)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="1026" y="236" text-anchor="middle" class="svg-h3" fill="#0d1030">Infrastructure</text>
  <text x="1026" y="255" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">Cloud/On-premise</text>
  <rect x="596" y="202" width="210" height="62" rx="10" fill="rgba(255,59,107,0.06)" stroke="rgba(255,59,107,0.8)" stroke-width="1.6" />
  <text x="701" y="228" text-anchor="middle" class="svg-label" fill="#0d1030">Infrastructure</text>
  <text x="701" y="248" text-anchor="middle" class="svg-label" fill="#0d1030">monitoring</text>
  <line x1="806" y1="233" x2="924" y2="233" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />
  <text x="865" y="223" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">monitor</text>
  <line x1="596" y1="240" x2="540" y2="240" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />
  <text x="568" y="230" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">observe</text>

  <!-- code lane -->
  <rect x="596" y="308" width="528" height="142" rx="14" fill="rgba(22,180,226,0.04)" stroke="rgba(12,127,163,0.4)" stroke-width="1.5" />
  <text x="616" y="334" class="svg-kicker" fill="#0c7fa3">CODE</text>
  <rect x="616" y="339" width="168" height="4" rx="2" fill="url(#loop-gradient)" />
  <rect x="616" y="346" width="168" height="62" rx="9" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="700" y="376" text-anchor="middle" class="svg-label" fill="#0d1030">Specify–Build–Merge</text>
  <text x="700" y="396" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)" style="font-size: 8.5px;">Spec → Plan → Build → Merge → Verify</text>
  <line x1="784" y1="377" x2="808" y2="377" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-start="url(#loop-arrow)" marker-end="url(#loop-arrow)" />
  <path d="M812 356 a60 10 0 0 0 120 0 v42 a60 10 0 0 1 -120 0 z" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <ellipse cx="872" cy="356" rx="60" ry="10" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <text x="872" y="382" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="872" y="399" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8.5px;">spec.md · design.md</text>
  <rect x="972" y="346" width="136" height="62" rx="9" fill="rgba(13,16,48,0.05)" stroke="rgba(13,16,48,0.35)" stroke-width="1.5" />
  <text x="1040" y="374" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="1040" y="394" text-anchor="middle" class="svg-label" fill="#0d1030">Actions</text>
  <text x="1040" y="430" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">promote the artifact</text>
  <line x1="972" y1="377" x2="936" y2="377" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <line x1="1040" y1="346" x2="1040" y2="268" stroke="rgba(13,16,48,0.5)" stroke-width="2" marker-end="url(#loop-arrow)" />
  <text x="1052" y="305" class="svg-caption" fill="rgba(13,16,48,0.58)">deploy</text>

</svg>
</div>
</div>

<!--
The complete system, using vulnerability remediation as the worked example:
- Security feeds and dependency audits flow into vulnerability monitoring.
- An incident manager investigates and proposes a fix. It proposes; it does not ship.
- The fix enters at the production gate and runs the same loop as any change: specify, build, and merge — in a code lane and an infrastructure lane.
- The result is deployed and updated, and infrastructure monitoring watches it and raises the next signal.
- Nothing bypasses the path. The safeguards on each pass are the ones the trust radius requires.
-->

---
title: Thank you.
sync: cta
class: slide-split short-bar
---
<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>25 years of critical-software judgement, now applied to accountable delivery at AI speed.</p>
</div>
</div>

<div class="split-content-pane" style="justify-content: center;">
<div>
<Eyebrow text="PROPELLERHEAD · ACCOUNTABLE SOFTWARE DELIVERY" />

# Thank you.

<p class="ed-prose">Start with one change you can name. Place it by its reach, data, dependence, and continuity, set the safeguards to match its stakes, and deliver it through the system.</p>
<p class="ed-prose"><strong>One change is enough to start.</strong> What it creates—the specification, the evidence, and the delivery knowledge—stays with you for the next one.</p>
</div>
</div>

<!--
The trust model gives the audience a way to judge how much safeguard a change needs.
Close by returning to the opening in personal terms: what is the first change they would trust AI to deliver? One change is enough to start; set the safeguards to match its stakes.
-->
