---
theme: default
title: Recommendation
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

<Eyebrow text="PROPELLERHEAD · RECOMMENDATION" />

<div class="section-hero-grid">
<div class="section-hero-left">

# [Client]<br>[Recommended outcome].

<p class="section-subtitle">[Client] · [Engagement] · [Date]</p>

</div>
</div>

<div class="hero-pillars">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Outcome · [Observable result to protect or create]</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Boundary · [What this recommendation includes and excludes]</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Decision · [Approval or commitment required]</span></div>
</div>

<!--
Template use:
- Replace every bracketed field before presenting. Preserve a genuine gap as [unknown].
- Name the engagement precisely: audit, architecture review, delivery review, or consultation.
- This deck begins with the client's evidence and ends in a decision. It is not a second approach deck.
- Findings sit under the three mechanisms of the approach: standardising delivery, specifying behaviour, sandboxing execution. The assessed trust radius sizes every safeguard the recommendation carries.

Opening:
- We will separate what we observed, what we infer, what remains unknown, and what we recommend.
- The purpose is one defensible next decision, not a generic transformation programme.
-->

---
title: Intended outcome
class: slide-light
---

<Eyebrow text="EXECUTIVE VIEW · INTENDED OUTCOME AND RECOMMENDATION" />

<div class="ed-split">
<div class="ed-prose-col">

# [Intended client outcome]

<p class="ed-prose">[One sentence naming who benefits and what will change for them.]</p>
<p class="ed-prose"><strong>[Why this outcome matters now, supported by engagement evidence.]</strong></p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="18" y="22" width="604" height="394" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="42" y="54" class="svg-kicker" fill="#0c7fa3">EXECUTIVE RECOMMENDATION</text>

  <rect x="42" y="82" width="260" height="126" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="62" y="110" class="svg-kicker" fill="#0c7fa3">INTENDED OUTCOME</text>
  <text x="62" y="140" class="svg-h3" fill="#0d1030">[Observable client result]</text>
  <text x="62" y="166" class="svg-caption" fill="rgba(13,16,48,0.62)">For [customer, operator, or business owner]</text>
  <text x="62" y="188" class="svg-caption" fill="#0c7fa3">Measured by [evidence]</text>

  <rect x="318" y="82" width="260" height="126" rx="12" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.5" />
  <text x="338" y="110" class="svg-kicker" fill="#a4238f">CURRENT CONSTRAINT</text>
  <text x="338" y="140" class="svg-h3" fill="#0d1030">[Primary constraint]</text>
  <text x="338" y="166" class="svg-caption" fill="rgba(13,16,48,0.62)">Consequence: [why it matters now]</text>
  <text x="338" y="188" class="svg-caption" fill="#a4238f">Evidence: [source or finding]</text>

  <rect x="42" y="224" width="260" height="126" rx="12" fill="#0d1030" />
  <text x="62" y="252" class="svg-kicker" fill="#78d8f2">PROPOSED INTERVENTION</text>
  <text x="62" y="282" class="svg-h3" fill="#ffffff">[First bounded delivery]</text>
  <text x="62" y="308" class="svg-caption" fill="rgba(255,255,255,0.72)">Includes [boundary] · Excludes [boundary]</text>
  <text x="62" y="330" class="svg-caption" fill="#f59ef5">Owner: [name or role]</text>

  <rect x="318" y="224" width="260" height="126" rx="12" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="338" y="252" class="svg-kicker" fill="#0c7fa3">DECISION REQUESTED</text>
  <text x="338" y="282" class="svg-h3" fill="#0d1030">[Approve the first move]</text>
  <text x="338" y="308" class="svg-caption" fill="rgba(13,16,48,0.62)">Owner: [name or role] · By [date]</text>
  <text x="338" y="330" class="svg-caption" fill="#0c7fa3">Approve · amend · decline</text>

  <text x="320" y="390" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">[Because of the evidence, we recommend this bounded move to achieve this outcome.]</text>
</svg>
</div>
</div>

<!--
Customisation:
- Write this slide last, then place it second.
- Each field must be supported later by evidence, an owner, and completion criteria.
- Do not hide uncertainty in executive language. If a decision-critical fact is absent, write [unknown].

Decision prompt:
- Is this the decision the client believes they are being asked to make?
-->

---
title: What we observed
class: slide-light
---

<Eyebrow text="EVIDENCE BASE · OBSERVATIONS, IMPLICATIONS, AND UNKNOWNS" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# What we observed.

</div>
<p class="ed-band-prose">[Scope of the consultation, evidence reviewed, and limits on the review.] <strong>[One sentence summarising the evidence pattern relevant to the intended outcome.]</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="54" width="348" height="224" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="42" y="84" class="svg-kicker" fill="#0c7fa3">01 · VERIFIED OBSERVATIONS</text>
  <text x="42" y="116" class="svg-h3" fill="#0d1030">What the evidence directly supports</text>
  <line x1="42" y1="132" x2="342" y2="132" stroke="rgba(13,16,48,0.12)" />
  <text x="42" y="164" class="svg-label" fill="#0d1030">[Observation · evidence source]</text>
  <text x="42" y="194" class="svg-label" fill="#0d1030">[Observation · evidence source]</text>
  <text x="42" y="224" class="svg-label" fill="#0d1030">[Observation · evidence source]</text>
  <text x="42" y="258" class="svg-caption" fill="#0c7fa3">Evidence reviewed: [artefacts and people]</text>

  <line x1="366" y1="166" x2="400" y2="166" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="394,161 404,166 394,171" fill="#0d1030" />

  <rect x="402" y="54" width="348" height="224" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.5" />
  <text x="426" y="84" class="svg-kicker" fill="#0d1030">02 · IMPLICATIONS</text>
  <text x="426" y="116" class="svg-h3" fill="#0d1030">What the observations mean here</text>
  <line x1="426" y1="132" x2="726" y2="132" stroke="rgba(13,16,48,0.12)" />
  <text x="426" y="164" class="svg-label" fill="#0d1030">[Likely delivery consequence]</text>
  <text x="426" y="194" class="svg-label" fill="#0d1030">[Likely control gap]</text>
  <text x="426" y="224" class="svg-label" fill="#0d1030">[Likely dependency]</text>
  <text x="426" y="258" class="svg-caption" fill="rgba(13,16,48,0.58)">Confidence: [high · medium · low]</text>

  <line x1="750" y1="166" x2="784" y2="166" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="778,161 788,166 778,171" fill="#a4238f" />

  <rect x="786" y="54" width="336" height="224" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.7" />
  <text x="810" y="84" class="svg-kicker" fill="#a4238f">03 · MATERIAL UNKNOWNS</text>
  <text x="810" y="116" class="svg-h3" fill="#0d1030">What could change the decision</text>
  <line x1="810" y1="132" x2="1098" y2="132" stroke="rgba(164,35,143,0.16)" />
  <text x="810" y="164" class="svg-label" fill="#0d1030">[unknown: decision-critical gap]</text>
  <text x="810" y="194" class="svg-label" fill="#0d1030">[unknown: owner or authority]</text>
  <text x="810" y="224" class="svg-label" fill="#0d1030">[unknown: technical constraint]</text>
  <text x="810" y="258" class="svg-caption" fill="#a4238f">Decision impact: [resolve · bound · accept]</text>

  <rect x="250" y="304" width="640" height="38" rx="19" fill="#0d1030" />
  <text x="570" y="328" text-anchor="middle" class="svg-label" fill="#ffffff">Evidence supports: [bounded conclusion that informs the recommendation]</text>
</svg>
</div>
</div>

<!--
Customisation:
- Use three to five observations, not a transcript dump.
- Cite the source in the speaker notes or client appendix.
- An inference may be strong; it is still not an observation.
- Unknowns are part of the recommendation when they change sequence, scope, or authority.

Decision prompt:
- Which unknowns must be resolved before commitment, and which can be bounded inside the first delivery?
-->

---
title: Findings across the delivery system
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="FINDINGS · STANDARDISING DELIVERY, SPECIFYING BEHAVIOUR, SANDBOXING EXECUTION" />

# Findings across the delivery system.

<p class="slide-lead">[One-sentence synthesis of how the findings affect the intended outcome.]</p>

<div class="ed-band-canvas" style="padding-top: 4px;">
<svg viewBox="0 0 1140 400" aria-hidden="true">
  <rect x="18" y="42" width="348" height="258" rx="16" fill="rgba(10,7,16,0.72)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="42" y="72" class="svg-kicker" fill="#78d8f2">01 · STANDARDISING DELIVERY</text>
  <text x="42" y="104" class="svg-h2" fill="#ffffff">Toolchain and pipeline</text>
  <line x1="42" y1="120" x2="342" y2="120" stroke="rgba(255,255,255,0.14)" />
  <text x="42" y="140" class="svg-caption" fill="rgba(255,255,255,0.6)">Workspaces · codex · compiler rules · pipeline</text>
  <text x="42" y="168" class="svg-kicker" fill="rgba(255,255,255,0.5)">OBSERVED</text>
  <text x="42" y="192" class="svg-body" fill="#ffffff">[Finding F-01]</text>
  <text x="42" y="222" class="svg-kicker" fill="rgba(255,255,255,0.5)">CONSEQUENCE</text>
  <text x="42" y="246" class="svg-body" fill="#ffffff">[Why the finding matters]</text>
  <text x="42" y="280" class="svg-caption" fill="#78d8f2">Intervention: [bounded change]</text>

  <rect x="396" y="42" width="348" height="258" rx="16" fill="rgba(10,7,16,0.72)" stroke="#f59ef5" stroke-width="1.8" />
  <text x="420" y="72" class="svg-kicker" fill="#f59ef5">02 · SPECIFYING BEHAVIOUR</text>
  <text x="420" y="104" class="svg-h2" fill="#ffffff">Specification</text>
  <line x1="420" y1="120" x2="720" y2="120" stroke="rgba(255,255,255,0.14)" />
  <text x="420" y="140" class="svg-caption" fill="rgba(255,255,255,0.6)">Evidence · precedence · spec, design, plan · build</text>
  <text x="420" y="168" class="svg-kicker" fill="rgba(255,255,255,0.5)">OBSERVED</text>
  <text x="420" y="192" class="svg-body" fill="#ffffff">[Finding F-02]</text>
  <text x="420" y="222" class="svg-kicker" fill="rgba(255,255,255,0.5)">CONSEQUENCE</text>
  <text x="420" y="246" class="svg-body" fill="#ffffff">[Why the finding matters]</text>
  <text x="420" y="280" class="svg-caption" fill="#f59ef5">Intervention: [bounded change]</text>

  <rect x="774" y="42" width="348" height="258" rx="16" fill="rgba(10,7,16,0.72)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="798" y="72" class="svg-kicker" fill="#78d8f2">03 · SANDBOXING EXECUTION</text>
  <text x="798" y="104" class="svg-h2" fill="#ffffff">Runtime authority</text>
  <line x1="798" y1="120" x2="1098" y2="120" stroke="rgba(255,255,255,0.14)" />
  <text x="798" y="140" class="svg-caption" fill="rgba(255,255,255,0.6)">Component · capabilities · recipe · host policy</text>
  <text x="798" y="168" class="svg-kicker" fill="rgba(255,255,255,0.5)">OBSERVED</text>
  <text x="798" y="192" class="svg-body" fill="#ffffff">[Finding F-03]</text>
  <text x="798" y="222" class="svg-kicker" fill="rgba(255,255,255,0.5)">CONSEQUENCE</text>
  <text x="798" y="246" class="svg-body" fill="#ffffff">[Why the finding matters]</text>
  <text x="798" y="280" class="svg-caption" fill="#78d8f2">Intervention: [bounded change]</text>

  <rect x="196" y="332" width="748" height="42" rx="21" fill="rgba(255,255,255,0.08)" stroke="rgba(255,255,255,0.25)" />
  <text x="570" y="358" text-anchor="middle" class="svg-label" fill="#ffffff">Overall consequence: [what fails or remains exposed if these findings stand]</text>
</svg>
</div>

<!--
Customisation:
- Keep findings in the mechanism where the control belongs.
- Operation findings—release ownership, observation, recovery—belong to Standardising delivery. Operation is the end of the path, not a fourth mechanism.
- A finding may span mechanisms. Name the primary control, then show the dependency later.
- The trust radius on the next slide is not a finding. It is the consequence the findings are measured against.
- Replace every bracketed line with client evidence; remove an empty card only when a mechanism was explicitly out of scope.

Decision prompt:
- Which finding changes the safety or usefulness of the first delivery most?
-->

---
title: Assessed trust radius
class: slide-light
---

<Eyebrow text="TRUST RADIUS · THE HIGHEST CONSEQUENCE SETS THE REQUIRED STATE" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 320" aria-hidden="true">
  <rect x="18" y="18" width="1104" height="236" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <rect x="18" y="18" width="1104" height="48" rx="14" fill="#0d1030" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">DIMENSION</text>
  <text x="196" y="48" class="svg-kicker" fill="#78d8f2">QUESTION</text>
  <text x="456" y="48" class="svg-kicker" fill="#78d8f2">OBSERVED</text>
  <text x="864" y="48" class="svg-kicker" fill="#78d8f2">STATE · 0 EXPLORE → 3 SUPPORT</text>

  <line x1="170" y1="66" x2="170" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="430" y1="66" x2="430" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="838" y1="66" x2="838" y2="254" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="113" x2="1122" y2="113" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="160" x2="1122" y2="160" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="207" x2="1122" y2="207" stroke="rgba(13,16,48,0.12)" />

  <text x="42" y="95" class="svg-label" fill="#0d1030">Reach</text>
  <text x="196" y="95" class="svg-body" fill="#0d1030">What can it access?</text>
  <text x="456" y="95" class="svg-body" fill="#0d1030">[Systems, services, capabilities reached · evidence]</text>
  <text x="864" y="95" class="svg-label" fill="#0c7fa3">[0–3] · [state]</text>

  <text x="42" y="142" class="svg-label" fill="#0d1030">Data</text>
  <text x="196" y="142" class="svg-body" fill="#0d1030">What can it read or change?</text>
  <text x="456" y="142" class="svg-body" fill="#0d1030">[Data read, retained, produced, changed · classification]</text>
  <text x="864" y="142" class="svg-label" fill="#0c7fa3">[0–3] · [state]</text>

  <text x="42" y="189" class="svg-label" fill="#0d1030">Dependence</text>
  <text x="196" y="189" class="svg-body" fill="#0d1030">Who relies on it?</text>
  <text x="456" y="189" class="svg-body" fill="#0d1030">[Who or what stops when it is unavailable or wrong]</text>
  <text x="864" y="189" class="svg-label" fill="#a4238f">[0–3] · [state]</text>

  <text x="42" y="236" class="svg-label" fill="#0d1030">Continuity</text>
  <text x="196" y="236" class="svg-body" fill="#0d1030">What must survive failure?</text>
  <text x="456" y="236" class="svg-body" fill="#0d1030">[Recovery obligation · discarded, restored, or recovered]</text>
  <text x="864" y="236" class="svg-label" fill="#a4238f">[0–3] · [state]</text>

  <rect x="120" y="270" width="900" height="40" rx="20" fill="#0d1030" />
  <text x="570" y="295" text-anchor="middle" class="svg-label" fill="#ffffff">Trust radius: State [N] · [furthest dimension] → required boundary: [isolate · authoring · runtime · full system]</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# The trust radius<br>sets the safeguards.

</div>
<div>
<p class="ed-prose"><strong>[Which dimension pushes the application furthest, and the evidence for that answer.]</strong></p>
<p class="ed-prose">[What the required state means for the first delivery, what it does not require, and what would move the state in either direction.]</p>
</div>
</div>
</div>

<!--
Customisation:
- Place each dimension independently on the four-state scale. Take the furthest answer, not the average.
- Choose the least costly state that matches the actual consequence. Do not move the system to the right out of caution; that is cost without safety.
- Cite the evidence for each answer. An answer the client disputes is a material unknown, not a decision.
- Record the reassessment trigger: which change in reach, data, dependence, or continuity would move the state.
- The required boundary sizes the target state and the first delivery that follow.

Decision prompt:
- Does the client agree with the furthest answer, and who owns reassessing it?
-->

---
title: Material constraints and dependencies
class: slide-light
---

<Eyebrow text="CONSTRAINTS · WHAT MUST CHANGE NOW, DURING DELIVERY, OR LATER" />

<div class="ed-split">
<div class="ed-prose-col">

# Material constraints<br>and dependencies.

<p class="ed-prose">[Constraint that must be resolved before the client can safely commit.]</p>
<p class="ed-prose"><strong>[Dependency or risk best tested inside the first bounded delivery.]</strong></p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="18" y="22" width="604" height="392" rx="16" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="42" y="52" class="svg-kicker" fill="#0c7fa3">PROPOSED SEQUENCE · [WHY THIS ORDER]</text>

  <rect x="42" y="82" width="536" height="84" rx="12" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="62" y="108" class="svg-kicker" fill="#a4238f">NOW · BEFORE COMMITMENT</text>
  <text x="62" y="134" class="svg-h3" fill="#0d1030">[Blocker, unsafe assumption, or missing authority]</text>
  <text x="62" y="154" class="svg-caption" fill="#a4238f">Exit: [decision evidence]</text>

  <line x1="310" y1="166" x2="310" y2="190" stroke="#0d1030" stroke-width="1.5" />
  <polygon points="305,184 310,194 315,184" fill="#0d1030" />

  <rect x="42" y="194" width="536" height="84" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="62" y="220" class="svg-kicker" fill="#0c7fa3">FIRST DELIVERY · LEARN IN THE WORK</text>
  <text x="62" y="246" class="svg-h3" fill="#0d1030">[Control or assumption the delivery will prove]</text>
  <text x="62" y="266" class="svg-caption" fill="#0c7fa3">Exit: [working evidence]</text>

  <line x1="310" y1="278" x2="310" y2="302" stroke="#0d1030" stroke-width="1.5" />
  <polygon points="305,296 310,306 315,296" fill="#0d1030" />

  <rect x="42" y="306" width="536" height="84" rx="12" fill="#0d1030" />
  <text x="62" y="332" class="svg-kicker" fill="#78d8f2">LATER · SCALE ONLY AFTER PROOF</text>
  <text x="62" y="358" class="svg-h3" fill="#ffffff">[Repeatability, wider adoption, or optimisation]</text>
  <text x="62" y="378" class="svg-caption" fill="#f59ef5">Entry: [proven path]</text>

  <text x="320" y="426" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">Planning assumption: [the fact or dependency that would change this sequence]</text>
</svg>
</div>
</div>

<!--
Customisation:
- Put no more than three findings in each horizon.
- "Now" means it changes whether work should start.
- "First delivery" means the safest way to answer is to build within a bounded experiment.
- "Later" means the work is real but premature until a path is repeatable.

Decision prompt:
- What would make the recommended sequence wrong?
-->

---
title: Recommended target state
class: slide-light
---

<Eyebrow text="TARGET STATE · ONE CONTROLLED PATH FROM INTENT TO OPERATION" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 310" aria-hidden="true">
  <rect x="174" y="12" width="792" height="38" rx="19" fill="#0d1030" />
  <text x="570" y="36" text-anchor="middle" class="svg-kicker" fill="#ffffff">PROTECTED CONTROL PLANE · OUTCOME, SOURCE PRECEDENCE, ACCEPTANCE, RELEASE</text>

  <line x1="204" y1="50" x2="204" y2="82" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="474" y1="50" x2="474" y2="82" stroke="#a4238f" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="744" y1="50" x2="744" y2="82" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />

  <rect x="18" y="84" width="252" height="150" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="40" y="112" class="svg-kicker" fill="#0c7fa3">STANDARDISING DELIVERY</text>
  <text x="40" y="140" class="svg-h3" fill="#0d1030">Governed path</text>
  <text x="40" y="168" class="svg-caption" fill="rgba(13,16,48,0.64)">[Workspaces, codex, compiler rules]</text>
  <text x="40" y="190" class="svg-caption" fill="rgba(13,16,48,0.64)">[Pipeline gates and worker grants]</text>
  <text x="40" y="218" class="svg-caption" fill="#0c7fa3">Closes [finding id]</text>

  <line x1="270" y1="159" x2="324" y2="159" stroke="#0d1030" stroke-width="2" />
  <polygon points="318,154 328,159 318,164" fill="#0d1030" />

  <rect x="330" y="84" width="252" height="150" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="352" y="112" class="svg-kicker" fill="#a4238f">SPECIFYING BEHAVIOUR · EMERY</text>
  <text x="352" y="140" class="svg-h3" fill="#0d1030">Sealed specification</text>
  <text x="352" y="168" class="svg-caption" fill="rgba(13,16,48,0.64)">[Evidence and authority inputs]</text>
  <text x="352" y="190" class="svg-caption" fill="rgba(13,16,48,0.64)">[spec.md, design.md, derived plan]</text>
  <text x="352" y="218" class="svg-caption" fill="#a4238f">Closes [finding id]</text>

  <line x1="582" y1="159" x2="636" y2="159" stroke="#0d1030" stroke-width="2" />
  <polygon points="630,154 640,159 630,164" fill="#0d1030" />

  <rect x="642" y="84" width="252" height="150" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="664" y="112" class="svg-kicker" fill="#0c7fa3">SANDBOXING EXECUTION · OMNIA</text>
  <text x="664" y="140" class="svg-h3" fill="#0d1030">Explicit runtime authority</text>
  <text x="664" y="168" class="svg-caption" fill="rgba(13,16,48,0.64)">[Capabilities the change requires]</text>
  <text x="664" y="190" class="svg-caption" fill="rgba(13,16,48,0.64)">[Recipe: bindings, policy, credentials]</text>
  <text x="664" y="218" class="svg-caption" fill="#0c7fa3">Closes [finding id]</text>

  <line x1="894" y1="159" x2="948" y2="159" stroke="#a4238f" stroke-width="2" />
  <polygon points="942,154 952,159 942,164" fill="#a4238f" />

  <rect x="954" y="84" width="168" height="150" rx="14" fill="#0d1030" />
  <text x="1038" y="112" text-anchor="middle" class="svg-kicker" fill="#78d8f2">OPERATE</text>
  <text x="1038" y="142" text-anchor="middle" class="svg-h3" fill="#ffffff">Known path</text>
  <text x="1038" y="166" text-anchor="middle" class="svg-h3" fill="#ffffff">to production</text>
  <text x="1038" y="194" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.72)">Deploy · observe</text>
  <text x="1038" y="214" text-anchor="middle" class="svg-caption" fill="#f59ef5">Recover · own</text>

  <rect x="194" y="262" width="752" height="36" rx="18" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.3)" />
  <text x="570" y="285" text-anchor="middle" class="svg-label" fill="#0d1030">Client result: [what this controlled path makes possible]</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Recommended<br>target state.

</div>
<div>
<p class="ed-prose"><strong>[Smallest controlled path to the intended outcome.]</strong></p>
<p class="ed-prose">[What stays specific; what becomes reusable after proof.]</p>
</div>
</div>
</div>

<!--
Customisation:
- Replace the bracketed text with the minimum target state implied by the findings and the assessed trust radius. A Govern-state system needs the authoring boundary, not the full delivery system.
- Do not draw a future platform the first delivery cannot exercise.
- Name each finding this state closes. If a component closes none, remove it.
- Operate is the end of the path. Its controls—release ownership, observation, recovery—are part of Standardising delivery.
- Keep release authority outside Emery and runtime infrastructure outside Omnia's component.

Decision prompt:
- Is this the smallest target state that can produce the intended outcome safely at the assessed trust radius?
-->

---
title: First bounded delivery
class: slide-sandbox
---

<Eyebrow text="FIRST DELIVERY · USEFUL OUTCOME AND SYSTEM PROOF" />

<div class="ed-split">
<div class="ed-prose-col">

# [First bounded<br>delivery name]

<p class="ed-prose" style="color: rgba(255,255,255,0.76);">[What will be delivered and who will benefit from it.]</p>
<p class="ed-prose" style="color: rgba(255,255,255,0.76);"><strong style="color: #ffffff;">[Why this is the safest useful first move, tied to the findings and unknowns.]</strong></p>

</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <rect x="18" y="24" width="604" height="382" rx="16" fill="rgba(10,7,16,0.72)" stroke="#78d8f2" stroke-width="1.8" />
  <text x="42" y="56" class="svg-kicker" fill="#78d8f2">FIRST BOUNDED DELIVERY · [NAME]</text>

  <rect x="42" y="84" width="254" height="126" rx="12" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.22)" />
  <text x="62" y="112" class="svg-kicker" fill="rgba(255,255,255,0.6)">OUTCOME</text>
  <text x="62" y="142" class="svg-h3" fill="#ffffff">[Observable behaviour]</text>
  <text x="62" y="168" class="svg-caption" fill="rgba(255,255,255,0.7)">For [beneficiary]</text>
  <text x="62" y="190" class="svg-caption" fill="#78d8f2">Owner: [name or role]</text>

  <rect x="312" y="84" width="266" height="126" rx="12" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.22)" />
  <text x="332" y="112" class="svg-kicker" fill="rgba(255,255,255,0.6)">BOUNDARY</text>
  <text x="332" y="142" class="svg-label" fill="#ffffff">Includes: [one useful path]</text>
  <text x="332" y="168" class="svg-label" fill="#ffffff">Excludes: [adjacent work]</text>
  <text x="332" y="190" class="svg-caption" fill="#f59ef5">Stop condition: [condition]</text>

  <rect x="42" y="226" width="254" height="126" rx="12" fill="rgba(22,180,226,0.1)" stroke="#78d8f2" stroke-width="1.4" />
  <text x="62" y="254" class="svg-kicker" fill="#78d8f2">PROOF</text>
  <text x="62" y="284" class="svg-label" fill="#ffffff">[Acceptance evidence]</text>
  <text x="62" y="308" class="svg-label" fill="#ffffff">[Operational evidence]</text>
  <text x="62" y="332" class="svg-caption" fill="rgba(255,255,255,0.7)">Unknown tested: [unknown]</text>

  <rect x="312" y="226" width="266" height="126" rx="12" fill="rgba(164,35,143,0.12)" stroke="#f59ef5" stroke-width="1.4" />
  <text x="332" y="254" class="svg-kicker" fill="#f59ef5">DECISION AT THE END</text>
  <text x="332" y="284" class="svg-label" fill="#ffffff">[Release or hold]</text>
  <text x="332" y="308" class="svg-label" fill="#ffffff">[Repeat, revise, or stop]</text>
  <text x="332" y="332" class="svg-caption" fill="rgba(255,255,255,0.7)">Decider: [name or role]</text>

  <text x="320" y="386" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.58)">Duration: [range] · Trust state: [0–3] · Dependencies: [critical dependencies]</text>
</svg>
</div>
</div>

<!--
Customisation:
- A first delivery has one outcome, one named owner, and one end decision.
- Set its safeguards from the assessed trust radius, not from caution. Name the state the delivery runs in and the boundary that state requires.
- Exclusions should name the adjacent work most likely to dilute the learning.
- Acceptance evidence tests behaviour against the sealed specification. Operational evidence tests the path to production and ownership.
- Do not promise release; define who makes that decision and on what evidence.

Decision prompt:
- Will this delivery expose the highest-consequence assumptions without requiring a programme first?
-->

---
title: Subsequent waves and dependencies
class: slide-light
---

<Eyebrow text="ROADMAP · SUBSEQUENT WAVES AND DEPENDENCIES" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Subsequent waves<br>and dependencies.

</div>
<p class="ed-band-prose">[Why the work is sequenced this way and what must be true before it starts.] <strong>[The dependency or evidence that could change this roadmap.]</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <line x1="116" y1="178" x2="1024" y2="178" stroke="rgba(13,16,48,0.18)" stroke-width="2" />

  <rect x="18" y="72" width="248" height="212" rx="14" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.6" />
  <text x="40" y="102" class="svg-kicker" fill="#a4238f">00 · CLOSE THE BLOCKER</text>
  <text x="40" y="132" class="svg-h3" fill="#0d1030">[Foundation or unknown]</text>
  <line x1="40" y1="148" x2="244" y2="148" stroke="rgba(164,35,143,0.16)" />
  <text x="40" y="178" class="svg-kicker" fill="rgba(13,16,48,0.5)">RESULT</text>
  <text x="40" y="202" class="svg-body" fill="#0d1030">[Decision can safely proceed]</text>
  <text x="40" y="234" class="svg-kicker" fill="rgba(13,16,48,0.5)">EXIT EVIDENCE</text>
  <text x="40" y="258" class="svg-caption" fill="#a4238f">[Evidence or authority]</text>

  <polygon points="276,173 286,178 276,183" fill="#0d1030" />

  <rect x="290" y="54" width="248" height="248" rx="14" fill="#0d1030" />
  <text x="312" y="84" class="svg-kicker" fill="#78d8f2">01 · FIRST BOUNDED DELIVERY</text>
  <text x="312" y="114" class="svg-h3" fill="#ffffff">[Useful outcome]</text>
  <line x1="312" y1="130" x2="516" y2="130" stroke="rgba(255,255,255,0.14)" />
  <text x="312" y="160" class="svg-kicker" fill="rgba(255,255,255,0.5)">RESULT</text>
  <text x="312" y="184" class="svg-body" fill="#ffffff">[Path exercised end to end]</text>
  <text x="312" y="216" class="svg-kicker" fill="rgba(255,255,255,0.5)">EXIT EVIDENCE</text>
  <text x="312" y="240" class="svg-caption" fill="#78d8f2">[Behaviour and operation]</text>
  <text x="312" y="278" class="svg-caption" fill="#f59ef5">Decision: repeat · revise · stop</text>

  <polygon points="548,173 558,178 548,183" fill="#0d1030" />

  <rect x="562" y="72" width="248" height="212" rx="14" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="584" y="102" class="svg-kicker" fill="#0c7fa3">02 · MAKE THE PATH REPEATABLE</text>
  <text x="584" y="132" class="svg-h3" fill="#0d1030">[Second change]</text>
  <line x1="584" y1="148" x2="788" y2="148" stroke="rgba(13,16,48,0.12)" />
  <text x="584" y="178" class="svg-kicker" fill="rgba(13,16,48,0.5)">RESULT</text>
  <text x="584" y="202" class="svg-body" fill="#0d1030">[Reusable control or asset]</text>
  <text x="584" y="234" class="svg-kicker" fill="rgba(13,16,48,0.5)">EXIT EVIDENCE</text>
  <text x="584" y="258" class="svg-caption" fill="#0c7fa3">[Repeatability demonstrated]</text>

  <polygon points="820,173 830,178 820,183" fill="#0d1030" />

  <rect x="834" y="72" width="288" height="212" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.25)" stroke-width="1.5" />
  <text x="856" y="102" class="svg-kicker" fill="#0d1030">03 · SCALE WHERE IT IS EARNED</text>
  <text x="856" y="132" class="svg-h3" fill="#0d1030">[Wider adoption]</text>
  <line x1="856" y1="148" x2="1100" y2="148" stroke="rgba(13,16,48,0.12)" />
  <text x="856" y="178" class="svg-kicker" fill="rgba(13,16,48,0.5)">RESULT</text>
  <text x="856" y="202" class="svg-body" fill="#0d1030">[Operating model at useful scale]</text>
  <text x="856" y="234" class="svg-kicker" fill="rgba(13,16,48,0.5)">ENTRY EVIDENCE</text>
  <text x="856" y="258" class="svg-caption" fill="#0d1030">[Proven path and funded owner]</text>

  <text x="570" y="332" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.54)">Planning basis: [assumptions behind sequence, effort, and timing]</text>
</svg>
</div>
</div>

<!--
Customisation:
- Remove Wave 0 when nothing blocks a safe first delivery.
- Do not attach dates in this recommendation deck unless scope and dependencies were actually estimated.
- Wave 2 must exercise the path again; documentation alone does not demonstrate repeatability.
- Wave 3 requires a funded owner and evidence that scaling is preferable to another bounded delivery.

Decision prompt:
- What evidence must each wave produce before the next commitment is made?
-->

---
title: Owners and decision rights
class: slide-light
---

<Eyebrow text="ACCOUNTABILITY · DECISIONS, OWNERS, AND COMPLETION EVIDENCE" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 320" aria-hidden="true">
  <rect x="18" y="18" width="1104" height="276" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />

  <rect x="18" y="18" width="1104" height="48" rx="14" fill="#0d1030" />
  <text x="42" y="48" class="svg-kicker" fill="#78d8f2">DECISION AREA</text>
  <text x="330" y="48" class="svg-kicker" fill="#78d8f2">ACCOUNTABLE OWNER</text>
  <text x="580" y="48" class="svg-kicker" fill="#78d8f2">DECISION OR ACTION</text>
  <text x="888" y="48" class="svg-kicker" fill="#78d8f2">DONE WHEN</text>

  <line x1="304" y1="66" x2="304" y2="294" stroke="rgba(13,16,48,0.12)" />
  <line x1="554" y1="66" x2="554" y2="294" stroke="rgba(13,16,48,0.12)" />
  <line x1="862" y1="66" x2="862" y2="294" stroke="rgba(13,16,48,0.12)" />

  <line x1="18" y1="104" x2="1122" y2="104" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="142" x2="1122" y2="142" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="180" x2="1122" y2="180" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="218" x2="1122" y2="218" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="256" x2="1122" y2="256" stroke="rgba(13,16,48,0.12)" />

  <text x="42" y="91" class="svg-label" fill="#0d1030">Direction</text>
  <text x="330" y="91" class="svg-body" fill="#0d1030">[Outcome owner]</text>
  <text x="580" y="91" class="svg-body" fill="#0d1030">Accept outcome, scope, and trade-offs</text>
  <text x="888" y="91" class="svg-caption" fill="#0c7fa3">[Outcome and measures agreed]</text>

  <text x="42" y="129" class="svg-label" fill="#0d1030">Trust radius</text>
  <text x="330" y="129" class="svg-body" fill="#0d1030">[Outcome and security owners]</text>
  <text x="580" y="129" class="svg-body" fill="#0d1030">Agree the state and when to reassess</text>
  <text x="888" y="129" class="svg-caption" fill="#a4238f">[Four dimensions assessed; state set]</text>

  <text x="42" y="167" class="svg-label" fill="#0d1030">Delivery standards</text>
  <text x="330" y="167" class="svg-body" fill="#0d1030">[Engineering or platform owner]</text>
  <text x="580" y="167" class="svg-body" fill="#0d1030">Approve codex, compiler rules, gates</text>
  <text x="888" y="167" class="svg-caption" fill="#0c7fa3">[Controls enforced in the work]</text>

  <text x="42" y="205" class="svg-label" fill="#0d1030">Specification</text>
  <text x="330" y="205" class="svg-body" fill="#0d1030">[Domain or product owner]</text>
  <text x="580" y="205" class="svg-body" fill="#0d1030">Set source precedence; review and seal</text>
  <text x="888" y="205" class="svg-caption" fill="#a4238f">[Conflicts resolved; revision sealed]</text>

  <text x="42" y="243" class="svg-label" fill="#0d1030">Runtime authority</text>
  <text x="330" y="243" class="svg-body" fill="#0d1030">[Security or service owner]</text>
  <text x="580" y="243" class="svg-body" fill="#0d1030">Approve capabilities and the runtime recipe</text>
  <text x="888" y="243" class="svg-caption" fill="#0c7fa3">[Only declared reach is available]</text>

  <text x="42" y="281" class="svg-label" fill="#0d1030">Release and operation</text>
  <text x="330" y="281" class="svg-body" fill="#0d1030">[Release and operational owner]</text>
  <text x="580" y="281" class="svg-body" fill="#0d1030">Authorise release; own observe and recover</text>
  <text x="888" y="281" class="svg-caption" fill="#a4238f">[Release and recovery evidence]</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Owners and<br>decision rights.

</div>
<div>
<p class="ed-prose"><strong>[How decisions will be made across the client and Propellerhead.]</strong></p>
<p class="ed-prose">[Escalation path, delegated authority, and governance cadence.]</p>
</div>
</div>
</div>

<!--
Customisation:
- Use names where the engagement established them; use roles where assignment is still a recommendation.
- One person may hold several roles. Do not collapse the decisions themselves.
- A contributor or reviewer is not automatically the accountable owner.
- The "done when" column is the acceptance boundary for the recommended action.

Decision prompt:
- Does each owner have both the authority and the capacity to make the named decision?
-->

---
title: Evidence and success measures
class: slide-light
---

<Eyebrow text="SUCCESS MEASURES · EVIDENCE REQUIRED TO ACCEPT THE RESULT" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Evidence and<br>success measures.

</div>
<p class="ed-band-prose">[How the client will know the recommended change delivered the intended outcome.] <strong>[Where the evidence will come from and who accepts it.]</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="18" y="28" width="1104" height="278" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <rect x="18" y="28" width="1104" height="48" rx="14" fill="#0d1030" />
  <text x="42" y="58" class="svg-kicker" fill="#78d8f2">MEASURE</text>
  <text x="286" y="58" class="svg-kicker" fill="#78d8f2">BASELINE</text>
  <text x="474" y="58" class="svg-kicker" fill="#78d8f2">SUCCESS THRESHOLD</text>
  <text x="736" y="58" class="svg-kicker" fill="#78d8f2">EVIDENCE</text>
  <text x="974" y="58" class="svg-kicker" fill="#78d8f2">OWNER</text>

  <line x1="260" y1="76" x2="260" y2="306" stroke="rgba(13,16,48,0.12)" />
  <line x1="448" y1="76" x2="448" y2="306" stroke="rgba(13,16,48,0.12)" />
  <line x1="710" y1="76" x2="710" y2="306" stroke="rgba(13,16,48,0.12)" />
  <line x1="948" y1="76" x2="948" y2="306" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="122" x2="1122" y2="122" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="168" x2="1122" y2="168" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="214" x2="1122" y2="214" stroke="rgba(13,16,48,0.12)" />
  <line x1="18" y1="260" x2="1122" y2="260" stroke="rgba(13,16,48,0.12)" />

  <text x="42" y="105" class="svg-label" fill="#0d1030">Client outcome</text>
  <text x="286" y="105" class="svg-body" fill="#0d1030">[Current state]</text>
  <text x="474" y="105" class="svg-body" fill="#0d1030">[Observable target]</text>
  <text x="736" y="105" class="svg-caption" fill="#0c7fa3">[Source or test]</text>
  <text x="974" y="105" class="svg-caption" fill="#0d1030">[Owner]</text>

  <text x="42" y="151" class="svg-label" fill="#0d1030">Delivery standards</text>
  <text x="286" y="151" class="svg-body" fill="#0d1030">[Current state]</text>
  <text x="474" y="151" class="svg-body" fill="#0d1030">[Observable target]</text>
  <text x="736" y="151" class="svg-caption" fill="#0c7fa3">[Gate results or pipeline run]</text>
  <text x="974" y="151" class="svg-caption" fill="#0d1030">[Owner]</text>

  <text x="42" y="197" class="svg-label" fill="#0d1030">Specification</text>
  <text x="286" y="197" class="svg-body" fill="#0d1030">[Current state]</text>
  <text x="474" y="197" class="svg-body" fill="#0d1030">[Observable target]</text>
  <text x="736" y="197" class="svg-caption" fill="#0c7fa3">[Sealed revision or verified wave]</text>
  <text x="974" y="197" class="svg-caption" fill="#0d1030">[Owner]</text>

  <text x="42" y="243" class="svg-label" fill="#0d1030">Runtime boundary</text>
  <text x="286" y="243" class="svg-body" fill="#0d1030">[Current state]</text>
  <text x="474" y="243" class="svg-body" fill="#0d1030">[Observable target]</text>
  <text x="736" y="243" class="svg-caption" fill="#a4238f">[Recipe, policy, or test]</text>
  <text x="974" y="243" class="svg-caption" fill="#0d1030">[Owner]</text>

  <text x="42" y="289" class="svg-label" fill="#0d1030">Operation</text>
  <text x="286" y="289" class="svg-body" fill="#0d1030">[Current state]</text>
  <text x="474" y="289" class="svg-body" fill="#0d1030">[Observable target]</text>
  <text x="736" y="289" class="svg-caption" fill="#a4238f">[Release or recovery proof]</text>
  <text x="974" y="289" class="svg-caption" fill="#0d1030">[Owner]</text>

  <rect x="226" y="322" width="688" height="28" rx="14" fill="rgba(22,180,226,0.06)" stroke="rgba(12,127,163,0.3)" />
  <text x="570" y="341" text-anchor="middle" class="svg-caption" fill="#0d1030">Acceptance authority: [name or role] · Review point: [date or event]</text>
</svg>
</div>
</div>

<!--
Customisation:
- Use measures the engagement can baseline or mark the baseline [unknown].
- Prefer observable evidence over self-reported completion.
- Tie each measure to the intended outcome, a material finding, or an explicit risk.
- Name who accepts the evidence, not only who produces it.

Decision prompt:
- Would this evidence let the client accept or reject the result without relying on our opinion?
-->

---
title: Decision requested
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>A bounded recommendation tied to evidence, ownership, and observable completion.</p>
</div>
</div>

<div class="split-content-pane">
<div>
<Eyebrow text="PROPELLERHEAD · THE DECISION" />

# [Decision requested]

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Commitment</h4>
<p>[Approve the bounded delivery, resolve a prerequisite, reframe the outcome, or stop.]</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Immediate next action</h4>
<p>[Action] · Owner: [name or role] · Due: [date or event]</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Conditions</h4>
<p>[Dependencies, exclusions, or evidence required before work begins.]</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">DECISION REQUESTED · [APPROVE / AMEND / DECLINE]</div>
<div class="cta-card-title">[Named decision owner] · [Decision date]</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>Decide</span>
</div>
</div>
</div>

<!--
Customisation:
- State one recommended decision, not a menu of equivalent options.
- State the immediate next action, its owner, and the evidence that closes it.
- A recommendation to stop can be a successful consultation outcome.
- Delete every bracket before presenting.

Close:
- Ask the named decision owner to accept, amend, or decline the recommendation.
-->
