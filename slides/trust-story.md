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

<p class="section-subtitle">Earned trust for the software your people build themselves — four tiers and the three clearances between them.</p>
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
title: Everyone is building
class: slide-light
---

<Eyebrow text="THE SITUATION · EVERYONE IS BUILDING" />

# Everyone is building.

<p class="slide-lead">AI made the first version of an app nearly free. The tools arrived years before the system around them.</p>

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
<span class="punchline-tag">The gap</span>
<p>The volume is coming either way. <strong>The only question is whether it lands somewhere you can see it.</strong></p>
</div>

<!--
Three numbers, then move on. Do not read the sources aloud; they are on the slide for the sceptics.

- 40%: Gartner's figure for how much of the internal tooling of large enterprises will be AI-built within three years. This is not a fringe behaviour; it is the next way internal software gets made.
- 56%: the share of staff already using AI tools nobody approved. The behaviour precedes the policy, everywhere.
- 0 of 5,600: a scan of production apps built with these tools found not one with the basic web protections a junior engineer would add on day one.
- The point is not to frighten anyone. It is that the volume is arriving regardless, and the choice is only whether it arrives inside something you can see.
-->

---
title: The same three gaps
class: slide-light
---

<Eyebrow text="WHAT WE'VE LEARNED · WORKING WITH CLIENTS" />

# The same three gaps, every time.

<p class="slide-lead">In every organisation we have helped with citizen coding, the builders were capable. What was missing was the system around them.</p>

<div class="gaps-grid">
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">01 · WORKSPACE</span><span>MISSING</span></div>
<h3>No governed place to build</h3>
<p>Building happens on personal devices or straight inside production tenants. No sandbox, no repository, no record of what exists.</p>
<div class="gap-then"><strong>So</strong> nobody can say how many apps there are, or what they touch.</div>
</div>
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">02 · TOOLKIT</span><span>UNDECIDED</span></div>
<h3>No approved toolkit</h3>
<p>Teams cannot tell which assistants, models, and entry paths are sanctioned. So they choose for themselves, and keep quiet about it.</p>
<div class="gap-then"><strong>So</strong> every app is built differently, and assurance cannot be reused.</div>
</div>
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">03 · ASSURANCE</span><span>PERSONAL</span></div>
<h3>No consistent assurance</h3>
<p>Security and quality depend on the individual. The same app is safe or unsafe depending on who happened to build it.</p>
<div class="gap-then"><strong>So</strong> the careful builders carry the risk created by the rest.</div>
</div>
</div>

<div class="punchline-bar">
<span class="punchline-tag">The lesson</span>
<p>The issue was never capability. <strong>It was the missing system, and a system can be designed.</strong></p>
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
- Click. The blue steps are the Trust Model. Control steps up at four points, each one triggered by the app earning more trust: more people, live data, a Monday morning the business cannot afford.
- What is left between the steps and the curve are slivers, not a chasm. That is the whole idea. Proportionate, and timed to value.
-->

---
title: The Trust Model
class: slide-light
---

<Eyebrow text="THE ANSWER · TRUST MODEL" />

# Four tiers of earned trust. Three clearances.

<p class="slide-lead">Each tier is a complete, safe place for an app to be. An app moves up only when a signal says it must, and only once the next clearance is in place.</p>

<TrustLadder />

<!--
The model on one slide. Read it left to right.

- Tier 0, Unmanaged: one person, one laptop. This is where every app starts, and it is discovery, not a violation.
- Cleared to share, in days: a repository, the approved template, automated gates. That takes you to Tier 1, Governed. The Codex is the name we give that workspace, and about three quarters of apps should live and die here, happily.
- Cleared for live data, in weeks: a named owner and a scoped data contract. Tier 2, Connected. Real data, bounded risk.
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
- The data lane is deliberately boring: synthetic or local only. There is nothing in this sandbox worth stealing.
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

# The team wants live roster data. A scoped contract replaces the spreadsheet export.

<div class="chapter-notes">
<div class="chapter-note is-clearance">
<div class="chapter-note-head"><span>Cleared for live data</span><em>Weeks</em></div>
<p>A named owner and a backup. A data contract scoped to exactly the rows and actions the planner needs. One review. <strong>The app gets the access it is granted, and nothing more.</strong> <span class="enabler"><img src="/images/omnia-icon.svg" alt="" />Omnia</span></p>
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

- The answer is a contract. The planner asks for the roster through a scoped capability: these rows, these actions, under the caller's identity, every call audited. It never holds a credential.
- That is what Omnia is for. It is a runtime where an app can only touch what it has explicitly been granted. No ambient access, so a mistake in the app cannot become a mistake in the data.
- The clearance is light: a named owner with a backup, a contract, one review that takes hours. Weeks end to end, mostly waiting for the data owner.
- The builder's loop does not change. She still iterates the planner herself.
- The next signal is reach or criticality: twenty-five people, or a Monday morning the business cannot afford.
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
<p><strong>Emery</strong> reads what the planner actually does into a specification. Engineers rebuild from it in verified waves. It runs on <strong>Omnia</strong> under an engineering SLA. <span class="enabler"><img src="/images/emery-icon.svg" alt="" />Emery</span><span class="enabler"><img src="/images/omnia-icon.svg" alt="" />Omnia</span></p>
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
- Emery is the third option. It reads what the app actually does, the rules, the edge cases, the scenarios people rely on, into a durable specification. Engineers rebuild from that specification in verified waves, each wave checked against it before it merges.
- The rebuilt planner runs on Omnia with least-privilege grants, so the data story from Tier 2 carries straight through, now including regulated data if it needs it.
- The handover protects both sides. The operations lead becomes product owner; she keeps the thing she built and gets no pager. Engineering owns the code, the security, and the SLA.
- The line to remember: keep the criteria, drop the code. The prototype already paid for discovering what the software should do.
-->

---
title: Who owns what
class: slide-light
---

<Eyebrow text="THE OPERATING MODEL · WHO OWNS WHAT" />

# Engineering time goes where the risk is.

<p class="slide-lead">The template is built once. Data contracts are published per domain. Hands-on engineering starts only at Tier 3, which is exactly why the other tiers stay fast.</p>

<OwnersGrid />

<div class="foot-note">Shell planned for 500 citizen developers and now runs more than 4,000, under a three-zone model with volunteer coaches embedded in every community. <strong>The coaches, not the controls, are what made it scale.</strong></div>

<!--
The question every CIO asks next: how much engineering time does this consume?

- Read the bottom row. Engineering builds the template once at Tier 1. At Tier 2 it owns the data contracts, not the apps. Only at Tier 3 does it take on an app itself.
- The middle row is the enabling team and the coaches. In every programme that has scaled, Shell being the best documented, the coaches embedded in business teams mattered more than any control.
- The top row is the builder. Notice she never loses the app. At Tier 3 she becomes its product owner.
-->

---
title: Score any app
class: slide-light
---

<Eyebrow text="THE SCORECARD · FOUR QUESTIONS" />

# Score any app in five minutes.

<p class="slide-lead">Four questions about the app, never about the builder. Here is the shift planner at month nine.</p>

<Scorecard :mark="{ reach: 3, data: 2, criticality: 3, continuity: 2 }" subject="The shift planner, month 9" />

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
title: Start with one app
class: slide-split short-bar
---

<div class="split-image-pane">
<AmbientVideo src="/videos/cta-video.mp4" variant="split" />
<div class="split-brand-overlay">
<img src="/images/Propellerhead_Logo.svg" alt="Propellerhead" />
<p>25 years of building critical software across Aotearoa and the world — combining AI speed with senior engineering judgment.</p>
</div>
</div>

<div class="split-content-pane">
<div>
<Eyebrow text="PROPELLERHEAD · NEXT STEPS" />

# Start with<br>one app.

<div class="opportunity-rows" style="margin-top: 8px;">
<div class="opportunity-row">
<span class="opportunity-row-num">01</span>
<div>
<h4>Map what already exists</h4>
<p>Two weeks. Find every AI-built tool you can, score each on the four questions, and see the whole estate on one page.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Open Tiers 1 and 2</h4>
<p>Days, not months. The governed workspace, the gates, and the first scoped data contract, with one builder and one real use case.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Graduate one proven winner</h4>
<p>A funded project. Emery to specify and verify, Omnia to run, our engineers on call, and your builder as product owner.</p>
</div>
</div>
</div>
</div>

<div class="cta-card">
<div>
<div class="cta-card-label">READY WHEN YOU ARE</div>
<div class="cta-card-title">Let's score your first candidates.</div>
</div>
<div class="cta-pill">
<span class="cta-pill-dot">→</span>
<span>propellerhead.co.nz</span>
</div>
</div>
</div>

<!--
Close with one ask, not three.

- The first step costs two weeks and commits you to nothing: find what exists and score it. Most organisations are surprised by the count.
- Opening Tiers 1 and 2 is days of work with one builder and one real use case. You will see the loop end to end.
- Graduating a winner is a funded project, and by then you will know which app deserves it.
- The ask today: let us score your first candidates together.
-->
