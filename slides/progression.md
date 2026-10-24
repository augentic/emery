---
theme: default
title: The riskiest moment is when a prototype succeeds
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · SOFTWARE DELIVERY IN THE AGE OF AI" />

<div class="section-hero-grid">
<div class="section-hero-left">

# The riskiest moment is<br>when a prototype succeeds.

<p class="section-subtitle">We help organisations build and modernise critical software safely, now that anyone — or anything — can write the code.</p>
</div>
</div>

<TrustLadder variant="strip" />

<!--
Open on the title and let it sit for a moment.

- Everyone in this room has seen it: someone in operations or finance builds something useful with an AI assistant in an afternoon, and it works.
- Nothing goes wrong on that day. The trouble starts later, when it succeeds: colleagues depend on it, it quietly reaches real data, and nobody owns it.
- Hold back the title's second meaning. The same moment arrived years ago for the systems this organisation already runs; the turn comes back to it.
- This deck is about delivering software safely through that moment.
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

<p class="ed-prose">We do not trust software because a senior engineer typed it, distrust it because a model generated it, or trust it because it has run for twenty years. <strong>Authorship is not assurance.</strong></p>
<p class="ed-prose">A model may propose behaviour and write code. It never decides which source is authoritative, grants itself live access, or promotes its own output.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 340" aria-hidden="true">
  <text x="20" y="26" class="svg-kicker" fill="#0c7fa3">THREE AUTHORS · ONE ASSURANCE SYSTEM</text>

  <rect x="20" y="52" width="220" height="74" rx="12" fill="#ffffff" stroke="rgba(13,16,48,0.24)" stroke-width="1.5" />
  <text x="40" y="79" class="svg-kicker" fill="#0d1030">HUMAN-WRITTEN</text>
  <text x="40" y="105" class="svg-h3" fill="#0d1030">Looks plausible</text>

  <rect x="20" y="146" width="220" height="74" rx="12" fill="rgba(164,35,143,0.06)" stroke="#a4238f" stroke-width="1.5" />
  <text x="40" y="173" class="svg-kicker" fill="#a4238f">MODEL-GENERATED</text>
  <text x="40" y="199" class="svg-h3" fill="#0d1030">Looks plausible</text>

  <rect x="20" y="240" width="220" height="74" rx="12" fill="#f5f6fa" stroke="rgba(13,16,48,0.34)" stroke-width="1.5" stroke-dasharray="5 4" />
  <text x="40" y="267" class="svg-kicker" fill="rgba(13,16,48,0.62)">INHERITED</text>
  <text x="40" y="293" class="svg-h3" fill="#0d1030">Has always worked</text>

  <path d="M 240 89 C 282 89, 282 183, 318 183" fill="none" stroke="#0d1030" stroke-width="1.7" />
  <path d="M 240 183 L 318 183" fill="none" stroke="#a4238f" stroke-width="1.7" />
  <path d="M 240 277 C 282 277, 282 183, 318 183" fill="none" stroke="rgba(13,16,48,0.5)" stroke-width="1.7" stroke-dasharray="5 4" />
  <polygon points="312,178 322,183 312,188" fill="#0d1030" />

  <rect x="326" y="52" width="294" height="262" rx="16" fill="#0d1030" />
  <text x="354" y="84" class="svg-kicker" fill="#78d8f2">TRUST COMES FROM THE SYSTEM</text>

  <circle cx="366" cy="124" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="366" y="129" text-anchor="middle" class="svg-pill" fill="#78d8f2">01</text>
  <text x="396" y="120" class="svg-label" fill="#ffffff">Intent is explicit</text>
  <text x="396" y="138" class="svg-caption" fill="rgba(255,255,255,0.66)">Reviewed spec and source authority</text>

  <circle cx="366" cy="184" r="16" fill="rgba(255,255,255,0.1)" stroke="#78d8f2" stroke-width="1.5" />
  <text x="366" y="189" text-anchor="middle" class="svg-pill" fill="#78d8f2">02</text>
  <text x="396" y="180" class="svg-label" fill="#ffffff">Authority is bounded</text>
  <text x="396" y="198" class="svg-caption" fill="rgba(255,255,255,0.66)">WASI capabilities outside the code</text>

  <circle cx="366" cy="244" r="16" fill="rgba(255,255,255,0.1)" stroke="#f59ef5" stroke-width="1.5" />
  <text x="366" y="249" text-anchor="middle" class="svg-pill" fill="#f59ef5">03</text>
  <text x="396" y="240" class="svg-label" fill="#ffffff">Promotion is evidenced</text>
  <text x="396" y="258" class="svg-caption" fill="rgba(255,255,255,0.66)">Checks pass; an owner accepts</text>

  <text x="473" y="292" text-anchor="middle" class="svg-caption" fill="#78d8f2">The author never becomes the control boundary.</text>
</svg>
</div>
</div>

<!--
This is the worldview in one slide.
- Human authorship was never proof of correctness; it was a proxy supported by scarce expertise and review.
- AI removes the scarcity and breaks that proxy.
- Long-running code is the third author. Familiar is not the same as understood: it has always worked, until someone needs to change it.
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

<Eyebrow text="ONE APP, FOUR TIERS · AN ILLUSTRATIVE SHIFT PLANNER" />

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

<ControlStack :tier="0" />
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

<Eyebrow text="ONE APP, FOUR TIERS · AN ILLUSTRATIVE SHIFT PLANNER" />

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

<ControlStack :tier="1" />
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

<Eyebrow text="ONE APP, FOUR TIERS · AN ILLUSTRATIVE SHIFT PLANNER" />

<div class="chapter-grid">
<div class="chapter-left">
<span class="chapter-when">Month 3 · Tier 2</span>

# The team wants live roster data. A capability contract replaces the spreadsheet export.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared for live data</span><em>Weeks</em></div>
<p>A named business owner and backup, an explicit capability contract with the roster system, and one access review. <strong>The component carries business logic, never credentials; the WASI runtime mediates its effects through the host.</strong></p>
</div>
<div class="chapter-note">
<div class="chapter-note-head"><span>The next signal</span><em>→ Tier 3</em></div>
<p>Twenty-five people, or one Monday morning the business cannot afford.</p>
</div>
</div>
</div>

<ControlStack :tier="2" />
</div>

<!--
Month three: Tier 2.
- Instead of handing out raw database credentials or saying no, the planner declares a capability contract.
- The WASI runtime mediates every call through the host, keeping credentials, telemetry, and policy outside generated code.
- Remember the roster system behind that contract. We come back to it at the turn.
- Next signal: twenty-five users, regulated data, or a Monday morning the business cannot afford.
-->

---
title: Month 9 · Supported
class: slide-chapter
---

<AmbientVideo src="/videos/approach-video.mp4" variant="approach" />

<Eyebrow text="ONE APP, FOUR TIERS · AN ILLUSTRATIVE SHIFT PLANNER" />

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

<ControlStack :tier="3" />
</div>

<!--
Month nine: Tier 3 graduation.
- Graduation avoids two bad options: forcing engineers to maintain nine months of prompt-generated code, or rewriting from a blank page and losing the domain rules.
- Spec-driven recovery extracts the rules and scenarios into a reviewable specification, then rebuilds the WASI component in verified waves.
- The operations lead becomes Product Owner without a pager; engineering owns the rebuilt code and the SLA.
-->

---
title: The model cannot confer authority
class: slide-light
---

<Eyebrow text="HOW IT GRADUATES · SPEC-DRIVEN DEVELOPMENT" />

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
How graduation works under the hood.
- The model is valuable because it extracts and implements quickly, but it is never its own source of truth or release gate.
- Disagreements between the product brief and the prototype's code are surfaced explicitly for human review before sealing.
- That review is delivery work, not friction: it is where accountable people decide what the software should do.
-->

---
title: Every legacy system was once a prototype
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="THE SAME MOMENT, YEARS LATER" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Every legacy system was once<br>a prototype that succeeded.

<p class="section-subtitle">Its authors have moved on. Its rules live only in the code.<br>AI now makes rewriting it look cheap.</p>
</div>
</div>

<TrustLadder variant="strip" :current="[0, 3]" />

<!--
The turn. Let the title land before you speak.
- Every critical system in the estate started as somebody's useful prototype. It succeeded, people came to depend on it, and nobody wrote down what it must do.
- It is the shift planner's moment, years on: it started at Tier 0, and now the business stands on it.
- AI makes rewriting it feel cheap. That is exactly what makes it dangerous.
-->

---
title: Tier 3 by dependence
class: slide-light
---

<Eyebrow text="MODERNISATION · RECOVER THE PRODUCT, NOT JUST THE CODE" />

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <circle cx="92" cy="142" r="56" fill="#f5f6fa" stroke="#0d1030" stroke-width="1.8" stroke-dasharray="6 4" />
  <text x="92" y="128" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">15 YEARS OLD</text>
  <text x="92" y="148" text-anchor="middle" class="svg-label" fill="#0d1030">Roster System</text>
  <text x="92" y="164" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">No written spec</text>

  <path d="M 148 120 C 230 120, 250 42, 360 42 L 690 42" fill="none" stroke="rgba(180,35,78,0.4)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="684" y1="34" x2="698" y2="50" stroke="#b4234e" stroke-width="2" />
  <line x1="698" y1="34" x2="684" y2="50" stroke="#b4234e" stroke-width="2" />
  <text x="370" y="28" class="svg-kicker" fill="#b4234e">TRAP 01 · KEEP PATCHING IT</text>
  <text x="716" y="38" class="svg-body" fill="rgba(13,16,48,0.62)">Every change is a gamble, while the</text>
  <text x="716" y="54" class="svg-body" fill="rgba(13,16,48,0.62)">people who understand it move on</text>

  <path d="M 148 164 C 230 164, 250 242, 360 242 L 690 242" fill="none" stroke="rgba(180,35,78,0.4)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="684" y1="234" x2="698" y2="250" stroke="#b4234e" stroke-width="2" />
  <line x1="698" y1="234" x2="684" y2="250" stroke="#b4234e" stroke-width="2" />
  <text x="370" y="228" class="svg-kicker" fill="#b4234e">TRAP 02 · TRANSLATE IT WITH AI</text>
  <text x="716" y="238" class="svg-body" fill="rgba(13,16,48,0.62)">Reproduces the old problems in a</text>
  <text x="716" y="254" class="svg-body" fill="rgba(13,16,48,0.62)">new language, only faster</text>

  <line x1="148" y1="142" x2="352" y2="142" stroke="#0c7fa3" stroke-width="2.5" />
  <polygon points="346,137 356,142 346,147" fill="#0c7fa3" />
  <rect x="358" y="108" width="236" height="68" rx="12" fill="#0d1030" />
  <text x="476" y="134" text-anchor="middle" class="svg-kicker" fill="#78d8f2">THIRD OPTION · RECOVER INTENT</text>
  <text x="476" y="156" text-anchor="middle" class="svg-h3" fill="#ffffff">Reviewed Specification</text>
  <line x1="594" y1="142" x2="736" y2="142" stroke="#a4238f" stroke-width="2.5" />
  <polygon points="730,137 740,142 730,147" fill="#a4238f" />
  <rect x="742" y="108" width="216" height="68" rx="12" fill="rgba(164,35,143,0.1)" stroke="#a4238f" stroke-width="1.8" />
  <text x="850" y="134" text-anchor="middle" class="svg-kicker" fill="#a4238f">VERIFIED WAVES</text>
  <text x="850" y="156" text-anchor="middle" class="svg-h3" fill="#0d1030">Clean Rebuild</text>
  <line x1="958" y1="142" x2="1022" y2="142" stroke="#a4238f" stroke-width="2.5" />
  <polygon points="1016,137 1026,142 1016,147" fill="#a4238f" />
  <circle cx="1076" cy="142" r="50" fill="#a4238f" />
  <text x="1076" y="130" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.85)">T3 · WASI</text>
  <text x="1076" y="148" text-anchor="middle" class="svg-label" fill="#ffffff">Runtime</text>
  <text x="1076" y="164" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.85)">+ SLA</text>

  <line x1="1076" y1="192" x2="1076" y2="228" stroke="#0c7fa3" stroke-width="1.6" stroke-dasharray="3 3" />
  <rect x="1012" y="230" width="128" height="44" rx="10" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.4" />
  <text x="1076" y="248" text-anchor="middle" class="svg-pill" fill="#0c7fa3">SHIFT PLANNER</text>
  <text x="1076" y="264" text-anchor="middle" class="svg-caption" fill="#0d1030">Same contract</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Tier 3 by dependence.<br>Tier 0 by evidence.

</div>
<div>
<p class="ed-prose">The roster system behind the planner's contract is fifteen years old. Every shift depends on it, yet its only specification is its code, like a prototype on a laptop. <strong>AI makes rewriting it cheap enough to be dangerous.</strong></p>
<p class="ed-prose">The answer is the planner's own path: recover what must survive, let accountable people settle the conflicts, rebuild in verified waves, and land it behind the contract the planner already uses.</p>
</div>
</div>
</div>

<!--
The second door: the systems the organisation already runs.
- Remember the roster data at month three. The system behind that contract is fifteen years old. Every shift depends on it, and nobody can say with confidence what all of it does.
- That is Tier 3 by dependence and Tier 0 by evidence: the position of a prototype on a laptop, with far more riding on it.
- Two traps. Keep patching it while the people who understand it leave, or let AI translate it and copy its problems into a new language, faster. AI can generate a new implementation quickly; it cannot decide what should survive.
- The third option is the graduation path again: recover intent from code, documents, tests, and people. Where they disagree, the client's accountable people decide. Then rebuild in verified waves.
- The contract is the seam. Where the rebuilt service keeps it, the planner is untouched. Data migration still needs its own plan.
-->

---
title: Customise what differentiates
class: slide-light
---

<Eyebrow text="THE FOUNDATION · BUY AN OUTCOME, NOT A TOPOLOGY" />

# Customise what differentiates the business.<br>Standardise everything beneath it.

<p class="slide-lead">The planner's workflow and the roster's rules are unique; their networking, identity, state, messaging, and recovery are not. Every service lands on one runtime, chosen as a plan. <strong>A standard target also gives AI less technology to invent.</strong></p>

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

<!--
The commercial and architectural pivot.
- The customer owns outcomes and constraints: workload, service levels, recovery, residency, connectivity, and cost.
- Versioned recipes satisfy them without a bespoke cloud design exercise for every app.
- Security is never a premium feature: the same boundary applies across S, M, and L.
- The standard target is also why generated code can meet a production bar: the model has less technology to invent, and every grant is visible.
-->

---
title: We made this bet before AI made it obvious
class: slide-light
---

<Eyebrow text="WHY BELIEVE US · BUILT FOR PEOPLE, READY FOR GENERATION" />

<div class="ed-split">
<div class="ed-prose-col">

# We made this bet<br>before AI made<br>it obvious.

<p class="ed-prose">We built our WebAssembly runtime so our engineers could stop selecting stacks. In October 2025, AI-generated code met our production bar: <strong>the same review and verification bar we hold hand-written code to.</strong></p>
<p class="ed-prose">The design decision that paid off had been made years earlier, for people.</p>
<p class="ed-footnote">Open source under MIT / Apache-2.0: <strong>Omnia</strong>, the runtime, and <strong>Emery</strong>, the delivery engine. We would rather you read the code than take our word for it.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <text x="20" y="26" class="svg-kicker" fill="#0c7fa3">THE ORIGIN · THREE BEATS</text>

  <line x1="40" y1="70" x2="40" y2="230" stroke="rgba(13,16,48,0.16)" stroke-width="2" />

  <circle cx="40" cy="70" r="15" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="40" y="74" text-anchor="middle" class="svg-pill" fill="#0c7fa3">01</text>
  <text x="72" y="62" class="svg-kicker" fill="#0c7fa3">YEARS AGO · BUILT FOR PEOPLE</text>
  <text x="72" y="84" class="svg-h3" fill="#0d1030">A runtime so engineers stop selecting stacks</text>
  <text x="72" y="102" class="svg-caption" fill="rgba(13,16,48,0.58)">Rust on Wasmtime · business behaviour against typed capabilities</text>

  <circle cx="40" cy="150" r="15" fill="#ffffff" stroke="#a4238f" stroke-width="1.8" />
  <text x="40" y="154" text-anchor="middle" class="svg-pill" fill="#a4238f">02</text>
  <text x="72" y="142" class="svg-kicker" fill="#a4238f">OCTOBER 2025 · READY FOR GENERATION</text>
  <text x="72" y="164" class="svg-h3" fill="#0d1030">Generated code meets our production bar</text>
  <text x="72" y="182" class="svg-caption" fill="rgba(13,16,48,0.58)">Held to the same review and verification as hand-written code</text>

  <circle cx="40" cy="230" r="15" fill="#0d1030" />
  <text x="40" y="234" text-anchor="middle" class="svg-pill" fill="#78d8f2">03</text>
  <text x="72" y="222" class="svg-kicker" fill="#0d1030">NOW · CONSTRAINT AS LEVERAGE</text>
  <text x="72" y="244" class="svg-h3" fill="#0d1030">Less technology for AI to invent</text>
  <text x="72" y="262" class="svg-caption" fill="rgba(13,16,48,0.58)">The agent implements a reviewed specification. It does not design the universe.</text>

  <rect x="20" y="294" width="600" height="138" rx="16" fill="#0d1030" />
  <text x="44" y="322" class="svg-kicker" fill="#78d8f2">WHY IT HOLDS AT AI SPEED · ENFORCED BY CONSTRUCTION</text>
  <text x="44" y="352" class="svg-label" fill="#ffffff">Sandboxed by default</text>
  <text x="44" y="370" class="svg-caption" fill="rgba(255,255,255,0.66)">Nothing reachable the host did not grant</text>
  <text x="332" y="352" class="svg-label" fill="#ffffff">Typed, narrow interfaces</text>
  <text x="332" y="370" class="svg-caption" fill="rgba(255,255,255,0.66)">Satisfies the interface or does not compile</text>
  <text x="44" y="400" class="svg-label" fill="#ffffff">A small generation problem</text>
  <text x="44" y="418" class="svg-caption" fill="rgba(255,255,255,0.66)">No frameworks, SDKs, or architecture to choose</text>
  <text x="332" y="400" class="svg-label" fill="#ffffff">Every grant is visible</text>
  <text x="332" y="418" class="svg-caption" fill="rgba(255,255,255,0.66)">What it can touch is enumerable from host config</text>
</svg>
</div>
</div>

<!--
Why believe us, part one: we did not pivot to AI. AI arrived at a design we had already made.
- We built the runtime so our own engineers could stop choosing frameworks, importing provider SDKs, and wiring infrastructure into application code.
- Generated code met our production bar in October 2025, held to the same review and verification as hand-written code. We were not surprised by the direction, only by the date.
- A governed framework can approximate some of this by discipline. The component model enforces it by construction, and construction is what holds as the volume and speed of code keep increasing.
- Both halves are open source. Invite the room to read the code rather than take our word for it.
-->

---
title: Plausible is not correct
class: slide-light
---

<Eyebrow text="WHY BELIEVE US · EVIDENCE, NOT AUTHORSHIP" />

# Plausible is not correct.

<p class="slide-lead">In an AI-assisted rebuild of a live real-time rail service — train-control messages in, vehicle positions out — we recovered a specification from its code, then held the result to the 22 recorded cases its hand-written counterpart is tested against.</p>

<div class="stat-grid">
<div class="stat">
<div class="stat-num">0<span>/ 22</span></div>
<p class="stat-label">cases passed by an early build. Behind it: a delay with its sign inverted, and rejected messages reported as processed.</p>
<span class="stat-src">Propellerhead evaluation, October 2026</span>
</div>
<div class="stat">
<div class="stat-num">21<span>/ 22</span></div>
<p class="stat-label">cases passed by the latest build, after we fixed the delivery system rather than patching the generated code.</p>
<span class="stat-src">The same 22 recorded cases</span>
</div>
<div class="stat">
<div class="stat-num">12<span>/ 12</span></div>
<p class="stat-label">recorded production messages passed. Every one of them.</p>
<span class="stat-src">Client and system anonymised</span>
</div>
</div>

<div class="punchline-bar">
<span class="punchline-tag">Fix the system, not the output</span>
<p>Each defect went back into our checks, prompts, and tooling, <strong>so every future rebuild inherits the fix.</strong></p>
</div>

<!--
Why believe us, part two: evidence on a real system, not a demo.
- The service consumes train-control messages and publishes vehicle positions. We recovered its specification from the code and rebuilt it as a WebAssembly service.
- The early build looked reasonable line by line. The recorded cases exposed what review would likely have missed: a delay computed with its sign inverted, true-or-false flags read as numbers, rejected messages reported as processed.
- We did not hand-patch the output. Each defect went back into the delivery system: a check the build must pass, guidance the model follows, a rule for reading the source, or a fix to the tooling that lays out the code. The latest rebuild passes 21 of the 22 cases, including all 12 recorded production messages.
- This is an internal evaluation on an anonymised client system, described at the level achieved. It is the premise in action: plausible is not correct; evidence decides.
-->

---
title: From code for hire to delivery you can trust
class: slide-light
---

<Eyebrow text="OUR ROLE · FROM CODE SUPPLIER TO TRUSTED DELIVERY" />

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 420" aria-hidden="true">
  <text x="20" y="34" class="svg-kicker" fill="rgba(13,16,48,0.5)">CODE FOR HIRE</text>
  <text x="296" y="34" class="svg-kicker" fill="#a4238f">DELIVERY YOU CAN TRUST</text>
  <line x1="20" y1="48" x2="620" y2="48" stroke="rgba(13,16,48,0.16)" stroke-width="1.2" />

  <text x="20" y="92" class="svg-kicker" fill="#0c7fa3">01 · WHO BUILDS</text>
  <text x="20" y="114" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Our engineers build it</text>
  <line x1="226" y1="108" x2="278" y2="108" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,103 282,108 272,113" fill="#0c7fa3" />
  <text x="296" y="102" class="svg-h3" fill="#0d1030">Your people build too, safely</text>
  <text x="296" y="122" class="svg-caption" fill="rgba(13,16,48,0.6)">The Codex, coaching, and clearances; winners graduate</text>
  <line x1="20" y1="146" x2="620" y2="146" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <text x="20" y="186" class="svg-kicker" fill="#0c7fa3">02 · WHAT IT RUNS ON</text>
  <text x="20" y="208" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">A platform per project</text>
  <line x1="226" y1="202" x2="278" y2="202" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,197 282,202 272,207" fill="#0c7fa3" />
  <text x="296" y="196" class="svg-h3" fill="#0d1030">One standard runtime, as a plan</text>
  <text x="296" y="216" class="svg-caption" fill="rgba(13,16,48,0.6)">Plans S, M, and L on one security baseline</text>
  <line x1="20" y1="240" x2="620" y2="240" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <text x="20" y="280" class="svg-kicker" fill="#0c7fa3">03 · WHAT YOU KEEP</text>
  <text x="20" y="302" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Code as the deliverable</text>
  <line x1="226" y1="296" x2="278" y2="296" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,291 282,296 272,301" fill="#0c7fa3" />
  <text x="296" y="290" class="svg-h3" fill="#0d1030">A specification you own</text>
  <text x="296" y="310" class="svg-caption" fill="rgba(13,16,48,0.6)">The code becomes replaceable; the intent stays yours</text>
  <line x1="20" y1="334" x2="620" y2="334" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <text x="20" y="374" class="svg-kicker" fill="#0c7fa3">04 · HOW YOU PAY</text>
  <text x="20" y="396" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Hours, then a handover</text>
  <line x1="226" y1="390" x2="278" y2="390" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,385 282,390 272,395" fill="#0c7fa3" />
  <text x="296" y="384" class="svg-h3" fill="#0d1030">Bounded outcomes, assured</text>
  <text x="296" y="404" class="svg-caption" fill="rgba(13,16,48,0.6)">Outcome-priced where readiness has bounded the work</text>
</svg>
</div>
<div class="ed-prose-col">

# From code for hire<br>to delivery<br>you can trust.

<p class="ed-prose">Generative AI made code abundant, so <strong>our value is no longer the lines we type.</strong> It is the system around them: how your people build safely, how intent stays durable, how authority stays bounded, and who answers for the result in production.</p>
<p class="ed-prose">Every engagement starts with readiness, delivers in bounded waves, and continues as assurance, <strong>outcome-priced where the evidence supports it.</strong></p>
</div>
</div>

<!--
This is the reinvented Propellerhead.
- We used to sell engineering time and hand over code. Code is now abundant, so the value moves to the system around it.
- Your people can build too. The Codex, coaching, and clearances make that safe, and the winners graduate to us.
- Every service lands on one standard runtime, chosen as a plan, so we stop designing a new platform for every project.
- The asset you keep is the reviewed specification. The code becomes replaceable.
- We price outcomes once readiness has bounded the work, never before.
-->

---
title: Bring one prototype that succeeded
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

# Bring one prototype<br>that succeeded.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>One that succeeded last month</h4>
<p>A citizen-built tool people now depend on. We score your estate, stand up the Codex, and graduate what has earned it.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>One that succeeded twenty years ago</h4>
<p>A critical system nobody can safely change. We recover what it must keep doing and help your people decide what survives.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Either way, start with readiness</h4>
<p>A short, paid engagement that traces one real change to production and ends in a decision: proceed, fix a foundation, or stop.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">AI DELIVERY READINESS</div>
<div class="cta-card-title">Leave knowing what to do next.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>propellerhead.co.nz</span>
</div>
</div>
</div>

<!--
Close with one ask and two ways in.
- Bring one prototype that succeeded. It might have succeeded last month, in a chat window, or twenty years ago, in the system every shift depends on.
- Either way, the first step is the same: AI Delivery Readiness, a short, paid engagement that traces one real change from idea to production.
- Sometimes the responsible answer is to fix a specific foundation first, reframe the work, or stop it. The client owns the outputs either way.
-->
