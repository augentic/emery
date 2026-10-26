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
title: Implementation is changing. Responsibility is not.
sync: shift
class: slide-light
---

<Eyebrow text="THE SHIFT · IMPLEMENTATION SPEED CHANGES THE SYSTEM" />

<div class="ed-split">
<div class="ed-prose-col">

# Implementation is changing.<br>Responsibility is not.

<p class="ed-prose">AI compresses the work of turning an idea into plausible code.</p>
<p class="ed-prose"><strong>Direction, authority, evidence, and ownership remain with the organisation.</strong></p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="14" y="22" width="612" height="108" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="38" y="52" class="svg-kicker" fill="#0c7fa3">IMPLEMENTATION EFFORT</text>
  <text x="38" y="88" class="svg-h2" fill="#0d1030">Weeks</text>
  <line x1="130" y1="81" x2="488" y2="81" stroke="rgba(12,127,163,0.42)" stroke-width="2" />
  <polygon points="482,76 492,81 482,86" fill="#a4238f" />
  <text x="602" y="88" text-anchor="end" class="svg-h2" fill="#a4238f">Minutes</text>
  <text x="38" y="114" class="svg-caption" fill="rgba(13,16,48,0.58)">More plausible code, sooner</text>

  <rect x="14" y="148" width="612" height="224" rx="14" fill="#0d1030" />
  <text x="38" y="180" class="svg-kicker" fill="#78d8f2">RESPONSIBILITY REMAINS</text>

  <rect x="38" y="200" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="56" y="226" class="svg-label" fill="#ffffff">Direction</text>
  <text x="56" y="248" class="svg-caption" fill="rgba(255,255,255,0.68)">What should change?</text>
  <rect x="332" y="200" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="350" y="226" class="svg-label" fill="#ffffff">Authority</text>
  <text x="350" y="248" class="svg-caption" fill="rgba(255,255,255,0.68)">Who decides?</text>

  <rect x="38" y="282" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="56" y="308" class="svg-label" fill="#ffffff">Evidence</text>
  <text x="56" y="330" class="svg-caption" fill="rgba(255,255,255,0.68)">Is it safe to accept?</text>
  <rect x="332" y="282" width="270" height="64" rx="9" fill="rgba(255,255,255,0.08)" />
  <text x="350" y="308" class="svg-label" fill="#ffffff">Ownership</text>
  <text x="350" y="330" class="svg-caption" fill="rgba(255,255,255,0.68)">Who answers for it?</text>

  <rect x="70" y="394" width="500" height="36" rx="18" fill="rgba(164,35,143,0.07)" stroke="rgba(164,35,143,0.42)" />
  <text x="320" y="417" text-anchor="middle" class="svg-caption" fill="#a4238f">Faster implementation shifts pressure onto the delivery system.</text>
</svg>
</div>
</div>

<!--
AI compresses implementation effort. It does not transfer responsibility to the model.
Direction, authority, evidence, and ownership remain with the organisation.
The next slide shows the operational response: automate the path around the code.
-->

---
title: Automate the path from code to production.
sync: delivery-automation
class: slide-light
---

<Eyebrow text="THE DELIVERY SHIFT · AUTOMATE THE MECHANICS" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Automate the path<br>from code to production.

</div>
<p class="ed-band-prose"><strong>Faster implementation matters only when evidence and delivery move at the same speed.</strong> We removed mechanical delay by automating the path around the code.</p>
</div>

<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="214" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">01 · PROVE</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Test every change</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="158" class="svg-label" fill="#0d1030">Unit · integration · end-to-end</text>
  <text x="38" y="204" class="svg-caption" fill="rgba(13,16,48,0.62)">Evidence runs without manual coordination</text>

  <line x1="346" y1="143" x2="392" y2="143" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="386,138 396,143 386,148" fill="#0d1030" />

  <rect x="398" y="36" width="344" height="214" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#78d8f2">02 · ASSURE</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Analyse every change</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="156" class="svg-label" fill="#ffffff">Linting · security ·</text>
  <text x="420" y="182" class="svg-label" fill="#ffffff">dependencies · automated review</text>
  <text x="420" y="222" class="svg-caption" fill="rgba(255,255,255,0.7)">Policy is part of every change</text>

  <line x1="742" y1="143" x2="788" y2="143" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="782,138 792,143 782,148" fill="#a4238f" />

  <rect x="794" y="36" width="330" height="214" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#a4238f">03 · DELIVER</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Deliver every accepted change</text>
  <line x1="816" y1="122" x2="1102" y2="122" stroke="rgba(164,35,143,0.18)" />
  <text x="816" y="156" class="svg-label" fill="#0d1030">PR · build · provision ·</text>
  <text x="816" y="182" class="svg-label" fill="#0d1030">deploy · production</text>
  <text x="816" y="222" class="svg-caption" fill="rgba(13,16,48,0.62)">One repeatable path to production</text>

  <rect x="170" y="282" width="800" height="48" rx="24" fill="#0d1030" />
  <text x="570" y="312" text-anchor="middle" class="svg-label" fill="#ffffff">Repeatable evidence and automation replace manual hand-offs.</text>
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
title: The bottleneck becomes the feature throttle.
sync: outcome
class: slide-light
---

<Eyebrow text="THE NEW BOTTLENECK · GOVERN FEATURE DEMAND" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# The bottleneck becomes<br>the feature throttle.

</div>
<p class="ed-band-prose"><strong>Faster delivery should increase learning—not feature volume.</strong> Cheap experiments inform a deliberate production backlog rather than bypassing it.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="16" y="36" width="330" height="224" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="38" y="68" class="svg-kicker" fill="#0c7fa3">SIGNALS AND EXPERIMENTS</text>
  <text x="38" y="104" class="svg-h2" fill="#0d1030">Learn cheaply</text>
  <line x1="38" y1="122" x2="324" y2="122" stroke="rgba(12,127,163,0.18)" />
  <text x="38" y="154" class="svg-label" fill="#0d1030">Customer needs</text>
  <text x="190" y="154" class="svg-label" fill="#0d1030">Operational evidence</text>
  <text x="38" y="190" class="svg-label" fill="#0d1030">Product strategy</text>
  <text x="190" y="190" class="svg-label" fill="#0d1030">Rapid prototypes</text>
  <text x="38" y="232" class="svg-caption" fill="rgba(13,16,48,0.62)">Test assumptions before committing</text>

  <line x1="346" y1="148" x2="392" y2="148" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="386,143 396,148 386,153" fill="#0d1030" />

  <rect x="398" y="36" width="344" height="224" rx="14" fill="#0d1030" />
  <text x="420" y="68" class="svg-kicker" fill="#f59ef5">FEATURE THROTTLE</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Choose deliberately</text>
  <line x1="420" y1="122" x2="720" y2="122" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="154" class="svg-label" fill="#ffffff">Value</text>
  <text x="582" y="154" class="svg-label" fill="#ffffff">Evidence</text>
  <text x="420" y="190" class="svg-label" fill="#ffffff">Strategic fit</text>
  <text x="582" y="190" class="svg-label" fill="#ffffff">Ownership</text>
  <text x="420" y="232" class="svg-caption" fill="rgba(255,255,255,0.7)">Product judgement—not extra ceremony</text>

  <line x1="742" y1="148" x2="788" y2="148" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="782,143 792,148 782,153" fill="#a4238f" />

  <rect x="794" y="36" width="330" height="224" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="816" y="68" class="svg-kicker" fill="#a4238f">PRODUCTION COMMITMENT</text>
  <text x="816" y="104" class="svg-h2" fill="#0d1030">Deliberate production</text>
  <text x="816" y="132" class="svg-h2" fill="#0d1030">backlog</text>
  <line x1="816" y1="150" x2="1102" y2="150" stroke="rgba(164,35,143,0.18)" />
  <text x="816" y="184" class="svg-caption" fill="rgba(13,16,48,0.66)">Only work with enough evidence</text>
  <text x="816" y="206" class="svg-caption" fill="rgba(13,16,48,0.66)">and an owner enters delivery</text>

  <rect x="170" y="286" width="800" height="48" rx="24" fill="#0d1030" stroke="#a4238f" stroke-width="1.6" />
  <text x="570" y="316" text-anchor="middle" class="svg-label" fill="#ffffff">Prototypes inform the backlog. They do not bypass it.</text>
</svg>
</div>
</div>

<!--
Once implementation and delivery accelerate, production capacity should not be filled with features by default.
- Customer signals, operational evidence, strategy, and cheap experiments become inputs.
- Citizen- and vibe-coded proofs of concept are useful because they test assumptions and expose needs quickly.
- A prototype gains no production authority merely because it works.
- The feature throttle asks whether there is value, evidence, strategic fit, and an owner.
- The result may enter the production backlog, return for another experiment, or stop.
The throttle is product judgement, not extra ceremony. Faster delivery should produce faster learning and fewer weak commitments.

Client question:
- Which backlog decisions could be informed by a cheap prototype before becoming production commitments?
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
title: Trust is contextual, not a maturity score.
sync: trust-model
class: slide-light
---

<Eyebrow text="TRUST MODEL · REQUIRED CONTROLS FOLLOW CONSEQUENCE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span class="is-active">03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# Trust is contextual,<br>not a maturity score.

</div>
<p class="ed-band-prose"><strong>Four operational states determine which controls an application requires.</strong> Moving right is not progress; it is more consequence to manage.</p>
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

  <text x="570" y="335" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Keep the application in the least costly state consistent with its actual consequence.</text>
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
<span class="is-active">01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span class="is-active">03 · Constrain code</span>
</div>

<div class="ed-band">
<div class="ed-band-top">
<div>

# The highest consequence<br>sets the trust radius.

</div>
<p class="ed-band-prose"><strong>Assess reach, data, dependence, and continuity against the four-state model.</strong></p>
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
  <text x="570" y="298" text-anchor="middle" class="svg-label" fill="#0d1030" style="font-weight: 600;">The furthest answer defines the trust radius—and the controls it requires.</text>

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
Do not average the answers. The highest-consequence answer sets the application's required state and therefore its controls.
This is the application's trust radius: the widest consequence it can create or inherit.
The radius can contract as well as expand. Reassess whenever one of the four dimensions changes.

Client question:
- Which dimension pushes your application furthest to the right today?
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
title: Start with one real change. Make the next one easier.
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

# Start with one real change.<br>Make the next one easier.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Choose the first move</h4>
<p>We work out what is really in the way, set the boundaries and shape a first delivery worth doing.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Get it live</h4>
<p>New system or old one: we use AI where it helps, senior people where judgement matters and a production path we already know.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Keep the gains</h4>
<p>You keep the specifications, tests, infrastructure and delivery knowledge. The next change starts ahead, not at zero.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">THE FIRST MOVE</div>
<div class="cta-card-title">What is really in the way?</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>Find the first move</span>
</div>
</div>
</div>

<!--
Close on the same path as the website:
- Choose the first move: work out what is really in the way, set the boundaries, and shape a first delivery worth doing.
- Get it live: use AI where it helps, senior people where judgement matters, and a production path we already know.
- Keep the gains: retain the specifications, tests, infrastructure, and delivery knowledge so the next change starts ahead.
- The call to action is direct: find the first move.

Final question:
- What is really in the way?
-->
