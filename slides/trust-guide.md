---
theme: default
title: App Trust Tiers · Field Guide
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · FIELD GUIDE" />

<div class="section-hero-grid">
<div class="section-hero-left">

# App Trust Tiers

<p class="section-subtitle">A field guide to the software your people build themselves: four tiers, three clearances, one scorecard.</p>
</div>
</div>

<TrustLadder variant="strip" />

<!--
This deck is the reference. It is written to be read, left behind after the story has been told, and used in the first workshop.

- Part one: how to read the model, and what it is not.
- Parts two to five: one section per tier, each with an outline, the control stack, what admits an app, and the signals that move it on.
- Part six: applying it, with the three sign-offs, the owners, the Codex, and the scorecard.
-->

---
title: Trust, not maturity
class: slide-light
---

<Eyebrow text="HOW TO READ THE MODEL · TRUST, NOT MATURITY" />

# Trust, not maturity.

<p class="slide-lead">A maturity model grades an organisation. A trust tier describes one app: how far it can be trusted today, and what has to be true before it is trusted further.</p>

<div class="about-grid">
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">01 · THE SUBJECT</span><span>ONE APP</span></div>
<h3>The app, never the builder</h3>
<p>A tier is a property of one app, measured by who depends on it, what it touches, what happens on Monday morning if it stops, and who sustains it. The same builder may own apps at three tiers.</p>
</div>
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">02 · THE DIRECTION</span><span>NO LADDER</span></div>
<h3>Higher is not better</h3>
<p>Each tier is a complete, safe place for an app to be. Most apps should live their whole lives at Tier 1. An app moves up only when a signal says it must, and only once the next clearance is in place.</p>
</div>
<div class="cc-card">
<div class="card-eyebrow"><span class="card-num">03 · THE SHAPE</span><span>CUMULATIVE</span></div>
<h3>A staircase, not a menu</h3>
<p>Every tier keeps everything below it and adds one layer. The layers are the clearances: cleared to share, cleared for live data, cleared for production. Nothing is skipped.</p>
</div>
</div>

<p class="lineage-line">Lineage: <strong>Agile Fluency</strong> for zones as complete choices rather than grades · <strong>CMMI</strong> for cumulative levels · <strong>NASA technology readiness levels</strong> for readiness as the measure · <strong>Shell's green, amber and red zones</strong> for citizen development at scale.</p>

<!--
Three rules of reading, and the lineage for anyone who wants to check the thinking.

- The subject is the app. We never grade a person; a builder may have a throwaway at Tier 0 and a department tool at Tier 2 on the same day.
- There is no ladder to climb. Tier 1 is a destination for most apps, and that is a success, not a stall.
- The tiers are cumulative. Each one adds exactly one layer, and the layer is what the clearance checks.
-->

---
title: The model at a glance
class: slide-light
---

<Eyebrow text="HOW TO READ THE MODEL · AT A GLANCE" />

# The model at a glance.

<p class="slide-lead">Four tiers, three clearances between them, and the share of a healthy estate at each. Each column keeps the layers below it and adds one.</p>

<TrustLadder />

<!--
The one diagram that recurs through both decks.

- Read left to right: each column adds one layer, and each gap between columns is a clearance with an indicative cost in time.
- The bottom row is what a healthy estate looks like after a year: nothing left at Tier 0 for long, three quarters at Tier 1, a fifth at Tier 2, one in twenty supported.
- If the estate is inverted, with most apps at Tier 0, the gap is the workspace. If nothing reaches Tier 3, the gap is usually funding for graduation.
-->

---
title: What it is not
class: slide-light
---

<Eyebrow text="HOW TO READ THE MODEL · WHAT IT IS NOT" />

# What it is not.

<p class="slide-lead">Four things people hear when they first see a model like this, and why none of them is true here.</p>

<div class="not-grid">
<div class="not-item">
<span class="not-item-num">01</span>
<div>
<h3>Not a <s>ban</s></h3>
<p>Tier 0 is where every app starts, and it is discovery. Nobody is punished for building. <strong>The invitation is to bring it into the workspace, where it is safer and easier to build.</strong></p>
</div>
</div>
<div class="not-item">
<span class="not-item-num">02</span>
<div>
<h3>Not a <s>review board</s></h3>
<p>The first clearance is automated and self-service. A person signs once for live data and once for production. <strong>Nothing waits in a queue.</strong></p>
</div>
</div>
<div class="not-item">
<span class="not-item-num">03</span>
<div>
<h3>Not a <s>ladder everyone climbs</s></h3>
<p>Roughly three in four apps belong at Tier 1 for life. <strong>Graduation is for the one in twenty that fails the Monday 9am test</strong>, where operations stall if it stops.</p>
</div>
</div>
<div class="not-item">
<span class="not-item-num">04</span>
<div>
<h3>Not a <s>technology decision</s></h3>
<p>The model is about trust and ownership. <strong>Omnia and Emery make two of the clearances cheaper; the tiers stand without them.</strong></p>
</div>
</div>
</div>

<!--
Say these four out loud early. Each one is an objection that otherwise sits unspoken in the room.

- Not a ban: the moment builders think Tier 0 is forbidden, they hide, and the inventory becomes fiction.
- Not a review board: two human sign-offs in an app's whole life, each with a defined owner and a defined piece of evidence.
- Not a ladder: three quarters of apps stay at Tier 1, and that is the model working.
- Not a technology decision: Omnia and Emery are how we make Tiers 2 and 3 cheap. The model holds with other tools.
-->

---
title: Tier 0
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="TIER 0 · UNMANAGED" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Tier 0

<p class="section-subtitle"><strong>Unmanaged</strong> — one person, one laptop. Discovery.</p>
</div>
<div class="section-big-glyph">T0</div>
</div>

<TrustLadder variant="strip" :current="0" />

---
title: T0 · Outline
class: slide-light
---

<Eyebrow text="TIER 0 · UNMANAGED · OUTLINE" />

# One person, one laptop.

<p class="slide-lead">Every app begins here, and that is fine. Tier 0 is discovery: the cheapest way an organisation learns what its people need. The one rule is that nothing live and nothing shared stays here.</p>

<div class="tier-outline-grid">
<div class="tier-profile-card is-t0">
<div>
<div class="tier-big-tag">T0 · Unmanaged</div>
<h2>Where every app starts.</h2>
<p>A working prototype on one laptop, built with whatever AI tool was to hand. Nothing has gone wrong, and nothing is in place.</p>
</div>
<div class="tier-meta-list">
<div class="tier-meta-row"><span class="tier-meta-label">Reach</span><span class="tier-meta-val">The builder alone</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Data &amp; state</span><span class="tier-meta-val">Personal files, pasted extracts</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Monday 9am test</span><span class="tier-meta-val">Nobody notices</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Continuity</span><span class="tier-meta-val">None. One laptop, no history</span></div>
</div>
</div>

<div class="tier-features-2x2">
<div class="tier-feature-box is-dashed">
<div><div class="tier-feature-head"><span>Build</span><span>Absent</span></div>
<h3>Wherever the builder is</h3></div>
<p>A personal laptop and a consumer AI chat. The code may exist only inside a conversation.</p>
</div>
<div class="tier-feature-box is-dashed">
<div><div class="tier-feature-head"><span>Check</span><span>Absent</span></div>
<h3>Nothing runs</h3></div>
<p>No tests, no scans, no second pair of eyes. Safety depends entirely on the builder.</p>
</div>
<div class="tier-feature-box is-dashed">
<div><div class="tier-feature-head"><span>Data</span><span>Absent</span></div>
<h3>Pasted, not connected</h3></div>
<p>Extracts copied by hand, or a personal credential pasted into a prompt.</p>
</div>
<div class="tier-feature-box is-dashed">
<div><div class="tier-feature-head"><span>Run</span><span>Absent</span></div>
<h3>Runs when the builder runs it</h3></div>
<p>There is no deployment. When the laptop closes, the app is off.</p>
</div>
</div>
</div>

<!--
The outline of Tier 0 is an outline of absences, which is the point: nothing is in place, and nothing needs to be, as long as the app stays personal.

- The four boxes are the four lanes of the control stack, dashed because each is empty here.
- The profile card's four lines are the scorecard's four questions, answered for a typical Tier 0 app.
-->

---
title: T0 · Stack and signals
class: slide-light
---

<Eyebrow text="TIER 0 · UNMANAGED · CONTROLS AND SIGNALS" />

# Nothing is in place. Nothing has gone wrong yet.

<ControlStack :tier="0" />

<div class="move-strip">
<div class="move-card">
<div class="move-head"><span>What admits an app</span><em>No clearance</em></div>
<p class="move-prose">Tier 0 has no entry gate. It is the state an app is in before anyone but its builder knows it exists. <strong>A healthy estate holds none here for long: an app either moves into the workspace or is deleted.</strong></p>
</div>
<div class="move-card is-signals">
<div class="move-head"><span>Signals to move up</span><em>→ Tier 1 · Governed</em></div>
<div class="move-items">
<div class="move-item"><strong>A colleague asks for the link</strong><span>Reach goes past one</span></div>
<div class="move-item"><strong>It has run for more than a week</strong><span>It has become a habit</span></div>
<div class="move-item"><strong>It holds anything you would miss</strong><span>Data worth keeping needs history</span></div>
</div>
</div>
</div>

<!--
The control stack at Tier 0, and the signals that say it is time to move.

- The lit column is the only one that matters for this app today: personal laptop, nothing runs, pasted extracts, runs when the builder runs it.
- No clearance admits an app to Tier 0; it simply exists. The estate rule is that nothing stays here for long.
- Three signals, any one of which is enough: someone else wants it, it has become a habit, or it holds something you would miss.
-->

---
title: Tier 1
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="TIER 1 · GOVERNED" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Tier 1

<p class="section-subtitle"><strong>Governed</strong> — shared with a team, inside the Codex.</p>
</div>
<div class="section-big-glyph">T1</div>
</div>

<TrustLadder variant="strip" :current="1" />

---
title: T1 · Outline
class: slide-light
---

<Eyebrow text="TIER 1 · GOVERNED · OUTLINE" />

# Shared with a team, inside the Codex.

<p class="slide-lead">The destination for most apps. About three in four should live their whole lives here: shared with a team, inside the governed workspace, reaching nothing live. The clearance is set up once and takes days.</p>

<div class="tier-outline-grid">
<div class="tier-profile-card">
<div>
<div class="tier-big-tag">T1 · Governed</div>
<h2>Shared, inside the Codex.</h2>
<p>The app lives in the governed workspace: the approved template, Enterprise GitHub, automated gates on every change. It reaches nothing live.</p>
</div>
<div class="tier-meta-list">
<div class="tier-meta-row"><span class="tier-meta-label">Reach</span><span class="tier-meta-val">2–4 people, a team trial</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Data &amp; state</span><span class="tier-meta-val">Synthetic or local only</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Monday 9am test</span><span class="tier-meta-val">A mild annoyance</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Continuity</span><span class="tier-meta-val">Enterprise GitHub, automated checks</span></div>
</div>
</div>

<div class="tier-features-2x2">
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Build</span><span>The Codex</span></div>
<h3>One workspace, one template</h3></div>
<p>An approved AI editor that follows the organisation's rules, one template, one repository per app.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Check</span><span>The Codex</span></div>
<h3>Gates on every change</h3></div>
<p>Secret scan, dependency audit, and the template's tests. Advice in the editor, enforcement in the pipeline.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Data</span><span>Nothing live</span></div>
<h3>Synthetic or local only</h3></div>
<p>There is nothing in this sandbox worth stealing, so there is nothing to clear before sharing it.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Run</span><span>Best effort</span></div>
<h3>Isolated sandbox</h3></div>
<p>Deployed to a sandbox with no route to production. The builder keeps it running; nobody is on call.</p>
</div>
</div>
</div>

<!--
Tier 1 is the workspace, and the workspace is what we call the Codex.

- Build: one approved editor, one template, Enterprise GitHub. The builder's day looks the same; the location changed.
- Check: the gates. These are the automated controls that make the first clearance self-service.
- Data: nothing live, on purpose. This is what makes the clearance cheap.
- Run: a sandbox with no path to production, best effort. The builder keeps it alive.
-->

---
title: T1 · Stack and signals
class: slide-light
---

<Eyebrow text="TIER 1 · GOVERNED · CONTROLS AND SIGNALS" />

# The Codex guides. The gates enforce.

<ControlStack :tier="1" />

<div class="move-strip">
<div class="move-card">
<div class="move-head"><span>What admits an app</span><em>Cleared to share · Days</em></div>
<div class="move-items">
<div class="move-item"><strong>Code in the workspace</strong><span>Enterprise GitHub, the approved template</span></div>
<div class="move-item"><strong>Gates green</strong><span>Secret scan, dependency audit, tests</span></div>
<div class="move-item"><strong>Nothing live configured</strong><span>No production source or credential</span></div>
</div>
</div>
<div class="move-card is-signals">
<div class="move-head"><span>Signals to move up</span><em>→ Tier 2 · Connected</em></div>
<div class="move-items">
<div class="move-item"><strong>The team wants live data</strong><span>Exports are stale or by hand</span></div>
<div class="move-item"><strong>Five or more people rely on it</strong><span>Reach 5–24</span></div>
<div class="move-item"><strong>A missed day would slow the team</strong><span>Monday 9am: a slow day</span></div>
</div>
</div>
</div>

<!--
Two kinds of control at Tier 1, kept apart on purpose.

- The Codex guides: rules the AI editor follows, a template that starts from a safe shape, coaches who answer in hours. Advisory, and cheap to follow.
- The gates enforce: secret scan, dependency audit, tests, on every change. Mandatory, and impossible to skip.
- The clearance is three pieces of evidence the pipeline produces by itself. No meeting.
- The signals to move up all say the same thing: the app has started to matter beyond a trial.
-->

---
title: Tier 2
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="TIER 2 · CONNECTED" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Tier 2

<p class="section-subtitle"><strong>Connected</strong> — live data, through scoped contracts.</p>
</div>
<div class="section-big-glyph">T2</div>
</div>

<TrustLadder variant="strip" :current="2" />

---
title: T2 · Outline
class: slide-light
---

<Eyebrow text="TIER 2 · CONNECTED · OUTLINE" />

# Live data, through scoped contracts.

<p class="slide-lead">Where real value starts, and where most programmes stumble, either by handing out credentials or by saying no. The answer is a contract: the app gets exactly the access it is granted, and nothing more.</p>

<div class="tier-outline-grid">
<div class="tier-profile-card">
<div>
<div class="tier-big-tag">T2 · Connected</div>
<h2>Live data, by contract.</h2>
<p>The app reads and writes real data through scoped, audited capabilities. A named business owner stands behind it. It runs on the platform, not a laptop.</p>
</div>
<div class="tier-meta-list">
<div class="tier-meta-row"><span class="tier-meta-label">Reach</span><span class="tier-meta-val">5–24 people, a department</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Data &amp; state</span><span class="tier-meta-val">Live internal data, through contracts</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Monday 9am test</span><span class="tier-meta-val">A slow day, with a manual fallback</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Continuity</span><span class="tier-meta-val">A named business owner and a backup</span></div>
</div>
</div>

<div class="tier-features-2x2">
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Build</span><span>Same workspace</span></div>
<h3>Plus a data contract</h3></div>
<p>The builder's loop does not change. Data is asked for by contract, never by credential.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Check</span><span>Plus contracts</span></div>
<h3>Contract and schema checks</h3></div>
<p>The app can only call what its contract names. A new table or action is a contract change, reviewed once.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Data</span><span>Omnia</span></div>
<h3>Scoped, audited capabilities</h3></div>
<p>Exactly the rows and actions the app needs, under the caller's identity, every call logged. Omnia grants exactly this.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Run</span><span>Owned</span></div>
<h3>Platform-hosted, business-owned</h3></div>
<p>Runs on the platform. The business owner answers for it; engineering owns the contracts, not the app.</p>
</div>
</div>
</div>

<!--
Tier 2 is the step that decides whether a programme delivers value or just governs toys.

- Build: nothing changes for the builder except how data is asked for.
- Check: the pipeline adds contract and schema checks, so the app cannot call what it was not granted.
- Data: this is Omnia's role. A runtime where an app has only the capabilities it was granted, every call under the caller's identity and audited.
- Run: hosted on the platform, with a named business owner. Engineering owns the contracts, not the app.
-->

---
title: T2 · Stack and signals
class: slide-light
---

<Eyebrow text="TIER 2 · CONNECTED · CONTROLS AND SIGNALS" />

# Every read and write goes through a contract.

<ControlStack :tier="2" />

<div class="move-strip">
<div class="move-card">
<div class="move-head"><span>What admits an app</span><em>Cleared for live data · Weeks</em></div>
<div class="move-items">
<div class="move-item"><strong>A named owner and a backup</strong><span>In the business, not engineering</span></div>
<div class="move-item"><strong>A scoped data contract</strong><span>Rows and actions, nothing more</span></div>
<div class="move-item"><strong>One lightweight review</strong><span>Of the contract, not the code</span></div>
</div>
</div>
<div class="move-card is-signals">
<div class="move-head"><span>Signals to move up</span><em>→ Tier 3 · Supported</em></div>
<div class="move-items">
<div class="move-item"><strong>25 or more people, or anyone external</strong><span>Cross-team or customers</span></div>
<div class="move-item"><strong>It fails the Monday 9am test</strong><span>Operations stall without it</span></div>
<div class="move-item"><strong>It touches regulated or core records</strong><span>PII, finance, safety</span></div>
</div>
</div>
</div>

<!--
The clearance for live data is the one most organisations make too heavy or skip altogether.

- Three pieces of evidence: a named owner with a backup, a contract scoped to what the app needs, and one review of that contract. Hours of work, weeks of elapsed time, mostly waiting for the data owner.
- The review is of the contract, never the code. The code is the builder's; the data is the organisation's.
- The signals to move up are the three that make an app critical: reach, the Monday 9am test, and regulated data. Any one is enough.
-->

---
title: Tier 3
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="TIER 3 · SUPPORTED" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Tier 3

<p class="section-subtitle"><strong>Supported</strong> — production, with engineers on call.</p>
</div>
<div class="section-big-glyph">T3</div>
</div>

<TrustLadder variant="strip" :current="3" />

---
title: T3 · Outline
class: slide-light
---

<Eyebrow text="TIER 3 · SUPPORTED · OUTLINE" />

# Production, with engineers on call.

<p class="slide-lead">For the one app in twenty that fails the Monday 9am test. Graduation is a funded project, and it protects both sides: the builder keeps the product, engineering owns the code.</p>

<div class="tier-outline-grid">
<div class="tier-profile-card is-t3">
<div>
<div class="tier-big-tag">T3 · Supported</div>
<h2>Rebuilt, verified, on call.</h2>
<p>Rebuilt from a specification of what the prototype actually does, verified requirement by requirement, run under an SLA. The builder becomes product owner.</p>
</div>
<div class="tier-meta-list">
<div class="tier-meta-row"><span class="tier-meta-label">Reach</span><span class="tier-meta-val">25 or more, or external</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Data &amp; state</span><span class="tier-meta-val">Regulated and core records allowed</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Monday 9am test</span><span class="tier-meta-val">Operations stall</span></div>
<div class="tier-meta-row"><span class="tier-meta-label">Continuity</span><span class="tier-meta-val">Engineering on call, under an SLA</span></div>
</div>
</div>

<div class="tier-features-2x2">
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Build</span><span>Emery</span></div>
<h3>Rebuilt from the specification</h3></div>
<p>Emery reads the prototype into a specification. Engineers rebuild from it in verified waves. Keep the criteria, drop the code.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Check</span><span>Emery</span></div>
<h3>Every requirement verified</h3></div>
<p>Each wave is checked against the specification before it merges. Nothing lands that the specification does not state.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Data</span><span>Omnia</span></div>
<h3>Least privilege, regulated allowed</h3></div>
<p>Each capability is granted explicitly. Regulated data is admitted by name, never by default.</p>
</div>
<div class="tier-feature-box">
<div><div class="tier-feature-head"><span>Run</span><span>Omnia</span></div>
<h3>Production runtime, on call</h3></div>
<p>Engineering takes the pager under an SLA. The builder owns the backlog, not the incidents.</p>
</div>
</div>
</div>

<!--
Tier 3 is production, and it is rare by design.

- Build and check are Emery's role: read what the prototype actually does into a specification, rebuild from it in waves, verify every requirement before it merges.
- Data and run are Omnia's: least-privilege grants, regulated data admitted by name, a production runtime with engineering on call.
- The handover is the point. The builder keeps the product as its owner. Engineering owns the code and the pager.
-->

---
title: T3 · Stack and the rebuild
class: slide-light
---

<Eyebrow text="TIER 3 · SUPPORTED · CONTROLS AND THE REBUILD" />

# Keep the criteria. Drop the code.

<ControlStack :tier="3" />

<div class="move-strip">
<div class="move-card">
<div class="move-head"><span>What admits an app</span><em>Cleared for production · A funded project</em></div>
<div class="move-items">
<div class="move-item"><strong>A specification, scenarios included</strong><span>Emery derives it from the app itself</span></div>
<div class="move-item"><strong>A verified rebuild</strong><span>Every requirement checked before merge</span></div>
<div class="move-item"><strong>An SLA and a product owner</strong><span>Engineering on call; the builder owns the backlog</span></div>
</div>
</div>
<div class="move-card is-signals">
<div class="move-head"><span>Why rebuild, not harden</span><em>The third option</em></div>
<p class="move-prose">Hardening a prototype in place keeps months of generated code nobody understands. Starting from a blank page loses what the prototype learned. <strong>The specification is the third option: it keeps every behaviour people rely on and lets engineers build it properly.</strong></p>
</div>
</div>

<!--
The production clearance, and the argument for rebuilding from a specification rather than hardening in place.

- Three pieces of evidence: the specification, the verified rebuild, and the operating agreement with an SLA and a product owner.
- Hardening in place inherits code nobody understands. Starting over loses the discovery the prototype paid for. The specification keeps the second and avoids the first.
- Emery's role is to make that specification cheap to derive and the rebuild cheap to verify.
-->

---
title: Applying the model
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="APPLYING THE MODEL" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Applying the model

<p class="section-subtitle">Three sign-offs, who owns what, the Codex, and the scorecard.</p>
</div>
</div>

<TrustLadder variant="strip" />

---
title: Three clearances
class: slide-light
---

<Eyebrow text="APPLYING THE MODEL · THREE CLEARANCES" />

# Three clearances, three sign-offs.

<p class="slide-lead">A clearance is evidence, not a meeting. Most of it is produced by the pipeline. A person signs once for live data and once for production.</p>

<div class="signoff-grid">
<div class="signoff-col">
<div class="signoff-head">
<span class="signoff-head-code">Clearance 01 · T0 → T1</span>
<h3>Cleared to share</h3>
<span>Days · self-service</span>
</div>
<div class="signoff-row"><span>Code in Enterprise GitHub, from the approved template</span><span class="signoff-who is-auto">Pipeline</span></div>
<div class="signoff-row"><span>Secret scan, dependency audit, and tests pass on every change</span><span class="signoff-who is-auto">Pipeline</span></div>
<div class="signoff-row"><span>No live data source or production credential configured</span><span class="signoff-who is-auto">Pipeline</span></div>
<div class="signoff-row"><span>The builder has done the one-hour onboarding</span><span class="signoff-who">Builder</span></div>
</div>
<div class="signoff-col">
<div class="signoff-head">
<span class="signoff-head-code">Clearance 02 · T1 → T2</span>
<h3>Cleared for live data</h3>
<span>Weeks · one review</span>
</div>
<div class="signoff-row"><span>A named business owner and a backup</span><span class="signoff-who">Owner</span></div>
<div class="signoff-row"><span>A data contract scoped to the rows and actions the app needs</span><span class="signoff-who">Data owner</span></div>
<div class="signoff-row"><span>Every call runs under the caller's identity and is audited</span><span class="signoff-who is-auto">Platform</span></div>
<div class="signoff-row"><span>One lightweight review of the contract, not the code</span><span class="signoff-who">Platform team</span></div>
</div>
<div class="signoff-col is-t3">
<div class="signoff-head">
<span class="signoff-head-code">Clearance 03 · T2 → T3</span>
<h3>Cleared for production</h3>
<span>A funded project · engineering sign-off</span>
</div>
<div class="signoff-row"><span>A specification of what the app actually does, scenarios included</span><span class="signoff-who">Engineering</span></div>
<div class="signoff-row"><span>Rebuilt from the specification; every requirement verified before merge</span><span class="signoff-who is-auto">Pipeline</span></div>
<div class="signoff-row"><span>Least-privilege grants; regulated data admitted explicitly</span><span class="signoff-who">Security</span></div>
<div class="signoff-row"><span>An SLA, an on-call rota, and a product owner named</span><span class="signoff-who">Engineering</span></div>
</div>
</div>

<!--
The three clearances as a sign-off sheet. This is the slide the first workshop edits.

- Blue tags are evidence the pipeline or platform produces by itself. Grey tags are a person signing.
- Count the grey tags: one builder onboarding, three signatures for live data, three for production. That is every human sign-off in an app's life.
- The thresholds, the owners, and the wording are starting points. Change them in the workshop; keep the shape.
-->

---
title: Who owns what
class: slide-light
---

<Eyebrow text="APPLYING THE MODEL · WHO OWNS WHAT" />

# Who owns what.

<p class="slide-lead">The template is built once. Data contracts are published per domain. Hands-on engineering starts only at Tier 3, which is exactly why the other tiers stay fast.</p>

<OwnersGrid />

<div class="foot-note">Shell planned for 500 citizen developers and now runs more than 4,000, under a three-zone model with volunteer coaches embedded in every community. <strong>The coaches, not the controls, are what made it scale.</strong></div>

<!--
Three parties, four tiers.

- The builder never loses the app. At Tier 3 she becomes its product owner.
- The platform team and the coaches do the enabling work: the workspace, the template, the gates, the contracts, one review. In every programme that has scaled, the coaches mattered more than the controls.
- Engineering's time is spent where the risk is: the template once, the contracts per domain, and the app itself only at Tier 3.
-->

---
title: The Codex
class: slide-light
---

<Eyebrow text="APPLYING THE MODEL · THE CODEX" />

# The Codex guides. The gates enforce.

<p class="slide-lead">Two kinds of control, kept apart on purpose. Advice lives in the editor, where it is cheap to follow. Enforcement lives in the pipeline, where it cannot be skipped.</p>

<div class="codex-grid">
<div class="codex-col">
<div class="codex-head"><h3>Guides</h3><span>Advisory · in the editor</span></div>
<div class="codex-item"><span class="codex-stage">In the editor</span><div><strong>Rules the assistant follows.</strong> The template's conventions, the forbidden patterns, and the required tests, written as rules the AI editor reads on every prompt.</div></div>
<div class="codex-item"><span class="codex-stage">In the template</span><div><strong>A working skeleton.</strong> Authentication, logging, configuration, and a test harness already wired, so a builder starts from a safe shape rather than a blank file.</div></div>
<div class="codex-item"><span class="codex-stage">In the community</span><div><strong>Coaches, not reviewers.</strong> Volunteers embedded in business teams who answer in hours. Coaching scales; gatekeeping does not.</div></div>
<p class="codex-foot">A builder can ignore the guidance. The gates catch what matters.</p>
</div>
<div class="codex-col is-enforces">
<div class="codex-head"><h3>Enforces</h3><span>Mandatory · in the pipeline</span></div>
<div class="codex-item"><span class="codex-stage">Every change</span><div><strong>Secret scan.</strong> A credential in the code fails the build. The most common Tier 0 habit, caught the first time it is pushed.</div></div>
<div class="codex-item"><span class="codex-stage">Every change</span><div><strong>Dependency audit and tests.</strong> Known-vulnerable packages are blocked and the template's tests must pass. Zero of 5,600 becomes impossible by construction.</div></div>
<div class="codex-item"><span class="codex-stage">From Tier 2</span><div><strong>Contract and schema checks.</strong> The app can only call what its contract names. A new table or action is a contract change, reviewed once.</div></div>
<p class="codex-foot">Nothing merges, and nothing deploys, without them.</p>
</div>
</div>

<!--
The distinction we found missing in every assessment: advice and enforcement had been mixed, so either everything blocked or nothing did.

- Guides: rules the editor reads, a template that starts safe, coaches who answer quickly. Cheap to follow, free to ignore.
- Enforces: secret scan, dependency audit, tests, and from Tier 2 the contract checks. Nothing merges without them.
- Put the expensive-to-ignore things in the pipeline and the cheap-to-follow things in the editor, and the model runs itself.
-->

---
title: Score any app
class: slide-light
---

<Eyebrow text="APPLYING THE MODEL · THE SCORECARD" />

# Score any app in five minutes.

<p class="slide-lead">Four questions about the app, never about the builder. The highest column any answer reaches sets the tier. The thresholds are starting points; set yours in the first workshop.</p>

<Scorecard />

<!--
The scorecard, blank, for the workshop.

- Reach, data, the Monday 9am test, continuity. Mark one column per row.
- The highest column wins. A one-person app that touches regulated data is a Tier 3 question, however few use it.
- Change the thresholds to fit the organisation; the four questions stay.
-->

---
title: Where to start
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
<Eyebrow text="PROPELLERHEAD · WHERE TO START" />

# Where to<br>start.

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
The same three steps as the story deck, so the two documents end in one place.

- Map first. It costs two weeks and commits you to nothing.
- Open Tiers 1 and 2 with one builder and one real use case.
- Graduate one winner as a funded project once the estate shows you which one.
-->
