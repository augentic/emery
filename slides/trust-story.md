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

- Everyone in this room has seen it: someone in operations or finance builds something useful with an AI assistant in an afternoon, and it works.
- Nothing goes wrong on that day. The trouble starts later, when it succeeds: colleagues depend on it, it quietly reaches real data, and nobody owns it.
- This deck is about that moment, and about a simple way to handle it: four trust tiers, three clearances, one scorecard.
-->

---
title: Code is abundant
class: slide-light
---

<Eyebrow text="THE SHIFT · GENERATIVE AI CHANGED THE ECONOMICS" />

# Code is abundant.<br>Responsibility is not.

<p class="slide-lead">Generative AI collapsed the cost of a first implementation. It did not collapse the cost of deciding what should be true, bounding what software may touch, or owning it on Monday morning.</p>

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
The economics slide.
- Generative AI changes who can make software and how much software an organisation can produce.
- It does not transfer accountability to the model. Intent, access, evidence, and operations remain the organisation's.
- Our role has moved from typing lines of code to designing the system around implementation.
-->

---
title: Authorship is not assurance
class: slide-light
---

<Eyebrow text="OUR PREMISE · TRUST THE EVIDENCE, NOT THE AUTHOR" />

<div class="ed-split">
<div class="ed-prose-col">

# Any implementation<br>can be wrong.

<p class="ed-prose">We do not trust software because a senior engineer typed it, or distrust it because a model generated it. <strong>Authorship is not assurance.</strong></p>
<p class="ed-prose">A model may propose behaviour and write code. It never decides which source is authoritative, grants itself live access, or promotes its own output.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="20" y="30" class="svg-kicker" fill="#0c7fa3">TWO AUTHORS · ONE ASSURANCE SYSTEM</text>

  <rect x="20" y="64" width="220" height="78" rx="12" fill="#ffffff" stroke="rgba(13,16,48,0.24)" stroke-width="1.5" />
  <text x="40" y="92" class="svg-kicker" fill="#0d1030">HUMAN-WRITTEN</text>
  <text x="40" y="118" class="svg-h3" fill="#0d1030">Looks plausible</text>

  <rect x="20" y="168" width="220" height="78" rx="12" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.5" />
  <text x="40" y="196" class="svg-kicker" fill="#a4238f">MODEL-GENERATED</text>
  <text x="40" y="222" class="svg-h3" fill="#0d1030">Looks plausible</text>

  <path d="M 240 103 C 282 103, 282 155, 318 155" fill="none" stroke="#0d1030" stroke-width="1.7" />
  <path d="M 240 207 C 282 207, 282 155, 318 155" fill="none" stroke="#a4238f" stroke-width="1.7" />
  <polygon points="312,150 322,155 312,160" fill="#0d1030" />

  <rect x="326" y="64" width="294" height="286" rx="16" fill="#0d1030" />
  <text x="354" y="96" class="svg-kicker" fill="#78d8f2">TRUST COMES FROM THE SYSTEM</text>

  <circle cx="366" cy="138" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="366" y="143" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="396" y="134" class="svg-label" fill="#ffffff">Intent is explicit</text>
  <text x="396" y="152" class="svg-caption" fill="rgba(255,255,255,0.66)">Reviewed spec and source authority</text>

  <circle cx="366" cy="207" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="366" y="212" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="396" y="203" class="svg-label" fill="#ffffff">Authority is bounded</text>
  <text x="396" y="221" class="svg-caption" fill="rgba(255,255,255,0.66)">WASI capabilities outside the code</text>

  <circle cx="366" cy="276" r="16" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="366" y="281" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="396" y="272" class="svg-label" fill="#ffffff">Promotion is evidenced</text>
  <text x="396" y="290" class="svg-caption" fill="rgba(255,255,255,0.66)">Checks pass; an owner accepts</text>

  <text x="473" y="326" text-anchor="middle" class="svg-caption" fill="#78d8f2">The author never becomes the control boundary.</text>
</svg>
</div>
</div>

<!--
This is the worldview in one slide.
- Human authorship was never proof of correctness; it was a proxy supported by scarce expertise and review.
- AI removes the scarcity and breaks that proxy.
- We replace trust in authorship with trust in evidence, bounded runtime authority, and accountable promotion.
-->

---
title: The same three gaps
class: slide-light
---

<Eyebrow text="WHAT WE'VE LEARNED · WORKING WITH CLIENTS" />

<div class="ed-split">
<div class="ed-prose-col">

# Three gaps,<br>every time.

<p class="ed-prose">In every client assessment we have run, the domain builders were capable and well-intentioned. What was missing was the system around them.</p>
<p class="ed-prose"><strong>The issue is never capability.</strong> Without a governed workspace, an approved AI stack, and automated gates, most tools stay hidden on laptops while engineering bottlenecks on the rest.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Three Left Root-Cause Nodes -->
  <circle cx="28" cy="66" r="5" fill="#b4234e" />
  <text x="44" y="54" class="svg-kicker" fill="#b4234e">01 · WORKSPACE MISSING</text>
  <text x="44" y="74" class="svg-h3" fill="#0d1030">No Governed Place to Build</text>
  <text x="44" y="92" class="svg-caption" fill="rgba(13,16,48,0.58)">Personal laptops &amp; live tenants · No repo</text>

  <circle cx="28" cy="166" r="5" fill="#b4234e" />
  <text x="44" y="154" class="svg-kicker" fill="#b4234e">02 · TOOLKIT UNDECIDED</text>
  <text x="44" y="174" class="svg-h3" fill="#0d1030">No Approved AI Stack</text>
  <text x="44" y="192" class="svg-caption" fill="rgba(13,16,48,0.58)">Teams pick their own tools and keep quiet</text>

  <circle cx="28" cy="266" r="5" fill="#b4234e" />
  <text x="44" y="254" class="svg-kicker" fill="#b4234e">03 · ASSURANCE PERSONAL</text>
  <text x="44" y="274" class="svg-h3" fill="#0d1030">Safety Depends on Who Built It</text>
  <text x="44" y="292" class="svg-caption" fill="rgba(13,16,48,0.58)">No automated secret scan, audit, or test gate</text>

  <!-- Connecting causal curves to Consequences on the right -->
  <path d="M 310 66 L 408 66" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,61 412,66 402,71" fill="#b4234e" />
  <path d="M 310 166 L 408 166" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,161 412,166 402,171" fill="#b4234e" />
  <path d="M 310 266 L 408 266" fill="none" stroke="rgba(180,35,78,0.45)" stroke-width="1.5" stroke-dasharray="4 4" />
  <polygon points="402,261 412,266 402,271" fill="#b4234e" />

  <!-- Right Consequence Column -->
  <line x1="426" y1="38" x2="426" y2="302" stroke="#0d1030" stroke-width="2" />
  <text x="446" y="54" class="svg-kicker" fill="#0d1030">SO · VISIBILITY LOST</text>
  <text x="446" y="74" class="svg-label" fill="#0d1030">No inventory of what exists</text>
  <text x="446" y="154" class="svg-kicker" fill="#0d1030">SO · REUSE LOST</text>
  <text x="446" y="174" class="svg-label" fill="#0d1030">Every app wired differently</text>
  <text x="446" y="254" class="svg-kicker" fill="#0d1030">SO · EXPOSURE SHARED</text>
  <text x="446" y="274" class="svg-label" fill="#0d1030">Careful builders carry the risk</text>

  <!-- Bottom Contrast Card: Inverted Estate vs Target Estate -->
  <rect x="20" y="334" width="600" height="84" rx="12" fill="#f5f6fa" stroke="rgba(13,16,48,0.14)" />
  <text x="40" y="362" class="svg-kicker" fill="#b4234e">THE INVERTED ESTATE WE FIND</text>
  <text x="40" y="382" class="svg-body" fill="#0d1030">Most apps hidden at Tier 0 · No governed middle · The rest queued for engineering</text>
  <text x="40" y="402" class="svg-kicker" fill="#0c7fa3">A HEALTHY ESTATE → ~0% T0 · ~75% T1 · ~20% T2 · ~5% T3</text>
</svg>
</div>
</div>

<!--
This is the credibility slide.
- Gap one: nowhere sanctioned to build, so building happens on laptops and in production tenants.
- Gap two: nobody has said which AI tools are approved, so people pick their own quietly.
- Gap three: whether an app is safe depends on who built it rather than shared pipeline checks.
- The result is an inverted estate: most apps hidden at Tier 0 and no middle tier between shadow IT and a full engineering handover.
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
Pause on this before the click.
- The white curve is adoption: how many people depend on the app over time. The dashed line is the control around it, flat at zero.
- Click. The blue steps are the Trust Model. Control rises through four levels as reach, live access, and business criticality change.
-->

---
title: The Trust Model
class: slide-light
---

<Eyebrow text="THE ANSWER · TRUST MODEL" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Four tiers.<br>Three clearances.

</div>
<p class="ed-band-prose">Each tier is a complete, proportionate operating state. Rather than grading the builder, <strong>the Trust Model matches control to the app's reach</strong> — moving up only when a real adoption signal says it must, and keeping every layer beneath it.</p>
</div>
<TrustLadder />
</div>

<!--
The model on one slide. Read it left to right.
- Tier 0, Unmanaged: one person, one laptop. Discovery, not a violation.
- Cleared to share, in days: a repository, the approved template, automated gates. Tier 1, Governed (~75% of apps live here happily).
- Cleared for live data, in weeks: a named owner and an explicit capability contract on the WASI runtime. Tier 2, Connected (~20%).
- Cleared for production, a funded project: a recovered specification, a verified rebuild, an SLA. Tier 3, Supported (~5%).
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
Now one app across four chapters. The top track advances from T0 to T3 while the four control lanes show what changes at each step.
- Week one: built on a personal laptop, nothing checks it, reads a pasted export, runs only when she runs it.
- Signal to move: a colleague asks for the link.
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
<p>A repository, the approved template, automated gates on every push. <strong>The Codex guides the builder; the gates enforce.</strong> Nothing live can be reached from here.</p>
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
Week three: Tier 1.
- The builder keeps building; what changes is where. Enterprise GitHub, the approved template, automated secret and dependency scans.
- Synthetic or local data only, air-gapped from live systems. Most internal apps stay right here for life.
- Signal to move: the team wants the live roster.
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

# The team wants live roster data. A capability contract replaces the spreadsheet export.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared for live data</span><em>Weeks</em></div>
<p>A named business owner and backup, an explicit capability contract, and one access review. <strong>The component carries business logic, never credentials; the WASI runtime mediates its effects through the host.</strong></p>
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
Month three: Tier 2.
- Instead of handing out raw database credentials or saying no, the planner declares a capability contract.
- The WASI runtime mediates every call through the host, keeping credentials, telemetry, and policy outside generated code.
- Next signal: twenty-five users, regulated data, or a Monday morning the business cannot afford.
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
<p><strong>Spec-driven development</strong> reconciles intent, docs, tests, and observed behaviour into a reviewable specification. Once sealed, the app is rebuilt in verified waves and runs on the <strong>WASI runtime</strong> under an engineering SLA.</p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The handover</span><em>Both sides protected</em></div>
<p>The operations lead becomes Product Owner. Engineering takes the pager. <strong>Keep the criteria. Drop the code.</strong></p>
</div>
</div>
</div>

<ControlStack :tier="3" dark compact />
</div>

<!--
Month nine: Tier 3 graduation.
- Graduation avoids two bad options: forcing engineers to maintain nine months of prompt-generated code, or rewriting from a blank page and losing the domain rules.
- Spec-driven recovery extracts the rules and scenarios into a reviewable specification, then rebuilds the WASI component in verified waves.
- The operations lead becomes Product Owner without a pager; engineering owns the rebuilt code and the SLA.
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
<p class="ed-band-prose">Four questions about the app, never about the builder. <strong>The highest column any answer reaches sets the tier.</strong> At month nine, three sites and Monday-morning criticality place the shift planner at Tier 3 — making graduation a clear funding decision.</p>
</div>
<Scorecard :mark="{ reach: 3, data: 2, criticality: 3, continuity: 2 }" subject="The shift planner, month 9" />
</div>

<!--
Score the shift planner right at the end of its four-chapter journey.
- Reach: three sites, 25+ people -> Tier 3.
- Data: live internal roster via capability contract -> Tier 2.
- Monday 9am: shifts go uncovered -> Tier 3.
- Continuity: named business owner + backup -> Tier 2.
- Highest column wins: Tier 3.
-->

---
title: The model cannot confer authority
class: slide-light
---

<Eyebrow text="SPEC-DRIVEN DEVELOPMENT · PROPOSAL IS NOT PROMOTION" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 660 440" aria-hidden="true">
  <!-- Top Promotion Pipeline -->
  <line x1="88" y1="66" x2="572" y2="66" stroke="rgba(13,16,48,0.16)" stroke-width="2" />

  <circle cx="88" cy="66" r="34" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="88" y="62" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">SOURCES</text>
  <text x="88" y="79" text-anchor="middle" class="svg-caption" fill="#0d1030">Evidence</text>

  <line x1="122" y1="66" x2="212" y2="66" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="206,61 216,66 206,71" fill="#0c7fa3" />

  <circle cx="250" cy="66" r="34" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="250" y="62" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">STAGED</text>
  <text x="250" y="79" text-anchor="middle" class="svg-caption" fill="#0d1030">Candidate</text>

  <line x1="284" y1="66" x2="374" y2="66" stroke="#a4238f" stroke-width="1.8" />
  <polygon points="368,61 378,66 368,71" fill="#a4238f" />
  <rect x="301" y="38" width="58" height="20" rx="10" fill="#a4238f" />
  <text x="330" y="52" text-anchor="middle" class="svg-pill" fill="#ffffff">SEAL</text>

  <circle cx="412" cy="66" r="34" fill="#0d1030" />
  <text x="412" y="62" text-anchor="middle" class="svg-kicker" fill="#78d8f2">SEALED</text>
  <text x="412" y="79" text-anchor="middle" class="svg-caption" fill="#ffffff">Revision</text>

  <line x1="446" y1="66" x2="536" y2="66" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="530,61 540,66 530,71" fill="#0d1030" />

  <circle cx="572" cy="66" r="34" fill="#a4238f" />
  <text x="572" y="62" text-anchor="middle" class="svg-kicker" fill="#ffffff">LABELLED</text>
  <text x="572" y="79" text-anchor="middle" class="svg-caption" fill="#ffffff">Verified</text>

  <!-- Concrete Recovered Spec Excerpt -->
  <rect x="36" y="116" width="588" height="96" rx="12" fill="rgba(22,180,226,0.06)" stroke="#0c7fa3" stroke-width="1.5" />
  <text x="56" y="138" class="svg-kicker" fill="#0c7fa3">A RECOVERED SHIFT PLANNER REQUIREMENT · AUTHORITY AND CONFLICT MADE VISIBLE</text>
  <text x="56" y="158" class="svg-label" fill="#0d1030">REQ-014 shifts.assign-cover · Route: POST /shifts · Status: divergence</text>
  <text x="56" y="175" class="svg-caption" fill="rgba(13,16,48,0.68)">Given an open shift · When the lead assigns cover · Then cap weekly hours at 40</text>
  <text x="56" y="190" class="svg-caption" fill="#b4234e">Divergence: the product brief's 40h cap outranks the prototype's 48h</text>

  <!-- Three Bounded Model Jobs -->
  <rect x="36" y="224" width="588" height="196" rx="16" fill="#0d1030" />
  <text x="64" y="252" class="svg-kicker" fill="#78d8f2">THE MODEL MAY PROPOSE INSIDE THREE BOUNDED JOBS</text>

  <circle cx="78" cy="288" r="15" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="78" y="293" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="108" y="284" class="svg-label" fill="#ffffff">Extract evidence into typed claims</text>
  <text x="108" y="301" class="svg-caption" fill="rgba(255,255,255,0.66)">Source authority rank and [unknown] gaps stay outside the model</text>

  <circle cx="78" cy="340" r="15" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="78" y="345" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="108" y="336" class="svg-label" fill="#ffffff">Implement one planned slice in an isolated worktree</text>
  <text x="108" y="353" class="svg-caption" fill="rgba(255,255,255,0.66)">Deterministic scaffold, worker grants, and lint checks bound the turn</text>

  <circle cx="78" cy="392" r="15" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="78" y="397" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="108" y="388" class="svg-label" fill="#ffffff">Repair what integrated wave verification finds</text>
  <text x="108" y="405" class="svg-caption" fill="rgba(255,255,255,0.66)">Only passing verification advances the production revision label</text>
</svg>
</div>
<div class="ed-prose-col">

# Models propose.<br>People authorise.

<p class="ed-prose"><strong>Prompts are not governance and repo access is not authority.</strong> Spec-driven development separates proposal from acceptance: source authority, the sealed specification, and runtime capabilities all sit outside the model turn.</p>
<p class="ed-prose">A candidate specification is staged for human review before it is sealed. Code is built in isolated slices, and the production label advances only when the integrated wave passes verification.</p>
</div>
</div>

<!--
How spec-driven graduation works under the hood.
- The model is valuable because it extracts and implements quickly, but it is never its own source of truth or release gate.
- Disagreements between the product brief and the prototype's code are surfaced explicitly for human review before sealing.
-->

---
title: Standardise everything beneath it
class: slide-light
---

<Eyebrow text="REPEATABLE INFRASTRUCTURE · BUY AN OUTCOME, NOT A TOPOLOGY" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Unique on top.<br>Standard beneath.

</div>
<p class="ed-band-prose">The shift planner's workflow is unique; its networking, identity, state, messaging, and recovery patterns are not. Customers select a <strong>service envelope</strong>, and versioned WASI runtime and Terraform recipes provision and operate the stack that meets it.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <rect x="356" y="10" width="428" height="62" rx="12" fill="#0d1030" />
  <text x="570" y="35" text-anchor="middle" class="svg-kicker" fill="#78d8f2">CUSTOMER-SPECIFIC · THE DIFFERENTIATOR</text>
  <text x="570" y="57" text-anchor="middle" class="svg-h3" fill="#ffffff">Reviewed specification + portable WASI component</text>

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

  <text x="570" y="342" text-anchor="middle" class="svg-kicker" fill="#a4238f">ONE SECURITY BASELINE · TERRAFORM PROVISIONS · WASI RUNTIME BINDS · WE OPERATE</text>
</svg>
</div>
</div>

<!--
The commercial and architectural pivot:
- The customer owns outcomes and constraints: workload, SLOs, recovery, residency, connectivity, and cost.
- Versioned recipes satisfy them without a bespoke cloud design exercise for every app.
- Security is never a premium feature: the same boundary applies across S, M, and L.
-->

---
title: A reinvented Propellerhead
class: slide-light
---

<Eyebrow text="OUR ROLE · FROM CODE SUPPLIER TO TRUSTED SOFTWARE PRODUCTION" />

<div class="ed-band">
<div class="ed-band-top">
<div>

# Abundant code,<br>made governable.

</div>
<p class="ed-band-prose">Our value is no longer proportional to the lines of code we type. We establish the operating system around abundant implementation: <strong>how apps earn trust, how intent becomes durable, how runtime authority is bounded, and who owns the result in production.</strong></p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <line x1="142" y1="156" x2="998" y2="156" stroke="rgba(13,16,48,0.18)" stroke-width="2" />

  <circle cx="142" cy="156" r="66" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="142" y="138" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · MODEL</text>
  <text x="142" y="160" text-anchor="middle" class="svg-h3" fill="#0d1030">Earned trust</text>
  <text x="142" y="180" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Estate scorecards</text>
  <text x="142" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Clearances, owners</text>

  <circle cx="428" cy="156" r="66" fill="rgba(22,180,226,0.07)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="428" y="138" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">02 · ENABLE</text>
  <text x="428" y="160" text-anchor="middle" class="svg-h3" fill="#0d1030">Open creation</text>
  <text x="428" y="180" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Codex and coaches</text>
  <text x="428" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Reusable contracts</text>

  <circle cx="714" cy="156" r="66" fill="#0d1030" />
  <text x="714" y="138" text-anchor="middle" class="svg-kicker" fill="#78d8f2">03 · STANDARDISE</text>
  <text x="714" y="160" text-anchor="middle" class="svg-h3" fill="#ffffff">The boundaries</text>
  <text x="714" y="180" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Spec-driven delivery</text>
  <text x="714" y="196" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Runtime + recipes</text>

  <circle cx="998" cy="156" r="72" fill="#a4238f" />
  <text x="998" y="136" text-anchor="middle" class="svg-kicker" fill="#ffffff">04 · OPERATE</text>
  <text x="998" y="160" text-anchor="middle" class="svg-h3" fill="#ffffff">Critical apps</text>
  <text x="998" y="182" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">Verified graduation</text>
  <text x="998" y="198" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.78)">SLA and the pager</text>

  <rect x="70" y="274" width="1000" height="48" rx="12" fill="rgba(22,180,226,0.07)" stroke="rgba(12,127,163,0.28)" />
  <text x="570" y="304" text-anchor="middle" class="svg-h3" fill="#0d1030">Domain experts discover · AI implements · the system constrains · accountable people own</text>
</svg>
</div>
</div>

<!--
This is the reinvented Propellerhead:
- Concentrating senior engineering where it compounds: shared capability boundaries, service recipes, and critical Tier 3 operations.
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
<h4>Prove the boundary and service envelope</h4>
<p>Map its capabilities to a standard WASI + Terraform recipe and demonstrate the evidence required to graduate it safely.</p>
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
Close with one concrete ask:
- Bring one useful application that has begun to matter and one live capability it needs.
- Score it, recover a candidate specification, and map it to a standard service envelope.
-->
