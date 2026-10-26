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
sync: shift
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
sync: outcome
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
sync: system
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
  <text x="38" y="112" class="svg-kicker" fill="#78d8f2">01 · SET RULES</text>
  <text x="38" y="142" class="svg-h2" fill="#ffffff">Toolchain controls</text>
  <line x1="38" y1="156" x2="342" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="38" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Approved tools and models</text>
  <text x="38" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Data, source, and worker boundaries</text>
  <text x="38" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Agent instructions and pipeline gates</text>
  <text x="38" y="270" class="svg-caption" fill="#78d8f2">Governs how code may be created</text>

  <rect x="392" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="418" y="112" class="svg-kicker" fill="#f59ef5">02 · RECORD INTENT</text>
  <text x="418" y="142" class="svg-h2" fill="#ffffff">Spec-driven delivery</text>
  <line x1="418" y1="156" x2="722" y2="156" stroke="rgba(255,255,255,0.12)" />
  <text x="418" y="184" class="svg-body" fill="rgba(255,255,255,0.84)">Evidence and explicit source authority</text>
  <text x="418" y="208" class="svg-body" fill="rgba(255,255,255,0.84)">Versioned spec.md and design.md</text>
  <text x="418" y="232" class="svg-body" fill="rgba(255,255,255,0.84)">Bounded implementation and verification</text>
  <text x="418" y="270" class="svg-caption" fill="#f59ef5">Governs what must remain true</text>

  <rect x="772" y="82" width="356" height="208" rx="16" fill="rgba(10,7,16,0.7)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="798" y="112" class="svg-kicker" fill="#78d8f2">03 · CONSTRAIN CODE</text>
  <text x="798" y="142" class="svg-h2" fill="#ffffff">Sandboxed execution</text>
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
  <text x="1102" y="372" text-anchor="end" class="svg-caption" fill="#f59ef5">The delivery engine and runtime are open source.</text>
</svg>
</div>

<!--
The complete system:
- This is the map for the next three sections. Take each control in turn.
- These are three different controls over three different questions.
- Toolchain controls govern how people and agents may work.
- Spec-driven delivery governs what the software must do and what evidence supports accepting it.
- Sandboxed execution governs what the running application may touch.
- Reusable delivery assets carry the result to production; they are supporting infrastructure, not a substitute fourth foundation.

Boundary:
- The coding agent's working environment and the application's runtime sandbox are separate trust boundaries.
-->

---
title: Isolate. Standardise. Automate.
sync: authoring
class: slide-light
---

<Eyebrow text="01 · SET RULES · TOOLCHAIN CONTROLS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Set rules</span>
<span>02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="ed-split">
<div class="ed-prose-col">

# Isolate.<br>Standardise.<br>Automate.

<p class="ed-prose">Teams move faster when experimentation happens in a governed place, through an approved path, under checks that apply to every change.</p>
<p class="ed-prose"><strong>The same engineering codex and controls travel with every builder</strong>—engineer, domain expert, or coding agent.</p>

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
  <text x="102" y="178" class="svg-kicker" fill="#a4238f">STANDARDISE</text>
  <text x="102" y="204" class="svg-h3" fill="#0d1030">Shared engineering codex</text>
  <text x="102" y="228" class="svg-caption" fill="rgba(13,16,48,0.62)">Standards, patterns, examples, and templates—written for people and agents</text>

  <rect x="14" y="264" width="612" height="102" rx="14" fill="#0d1030" />
  <circle cx="62" cy="315" r="22" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.4" />
  <text x="62" y="320" text-anchor="middle" class="svg-label" fill="#78d8f2">03</text>
  <text x="102" y="298" class="svg-kicker" fill="#78d8f2">AUTOMATE</text>
  <text x="102" y="324" class="svg-h3" fill="#ffffff">Compiler-enforced standards</text>
  <text x="102" y="344" class="svg-caption" fill="rgba(255,255,255,0.7)">Critical coding rules are checked automatically, alongside security,</text>
  <text x="102" y="360" class="svg-caption" fill="rgba(255,255,255,0.7)">dependency, quality, and test controls</text>

  <rect x="72" y="390" width="496" height="38" rx="19" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.4)" />
  <text x="320" y="414" text-anchor="middle" class="svg-caption" fill="#a4238f">Faster experimentation · Consistent controls · Auditable delivery</text>
</svg>
</div>
</div>

<!--
Foundation one — how code is authored:
- Isolate work from production while an idea is still changing.
- Publish one versioned engineering codex so people and agents work from the same standards, patterns, examples, and templates.
- Enforce critical coding rules as the code compiles, then apply the same security, dependency, quality, and test controls to every change.

Client question:
- Where does experimentation currently become inconsistent, risky, or difficult to audit?
-->

---
title: One safe path from build to trial
sync: authoring-path
class: slide-light
---

<Eyebrow text="01 · SET RULES · THE DELIVERY PATH" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Set rules</span>
<span>02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# One safe path<br>from build to trial.

</div>
<p class="ed-band-prose">A versioned engineering codex, approved tools, policies, and deployment patterns follow every change. <strong>The codex guides people and agents. Gates enforce what cannot be optional.</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <rect x="180" y="12" width="780" height="40" rx="20" fill="rgba(22,180,226,0.07)" stroke="rgba(12,127,163,0.32)" />
  <text x="570" y="37" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">APPROVED TOOLCHAIN · ENGINEERING CODEX · TOOLS · POLICIES · DEPLOYMENT PATTERN</text>

  <rect x="16" y="88" width="230" height="154" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="38" y="118" class="svg-kicker" fill="#0c7fa3">01 · CREATE</text>
  <text x="38" y="150" class="svg-h3" fill="#0d1030">AI-assisted build</text>
  <line x1="38" y1="166" x2="224" y2="166" stroke="rgba(13,16,48,0.12)" />
  <text x="38" y="196" class="svg-label" fill="#0d1030">Governed workspace</text>
  <text x="38" y="220" class="svg-caption" fill="rgba(13,16,48,0.6)">Fast iteration, no production reach</text>

  <line x1="246" y1="165" x2="296" y2="165" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="290,160 300,165 290,170" fill="#0d1030" />

  <rect x="300" y="88" width="230" height="154" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.24)" stroke-width="1.5" />
  <text x="322" y="118" class="svg-kicker" fill="#0d1030">02 · RECORD</text>
  <text x="322" y="150" class="svg-h3" fill="#0d1030">Enterprise source control</text>
  <line x1="322" y1="166" x2="508" y2="166" stroke="rgba(13,16,48,0.12)" />
  <text x="322" y="196" class="svg-label" fill="#0d1030">Traceable change</text>
  <text x="322" y="220" class="svg-caption" fill="rgba(13,16,48,0.6)">Reviewable history and ownership</text>

  <line x1="530" y1="165" x2="580" y2="165" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="574,160 584,165 574,170" fill="#a4238f" />

  <rect x="584" y="88" width="230" height="154" rx="14" fill="#0d1030" />
  <text x="606" y="118" class="svg-kicker" fill="#78d8f2">03 · ASSURE</text>
  <text x="606" y="150" class="svg-h3" fill="#ffffff">Automated gates</text>
  <line x1="606" y1="166" x2="792" y2="166" stroke="rgba(255,255,255,0.14)" />
  <text x="606" y="196" class="svg-label" fill="#ffffff">Compile-time rules · secrets</text>
  <text x="606" y="220" class="svg-caption" fill="rgba(255,255,255,0.7)">Dependencies · tests</text>

  <line x1="814" y1="165" x2="864" y2="165" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="858,160 868,165 858,170" fill="#a4238f" />

  <rect x="868" y="88" width="256" height="154" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="890" y="118" class="svg-kicker" fill="#a4238f">04 · TRIAL</text>
  <text x="890" y="150" class="svg-h3" fill="#0d1030">Controlled non-production</text>
  <line x1="890" y1="166" x2="1102" y2="166" stroke="rgba(164,35,143,0.18)" />
  <text x="890" y="196" class="svg-label" fill="#0d1030">Access and review</text>
  <text x="890" y="220" class="svg-caption" fill="rgba(13,16,48,0.6)">Evidence before promotion</text>

  <rect x="180" y="286" width="780" height="48" rx="12" fill="#0d1030" />
  <text x="570" y="316" text-anchor="middle" class="svg-label" fill="#ffffff">The codex guides people and agents. Gates enforce what cannot be optional.</text>
</svg>
</div>
</div>

<!--
Foundation one — how a change moves:
- The codex is human-readable and agent-usable: one reviewed source for standards, patterns, examples, and templates.
- The workspace carries the codex and approved tools into each change.
- Critical coding standards are enforced as the code compiles; mandatory controls also appear at source control, integration, and deployment boundaries.
- A change reaches a controlled trial environment before anyone considers production.

Client question:
- Which step in this path is currently missing, optional, or different for every team?
-->

---
title: AI coding moves right. Governance moves up.
sync: record-position
class: slide-light
---

<Eyebrow text="02 · RECORD INTENT · SPEC-DRIVEN DELIVERY" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# AI coding moves right.<br>Governance moves up.

</div>
<p class="ed-band-prose"><strong>AI coding changes implementation. It does not create governance.</strong> It does not establish authority, retain intent, or create an audit trail. Spec-driven delivery makes the separate upward move.</p>
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

  <text x="770" y="108" text-anchor="middle" class="svg-h2" fill="#a4238f">Spec-driven delivery</text>
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
- Spec-driven delivery is the separate upward move. It puts governed intent around AI-assisted implementation.

Client question:
- Has AI changed how code is produced without changing what governs its acceptance?
-->

---
title: The specification drives development.
sync: record-development
class: slide-light
---

<Eyebrow text="02 · RECORD INTENT · SPEC-DRIVEN DEVELOPMENT" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# The specification drives<br>development.

</div>
<p class="ed-band-prose"><strong>The specification stays in the development loop.</strong> It is reviewed and versioned as spec.md and design.md. A plan is derived for the delivery run, then implementation builds, integrates, and verifies against the specification.</p>
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
  <text x="380" y="196" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Versioned</text>
  <text x="380" y="216" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">in repo</text>

  <line x1="440" y1="155" x2="468" y2="155" stroke="#a4238f" stroke-width="1.7" />
  <polygon points="462,150 472,155 462,160" fill="#a4238f" />

  <rect x="472" y="54" width="268" height="202" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="494" y="84" class="svg-kicker" fill="#a4238f">02 · DERIVE</text>
  <text x="494" y="116" class="svg-h3" fill="#0d1030">Plan</text>
  <line x1="494" y1="132" x2="718" y2="132" stroke="rgba(164,35,143,0.18)" />
  <text x="494" y="164" class="svg-label" fill="#0d1030">Ephemeral work</text>
  <text x="494" y="192" class="svg-label" fill="#0d1030">Dependencies</text>
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
  <text x="570" y="324" text-anchor="middle" class="svg-label" fill="#ffffff">The plan is disposable. The reviewed specification is not.</text>
</svg>
</div>
</div>

<!--
This is the recognisable spec-driven development loop.
- Specify reconciles intent, documentation, tests, and, where it exists, legacy source code into `spec.md` and `design.md`. Together, those files are the specification.
- A person reviews and seals that specification in the repository.
- Plan derives ephemeral work ordered by dependency from the reviewed specification. It can be discarded and generated again.
- Implement gives each unit of work the exact specification and plan entry it needs, builds and integrates it, and verifies the combined result against the specification.
- Verification is part of completing implementation, not a separate downstream activity; only a verified wave advances the revision's label.
- The coding agent and plan are replaceable. The reviewed specification is not.

This leads to the next boundary:
- The specification governs what must be built. The runtime governs what the resulting code may touch.
-->

---
title: Code is disposable. The specification is durable.
sync: record-truth
class: slide-light
---

<Eyebrow text="02 · RECORD INTENT · DURABLE SPECIFICATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Code is disposable.<br><span style="color:#a4238f">The specification is durable.</span>

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
title: Generated code has less freedom to surprise.
sync: runtime
class: slide-light
---

<Eyebrow text="03 · CONSTRAIN CODE · SANDBOXED EXECUTION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span>02 · Record intent</span>
<span class="is-active">03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Generated code has less freedom<br>to surprise.

</div>
<p class="ed-band-prose"><strong>Code reaches infrastructure only through capabilities supplied by the host.</strong></p>
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
title: Different infrastructure. The same application.
sync: runtime-portability
class: slide-light
---

<Eyebrow text="03 · CONSTRAIN CODE · PORTABLE INFRASTRUCTURE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span>02 · Record intent</span>
<span class="is-active">03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Different infrastructure.<br>The same application.

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
The website calls this host-selected infrastructure.
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
title: One delivery system. Two starting points.
sync: entry
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
  <text x="516" y="110" text-anchor="middle" class="svg-kicker" fill="#78d8f2">REVIEWED SPECIFICATION</text>
  <text x="516" y="140" text-anchor="middle" class="svg-h3" fill="#ffffff">What must remain true</text>
  <text x="516" y="164" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.7)">spec.md · design.md</text>
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
  <text x="1038" y="134" text-anchor="middle" class="svg-h3" fill="#ffffff">Sandboxed</text>
  <text x="1038" y="154" text-anchor="middle" class="svg-h3" fill="#ffffff">execution</text>
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
title: Evidence must be able to say no
sync: proof-result
class: slide-light
---

<Eyebrow text="EVIDENCE · FIXED REPLAY SUITE, NOT A MODEL DEMO" />

<div class="ed-split">
<div class="ed-prose-col">

# Evidence must be<br>able to say no.

<p class="ed-prose">We recovered a specification from an anonymised real-time rail service, rebuilt it as a WASI component, and held it to the fixed 22-case replay suite used for its hand-written counterpart.</p>
<p class="ed-prose">The early output looked plausible line by line. <strong>The recorded cases showed it was not correct.</strong> Evidence is useful because it can still say no.</p>
<p class="ed-footnote">Propellerhead internal evaluation, October 2026. Latest result: 21 of 22 cases, including all 12 recorded production messages. One case remains open and reported.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="14" y="38" class="svg-kicker" fill="rgba(13,16,48,0.52)">THE SAME FIXED REPLAY SUITE · 22 RECORDED CASES</text>

  <rect x="14" y="66" width="184" height="154" rx="12" fill="rgba(255,59,107,0.05)" stroke="rgba(180,35,78,0.4)" />
  <text x="106" y="132" text-anchor="middle" class="svg-h2" fill="#b4234e" style="font-size: 40px;">0 / 22</text>
  <text x="106" y="164" text-anchor="middle" class="svg-kicker" fill="#b4234e">EARLY BUILD</text>
  <text x="106" y="194" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Plausible. Wrong.</text>

  <line x1="198" y1="143" x2="224" y2="143" stroke="#0c7fa3" stroke-width="2" />
  <polygon points="218,138 228,143 218,148" fill="#0c7fa3" />

  <rect x="230" y="66" width="184" height="154" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="322" y="132" text-anchor="middle" class="svg-h2" fill="#0c7fa3" style="font-size: 40px;">7 / 22</text>
  <text x="322" y="164" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">SYSTEM IMPROVED</text>
  <text x="322" y="194" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Checks and guidance</text>

  <line x1="414" y1="143" x2="440" y2="143" stroke="#a4238f" stroke-width="2" />
  <polygon points="434,138 444,143 434,148" fill="#a4238f" />

  <rect x="446" y="66" width="180" height="154" rx="12" fill="#a4238f" />
  <text x="536" y="132" text-anchor="middle" class="svg-h2" fill="#ffffff" style="font-size: 40px;">21 / 22</text>
  <text x="536" y="164" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.86)">LATEST BUILD</text>
  <text x="536" y="194" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">12 / 12 production messages</text>

  <rect x="78" y="274" width="484" height="92" rx="14" fill="#0d1030" />
  <text x="320" y="312" text-anchor="middle" class="svg-h3" fill="#ffffff">One case remains open—and reported.</text>
  <text x="320" y="340" text-anchor="middle" class="svg-caption" fill="#78d8f2">The evidence still says no.</text>
</svg>
</div>
</div>

<!--
Evidence, stated at the level achieved:
- The early output looked plausible. The recorded cases showed it was not correct.
- The same fixed suite was used for the hand-written counterpart.
- The open case remains visible. Evidence is useful because it can still say no.
-->

---
title: Fix the system, not the output
sync: proof-learning
class: slide-light
---

<Eyebrow text="COMPOUNDING DELIVERY · EVERY FAILURE IMPROVES THE NEXT BUILD" />

<div class="ed-split">
<div class="ed-prose-col">

# Fix the system,<br>not the output.

<p class="ed-prose"><strong>No defect was hand-patched.</strong> Each failure changed the checks, guidance, extraction, or tooling that every later build inherits.</p>
<p class="ed-prose">The score is not the product claim. The compounding delivery system is.</p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="14" y="28" width="612" height="360" rx="14" fill="#0d1030" />
  <text x="38" y="62" class="svg-kicker" fill="#78d8f2">A FAILURE BECOMES A SYSTEM CAPABILITY</text>

  <text x="38" y="122" class="svg-label" fill="#ffffff">A consumer never registered</text>
  <line x1="250" y1="117" x2="324" y2="117" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,112 328,117 318,122" fill="#78d8f2" />
  <text x="344" y="122" class="svg-caption" fill="#78d8f2">Refuse unreachable publishers</text>

  <text x="38" y="182" class="svg-label" fill="#ffffff">A rejection looked processed</text>
  <line x1="250" y1="177" x2="324" y2="177" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,172 328,177 318,182" fill="#78d8f2" />
  <text x="344" y="182" class="svg-caption" fill="#78d8f2">Refuse swallowed errors</text>

  <text x="38" y="242" class="svg-label" fill="#ffffff">A type was laid out empty</text>
  <line x1="250" y1="237" x2="324" y2="237" stroke="rgba(255,255,255,0.24)" />
  <polygon points="318,232 328,237 318,242" fill="#f59ef5" />
  <text x="344" y="242" class="svg-caption" fill="#f59ef5">Repair the design-type reader</text>

  <line x1="38" y1="286" x2="602" y2="286" stroke="rgba(255,255,255,0.12)" />
  <text x="320" y="322" text-anchor="middle" class="svg-label" fill="#ffffff">No defect was hand-patched.</text>
  <text x="320" y="350" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">Every later build inherits the fix.</text>
</svg>
</div>
</div>

<!--
Compounding delivery:
- We did not rescue the demonstration with hand edits.
- Each defect became a reusable check, extraction rule, instruction, or tooling repair.
- The next build starts with every lesson the previous one paid for.
-->

---
title: Start with the thing that is stuck
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
<h4>Trace its full delivery path</h4>
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
