---
theme: default
title: The riskiest moment is when a prototype succeeds
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · TRUST MODEL" />

<div class="section-hero-grid">
<div class="section-hero-left">

# The riskiest moment is<br>when a prototype succeeds.

<p class="section-subtitle">An operating model for software created by domain experts, engineers, and AI — four tiers and three evidence-backed clearances.</p>
</div>
</div>

<TrustLadder variant="strip" />

<!--
Open on the title and let it sit for a moment.

- Everyone in this room has seen it: someone in operations or finance builds something useful with an AI assistant, in an afternoon, and it works.
- Nothing goes wrong on that day. The trouble starts later, when it succeeds: colleagues depend on it, it quietly reaches real data, and nobody owns it.
- This deck is about that moment, and about a simple way to handle it: four trust tiers, three clearances, one scorecard.
- The four tiers are on the strip at the bottom. We will walk one real-shaped app through all four.
-->

---
title: Code is abundant
class: slide-light
---

<Eyebrow text="THE SHIFT · GENERATIVE AI CHANGED THE ECONOMICS" />

# Code is abundant.<br>Responsibility is not.

<p class="slide-lead">Generative AI collapsed the cost of a first implementation. It did not collapse the cost of deciding what should be true, bounding what software may touch, proving what changed, or owning it on Monday morning.</p>

<div class="stat-grid">
<div class="stat">
<div class="stat-num">40<span>%</span></div>
<p class="stat-label">of internal tools at large enterprises will be built with vibe-coding platforms by 2029, up from under 10% in 2025.</p>
<span class="stat-src">Gartner, Market Guide for Enterprise Vibe Coding Platforms, April 2026</span>
</div>
<div class="stat">
<div class="stat-num">56<span>%</span></div>
<p class="stat-label">of employees already use AI tools their IT never approved. Fewer than a quarter use the governed alternative.</p>
<span class="stat-src">IDC, cited in CSA, The Vibe Coding Governance Gap, June 2026</span>
</div>
<div class="stat">
<div class="stat-num">0<span>/ 5,600</span></div>
<p class="stat-label">production apps built this way had CSRF protection or standard security headers when scanned. None of them.</p>
<span class="stat-src">CSA, The Vibe Coding Governance Gap, June 2026</span>
</div>
</div>

<div class="punchline-bar">
<span class="punchline-tag">The new bottleneck</span>
<p>The volume is coming either way. <strong>Trust, evidence, and ownership now have to scale farther than engineering headcount.</strong></p>
</div>

<!--
The economics slide. The numbers support the shift; the argument does not depend on any one forecast.

- Generative AI changes who can make software and how much software an organisation can produce.
- It does not transfer accountability to the model. Intent, access, evidence, and operations remain the organisation's.
- The old services model optimised the increasingly cheap part: translating requirements into lines of code.
- Our role has moved to the system around implementation.
-->

---
title: Authorship is not assurance
class: slide-light
---

<Eyebrow text="OUR PREMISE · TRUST THE EVIDENCE, NOT THE AUTHOR" />

<div class="ed-split">
<div class="ed-prose-col">

# We assume every<br>implementation can be wrong.

<p class="ed-prose">We do not trust software because a senior engineer typed it, or distrust it because a model generated it. <strong>Authorship is not assurance.</strong> The same revision needs the same evidence, boundaries, and accountable owner.</p>
<p class="ed-prose">A model may propose behaviour and implementation. It does not decide which source is authoritative, grant itself access, choose its production infrastructure, or promote its own output.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="20" y="30" class="svg-kicker" fill="#0c7fa3">TWO AUTHORS · ONE ASSURANCE SYSTEM</text>

  <rect x="20" y="64" width="220" height="78" rx="12" fill="#ffffff" stroke="rgba(13,16,48,0.24)" stroke-width="1.5" />
  <text x="40" y="92" class="svg-kicker" fill="#0d1030">HUMAN-WRITTEN</text>
  <text x="40" y="118" class="svg-h3" fill="#0d1030">A plausible implementation</text>

  <rect x="20" y="168" width="220" height="78" rx="12" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.5" />
  <text x="40" y="196" class="svg-kicker" fill="#a4238f">MODEL-GENERATED</text>
  <text x="40" y="222" class="svg-h3" fill="#0d1030">A plausible implementation</text>

  <path d="M 240 103 C 286 103, 286 155, 326 155" fill="none" stroke="#0d1030" stroke-width="1.7" />
  <path d="M 240 207 C 286 207, 286 155, 326 155" fill="none" stroke="#a4238f" stroke-width="1.7" />
  <polygon points="320,150 330,155 320,160" fill="#0d1030" />

  <rect x="334" y="64" width="286" height="286" rx="16" fill="#0d1030" />
  <text x="362" y="96" class="svg-kicker" fill="#78d8f2">TRUST COMES FROM THE SYSTEM AROUND IT</text>

  <circle cx="374" cy="138" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="374" y="143" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="404" y="134" class="svg-label" fill="#ffffff">Intent is explicit</text>
  <text x="404" y="152" class="svg-caption" fill="rgba(255,255,255,0.66)">Reviewed specification and source authority</text>

  <circle cx="374" cy="207" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="374" y="212" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="404" y="203" class="svg-label" fill="#ffffff">Authority is bounded</text>
  <text x="404" y="221" class="svg-caption" fill="rgba(255,255,255,0.66)">Explicit capabilities outside application code</text>

  <circle cx="374" cy="276" r="16" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="374" y="281" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="404" y="272" class="svg-label" fill="#ffffff">Promotion is evidenced</text>
  <text x="404" y="290" class="svg-caption" fill="rgba(255,255,255,0.66)">Checks pass and an accountable owner accepts</text>

  <text x="477" y="326" text-anchor="middle" class="svg-caption" fill="#78d8f2">The author never becomes the control boundary.</text>
</svg>
</div>
</div>

<!--
This is the worldview in one slide.

- Human authorship was never proof of correctness; it was a proxy supported by scarce expertise and review.
- AI removes the scarcity and breaks that proxy.
- We replace trust in authorship with trust in evidence, bounded authority, and accountable promotion.
- This applies to citizen builders, professional engineers, dependencies, and models alike.
-->

---
title: The same three gaps
class: slide-light
---

<Eyebrow text="WHAT WE'VE LEARNED · WORKING WITH CLIENTS" />

<div class="ed-split">
<div class="ed-prose-col">

# The same three gaps,<br>every time.

<p class="ed-prose">In every organisation we have assessed for citizen coding, the domain builders were capable and well-intentioned. What was missing was the system around them — and in the absence of a system, people build quietly on personal laptops or straight inside production tenants.</p>
<p class="ed-prose"><strong>The issue was never capability.</strong> When there is no sanctioned workspace, no approved toolkit, and no shared pipeline assurance, the careful builders end up carrying the risk created by the rest. A missing system is something you can design.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Three Left Root-Cause Nodes -->
  <circle cx="28" cy="96" r="5" fill="#b4234e" />
  <text x="44" y="84" class="svg-kicker" fill="#b4234e">01 · WORKSPACE MISSING</text>
  <text x="44" y="104" class="svg-h3" fill="#0d1030">No Governed Place to Build</text>
  <text x="44" y="122" class="svg-caption" fill="rgba(13,16,48,0.58)">Personal laptops &amp; live tenants · No repo</text>
  <circle cx="28" cy="220" r="5" fill="#b4234e" />
  <text x="44" y="208" class="svg-kicker" fill="#b4234e">02 · TOOLKIT UNDECIDED</text>
  <text x="44" y="228" class="svg-h3" fill="#0d1030">No Approved AI Stack</text>
  <text x="44" y="246" class="svg-caption" fill="rgba(13,16,48,0.58)">Teams pick their own tools and keep quiet</text>
  <circle cx="28" cy="344" r="5" fill="#b4234e" />
  <text x="44" y="332" class="svg-kicker" fill="#b4234e">03 · ASSURANCE PERSONAL</text>
  <text x="44" y="352" class="svg-h3" fill="#0d1030">Safety Depends on Who Built It</text>
  <text x="44" y="370" class="svg-caption" fill="rgba(13,16,48,0.58)">No automated secret scan, audit, or test gate</text>
  <!-- Connecting causal curves to Consequences on the right -->
  <path d="M 310 96 L 408 96" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,91 412,96 402,101" fill="#b4234e" />
  <path d="M 310 220 L 408 220" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,215 412,220 402,225" fill="#b4234e" />
  <path d="M 310 344 L 408 344" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,339 412,344 402,349" fill="#b4234e" />
  <!-- Right Consequence Column (Vertical Hairline Bar) -->
  <line x1="426" y1="62" x2="426" y2="386" stroke="#0d1030" stroke-width="2" />
  <text x="446" y="84" class="svg-kicker" fill="#0d1030">SO · VISIBILITY LOST</text>
  <text x="446" y="104" class="svg-label" fill="#0d1030">No reliable inventory of apps</text>
  <text x="446" y="120" class="svg-caption" fill="rgba(13,16,48,0.58)">Nobody knows what exists</text>
  <text x="446" y="208" class="svg-kicker" fill="#0d1030">SO · REUSE LOST</text>
  <text x="446" y="228" class="svg-label" fill="#0d1030">Every app built differently</text>
  <text x="446" y="244" class="svg-caption" fill="rgba(13,16,48,0.58)">Assurance cannot be shared</text>
  <text x="446" y="332" class="svg-kicker" fill="#0d1030">SO · EXPOSURE SHARED</text>
  <text x="446" y="352" class="svg-label" fill="#0d1030">Careful builders carry the risk</text>
  <text x="446" y="368" class="svg-caption" fill="rgba(13,16,48,0.58)">Created by the ungoverned rest</text>
</svg>
</div>
</div>

<!--
This is the credibility slide. Speak from experience, not from the cards.

- We have now done this assessment several times, in different sectors, and the findings are almost interchangeable.
- Gap one: there is nowhere sanctioned to build, so building happens on laptops and in production tenants, and nobody has an inventory.
- Gap two: nobody has said which tools are approved, so people pick their own and do not mention it. That is not defiance; it is the absence of an answer.
- Gap three: whether an app is safe depends entirely on who built it. The organisation has no assurance of its own.
- The conclusion we reached every time: the people are not the problem. The missing piece is a system, and a system is something you can design. That is what the next slides are.
-->

---
title: Value grows. Control doesn't.
class: slide-sandbox
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="THE COMPLICATION · THE RISKIEST MOMENT" />

# Value grows. Control doesn't.

<p class="slide-lead">Nothing goes wrong on day one. It goes wrong on the day the prototype succeeds, and nobody notices, because success looks exactly like success.</p>

<div class="curve-wrap">
<svg class="curve-svg" viewBox="0 0 1000 392" aria-hidden="true">
<defs>
<linearGradient id="trust-steps" x1="0" y1="0" x2="0" y2="1">
<stop offset="0%" stop-color="#16b4e2" stop-opacity="0.34" />
<stop offset="100%" stop-color="#16b4e2" stop-opacity="0.06" />
</linearGradient>
</defs>

<line x1="80" y1="40" x2="80" y2="330" stroke="rgba(255,255,255,0.18)" stroke-width="1" />
<line x1="80" y1="330" x2="950" y2="330" stroke="rgba(255,255,255,0.18)" stroke-width="1" />

<g v-click.hide="1">
<path d="M80 322 C400 322, 560 100, 950 80 L950 322 Z" fill="rgba(255,59,107,0.2)" />
<text x="700" y="262" class="curve-label is-muted">Exposure</text>
</g>

<g v-click="1">
<path d="M80 322 H300 V262 H520 V190 H740 V112 H950 V322 Z" fill="url(#trust-steps)" />
<path d="M80 322 H300 V262 H520 V190 H740 V112 H950" fill="none" stroke="#16b4e2" stroke-width="2.5" stroke-linejoin="round" />
<text x="90" y="302" class="curve-step-label">T0 · UNMANAGED</text>
<text x="508" y="283" class="curve-step-label" text-anchor="end">T1 · GOVERNED</text>
<text x="728" y="211" class="curve-step-label" text-anchor="end">T2 · CONNECTED</text>
<text x="938" y="133" class="curve-step-label" text-anchor="end">T3 · SUPPORTED</text>
</g>

<path d="M80 322 L950 322" fill="none" stroke="rgba(255,255,255,0.5)" stroke-width="2" stroke-dasharray="6 7" />
<path d="M80 322 C400 322, 560 100, 950 80" fill="none" stroke="#ffffff" stroke-width="4" stroke-linecap="round" />

<text x="300" y="196" class="curve-label">People depending on it</text>
<text x="938" y="311" class="curve-label is-muted" text-anchor="end">Controls around it</text>
<text x="90" y="350" class="curve-axis">Week 1</text>
<text x="300" y="350" class="curve-axis">Week 3</text>
<text x="520" y="350" class="curve-axis">Month 3</text>
<text x="740" y="350" class="curve-axis">Month 9</text>
</svg>

<div class="curve-caption">
<div class="curve-caption-swap">
<span v-click.hide="1">The gap between the two lines is the risk. <strong>It opens on the day the app becomes useful.</strong></span>
<span v-click="1"><strong>The Trust Model steps control up as value grows</strong>, at the moments the app earns it, not before.</span>
</div>
<div class="curve-key">
<span><i style="background: #ffffff;"></i>People depending on it</span>
<span><i style="background: rgba(255,255,255,0.5);"></i>Controls</span>
<span><i style="background: #16b4e2;"></i>Earned trust</span>
</div>
</div>
</div>

<!--
One picture. Pause on it before the click.

- The white curve is adoption: how many people depend on the app over time. The dashed line is the control around it, which for a prototype is flat at zero.
- The shaded gap is the exposure. Notice when it opens: not on day one, but when the thing gets good. The riskiest moment is success.
- Click. The blue steps are the Trust Model. Control rises through four levels as reach, live access, and business criticality change.
- What is left between the steps and the curve are slivers, not a chasm. That is the whole idea. Proportionate, and timed to value.
-->

---
title: The Trust Model
class: slide-light
---

<Eyebrow text="THE ANSWER · TRUST MODEL" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Four tiers of earned trust.<br>Three clearances.

</div>
<p class="ed-band-prose">Each tier is a complete, proportionate operating state. Rather than grading the builder, <strong>the Trust Model matches control to the app's reach</strong> — moving up only when a real adoption signal says it must, and keeping every layer beneath it.</p>
</div>
<TrustLadder />
</div>

<!--
The model on one slide. Read it left to right.

- Tier 0, Unmanaged: one person, one laptop. This is where every app starts, and it is discovery, not a violation.
- Cleared to share, in days: a repository, the approved template, automated gates. That takes you to Tier 1, Governed. The Codex is the name we give that workspace, and about three quarters of apps should live and die here, happily.
- Cleared for live systems, in weeks: a named owner and an explicit capability contract. Tier 2, Connected. Real access, bounded authority.
- Cleared for production, a funded project: a specification, a verified rebuild, an SLA. Tier 3, Supported. Perhaps one app in twenty.
- Two things to say out loud. Higher is not better; Tier 1 is a destination, not a waiting room. And a tier is a property of the app, not the person who built it.
- The lineage, if asked: Agile Fluency's zones, Carnegie Mellon's maturity levels, NASA's technology readiness levels, and Shell's green-amber-red governance.
-->

---
title: Week 1 · Unmanaged
class: slide-chapter
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="ONE APP, FOUR TIERS · THE SHIFT PLANNER" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Week 1 · Tier 0</span>

# An operations lead builds a shift planner in a chat window. It works.

<div class="chapter-notes">
<div class="chapter-note">
<div class="chapter-note-head"><span>What's in place</span><em>Nothing</em></div>
<p>A laptop, a consumer AI chat, a spreadsheet export. Nothing has gone wrong. <strong>Tier 0 is discovery, not a violation.</strong></p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The signal</span><em>→ Tier 1</em></div>
<p>A colleague asks for the link.</p>
</div>
</div>
</div>

<ControlStack :tier="0" dark compact />
</div>

<!--
Now one app, four chapters. The picture on the right stays the same; only the lit column moves.

- Week one. An operations lead is tired of building the roster in a spreadsheet, describes the problem to an AI assistant, and has a working planner by Friday.
- Look at the four lanes: built on a personal laptop, nothing checks it, it reads a pasted export, and it runs only when she runs it.
- Nothing is wrong yet. If we punished this, we would lose the best product discovery the organisation has.
- The signal is small and specific: a colleague asks for the link. That is the moment it stops being personal.
-->

---
title: Week 3 · Governed
class: slide-chapter
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="ONE APP, FOUR TIERS · THE SHIFT PLANNER" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Week 3 · Tier 1</span>

# Two colleagues ask for the link. It moves into the governed workspace.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared to share</span><em>Days</em></div>
<p>A repository, the approved template, automated gates on every change. <strong>The Codex guides the builder; the gates enforce.</strong> Nothing live can be reached from here.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The next signal</span><em>→ Tier 2</em></div>
<p>The team wants the live roster, not last week's export.</p>
</div>
</div>
</div>

<ControlStack :tier="1" dark compact />
</div>

<!--
Week three. The lit column moves one step right.

- The builder keeps building; what changes is where. The code lives in Enterprise GitHub, in the approved template, with an AI editor that follows the organisation's rules.
- Two kinds of control, and the distinction matters: the workspace guides, with rules the editor follows, and the pipeline enforces, with gates that block secrets, bad dependencies, and failing tests. Advice in the editor, enforcement in the pipeline.
- The data lane is deliberately boring: synthetic or local only, with no path to live systems. Exposure stays intentionally limited.
- This takes days to set up, once, and then every app gets it for free. Most apps should stay here for their whole lives.
- The next signal: the team wants the live roster.
-->

---
title: Month 3 · Connected
class: slide-chapter
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="ONE APP, FOUR TIERS · THE SHIFT PLANNER" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Month 3 · Tier 2</span>

# The team wants live roster data. An explicit capability contract replaces the spreadsheet export.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared for live data</span><em>Weeks</em></div>
<p>A named owner and a backup. An explicit capability contract for what the planner needs. One review. <strong>The component carries business logic, never infrastructure credentials; Omnia mediates its effects through the host.</strong> <span class="enabler"><img src="/images/omnia-icon.svg" alt="" />Omnia</span></p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The next signal</span><em>→ Tier 3</em></div>
<p>Twenty-five people, or one Monday morning the business cannot afford.</p>
</div>
</div>
</div>

<ControlStack :tier="2" dark compact />
</div>

<!--
Month three. This is the step most organisations get wrong, in one of two ways: they hand out database credentials, or they say no.

- The answer is an explicit capability contract. The planner asks for the roster by business role rather than importing a cloud SDK or carrying a connection string.
- That is what Omnia is for. Application effects cross one host-mediated boundary, where credentials, observability, resource limits, and progressively tighter policy can live outside generated code.
- The architecture does not make wrong logic correct. It makes its authority explicit and gives the platform one place to bound and observe its consequences.
- The clearance is light: a named owner with a backup, a contract, one review that takes hours. Weeks end to end, mostly waiting for the data owner.
- The builder's loop does not change. She still iterates the planner herself.
- The next signal is reach or criticality: twenty-five people, or a Monday morning the business cannot afford.
-->

---
title: Standardise everything beneath it
class: slide-light
---

<Eyebrow text="OUR NEXT PRODUCT LAYER · BUY AN OUTCOME, NOT A TOPOLOGY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Customise what differentiates<br>the business. Standardise<br>everything beneath it.

</div>
<p class="ed-band-prose">The shift planner's behaviour is unique. Its network, identity, data, messaging, telemetry, deployment, and recovery patterns usually are not. Our next product layer lets a customer choose a <strong>service envelope</strong>, then uses versioned Omnia and Terraform recipes to provision and operate the stack that meets it.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="356" y="10" width="428" height="62" rx="12" fill="#0d1030" />
  <text x="570" y="35" text-anchor="middle" class="svg-kicker" fill="#78d8f2">CUSTOMER-SPECIFIC · THE DIFFERENTIATOR</text>
  <text x="570" y="57" text-anchor="middle" class="svg-h3" fill="#ffffff">Specification + portable WASI component</text>

  <line x1="570" y1="72" x2="570" y2="112" stroke="#0c7fa3" stroke-width="2" />
  <polygon points="565,106 575,106 570,116" fill="#0c7fa3" />

  <rect x="284" y="118" width="572" height="62" rx="12" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="570" y="143" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">CUSTOMER CHOOSES THE OUTCOME AND NON-NEGOTIABLE CONSTRAINTS</text>
  <text x="570" y="165" text-anchor="middle" class="svg-h3" fill="#0d1030">Capacity · Availability · Recovery · Residency · Connectivity · Support · Cost</text>

  <line x1="570" y1="180" x2="570" y2="213" stroke="#0d1030" stroke-width="1.8" />
  <line x1="186" y1="213" x2="954" y2="213" stroke="#0d1030" stroke-width="1.5" />
  <line x1="186" y1="213" x2="186" y2="232" stroke="#0d1030" stroke-width="1.5" />
  <line x1="570" y1="213" x2="570" y2="232" stroke="#0d1030" stroke-width="1.5" />
  <line x1="954" y1="213" x2="954" y2="232" stroke="#0d1030" stroke-width="1.5" />

  <rect x="26" y="236" width="320" height="82" rx="12" fill="#ffffff" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" />
  <text x="50" y="262" class="svg-kicker" fill="#0c7fa3">PLAN S · BOUNDED ENVELOPE</text>
  <text x="50" y="286" class="svg-label" fill="#0d1030">Standard availability and recovery</text>
  <text x="50" y="304" class="svg-caption" fill="rgba(13,16,48,0.58)">Predictable capacity · Standard support</text>

  <rect x="410" y="236" width="320" height="82" rx="12" fill="rgba(22,180,226,0.07)" stroke="#0c7fa3" stroke-width="1.7" />
  <text x="434" y="262" class="svg-kicker" fill="#0c7fa3">PLAN M · RESILIENT ENVELOPE</text>
  <text x="434" y="286" class="svg-label" fill="#0d1030">High availability and autoscaling</text>
  <text x="434" y="304" class="svg-caption" fill="rgba(13,16,48,0.58)">Tighter recovery · Faster support</text>

  <rect x="794" y="236" width="320" height="82" rx="12" fill="rgba(164,35,143,0.07)" stroke="#a4238f" stroke-width="1.7" />
  <text x="818" y="262" class="svg-kicker" fill="#a4238f">PLAN L · CRITICAL ENVELOPE</text>
  <text x="818" y="286" class="svg-label" fill="#0d1030">Largest capacity and resilience profile</text>
  <text x="818" y="304" class="svg-caption" fill="rgba(13,16,48,0.58)">Strongest recovery and operating response</text>

  <text x="570" y="342" text-anchor="middle" class="svg-kicker" fill="#a4238f">THE DIRECTION · ONE SAFETY BASELINE · TERRAFORM PROVISIONS · OMNIA BINDS · WE OPERATE</text>
</svg>
</div>
</div>

<!--
This is the commercial and architectural pivot.

- The customer owns the outcomes and constraints: workload, SLOs, recovery, residency, connectivity, compliance, and cost.
- We are productising the implementation that satisfies them through a small catalogue of versioned recipes.
- The plans are service envelopes, not VM sizes. Capacity, resilience, and jurisdiction remain separate dimensions underneath the simple bundles.
- Security is never a premium feature. The same baseline applies to every plan.
- Full architecture remains transparent and auditable; it simply stops being a bespoke design exercise.
- The customer should not need to care because we care obsessively once, in the recipe.
-->

---
title: Month 9 · Supported
class: slide-chapter
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="ONE APP, FOUR TIERS · THE SHIFT PLANNER" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Month 9 · Tier 3</span>

# Three sites depend on it. If it fails on Monday morning, shifts go uncovered. It graduates.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared for production</span><em>A funded project</em></div>
<p><strong>Emery</strong> reconciles intent, documentation, tests, and observed behaviour into a staged specification. Once accepted, its plan is built in constrained waves and runs on <strong>Omnia</strong> under an engineering SLA. <span class="enabler"><img src="/images/emery-icon.svg" alt="" />Emery</span><span class="enabler"><img src="/images/omnia-icon.svg" alt="" />Omnia</span></p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The handover</span><em>Both sides protected</em></div>
<p>The operations lead becomes product owner. Engineering takes the pager. <strong>Keep the criteria. Drop the code.</strong></p>
</div>
</div>
</div>

<ControlStack :tier="3" dark compact />
</div>

<!--
Month nine. The planner now passes the Monday morning test: if it fails, shifts go uncovered at three sites. It has to graduate.

- Graduation used to mean one of two bad options: engineers inherit nine months of prompt-generated code, or they start again from a blank page and lose everything the planner learned.
- Emery is the third option. It reads intent, documentation, tests, and observed behaviour into one reviewable candidate. Where the sources disagree, the disagreement remains visible; where no source answers, the gap stays unknown.
- The candidate becomes authoritative only through an explicit seal. Its plan is then implemented in bounded slices, integrated, verified, and promoted wave by wave.
- The rebuilt planner keeps the same Omnia capability boundary, with production policy set outside the component for any regulated data it needs.
- The handover protects both sides. The operations lead becomes product owner; she keeps the thing she built and gets no pager. Engineering owns the code, the security, and the SLA.
- The line to remember: keep the criteria, drop the code. The prototype already paid for discovering what the software should do.
-->

---
title: The model cannot confer authority
class: slide-light
---

<Eyebrow text="SPEC-DRIVEN DELIVERY · PROPOSAL IS NOT PROMOTION" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 660 440" aria-hidden="true">
  <line x1="88" y1="86" x2="572" y2="86" stroke="rgba(13,16,48,0.16)" stroke-width="2" />

  <circle cx="88" cy="86" r="30" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="88" y="82" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">SOURCES</text>
  <text x="88" y="100" text-anchor="middle" class="svg-caption" fill="#0d1030">Evidence</text>

  <line x1="118" y1="86" x2="216" y2="86" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="210,81 220,86 210,91" fill="#0c7fa3" />

  <circle cx="250" cy="86" r="30" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="250" y="82" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">STAGED</text>
  <text x="250" y="100" text-anchor="middle" class="svg-caption" fill="#0d1030">Candidate</text>

  <line x1="280" y1="86" x2="378" y2="86" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="372,81 382,86 372,91" fill="#a4238f" />
  <rect x="302" y="58" width="56" height="22" rx="11" fill="#a4238f" />
  <text x="330" y="73" text-anchor="middle" class="svg-pill" fill="#ffffff">ACCEPT</text>

  <circle cx="412" cy="86" r="30" fill="#0d1030" />
  <text x="412" y="82" text-anchor="middle" class="svg-kicker" fill="#78d8f2">SEALED</text>
  <text x="412" y="100" text-anchor="middle" class="svg-caption" fill="#ffffff">Revision</text>

  <line x1="442" y1="86" x2="540" y2="86" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="534,81 544,86 534,91" fill="#0d1030" />

  <circle cx="572" cy="86" r="30" fill="#a4238f" />
  <text x="572" y="82" text-anchor="middle" class="svg-kicker" fill="#ffffff">LABELLED</text>
  <text x="572" y="100" text-anchor="middle" class="svg-caption" fill="#ffffff">Verified</text>

  <rect x="36" y="162" width="588" height="218" rx="16" fill="#0d1030" />
  <text x="64" y="194" class="svg-kicker" fill="#78d8f2">THE MODEL MAY PROPOSE INSIDE THREE BOUNDED JOBS</text>

  <circle cx="78" cy="238" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="78" y="243" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="108" y="234" class="svg-label" fill="#ffffff">Interpret evidence into typed claims</text>
  <text x="108" y="252" class="svg-caption" fill="rgba(255,255,255,0.66)">Source authority and unresolved gaps stay outside the answer</text>

  <circle cx="78" cy="300" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="78" y="305" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="108" y="296" class="svg-label" fill="#ffffff">Implement one planned slice in an isolated tree</text>
  <text x="108" y="314" class="svg-caption" fill="rgba(255,255,255,0.66)">The specification, scaffold, grants, and checks bound the turn</text>

  <circle cx="78" cy="352" r="16" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="78" y="357" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="108" y="348" class="svg-label" fill="#ffffff">Repair what integrated verification finds</text>
  <text x="108" y="366" class="svg-caption" fill="rgba(255,255,255,0.66)">Only the engine advances the verified revision label</text>
</svg>
</div>
<div class="ed-prose-col">

# The model can write code.<br>It cannot confer authority.

<p class="ed-prose"><strong>Prompts are not governance and repository access is not authority.</strong> Emery separates proposal from acceptance: source rankings, the current specification, worker grants, runtime capabilities, and the deployment recipe all live outside the model turn.</p>
<p class="ed-prose">A candidate is staged before it is current. Implementation occurs against one sealed plan in isolated slices. The production label advances only after the integrated tree passes its checks.</p>
</div>
</div>

<!--
The distinction between spec-driven development and an unconstrained coding agent.

- The model is valuable precisely because it can interpret and implement quickly.
- It is never asked to be its own source of truth, security boundary, or release authority.
- The stage/apply seam is where the client's approval policy belongs.
- Coverage gaps, conflicts, and unknowns must be visible and either closed or explicitly accepted under that policy.
-->

---
title: Score any app
class: slide-light
---

<Eyebrow text="THE SCORECARD · FOUR QUESTIONS" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Score any app<br>in five minutes.

</div>
<p class="ed-band-prose">Four questions about the app, never about the builder. <strong>The highest column any answer reaches sets the tier.</strong> Here is the shift planner at month nine: three sites and Monday-morning criticality place it at Tier 3, so the conversation is about funding the production clearance.</p>
</div>
<Scorecard :mark="{ reach: 3, data: 2, criticality: 3, continuity: 2 }" subject="The shift planner, month 9" />
</div>

<!--
This is the tool you take away. Score the planner live.

- Reach: three sites, well over twenty-five people. Tier 3.
- Data: live roster data through contracts, nothing regulated. Tier 2.
- Monday morning: shifts go uncovered. Tier 3.
- Continuity: a named owner and a backup. Tier 2.
- The highest column wins, so the planner belongs at Tier 3, and the conversation is about the production clearance, not about whether.
- The thresholds are starting points, not law; set yours in the first workshop. The four questions do not change.
-->

---
title: A reinvented Propellerhead
class: slide-light
---

<Eyebrow text="OUR ROLE · FROM CODE SUPPLIER TO TRUSTED SOFTWARE PRODUCTION" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# We build the system<br>that makes abundant<br>software governable.

</div>
<p class="ed-band-prose">Our value is no longer proportional to the amount of code we type. We establish the operating model around abundant implementation: <strong>how applications earn trust, how intent becomes durable, how authority is bounded, how infrastructure is standardised, and who owns the result in production.</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <line x1="142" y1="156" x2="998" y2="156" stroke="rgba(13,16,48,0.18)" stroke-width="2" />

  <circle cx="142" cy="156" r="66" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="142" y="138" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · MODEL</text>
  <text x="142" y="160" text-anchor="middle" class="svg-h3" fill="#0d1030">Earned trust</text>
  <text x="142" y="180" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Estate scorecards</text>
  <text x="142" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Clearances and ownership</text>

  <circle cx="428" cy="156" r="66" fill="rgba(22,180,226,0.07)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="428" y="138" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">02 · ENABLE</text>
  <text x="428" y="160" text-anchor="middle" class="svg-h3" fill="#0d1030">Open creation</text>
  <text x="428" y="180" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">The Codex and coaches</text>
  <text x="428" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Reusable contracts</text>

  <circle cx="714" cy="156" r="66" fill="#0d1030" />
  <text x="714" y="138" text-anchor="middle" class="svg-kicker" fill="#78d8f2">03 · STANDARDISE</text>
  <text x="714" y="160" text-anchor="middle" class="svg-h3" fill="#ffffff">The boundaries</text>
  <text x="714" y="180" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Specifications and capabilities</text>
  <text x="714" y="196" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Omnia + Terraform recipes</text>

  <circle cx="998" cy="156" r="72" fill="#a4238f" />
  <text x="998" y="136" text-anchor="middle" class="svg-kicker" fill="#ffffff">04 · OPERATE</text>
  <text x="998" y="160" text-anchor="middle" class="svg-h3" fill="#ffffff">Critical services</text>
  <text x="998" y="182" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">Verified graduation</text>
  <text x="998" y="198" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">Engineering SLA and pager</text>

  <rect x="70" y="274" width="1000" height="48" rx="12" fill="rgba(22,180,226,0.07)" stroke="rgba(12,127,163,0.28)" />
  <text x="570" y="304" text-anchor="middle" class="svg-h3" fill="#0d1030">Domain experts discover · AI implements · the system constrains · accountable people own</text>
</svg>
</div>
</div>

<!--
This is the reinvented Propellerhead.

- We are not withdrawing engineering. We are concentrating senior engineering where it compounds: the shared boundaries, recipes, and critical services.
- Domain builders retain product ownership. The platform makes their work visible and gives proven winners a safe route forward.
- A standard recipe is reviewed, benchmarked, costed, patched, and improved once, then reused.
- Bespoke architecture becomes the exception with a product case, never the default promise of a sales engagement.
-->

---
title: Start with one app
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>25 years of critical-software judgment, now turned into a repeatable system for abundant software creation.</p>
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
<h4>Score the app and its constraints</h4>
<p>Establish its trust tier, owner, workload, recovery, residency, connectivity, and operating needs.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Recover a reviewable specification</h4>
<p>Reconcile intent, documentation, tests, and observed behaviour; make every disagreement and gap visible before production.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Prove the plan and the boundary</h4>
<p>Map its capabilities to a standard service envelope and demonstrate the evidence required to graduate it safely.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">BRING ONE APP THAT ALREADY MATTERS</div>
<div class="cta-card-title">Let's prove the full trust loop.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>propellerhead.co.nz</span>
</div>
</div>
</div>

<!--
Close with one concrete ask.

- Bring one useful application that has begun to matter and one live capability it needs.
- We score it, recover a candidate specification, make uncertainty visible, and map it to a standard service envelope.
- The client sees the whole loop without first funding a bespoke platform.
- The domain builder keeps the product. The organisation gains a clear path to accountable production ownership.
-->
