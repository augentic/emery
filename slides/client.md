---
theme: default
title: Code is cheap. Delivery is hard.
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

<Eyebrow text="PROPELLERHEAD · ACCOUNTABLE SOFTWARE DELIVERY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Code is cheap.<br>Delivery is hard.

<p class="section-subtitle">Put AI inside a delivery system<br>it cannot redefine.</p>

</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Set rules · Govern how people and agents work</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Record intent · Preserve what must remain true</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Constrain code · Grant the result nothing by default</span></div>
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
title: Implementation changed. Responsibility did not.
class: slide-light
---

<Eyebrow text="THE SHIFT · AI MOVES THE DELIVERY BOTTLENECK" />

<div class="ed-split">
<div class="ed-prose-col">

# Implementation changed.<br>Responsibility did not.

<p class="ed-prose">A model can turn a plausible idea into plausible code in minutes. It cannot decide whether the idea is right, which source is authoritative, what it may reach, or who accepts the consequences.</p>
<p class="ed-prose"><strong>When AI compresses implementation, the bottleneck moves:</strong> from producing code to accepting a change into production with evidence and an owner.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="16" y="24" class="svg-kicker" fill="#0c7fa3">WHAT AI COMPRESSES</text>
  <rect x="16" y="42" width="220" height="276" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="36" y="76" class="svg-h3" fill="#0d1030">Implementation effort</text>
  <line x1="36" y1="92" x2="216" y2="92" stroke="rgba(13,16,48,0.12)" />
  <text x="36" y="126" class="svg-label" fill="#0d1030">Research and drafting</text>
  <text x="36" y="162" class="svg-label" fill="#0d1030">Code and test generation</text>
  <text x="36" y="198" class="svg-label" fill="#0d1030">Translation and repetition</text>
  <path d="M 52 242 L 192 242" stroke="rgba(12,127,163,0.35)" stroke-width="2" />
  <polygon points="186,237 196,242 186,247" fill="#0c7fa3" />
  <text x="52" y="230" class="svg-caption" fill="rgba(13,16,48,0.58)">Weeks</text>
  <text x="196" y="230" text-anchor="end" class="svg-caption" fill="#0c7fa3">Minutes</text>
  <text x="36" y="286" class="svg-caption" fill="#0c7fa3">More output, sooner</text>

  <text x="272" y="24" class="svg-kicker" fill="#a4238f">WHAT DOES NOT DISAPPEAR</text>
  <rect x="272" y="42" width="352" height="276" rx="14" fill="#0d1030" />
  <circle cx="302" cy="88" r="14" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.4" />
  <text x="302" y="93" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="330" y="84" class="svg-label" fill="#ffffff">Direction</text>
  <text x="330" y="102" class="svg-caption" fill="rgba(255,255,255,0.68)">What should change, for whom, and why</text>

  <circle cx="302" cy="148" r="14" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.4" />
  <text x="302" y="153" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="330" y="144" class="svg-label" fill="#ffffff">Authority</text>
  <text x="330" y="162" class="svg-caption" fill="rgba(255,255,255,0.68)">Which evidence wins and what code may reach</text>

  <circle cx="302" cy="208" r="14" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.4" />
  <text x="302" y="213" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="330" y="204" class="svg-label" fill="#ffffff">Evidence</text>
  <text x="330" y="222" class="svg-caption" fill="rgba(255,255,255,0.68)">Why the result is safe to accept</text>

  <circle cx="302" cy="268" r="14" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.4" />
  <text x="302" y="273" text-anchor="middle" class="svg-pill" fill="#f59ef5">04</text>
  <text x="330" y="264" class="svg-label" fill="#ffffff">Ownership</text>
  <text x="330" y="282" class="svg-caption" fill="rgba(255,255,255,0.68)">Who releases it and answers when it fails</text>

  <line x1="126" y1="318" x2="126" y2="352" stroke="#0c7fa3" stroke-width="1.8" />
  <line x1="448" y1="318" x2="448" y2="352" stroke="#a4238f" stroke-width="1.8" />
  <rect x="78" y="354" width="498" height="66" rx="12" fill="rgba(164,35,143,0.07)" stroke="#a4238f" stroke-width="1.8" />
  <text x="327" y="380" text-anchor="middle" class="svg-kicker" fill="#a4238f">THE NEW DELIVERY BOTTLENECK</text>
  <text x="327" y="403" text-anchor="middle" class="svg-h3" fill="#0d1030">An accepted change, safely in production</text>
</svg>
</div>
</div>

<!--
The economic shift:
- The model has compressed implementation effort, not organisational responsibility.
- Plausibility is now abundant too. More plausible output increases the need for clear authority and independent evidence.
- This is why a coding tool alone cannot be the delivery system.

Client question:
- Which of direction, authority, evidence, or ownership currently slows your changes most?
-->

---
title: Begin with the change
class: slide-light
---

<Eyebrow text="THE STARTING POINT · OUTCOME BEFORE TECHNOLOGY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Begin with<br>the change.

</div>
<p class="ed-band-prose">We start with the customer, operational, or commercial result—not a model, a backlog, or a target architecture. <strong>Faster implementation makes weak assumptions expensive sooner.</strong> Direction is delivery work.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="66" width="206" height="152" rx="14" fill="rgba(22,180,226,0.07)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="38" y="96" class="svg-kicker" fill="#0c7fa3">01 · BENEFIT</text>
  <text x="38" y="128" class="svg-h3" fill="#0d1030">Who benefits?</text>
  <line x1="38" y1="142" x2="204" y2="142" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="170" class="svg-caption" fill="rgba(13,16,48,0.6)">Customer, operator,</text>
  <text x="38" y="190" class="svg-caption" fill="rgba(13,16,48,0.6)">or business unit</text>

  <line x1="224" y1="142" x2="240" y2="142" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="234,137 244,142 234,147" fill="#0c7fa3" />

  <rect x="242" y="66" width="206" height="152" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.28)" stroke-width="1.6" />
  <text x="262" y="96" class="svg-kicker" fill="#0d1030">02 · OUTCOME</text>
  <text x="262" y="128" class="svg-h3" fill="#0d1030">What changes?</text>
  <line x1="262" y1="142" x2="428" y2="142" stroke="rgba(13,16,48,0.12)" />
  <text x="262" y="170" class="svg-caption" fill="rgba(13,16,48,0.6)">Observable behaviour,</text>
  <text x="262" y="190" class="svg-caption" fill="rgba(13,16,48,0.6)">not feature volume</text>

  <line x1="448" y1="142" x2="464" y2="142" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="458,137 468,142 458,147" fill="#0d1030" />

  <rect x="466" y="66" width="206" height="152" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.28)" stroke-width="1.6" />
  <text x="486" y="96" class="svg-kicker" fill="#0d1030">03 · EVIDENCE</text>
  <text x="486" y="128" class="svg-h3" fill="#0d1030">How will we know?</text>
  <line x1="486" y1="142" x2="652" y2="142" stroke="rgba(13,16,48,0.12)" />
  <text x="486" y="170" class="svg-caption" fill="rgba(13,16,48,0.6)">Acceptance checks and</text>
  <text x="486" y="190" class="svg-caption" fill="rgba(13,16,48,0.6)">operational measures</text>

  <line x1="672" y1="142" x2="688" y2="142" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="682,137 692,142 682,147" fill="#0d1030" />

  <rect x="690" y="66" width="206" height="152" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="710" y="96" class="svg-kicker" fill="#a4238f">04 · AUTHORITY</text>
  <text x="710" y="128" class="svg-h3" fill="#0d1030">Who decides?</text>
  <line x1="710" y1="142" x2="876" y2="142" stroke="rgba(164,35,143,0.18)" />
  <text x="710" y="170" class="svg-caption" fill="rgba(13,16,48,0.6)">Trade-offs, acceptance,</text>
  <text x="710" y="190" class="svg-caption" fill="rgba(13,16,48,0.6)">and release</text>

  <line x1="896" y1="142" x2="912" y2="142" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="906,137 916,142 906,147" fill="#a4238f" />

  <rect x="914" y="66" width="206" height="152" rx="14" fill="#0d1030" />
  <text x="1017" y="96" text-anchor="middle" class="svg-kicker" fill="#78d8f2">BOUNDED FIRST MOVE</text>
  <text x="1017" y="128" text-anchor="middle" class="svg-h3" fill="#ffffff">One useful</text>
  <text x="1017" y="150" text-anchor="middle" class="svg-h3" fill="#ffffff">production outcome</text>
  <line x1="936" y1="164" x2="1098" y2="164" stroke="rgba(255,255,255,0.14)" />
  <text x="1017" y="190" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Small enough to learn</text>

  <rect x="222" y="270" width="696" height="50" rx="12" fill="rgba(22,180,226,0.07)" stroke="rgba(12,127,163,0.3)" />
  <text x="570" y="300" text-anchor="middle" class="svg-label" fill="#0d1030">What should be easier, safer, or faster than it is today?</text>
</svg>
</div>
</div>

<!--
The first discipline:
- We do not begin with “Where can we use AI?” We begin with a result that should be easier, safer, or faster than it is today.
- The boundary is deliberately small enough to learn from and meaningful enough to expose the real delivery system.
- This is not a giant up-front specification. It is enough direction to stop fast implementation from hardening a weak assumption.

Client question:
- What is one change that should be easier, safer, or faster than it is today?
-->

---
title: Set rules. Record intent. Constrain code.
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="THE SYSTEM · THREE FOUNDATIONS, ONE PATH TO PRODUCTION" />

# Set rules. Record intent. Constrain code.

<p class="slide-lead">The model works inside the delivery system. The authority to define that system remains outside it.</p>

<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 400" aria-hidden="true">
  <rect x="180" y="6" width="780" height="38" rx="19" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.24)" />
  <text x="570" y="30" text-anchor="middle" class="svg-kicker" fill="#ffffff">PROTECTED CONTROL PLANE · PEOPLE SET OUTCOME, SOURCE PRECEDENCE, ACCEPTANCE, RELEASE</text>

  <line x1="205" y1="44" x2="205" y2="78" stroke="#78d8f2" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="570" y1="44" x2="570" y2="78" stroke="#f59ef5" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="935" y1="44" x2="935" y2="78" stroke="#78d8f2" stroke-width="1.5" stroke-dasharray="4 4" />

  <rect x="12" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="38" y="112" class="svg-kicker" fill="#78d8f2">01 · SET RULES · AUTHORING CONTROL</text>
  <text x="38" y="142" class="svg-h2" fill="#ffffff">Rules-based toolchain</text>
  <line x1="38" y1="156" x2="342" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="38" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Approved tools and models</text>
  <text x="38" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Data, source, and worker boundaries</text>
  <text x="38" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Agent instructions and pipeline gates</text>
  <text x="38" y="270" class="svg-caption" fill="#78d8f2">Governs how code may be created</text>

  <rect x="392" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="418" y="112" class="svg-kicker" fill="#f59ef5">02 · RECORD INTENT · DELIVERY CONTROL</text>
  <text x="418" y="142" class="svg-h2" fill="#ffffff">Emery</text>
  <line x1="418" y1="156" x2="722" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="418" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Evidence and explicit source authority</text>
  <text x="418" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Reviewed specification, design, and plan</text>
  <text x="418" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Bounded implementation and verification</text>
  <text x="418" y="270" class="svg-caption" fill="#f59ef5">Governs what must remain true</text>

  <rect x="772" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="798" y="112" class="svg-kicker" fill="#78d8f2">03 · CONSTRAIN CODE · RUN-TIME CONTROL</text>
  <text x="798" y="142" class="svg-h2" fill="#ffffff">Omnia</text>
  <line x1="798" y1="156" x2="1102" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="798" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Sandboxed WebAssembly components</text>
  <text x="798" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Typed capabilities and explicit grants</text>
  <text x="798" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Credentials and infrastructure in the host</text>
  <text x="798" y="270" class="svg-caption" fill="#78d8f2">Governs what running code may touch</text>

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
  <text x="1102" y="372" text-anchor="end" class="svg-caption" fill="#f59ef5">Emery and Omnia are open source.</text>
</svg>
</div>

<!--
The complete system:
- These are three different controls over three different questions.
- The rules-based toolchain governs how people and agents may work.
- Emery governs what the software must do and what evidence supports accepting it.
- Omnia governs what the running application may touch.
- Reusable delivery assets carry the result to production; they are supporting infrastructure, not a substitute fourth foundation.

Boundary:
- The coding agent's working environment and the application's runtime sandbox are separate trust boundaries.
-->

---
title: Rules travel with the work
class: slide-light
---

<Eyebrow text="FOUNDATION 01 · RULES-BASED AUTHORING" />

<div class="ed-split">
<div class="ed-prose-col">

# Rules travel<br>with the work.

<p class="ed-prose">A policy document does not govern an agent turn. The agreed controls must appear where work happens: in editor configuration, agent instructions, tool grants, templates, and pipeline checks.</p>
<p class="ed-prose"><strong>The same boundary applies to every author</strong>—engineer, domain expert, or coding agent.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="14" y="26" width="612" height="366" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="38" y="56" class="svg-kicker" fill="#0c7fa3">THE AUTHORING BOUNDARY · SHARED ENGINEERING CONTROLS</text>

  <rect x="38" y="82" width="256" height="226" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="58" y="110" class="svg-kicker" fill="#0c7fa3">GUIDANCE · EASY TO FOLLOW</text>
  <text x="58" y="138" class="svg-label" fill="#0d1030">Approved tools and models</text>
  <text x="58" y="168" class="svg-label" fill="#0d1030">Editor and agent instructions</text>
  <text x="58" y="198" class="svg-label" fill="#0d1030">Known project skeletons</text>
  <text x="58" y="228" class="svg-label" fill="#0d1030">Examples and reference material</text>
  <line x1="58" y1="252" x2="274" y2="252" stroke="rgba(13,16,48,0.12)" />
  <text x="58" y="278" class="svg-caption" fill="rgba(13,16,48,0.6)">Makes the safe path the shortest path</text>

  <line x1="294" y1="195" x2="342" y2="195" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="336,190 346,195 336,200" fill="#0d1030" />
  <text x="318" y="181" text-anchor="middle" class="svg-caption" fill="#0d1030">Change</text>

  <rect x="346" y="82" width="256" height="226" rx="12" fill="#0d1030" />
  <text x="366" y="110" class="svg-kicker" fill="#78d8f2">ENFORCEMENT · MUST PASS</text>
  <text x="366" y="138" class="svg-label" fill="#ffffff">Explicit tool and worker grants</text>
  <text x="366" y="168" class="svg-label" fill="#ffffff">Secret, dependency, and test gates</text>
  <text x="366" y="198" class="svg-label" fill="#ffffff">Protected publication</text>
  <text x="366" y="228" class="svg-label" fill="#ffffff">Named acceptance and release</text>
  <line x1="366" y1="252" x2="582" y2="252" stroke="rgba(255,255,255,0.14)" />
  <text x="366" y="278" class="svg-caption" fill="#78d8f2">Blocks what cannot be discretionary</text>

  <rect x="72" y="330" width="496" height="40" rx="20" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.4)" />
  <text x="320" y="355" text-anchor="middle" class="svg-label" fill="#a4238f">Tool access is granted explicitly. The agent cannot infer or extend it.</text>

  <text x="320" y="424" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">Guidance shapes proposals. Deterministic controls govern acceptance.</text>
</svg>
</div>
</div>

<!--
Foundation one — how code is authored:
- Advice and enforcement are kept deliberately separate.
- Guidance belongs in the editor and the working context, where it is cheap to follow.
- Non-negotiable controls belong in grants, pipelines, and explicit human decisions.
- This is how a rule survives a change of developer, agent, or model provider.

Client question:
- Today, what may a coding agent read, execute, publish, or send—and where is that actually enforced?
-->

---
title: Chat history is not an audit trail
class: slide-light
---

<Eyebrow text="FOUNDATION 02 · SPEC-DRIVEN DELIVERY WITH EMERY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Chat history is<br>not an audit trail.

</div>
<p class="ed-band-prose"><strong>Emery sits around coding agents, not in place of them.</strong> It keeps evidence, authority, reviewed intent, exact delivery inputs, and verification outside any model’s conversational context. Swap the model; the record stays.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <text x="16" y="24" class="svg-kicker" fill="#0c7fa3">AUTHORITATIVE INPUTS</text>
  <rect x="16" y="42" width="190" height="54" rx="9" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.4" />
  <text x="34" y="64" class="svg-kicker" fill="#0c7fa3">INTENT</text>
  <text x="34" y="82" class="svg-label" fill="#0d1030">Briefs and directives</text>
  <rect x="16" y="110" width="190" height="54" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.24)" stroke-width="1.4" />
  <text x="34" y="132" class="svg-kicker" fill="#0d1030">EVIDENCE</text>
  <text x="34" y="150" class="svg-label" fill="#0d1030">Documents and tests</text>
  <rect x="16" y="178" width="190" height="68" rx="9" fill="rgba(164,35,143,0.05)" stroke="#a4238f" stroke-width="1.4" />
  <text x="34" y="200" class="svg-kicker" fill="#a4238f">OBSERVED BEHAVIOUR</text>
  <text x="34" y="218" class="svg-label" fill="#0d1030">Code and runtime facts</text>
  <text x="34" y="234" class="svg-caption" fill="rgba(13,16,48,0.56)">Anchored to their source</text>

  <path d="M 206 69 C 250 69, 250 144, 286 144" fill="none" stroke="#0c7fa3" stroke-width="1.5" />
  <line x1="206" y1="137" x2="286" y2="144" stroke="#0d1030" stroke-width="1.5" />
  <path d="M 206 212 C 250 212, 250 144, 286 144" fill="none" stroke="#a4238f" stroke-width="1.5" />

  <rect x="290" y="52" width="230" height="194" rx="14" fill="#0d1030" />
  <text x="405" y="80" text-anchor="middle" class="svg-kicker" fill="#78d8f2">EVIDENCE + SOURCE PRECEDENCE</text>
  <circle cx="322" cy="114" r="4" fill="#78d8f2" />
  <text x="338" y="118" class="svg-label" fill="#ffffff">Corroborated evidence</text>
  <circle cx="322" cy="154" r="4" fill="#f59ef5" />
  <text x="338" y="158" class="svg-label" fill="#ffffff">Ranked divergence</text>
  <circle cx="322" cy="194" r="4" fill="#ff7597" />
  <text x="338" y="198" class="svg-label" fill="#ffffff">Visible conflicts</text>
  <text x="322" y="226" class="svg-caption" fill="rgba(255,255,255,0.68)">Unknown remains [unknown]</text>

  <line x1="520" y1="149" x2="578" y2="149" stroke="#a4238f" stroke-width="2" />
  <polygon points="572,144 582,149 572,154" fill="#a4238f" />
  <text x="550" y="136" text-anchor="middle" class="svg-pill" fill="#a4238f">SEAL</text>

  <rect x="584" y="52" width="238" height="194" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.8" />
  <text x="703" y="80" text-anchor="middle" class="svg-kicker" fill="#a4238f">VERSIONED DELIVERY REVISION</text>
  <rect x="606" y="98" width="194" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.3)" />
  <text x="624" y="121" class="svg-label" fill="#0d1030">Specification · behaviour</text>
  <rect x="606" y="144" width="194" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.3)" />
  <text x="624" y="167" class="svg-label" fill="#0d1030">Design · technology</text>
  <rect x="606" y="190" width="194" height="36" rx="8" fill="#ffffff" stroke="rgba(164,35,143,0.3)" />
  <text x="624" y="213" class="svg-label" fill="#0d1030">Build plan · dependencies</text>

  <line x1="822" y1="149" x2="878" y2="149" stroke="#0d1030" stroke-width="2" />
  <polygon points="872,144 882,149 872,154" fill="#0d1030" />

  <rect x="884" y="52" width="240" height="194" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="1004" y="80" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">BOUNDED IMPLEMENTATION</text>
  <text x="906" y="112" class="svg-label" fill="#0d1030">Dependency-ordered waves</text>
  <text x="906" y="140" class="svg-label" fill="#0d1030">Isolated slice workspaces</text>
  <text x="906" y="168" class="svg-label" fill="#0d1030">Integrated verification</text>
  <line x1="906" y1="188" x2="1102" y2="188" stroke="rgba(13,16,48,0.12)" />
  <text x="1004" y="216" text-anchor="middle" class="svg-h3" fill="#0c7fa3">Verified revision</text>

  <rect x="180" y="288" width="780" height="48" rx="12" fill="#0d1030" />
  <text x="570" y="317" text-anchor="middle" class="svg-label" fill="#ffffff">The model may extract, propose, implement, and repair. It never becomes the source of truth.</text>
</svg>
</div>
</div>

<!--
Foundation two — what must remain true:
- Sources stay distinct and claims stay connected to evidence.
- Authority is explicit. A product directive can outrank accidental behaviour in old code; equal authority can remain a conflict for people to settle.
- The model helps synthesise the candidate. A person seals the revision.
- The sealed revision, not the chat, directs bounded implementation and verification.

Client question:
- If the current model, vendor, and chat history disappeared tomorrow, where would the accepted intent and evidence remain?
-->

---
title: Generated code gets no ambient authority
class: slide-light
---

<Eyebrow text="FOUNDATION 03 · SANDBOXED EXECUTION WITH OMNIA" />

<div class="ed-split">
<div class="ed-prose-col">

# Generated code gets<br>no ambient authority.

<p class="ed-prose"><strong>Omnia hosts application logic as a WebAssembly component.</strong> It has no ambient filesystem, network, process, or environment access. The host supplies only the typed capabilities the application was granted.</p>
<p class="ed-prose">Credentials, concrete backends, and infrastructure policy stay outside application code. The boundary complements—not replaces—credential scope, network policy, and backend isolation.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="18" y="22" width="604" height="390" rx="18" fill="#0d1030" />
  <text x="42" y="52" class="svg-kicker" fill="#78d8f2">OMNIA HOST · RUST ON WASMTIME</text>
  <text x="598" y="52" text-anchor="end" class="svg-caption" fill="rgba(255,255,255,0.6)">Host owns policy and authority</text>

  <rect x="74" y="78" width="492" height="112" rx="12" fill="rgba(255,255,255,0.08)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="320" y="106" text-anchor="middle" class="svg-kicker" fill="#78d8f2">SANDBOXED WASI APPLICATION COMPONENT</text>
  <text x="320" y="136" text-anchor="middle" class="svg-h2" fill="#ffffff">Differentiating business behaviour</text>
  <text x="320" y="162" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">No provider SDKs · No credentials · No ambient access</text>

  <line x1="158" y1="190" x2="158" y2="232" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="153,226 163,226 158,236" fill="#78d8f2" />
  <line x1="320" y1="190" x2="320" y2="232" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="315,226 325,226 320,236" fill="#78d8f2" />
  <line x1="482" y1="190" x2="482" y2="232" stroke="#78d8f2" stroke-width="1.8" />
  <polygon points="477,226 487,226 482,236" fill="#78d8f2" />
  <rect x="206" y="202" width="228" height="26" rx="13" fill="rgba(214,58,214,0.16)" stroke="#f59ef5" />
  <text x="320" y="219" text-anchor="middle" class="svg-pill" fill="#f59ef5">TYPED CAPABILITY CONTRACTS</text>

  <rect x="74" y="240" width="138" height="68" rx="9" fill="rgba(255,255,255,0.1)" />
  <text x="143" y="267" text-anchor="middle" class="svg-label" fill="#ffffff">Data &amp; state</text>
  <text x="143" y="288" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.62)">SQL · key-value · blob</text>

  <rect x="251" y="240" width="138" height="68" rx="9" fill="rgba(255,255,255,0.1)" />
  <text x="320" y="267" text-anchor="middle" class="svg-label" fill="#ffffff">Flow &amp; calls</text>
  <text x="320" y="288" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.62)">Messaging · HTTP</text>

  <rect x="428" y="240" width="138" height="68" rx="9" fill="rgba(255,255,255,0.1)" />
  <text x="497" y="267" text-anchor="middle" class="svg-label" fill="#ffffff">Trust &amp; insight</text>
  <text x="497" y="288" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.62)">Identity · telemetry</text>

  <rect x="74" y="332" width="492" height="52" rx="10" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.18)" />
  <text x="98" y="354" class="svg-kicker" fill="#78d8f2">HOST-SUPPLIED</text>
  <text x="216" y="354" class="svg-body" fill="#ffffff">Backends · Credentials · Egress · Configuration · Observability</text>
  <text x="320" y="374" text-anchor="middle" class="svg-caption" fill="#f59ef5">No grant means no reach.</text>

  <text x="320" y="432" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">The authoring controls bound the agent. Omnia bounds the application it produced.</text>
</svg>
</div>
</div>

<!--
Foundation three — what the result may touch:
- Omnia starts from no ambient authority. Access exists only through a typed capability linked by the host.
- The guest asks for a role such as state or messaging. The host chooses the backend and holds its credentials.
- A poisoned dependency or generated mistake cannot reach something the host did not grant.

Boundary:
- This complements infrastructure security; it does not replace credential scope, network policy, or backend isolation.

Client question:
- If generated application code were compromised, what could it reach today simply because the process environment can?
-->

---
title: One delivery system. Two starting points.
class: slide-light
---

<Eyebrow text="WHERE IT APPLIES · NEW SYSTEMS AND MODERNISATION" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <rect x="18" y="20" width="250" height="100" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="40" y="48" class="svg-kicker" fill="#0c7fa3">NEW SYSTEM · POSSIBILITY</text>
  <text x="40" y="76" class="svg-h3" fill="#0d1030">A useful idea or prototype</text>
  <text x="40" y="98" class="svg-caption" fill="rgba(13,16,48,0.62)">Outcome · constraints · early evidence</text>

  <rect x="18" y="164" width="250" height="100" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="40" y="192" class="svg-kicker" fill="#a4238f">EXISTING SYSTEM · DEPENDENCE</text>
  <text x="40" y="220" class="svg-h3" fill="#0d1030">Code nobody can safely change</text>
  <text x="40" y="242" class="svg-caption" fill="rgba(13,16,48,0.62)">Intent · docs · tests · code · people</text>

  <path d="M 268 70 C 330 70, 334 142, 396 142" fill="none" stroke="#0c7fa3" stroke-width="2" />
  <path d="M 268 214 C 330 214, 334 142, 396 142" fill="none" stroke="#a4238f" stroke-width="2" />
  <polygon points="390,137 400,142 390,147" fill="#0d1030" />

  <rect x="402" y="82" width="228" height="120" rx="14" fill="#0d1030" />
  <text x="516" y="110" text-anchor="middle" class="svg-kicker" fill="#78d8f2">REVIEWED DELIVERY REVISION</text>
  <text x="516" y="140" text-anchor="middle" class="svg-h3" fill="#ffffff">What must remain true</text>
  <text x="516" y="164" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">Specification · design · plan</text>
  <text x="516" y="184" text-anchor="middle" class="svg-caption" fill="#f59ef5">Conflicts and unknowns visible</text>

  <line x1="630" y1="142" x2="696" y2="142" stroke="#0d1030" stroke-width="2" />
  <polygon points="690,137 700,142 690,147" fill="#0d1030" />

  <rect x="702" y="82" width="190" height="120" rx="14" fill="rgba(22,180,226,0.07)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="797" y="110" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">BOUNDED DELIVERY</text>
  <text x="797" y="140" text-anchor="middle" class="svg-h3" fill="#0d1030">Verified waves</text>
  <text x="797" y="164" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.64)">Small changes · clear checks</text>
  <text x="797" y="184" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Evidence before promotion</text>

  <line x1="892" y1="142" x2="952" y2="142" stroke="#a4238f" stroke-width="2" />
  <polygon points="946,137 956,142 946,147" fill="#a4238f" />

  <circle cx="1038" cy="142" r="78" fill="#a4238f" />
  <text x="1038" y="114" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.86)">RUNTIME BOUNDARY</text>
  <text x="1038" y="142" text-anchor="middle" class="svg-h3" fill="#ffffff">Omnia runtime</text>
  <text x="1038" y="164" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.82)">Explicit capabilities</text>
  <text x="1038" y="182" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.82)">Known path to operation</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# One delivery system.<br>Two starting points.

</div>
<div>
<p class="ed-prose"><strong>A demo proves possibility. Production proves value.</strong> New systems need enough durable direction to reach production without hardening weak assumptions.</p>
<p class="ed-prose"><strong>Recover the product, not just the code.</strong> Modernisation reconciles what an existing system does with what it should continue to do before anything is rewritten.</p>
</div>
</div>
</div>

<!--
Two entry paths:
- For a new system, the danger is turning a plausible demo into accidental architecture and accidental product policy.
- For an existing system, the danger is translating historical implementation and reproducing its problems in a new language.
- Both paths converge on reviewed intent, bounded implementation, independent verification, and constrained operation.

Client question:
- Is your immediate pressure a new capability that cannot reach production, or an existing system that cannot safely change?
-->

---
title: Fix the system, not the output
class: slide-light
---

<Eyebrow text="EVIDENCE · AN AI-ASSISTED REBUILD OF A LIVE SERVICE" />

<div class="ed-split">
<div class="ed-prose-col">

# Fix the system,<br>not the output.

<p class="ed-prose">We recovered a specification from an anonymised real-time rail service, rebuilt it as a WASI component, and held it to the fixed 22-case replay suite used for its hand-written counterpart.</p>
<p class="ed-prose">An early build passed none. <strong>No defect was hand-patched.</strong> Each one changed the checks, guidance, extraction, or tooling every later build inherits.</p>
<p class="ed-footnote">Propellerhead internal evaluation, October 2026. Latest result: 21 of 22 cases, including all 12 recorded production messages. One case remains open and reported.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="14" y="22" class="svg-kicker" fill="rgba(13,16,48,0.52)">THE SAME FIXED REPLAY SUITE · 22 RECORDED CASES</text>

  <rect x="14" y="42" width="178" height="116" rx="12" fill="rgba(255,59,107,0.05)" stroke="rgba(180,35,78,0.4)" />
  <text x="103" y="96" text-anchor="middle" class="svg-h2" fill="#b4234e" style="font-size: 36px;">0 / 22</text>
  <text x="103" y="122" text-anchor="middle" class="svg-kicker" fill="#b4234e">EARLY BUILD</text>
  <text x="103" y="142" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Plausible. Wrong.</text>

  <line x1="192" y1="100" x2="232" y2="100" stroke="#0c7fa3" stroke-width="2" />
  <polygon points="226,95 236,100 226,105" fill="#0c7fa3" />

  <rect x="238" y="42" width="178" height="116" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="327" y="96" text-anchor="middle" class="svg-h2" fill="#0c7fa3" style="font-size: 36px;">7 / 22</text>
  <text x="327" y="122" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">SYSTEM IMPROVED</text>
  <text x="327" y="142" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Checks and guidance</text>

  <line x1="416" y1="100" x2="456" y2="100" stroke="#a4238f" stroke-width="2" />
  <polygon points="450,95 460,100 450,105" fill="#a4238f" />

  <rect x="462" y="42" width="164" height="116" rx="12" fill="#a4238f" />
  <text x="544" y="96" text-anchor="middle" class="svg-h2" fill="#ffffff" style="font-size: 36px;">21 / 22</text>
  <text x="544" y="122" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.86)">LATEST BUILD</text>
  <text x="544" y="142" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">12 / 12 production messages</text>

  <rect x="14" y="190" width="612" height="214" rx="14" fill="#0d1030" />
  <text x="38" y="220" class="svg-kicker" fill="#78d8f2">A FAILURE BECOMES A SYSTEM CAPABILITY</text>

  <text x="38" y="258" class="svg-label" fill="#ffffff">A consumer never registered</text>
  <line x1="250" y1="253" x2="324" y2="253" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,248 328,253 318,258" fill="#78d8f2" />
  <text x="344" y="258" class="svg-caption" fill="#78d8f2">A check now refuses unreachable publishers</text>

  <text x="38" y="302" class="svg-label" fill="#ffffff">Rejections looked processed</text>
  <line x1="250" y1="297" x2="324" y2="297" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,292 328,297 318,302" fill="#78d8f2" />
  <text x="344" y="302" class="svg-caption" fill="#78d8f2">A check now refuses swallowed errors</text>

  <text x="38" y="346" class="svg-label" fill="#ffffff">A type was laid out empty</text>
  <line x1="250" y1="341" x2="324" y2="341" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,336 328,341 318,346" fill="#f59ef5" />
  <text x="344" y="346" class="svg-caption" fill="#f59ef5">The design-type reader was repaired</text>

  <line x1="38" y1="370" x2="602" y2="370" stroke="rgba(255,255,255,0.12)" />
  <text x="320" y="390" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">The next build starts with every lesson the previous one paid for.</text>
</svg>
</div>
</div>

<!--
Evidence, stated at the level achieved:
- The early output looked plausible line by line. The recorded cases showed it was not correct.
- We did not rescue the demonstration with hand edits.
- Each defect became a reusable capability in the delivery system: a check, a better extraction rule, clearer guidance, or repaired tooling.
- The open case remains visible. Evidence is useful because it can still say no.

The point:
- 21 is not the product claim. The compounding delivery system is.
-->

---
title: Start with the thing that is stuck
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>25 years of critical-software judgement, now applied to accountable delivery at AI speed.</p>
</div>
</div>

<div class="split-content-pane">
<div>
<Eyebrow text="PROPELLERHEAD · THE FIRST MOVE" />

# Start with the thing<br>that is stuck.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Bring one real change and its owner</h4>
<p>Something useful that should be easier, safer, or faster to put into production than it is today.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>We trace its full delivery path</h4>
<p>Direction, authoring rules, durable intent, verification, runtime authority, infrastructure, release, and ownership.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Leave with assets and a decision</h4>
<p>A bounded first delivery and its controls—or a clear reason to fix a foundation, reframe the work, or stop.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">AI DELIVERY READINESS · SHORT PAID ENGAGEMENT</div>
<div class="cta-card-title">Nominate one change. Name its owner.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>Talk to us</span>
</div>
</div>
</div>

<!--
Close with one ask:
- Bring one real change that matters and is harder to deliver than it should be.
- Readiness is a short, paid engagement, not a maturity score or a generic AI programme.
- It establishes the first useful boundary and ends in an explicit decision.
- Sometimes the responsible answer is not to build yet. That is still a valuable result.

Final question:
- What should be easier, safer, or faster than it is today?
-->
