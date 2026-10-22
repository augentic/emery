---
theme: default
title: Trust Model · Field Guide
aspectRatio: 16/9
canvasWidth: 1280
layout: default
class: slide-hero slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · FIELD GUIDE" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Trust Model

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
title: Earned trust, not maturity
class: slide-light
---

<Eyebrow text="HOW TO READ THE MODEL · EARNED TRUST, NOT MATURITY" />

<div class="ed-split">
<div class="ed-prose-col">

# Earned trust,<br>not maturity.

<p class="ed-prose">A maturity model grades an organisation. <strong>The Trust Model describes one app</strong> — measured by who depends on it, what it touches, what happens on Monday morning if it stops, and who sustains it. The same builder may own a throwaway script at Tier 0 and a department tool at Tier 2 on the same day.</p>
<p class="ed-prose"><strong>Higher is not better.</strong> Each tier is a complete, safe place for an app to live, and roughly three in four should stay at Tier 1 for life. An app expands its radius only when a real signal says it must — keeping every layer inside it and adding the next clearance.</p>
<p class="ed-footnote">Lineage: <strong>Agile Fluency</strong> (zones as destinations) · <strong>CMMI</strong> (cumulative layers) · <strong>NASA TRL</strong> (readiness) · <strong>Shell</strong> (citizen development at scale).</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 460" aria-hidden="true">
  <defs>
    <radialGradient id="t1-ring-fill" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#16b4e2" stop-opacity="0.18" />
      <stop offset="100%" stop-color="#a4238f" stop-opacity="0.06" />
    </radialGradient>
  </defs>
  <circle cx="215" cy="230" r="196" fill="none" stroke="rgba(13,16,48,0.14)" stroke-width="1.2" />
  <circle cx="215" cy="230" r="144" fill="none" stroke="rgba(13,16,48,0.22)" stroke-width="1.2" />
  <circle cx="215" cy="230" r="92" fill="url(#t1-ring-fill)" stroke="#0c7fa3" stroke-width="2" />
  <circle cx="215" cy="230" r="40" fill="#ffffff" stroke="#b4234e" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="215" y="226" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0</text>
  <text x="215" y="241" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">1 laptop</text>
  <line x1="215" y1="138" x2="438" y2="138" stroke="#0c7fa3" stroke-width="1.2" />
  <circle cx="215" cy="138" r="3.5" fill="#0c7fa3" />
  <text x="448" y="133" class="svg-kicker" fill="#0c7fa3">T1 · GOVERNED (~75%)</text>
  <text x="448" y="150" class="svg-label" fill="#0d1030">Destination for most apps</text>
  <text x="448" y="166" class="svg-caption" fill="rgba(13,16,48,0.58)">Cleared to share · The Codex</text>
  <line x1="215" y1="86" x2="438" y2="86" stroke="rgba(13,16,48,0.28)" stroke-width="1" />
  <circle cx="215" cy="86" r="3" fill="#0d1030" />
  <text x="448" y="81" class="svg-kicker" fill="rgba(13,16,48,0.55)">T2 · CONNECTED (~20%)</text>
  <text x="448" y="97" class="svg-body" fill="rgba(13,16,48,0.78)">Cleared for live data · Contracts</text>
  <line x1="215" y1="34" x2="438" y2="34" stroke="rgba(13,16,48,0.22)" stroke-width="1" />
  <circle cx="215" cy="34" r="3" fill="#a4238f" />
  <text x="448" y="29" class="svg-kicker" fill="#a4238f">T3 · SUPPORTED (~5%)</text>
  <text x="448" y="45" class="svg-body" fill="rgba(13,16,48,0.78)">Cleared for production · Spec + SLA</text>
  <line x1="244" y1="259" x2="354" y2="369" stroke="rgba(13,16,48,0.32)" stroke-width="1.2" />
  <circle cx="354" cy="369" r="3" fill="#0d1030" />
  <text x="368" y="366" class="svg-caption" fill="rgba(13,16,48,0.58)">Expanding reach requires</text>
  <text x="368" y="382" class="svg-label" fill="#0d1030">the next clearance boundary</text>
</svg>
</div>
</div>

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

<div class="ed-band">
<div class="ed-band-top">
<div>

# The model<br>at a glance.

</div>
<p class="ed-band-prose">Four tiers, three clearances between them, and the share of a healthy estate at each. Rather than isolated silos, <strong>each tier extends a continuous foundation</strong>: every control established at Tier 1 stays in place through Tier 3, and three quarters of all apps never need to move past Tier 1.</p>
</div>
<TrustLadder />
</div>

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

<div class="ed-split">
<div class="ed-prose-col">

# What it<br>is not.

<p class="ed-prose">When people first see a governance framework, they brace for four familiar traps: a ban on building, a slow review committee, a maturity ladder everyone is forced to climb, or a disguised tool mandate.</p>
<p class="ed-prose"><strong>None of those is true here.</strong> The moment builders think Tier 0 is forbidden, they hide and your inventory becomes fiction. The Trust Model replaces every gatekeeping reflex with an open workspace and proportionate clearances.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Column Headers -->
  <text x="20" y="34" class="svg-kicker" fill="#b4234e">WHAT PEOPLE BRACE FOR</text>
  <text x="296" y="34" class="svg-kicker" fill="#0c7fa3">WHAT THE TRUST MODEL ACTUALLY IS</text>
  <line x1="20" y1="48" x2="620" y2="48" stroke="rgba(13,16,48,0.16)" stroke-width="1.2" />

  <!-- Row 01 -->
  <text x="20" y="92" class="svg-kicker" fill="#b4234e">01 · NOT A BAN</text>
  <text x="20" y="114" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Punish Tier 0 shadow IT</text>
  <line x1="226" y1="108" x2="278" y2="108" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,103 282,108 272,113" fill="#0c7fa3" />
  <text x="296" y="102" class="svg-h3" fill="#0d1030">An invitation into the workspace</text>
  <text x="296" y="122" class="svg-caption" fill="rgba(13,16,48,0.6)">Tier 0 is discovery — bring it into The Codex where it is safer &amp; easier</text>
  <line x1="20" y1="146" x2="620" y2="146" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <!-- Row 02 -->
  <text x="20" y="186" class="svg-kicker" fill="#b4234e">02 · NOT A REVIEW BOARD</text>
  <text x="20" y="208" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Wait in a committee queue</text>
  <line x1="226" y1="202" x2="278" y2="202" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,197 282,202 272,207" fill="#0c7fa3" />
  <text x="296" y="196" class="svg-h3" fill="#0d1030">Automated first; two sign-offs in life</text>
  <text x="296" y="216" class="svg-caption" fill="rgba(13,16,48,0.6)">Pipeline clears T1; one contract sign-off at T2; one production sign-off at T3</text>
  <line x1="20" y1="240" x2="620" y2="240" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <!-- Row 03 -->
  <text x="20" y="280" class="svg-kicker" fill="#b4234e">03 · NOT A LADDER</text>
  <text x="20" y="302" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Every app must reach T3</text>
  <line x1="226" y1="296" x2="278" y2="296" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,291 282,296 272,301" fill="#0c7fa3" />
  <text x="296" y="290" class="svg-h3" fill="#0d1030">~75% belong at Tier 1 for life</text>
  <text x="296" y="310" class="svg-caption" fill="rgba(13,16,48,0.6)">Graduation is only for the ~5% that fail the Monday 9am test</text>
  <line x1="20" y1="334" x2="620" y2="334" stroke="rgba(13,16,48,0.1)" stroke-width="1" />

  <!-- Row 04 -->
  <text x="20" y="374" class="svg-kicker" fill="#b4234e">04 · NOT A TOOL MANDATE</text>
  <text x="20" y="396" class="svg-h3" fill="rgba(13,16,48,0.42)" text-decoration="line-through">Locked into one stack</text>
  <line x1="226" y1="390" x2="278" y2="390" stroke="#0c7fa3" stroke-width="1.6" />
  <polygon points="272,385 282,390 272,395" fill="#0c7fa3" />
  <text x="296" y="384" class="svg-h3" fill="#0d1030">Earned trust and clear ownership</text>
  <text x="296" y="404" class="svg-caption" fill="rgba(13,16,48,0.6)">Omnia and Emery make T2 &amp; T3 cheap; the model stands without them</text>
</svg>
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

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Outer boundary of absence -->
  <circle cx="320" cy="220" r="176" fill="none" stroke="rgba(180,35,78,0.18)" stroke-width="1.2" stroke-dasharray="6 6" />
  <!-- 4 Disconnected Radial Spokes (broken before reaching the center) -->
  <line x1="262" y1="162" x2="196" y2="96" stroke="rgba(13,16,48,0.22)" stroke-width="1.2" stroke-dasharray="3 4" />
  <line x1="378" y1="162" x2="444" y2="96" stroke="rgba(13,16,48,0.22)" stroke-width="1.2" stroke-dasharray="3 4" />
  <line x1="262" y1="278" x2="196" y2="344" stroke="rgba(13,16,48,0.22)" stroke-width="1.2" stroke-dasharray="3 4" />
  <line x1="378" y1="278" x2="444" y2="344" stroke="rgba(13,16,48,0.22)" stroke-width="1.2" stroke-dasharray="3 4" />
  <!-- Center Node: One Builder, One Laptop -->
  <circle cx="320" cy="220" r="64" fill="rgba(255,59,107,0.06)" stroke="#b4234e" stroke-width="1.8" stroke-dasharray="5 4" />
  <text x="320" y="204" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0 · DISCOVERY</text>
  <text x="320" y="226" text-anchor="middle" class="svg-h3" fill="#0d1030">One Laptop</text>
  <text x="320" y="244" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Reach: 1 builder</text>
  <!-- Top-Left Absence: BUILD -->
  <circle cx="152" cy="68" r="8" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="2 2" />
  <text x="132" y="54" text-anchor="end" class="svg-kicker" fill="#b4234e">BUILD · ABSENT</text>
  <text x="132" y="72" text-anchor="end" class="svg-label" fill="#0d1030">Consumer AI Chat</text>
  <text x="132" y="88" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.55)">Code lives in a prompt thread</text>
  <!-- Top-Right Absence: CHECK -->
  <circle cx="488" cy="68" r="8" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="2 2" />
  <text x="508" y="54" class="svg-kicker" fill="#b4234e">CHECK · ABSENT</text>
  <text x="508" y="72" class="svg-label" fill="#0d1030">Nothing Runs</text>
  <text x="508" y="88" class="svg-caption" fill="rgba(13,16,48,0.55)">No tests, scans, or repo history</text>
  <!-- Bottom-Left Absence: DATA -->
  <circle cx="152" cy="372" r="8" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="2 2" />
  <text x="132" y="362" text-anchor="end" class="svg-kicker" fill="#b4234e">DATA · ABSENT</text>
  <text x="132" y="380" text-anchor="end" class="svg-label" fill="#0d1030">Pasted Extracts</text>
  <text x="132" y="396" text-anchor="end" class="svg-caption" fill="rgba(13,16,48,0.55)">Spreadsheet copies or personal keys</text>
  <!-- Bottom-Right Absence: RUN -->
  <circle cx="488" cy="372" r="8" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="2 2" />
  <text x="508" y="362" class="svg-kicker" fill="#b4234e">RUN · ABSENT</text>
  <text x="508" y="380" class="svg-label" fill="#0d1030">No Deployment</text>
  <text x="508" y="396" class="svg-caption" fill="rgba(13,16,48,0.55)">When the laptop closes, it stops</text>
</svg>
</div>
<div class="ed-prose-col">
<div class="ed-tier-tag">
  <span class="ed-tier-glyph">T0</span>
  <span class="ed-tier-sub" style="color: #b4234e;">Unmanaged · Discovery</span>
</div>

# One person,<br>one laptop.

<p class="ed-prose">Every app begins here, and that is fine. Tier 0 is an outline of absences — no governed repo, no automated checks, pasted spreadsheet extracts, and no deployment beyond the builder's own machine. If it stops on Monday morning, nobody notices.</p>
<p class="ed-prose"><strong>Tier 0 is the cheapest product discovery an organisation has.</strong> Nothing needs to be in place while the tool stays personal. The one rule is that nothing shared and nothing connected to live systems stays here.</p>
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

<div class="ed-band">
<div class="ed-band-top">
<div>

# Nothing is in place.<br>Nothing has gone wrong.

</div>
<p class="ed-band-prose">Tier 0 has no entry gate: an app exists here before anyone but its builder knows about it. <strong>A healthy estate holds zero apps at Tier 0 for long</strong> — within days of proving useful, a prototype either moves into the governed workspace or is thrown away at zero cost.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 350" aria-hidden="true">
  <!-- Left Entry: Spontaneous Creation -->
  <circle cx="96" cy="175" r="54" fill="rgba(255,59,107,0.06)" stroke="#b4234e" stroke-width="1.8" stroke-dasharray="5 4" />
  <text x="96" y="162" text-anchor="middle" class="svg-kicker" fill="#b4234e">NO ENTRY GATE</text>
  <text x="96" y="182" text-anchor="middle" class="svg-h3" fill="#0d1030">T0 Prototype</text>
  <text x="96" y="198" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Afternoon experiment</text>
  <!-- Timeline to Fork -->
  <line x1="150" y1="175" x2="340" y2="175" stroke="rgba(13,16,48,0.3)" stroke-width="1.8" />
  <circle cx="340" cy="175" r="5" fill="#0d1030" />
  <text x="245" y="158" text-anchor="middle" class="svg-kicker" fill="rgba(13,16,48,0.55)">DAYS 1 – 7</text>
  <!-- Lower Branch: Discarded (Healthy throwaway) -->
  <path d="M 340 175 C 400 175, 420 295, 510 295" fill="none" stroke="rgba(13,16,48,0.25)" stroke-width="1.5" stroke-dasharray="4 4" />
  <circle cx="518" cy="295" r="4" fill="rgba(13,16,48,0.35)" />
  <text x="536" y="291" class="svg-label" fill="rgba(13,16,48,0.65)">Discarded after trial</text>
  <text x="536" y="309" class="svg-caption" fill="rgba(13,16,48,0.48)">Idea tested at near-zero cost · Nothing to clean up</text>
  <!-- Three Upper Signal Branches converging on T1 Clearance -->
  <path d="M 340 175 C 410 175, 420 65, 500 65" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <path d="M 340 175 L 500 140" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <path d="M 340 175 C 410 175, 420 215, 500 215" fill="none" stroke="#0c7fa3" stroke-width="1.6" />
  <circle cx="500" cy="65" r="4" fill="#0c7fa3" />
  <text x="518" y="61" class="svg-label" fill="#0d1030">A colleague asks for the link</text>
  <text x="518" y="78" class="svg-caption" fill="rgba(13,16,48,0.58)">Signal 01 · Reach goes past one person</text>
  <circle cx="500" cy="140" r="4" fill="#0c7fa3" />
  <text x="518" y="136" class="svg-label" fill="#0d1030">It runs for more than a week</text>
  <text x="518" y="153" class="svg-caption" fill="rgba(13,16,48,0.58)">Signal 02 · Experiment becomes a weekly habit</text>
  <circle cx="500" cy="215" r="4" fill="#0c7fa3" />
  <text x="518" y="211" class="svg-label" fill="#0d1030">It holds anything you would miss</text>
  <text x="518" y="228" class="svg-caption" fill="rgba(13,16,48,0.58)">Signal 03 · State needs versioned history</text>
  <!-- Convergence into Clearance 01 Gate -->
  <path d="M 760 65 C 830 65, 840 140, 895 140" fill="none" stroke="#0c7fa3" stroke-width="1.5" />
  <line x1="760" y1="140" x2="895" y2="140" stroke="#0c7fa3" stroke-width="1.5" />
  <path d="M 760 215 C 830 215, 840 140, 895 140" fill="none" stroke="#0c7fa3" stroke-width="1.5" />
  <polygon points="890,135 900,140 890,145" fill="#0c7fa3" />
  <!-- Destination Node: Tier 1 Governed -->
  <circle cx="995" cy="140" r="76" fill="rgba(22,180,226,0.1)" stroke="#0c7fa3" stroke-width="2" />
  <text x="995" y="116" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">CLEARED TO SHARE</text>
  <text x="995" y="140" text-anchor="middle" class="svg-h3" fill="#0d1030">T1 · Governed</text>
  <text x="995" y="160" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.65)">Inside the Codex</text>
  <text x="995" y="176" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Self-service in days</text>
</svg>
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

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 280" aria-hidden="true">
  <!-- Subtle enclosing arc/bracket for The Codex -->
  <line x1="40" y1="32" x2="800" y2="32" stroke="rgba(12,127,163,0.3)" stroke-width="1.2" />
  <line x1="40" y1="32" x2="40" y2="46" stroke="rgba(12,127,163,0.3)" stroke-width="1.2" />
  <line x1="800" y1="32" x2="800" y2="46" stroke="rgba(12,127,163,0.3)" stroke-width="1.2" />
  <text x="420" y="20" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">INSIDE THE CODEX · GOVERNED WORKSPACE (~75% OF APPS)</text>
  <!-- Horizontal flow line between stations 1, 2, 3 -->
  <line x1="176" y1="138" x2="364" y2="138" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="358,133 368,138 358,143" fill="#0c7fa3" />
  <text x="270" y="124" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Every push</text>
  <line x1="476" y1="138" x2="664" y2="138" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="658,133 668,138 658,143" fill="#0c7fa3" />
  <text x="570" y="124" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Passes gates</text>
  <!-- Station 01 -->
  <circle cx="120" cy="138" r="56" fill="#ffffff" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="120" y="126" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · BUILD</text>
  <text x="120" y="146" text-anchor="middle" class="svg-label" fill="#0d1030">AI Editor</text>
  <text x="120" y="162" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">+ Template</text>
  <text x="120" y="220" text-anchor="middle" class="svg-body" fill="rgba(13,16,48,0.68)">Pre-wired auth &amp; tests</text>
  <text x="120" y="238" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.48)">Enterprise GitHub repo</text>
  <!-- Station 02 -->
  <circle cx="420" cy="138" r="56" fill="#0d1030" />
  <text x="420" y="126" text-anchor="middle" class="svg-kicker" fill="#78d8f2">02 · CHECK</text>
  <text x="420" y="146" text-anchor="middle" class="svg-label" fill="#ffffff">CI Gates</text>
  <text x="420" y="162" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.68)">Automated</text>
  <text x="420" y="220" text-anchor="middle" class="svg-body" fill="rgba(13,16,48,0.68)">Secret scan &amp; dep audit</text>
  <text x="420" y="238" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.48)">Zero review meetings</text>
  <!-- Station 03 -->
  <circle cx="720" cy="138" r="56" fill="rgba(22,180,226,0.1)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="720" y="126" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">03 · RUN</text>
  <text x="720" y="146" text-anchor="middle" class="svg-label" fill="#0d1030">Sandbox</text>
  <text x="720" y="162" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">2–4 users</text>
  <text x="720" y="220" text-anchor="middle" class="svg-body" fill="rgba(13,16,48,0.68)">Synthetic or local data</text>
  <text x="720" y="238" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.48)">Best-effort by builder</text>
  <!-- Air-Gap Vertical Barrier -->
  <line x1="876" y1="24" x2="876" y2="254" stroke="#b4234e" stroke-width="2" stroke-dasharray="6 6" />
  <text x="876" y="14" text-anchor="middle" class="svg-kicker" fill="#b4234e">NETWORK AIR GAP</text>
  <!-- Unreached right node: Live Production Data -->
  <circle cx="1020" cy="138" r="56" fill="none" stroke="rgba(13,16,48,0.22)" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="1020" y="132" text-anchor="middle" class="svg-kicker" fill="rgba(13,16,48,0.45)">UNREACHABLE</text>
  <text x="1020" y="150" text-anchor="middle" class="svg-label" fill="rgba(13,16,48,0.55)">Live Systems</text>
  <text x="1020" y="220" text-anchor="middle" class="svg-body" fill="rgba(13,16,48,0.5)">Nothing live to expose</text>
  <text x="1020" y="238" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Requires Tier 2 clearance →</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Shared with a team,<br>inside the Codex.

</div>
<div>
<p class="ed-prose">Tier 1 is the destination for roughly three in four apps. A small team of two to four colleagues can trial the tool together while the builder's day stays effortless: one approved AI editor, one starter template, and one repository in Enterprise GitHub with automated checks on every push.</p>
<p class="ed-prose">Because the sandbox holds <strong>synthetic or local data only</strong> and is air-gapped from production systems, there is nothing inside it worth stealing — and if it stops on Monday morning, it is only a mild annoyance. That is why the clearance takes days, not months.</p>
</div>
<div></div>
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

<div class="ed-split">
<div class="ed-prose-col">

# Inside Tier 1 — and<br>when to leave it.

<p class="ed-prose">An app enters Tier 1 the moment a colleague asks for the link or the prototype becomes a weekly habit. Admission requires <strong>zero meetings</strong>: moving the code into the approved Enterprise GitHub template turns on secret scanning, dependency audits, and tests automatically.</p>
<p class="ed-prose">Three quarters of apps stay right here. An app only seeks the next clearance when <strong>manual exports go stale</strong>, <strong>five or more people</strong> rely on it across a department, or a missed Monday morning would slow the team down.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <circle cx="62" cy="220" r="44" fill="#ffffff" stroke="#b4234e" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="62" y="216" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0</text>
  <text x="62" y="232" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.68)">Prototype</text>
  <line x1="106" y1="220" x2="210" y2="220" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="204,215 214,220 204,225" fill="#0c7fa3" />
  <rect x="116" y="182" width="84" height="24" rx="12" fill="#e8f6fb" stroke="rgba(12,127,163,0.35)" />
  <text x="158" y="198" text-anchor="middle" class="svg-pill" fill="#0c7fa3">SELF-SERVICE</text>
  <text x="158" y="242" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Template + CI</text>
  <text x="158" y="256" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">No live keys</text>
  <circle cx="318" cy="220" r="98" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="2" />
  <text x="318" y="164" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T1 · GOVERNED STATE</text>
  <text x="318" y="186" text-anchor="middle" class="svg-h3" fill="#0d1030">The Codex</text>
  <line x1="254" y1="200" x2="382" y2="200" stroke="rgba(13,16,48,0.12)" />
  <text x="318" y="220" text-anchor="middle" class="svg-label" fill="#0d1030">Approved AI Editor</text>
  <text x="318" y="240" text-anchor="middle" class="svg-label" fill="#0d1030">Automated Push Gates</text>
  <text x="318" y="260" text-anchor="middle" class="svg-label" fill="#0d1030">Synthetic Data Sandbox</text>
  <text x="318" y="344" text-anchor="middle" class="svg-body" fill="#0c7fa3">~75% of apps stay here for life</text>
  <path d="M 412 184 C 456 184, 464 118, 520 118" fill="none" stroke="rgba(164,35,143,0.45)" stroke-width="1.5" />
  <polygon points="516,113 526,118 516,123" fill="#a4238f" />
  <text x="534" y="114" class="svg-label" fill="#0d1030">Live data needed</text>
  <text x="534" y="130" class="svg-caption" fill="rgba(13,16,48,0.55)">Signal → Tier 2</text>
  <line x1="416" y1="220" x2="520" y2="220" stroke="rgba(164,35,143,0.45)" stroke-width="1.5" />
  <polygon points="516,215 526,220 516,225" fill="#a4238f" />
  <text x="534" y="216" class="svg-label" fill="#0d1030">5+ people rely on it</text>
  <text x="534" y="232" class="svg-caption" fill="rgba(13,16,48,0.55)">Signal → Tier 2</text>
  <path d="M 412 256 C 456 256, 464 322, 520 322" fill="none" stroke="rgba(164,35,143,0.45)" stroke-width="1.5" />
  <polygon points="516,317 526,322 516,327" fill="#a4238f" />
  <text x="534" y="318" class="svg-label" fill="#0d1030">Monday 9am slows</text>
  <text x="534" y="334" class="svg-caption" fill="rgba(13,16,48,0.55)">Signal → Tier 2</text>
</svg>
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

<div class="ed-split">
<div class="ed-prose-col">
<div class="ed-tier-tag">
  <span class="ed-tier-glyph">T2</span>
  <span class="ed-tier-sub">Connected · ~20% of apps</span>
</div>

# Live data, through<br>scoped contracts.

<p class="ed-prose">Tier 2 is where departmental value starts (5 to 24 people), and where most programmes stumble — either by handing out raw database credentials or by saying no. The answer is a <strong>scoped capability contract</strong>: the builder's workflow inside the Codex does not change, and data is asked for by contract rather than key.</p>
<p class="ed-prose">The app runs on <strong>Omnia</strong> with a named business owner and backup behind it. If it stops on Monday morning, the team has a slow day with a manual fallback — while engineering owns the reusable data contracts, never the app itself.</p>
</div>
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Left Node: Department App inside The Codex -->
  <circle cx="96" cy="180" r="68" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="96" y="156" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T2 · APP</text>
  <text x="96" y="178" text-anchor="middle" class="svg-h3" fill="#0d1030">Department</text>
  <text x="96" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">5–24 users</text>
  <text x="96" y="212" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Named owner</text>
  <!-- Connector to Omnia Contract Gate -->
  <line x1="164" y1="180" x2="246" y2="180" stroke="#0d1030" stroke-width="1.8" />
  <polygon points="240,175 250,180 240,185" fill="#0d1030" />
  <text x="205" y="168" text-anchor="middle" class="svg-caption" fill="#0d1030">No raw keys</text>
  <!-- Center Gate: Omnia Scoped Contract Plane -->
  <line x1="256" y1="44" x2="256" y2="316" stroke="#0d1030" stroke-width="2.5" />
  <line x1="396" y1="44" x2="396" y2="316" stroke="#0d1030" stroke-width="2.5" />
  <rect x="256" y="56" width="140" height="248" fill="rgba(13,16,48,0.04)" />
  <text x="326" y="34" text-anchor="middle" class="svg-kicker" fill="#0d1030">OMNIA · DATA CONTRACT</text>
  <circle cx="326" cy="108" r="4" fill="#0c7fa3" />
  <text x="326" y="128" text-anchor="middle" class="svg-label" fill="#0d1030">Scoped Rows</text>
  <text x="326" y="144" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Exact tables &amp; verbs</text>
  <circle cx="326" cy="180" r="4" fill="#0c7fa3" />
  <text x="326" y="200" text-anchor="middle" class="svg-label" fill="#0d1030">Caller Identity</text>
  <text x="326" y="216" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">No ambient access</text>
  <circle cx="326" cy="252" r="4" fill="#0c7fa3" />
  <text x="326" y="272" text-anchor="middle" class="svg-label" fill="#0d1030">Every Call Audited</text>
  <text x="326" y="288" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Schema checked in CI</text>
  <!-- Connector from Omnia to Live Internal Data -->
  <line x1="396" y1="180" x2="478" y2="180" stroke="#0c7fa3" stroke-width="1.8" />
  <polygon points="472,175 482,180 472,185" fill="#0c7fa3" />
  <text x="437" y="168" text-anchor="middle" class="svg-caption" fill="#0c7fa3">Granted only</text>
  <!-- Right Node: Live Internal Data -->
  <circle cx="548" cy="180" r="64" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="548" y="164" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">CONNECTED</text>
  <text x="548" y="184" text-anchor="middle" class="svg-h3" fill="#0d1030">Live Internal</text>
  <text x="548" y="202" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.62)">Operational APIs &amp; DBs</text>
  <!-- Bottom Boundary: Regulated / Core Records blocked until Tier 3 -->
  <line x1="40" y1="358" x2="610" y2="358" stroke="#a4238f" stroke-width="1.5" stroke-dasharray="5 5" />
  <text x="326" y="384" text-anchor="middle" class="svg-kicker" fill="#a4238f">TIER 3 BOUNDARY · REGULATED PII, FINANCE &amp; SAFETY RECORDS</text>
  <text x="326" y="404" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Admitted only after an Emery specification, verified rebuild, and security sign-off</text>
</svg>
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

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 640 440" aria-hidden="true">
  <!-- Left Entry: Three inputs of Clearance 02 -->
  <text x="16" y="96" class="svg-kicker" fill="#0c7fa3">CLEARANCE 02 · WEEKS</text>
  <text x="16" y="124" class="svg-label" fill="#0d1030">Named Owner + Backup</text>
  <text x="16" y="140" class="svg-caption" fill="rgba(13,16,48,0.55)">In the business unit</text>
  <text x="16" y="196" class="svg-label" fill="#0d1030">Scoped Data Contract</text>
  <text x="16" y="212" class="svg-caption" fill="rgba(13,16,48,0.55)">Exact rows &amp; actions</text>
  <text x="16" y="268" class="svg-label" fill="#0d1030">One Contract Review</text>
  <text x="16" y="284" class="svg-caption" fill="rgba(13,16,48,0.55)">Never a code review</text>
  <path d="M 184 128 C 216 128, 220 204, 248 204" fill="none" stroke="#0c7fa3" stroke-width="1.5" />
  <line x1="184" y1="204" x2="248" y2="204" stroke="#0c7fa3" stroke-width="1.5" />
  <path d="M 184 276 C 216 276, 220 204, 248 204" fill="none" stroke="#0c7fa3" stroke-width="1.5" />
  <!-- Center Node: T2 Connected -->
  <circle cx="336" cy="204" r="84" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="336" y="162" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T2 · CONNECTED</text>
  <text x="336" y="186" text-anchor="middle" class="svg-h3" fill="#0d1030">Omnia Runtime</text>
  <line x1="278" y1="198" x2="394" y2="198" stroke="rgba(13,16,48,0.12)" />
  <text x="336" y="218" text-anchor="middle" class="svg-caption" fill="#0d1030">Contract &amp; schema CI</text>
  <text x="336" y="236" text-anchor="middle" class="svg-caption" fill="#0d1030">Caller-identity audit</text>
  <text x="336" y="254" text-anchor="middle" class="svg-caption" fill="#0d1030">Platform-hosted</text>
  <text x="336" y="316" text-anchor="middle" class="svg-body" fill="#0d1030">~20% of estate</text>
  <!-- Right Exit Signals to Tier 3 -->
  <path d="M 418 176 C 452 176, 458 118, 502 118" fill="none" stroke="#a4238f" stroke-width="1.6" />
  <polygon points="498,113 508,118 498,123" fill="#a4238f" />
  <text x="516" y="114" class="svg-label" fill="#0d1030">25+ or external users</text>
  <text x="516" y="130" class="svg-caption" fill="#a4238f">Signal → Tier 3</text>
  <line x1="420" y1="204" x2="502" y2="204" stroke="#a4238f" stroke-width="1.6" />
  <polygon points="498,199 508,204 498,209" fill="#a4238f" />
  <text x="516" y="200" class="svg-label" fill="#0d1030">Monday 9am stalls</text>
  <text x="516" y="216" class="svg-caption" fill="#a4238f">Signal → Tier 3</text>
  <path d="M 418 232 C 452 232, 458 290, 502 290" fill="none" stroke="#a4238f" stroke-width="1.6" />
  <polygon points="498,285 508,290 498,295" fill="#a4238f" />
  <text x="516" y="286" class="svg-label" fill="#0d1030">Regulated / PII data</text>
  <text x="516" y="302" class="svg-caption" fill="#a4238f">Signal → Tier 3</text>
</svg>
</div>
<div class="ed-prose-col">

# Every read and write<br>goes through a contract.

<p class="ed-prose">The clearance for live data is the one most organisations make too heavy or skip altogether. Here it requires three pieces of evidence: a named business owner with a backup, a contract scoped to the exact rows and actions needed, and <strong>one lightweight review of that contract — never the code</strong>.</p>
<p class="ed-prose">The app stays at Tier 2 unless it crosses one of the three criticality thresholds: <strong>25 or more users</strong> (or anyone external), failing the <strong>Monday 9am test</strong> where operations stall without it, or touching <strong>regulated records</strong>.</p>
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

<div class="ed-band">
<div class="ed-band-top">
<div>
<div class="ed-tier-tag" style="margin-bottom: 4px;">
  <span class="ed-tier-glyph">T3</span>
  <span class="ed-tier-sub" style="color: #a4238f;">Supported · ~5% of apps</span>
</div>

# Production, with<br>engineers on call.

</div>
<p class="ed-band-prose">For the one app in twenty that fails the Monday 9am test, reaches 25+ or external users, or touches regulated data. Graduation is a funded project that protects both sides: <strong>Emery</strong> extracts a durable specification from the working prototype so engineers can rebuild it in verified waves on <strong>Omnia</strong>, while the domain builder becomes Product Owner without ever carrying a pager.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 340" aria-hidden="true">
  <!-- Left Origin Node: Proven T2 Winner -->
  <circle cx="88" cy="175" r="56" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="88" y="158" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">PROVEN WINNER</text>
  <text x="88" y="178" text-anchor="middle" class="svg-h3" fill="#0d1030">T2 App</text>
  <text x="88" y="196" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">9 months of discovery</text>
  <!-- Split into Upper Track (Product Ownership) and Lower Track (Engineering & Runtime) -->
  <path d="M 144 155 C 220 155, 230 78, 320 78" fill="none" stroke="#0c7fa3" stroke-width="1.8" />
  <line x1="320" y1="78" x2="940" y2="78" stroke="#0c7fa3" stroke-width="1.8" stroke-dasharray="5 5" />
  <polygon points="934,73 944,78 934,83" fill="#0c7fa3" />
  <circle cx="320" cy="78" r="5" fill="#0c7fa3" />
  <text x="340" y="54" class="svg-kicker" fill="#0c7fa3">TRACK 01 · CONTINUITY OF INTENT (THE BUILDER)</text>
  <text x="340" y="72" class="svg-label" fill="#0d1030">Domain Builder becomes Product Owner — owns priorities &amp; scenarios, never the pager</text>
  <!-- Lower Track: Emery Specify -> Verified Wave Rebuild -> Omnia Production -->
  <path d="M 144 195 C 220 195, 230 236, 310 236" fill="none" stroke="#a4238f" stroke-width="2" />
  <text x="310" y="146" class="svg-kicker" fill="#a4238f">TRACK 02 · PRODUCTION ENGINEERING (EMERY + OMNIA)</text>
  <!-- Station A: Emery Specify -->
  <circle cx="360" cy="236" r="50" fill="rgba(164,35,143,0.08)" stroke="#a4238f" stroke-width="2" />
  <text x="360" y="224" text-anchor="middle" class="svg-kicker" fill="#a4238f">EMERY</text>
  <text x="360" y="244" text-anchor="middle" class="svg-label" fill="#0d1030">Specify</text>
  <text x="360" y="308" text-anchor="middle" class="svg-body" fill="#0d1030">Extracts rules &amp; scenarios</text>
  <text x="360" y="325" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">From the working prototype</text>
  <line x1="410" y1="236" x2="596" y2="236" stroke="#a4238f" stroke-width="2" />
  <polygon points="590,231 600,236 590,241" fill="#a4238f" />
  <text x="503" y="222" text-anchor="middle" class="svg-caption" fill="#a4238f">Drop prototype code</text>
  <!-- Station B: Emery Verified Build -->
  <circle cx="650" cy="236" r="50" fill="#0d1030" />
  <text x="650" y="224" text-anchor="middle" class="svg-kicker" fill="#78d8f2">EMERY</text>
  <text x="650" y="244" text-anchor="middle" class="svg-label" fill="#ffffff">Verify &amp; Build</text>
  <text x="650" y="308" text-anchor="middle" class="svg-body" fill="#0d1030">Rebuilt in verified waves</text>
  <text x="650" y="325" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Every requirement checked</text>
  <line x1="700" y1="236" x2="886" y2="236" stroke="#0d1030" stroke-width="2" />
  <polygon points="880,231 890,236 880,241" fill="#0d1030" />
  <text x="793" y="222" text-anchor="middle" class="svg-caption" fill="#0d1030">Least-privilege grants</text>
  <!-- Station C: Omnia Production SLA -->
  <circle cx="995" cy="175" r="68" fill="#a4238f" />
  <text x="995" y="152" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.85)">T3 · OMNIA</text>
  <text x="995" y="175" text-anchor="middle" class="svg-h3" fill="#ffffff">Supported</text>
  <text x="995" y="195" text-anchor="middle" class="svg-caption" fill="rgba(255,255,255,0.85)">Engineers on call · SLA</text>
  <text x="995" y="266" text-anchor="middle" class="svg-body" fill="#0d1030">Regulated data admitted explicitly</text>
</svg>
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

<div class="ed-bottom-split">
<div class="ed-bottom-canvas">
<svg viewBox="0 0 1140 285" aria-hidden="true">
  <!-- Origin: Month 9 Prototype -->
  <circle cx="92" cy="142" r="52" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="92" y="130" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">CRITICAL SIGNAL</text>
  <text x="92" y="150" text-anchor="middle" class="svg-label" fill="#0d1030">Proven App</text>
  <text x="92" y="166" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Monday 9am test</text>
  <!-- Top Dead-End Path: Harden in place -->
  <path d="M 144 120 C 230 120, 250 42, 360 42 L 690 42" fill="none" stroke="rgba(180,35,78,0.4)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="684" y1="34" x2="698" y2="50" stroke="#b4234e" stroke-width="2" />
  <line x1="698" y1="34" x2="684" y2="50" stroke="#b4234e" stroke-width="2" />
  <text x="370" y="28" class="svg-kicker" fill="#b4234e">BAD OPTION 01 · HARDEN IN PLACE</text>
  <text x="716" y="46" class="svg-body" fill="rgba(13,16,48,0.62)">Engineers inherit months of opaque prompt-generated code nobody can safely maintain</text>
  <!-- Bottom Dead-End Path: Blank-page rewrite -->
  <path d="M 144 164 C 230 164, 250 242, 360 242 L 690 242" fill="none" stroke="rgba(180,35,78,0.4)" stroke-width="1.5" stroke-dasharray="4 4" />
  <line x1="684" y1="234" x2="698" y2="250" stroke="#b4234e" stroke-width="2" />
  <line x1="698" y1="234" x2="684" y2="250" stroke="#b4234e" stroke-width="2" />
  <text x="370" y="228" class="svg-kicker" fill="#b4234e">BAD OPTION 02 · START FROM A BLANK PAGE</text>
  <text x="716" y="246" class="svg-body" fill="rgba(13,16,48,0.62)">Throws away the rules, edge cases, and workflows the prototype already paid to discover</text>
  <!-- Center Winning Spine: The Third Option (Specification + Verified Rebuild) -->
  <line x1="144" y1="142" x2="352" y2="142" stroke="#0c7fa3" stroke-width="2.5" />
  <polygon points="346,137 356,142 346,147" fill="#0c7fa3" />
  <rect x="358" y="108" width="236" height="68" rx="12" fill="#0d1030" />
  <text x="476" y="134" text-anchor="middle" class="svg-kicker" fill="#78d8f2">THE THIRD OPTION · EMERY</text>
  <text x="476" y="156" text-anchor="middle" class="svg-h3" fill="#ffffff">Durable Specification</text>
  <line x1="594" y1="142" x2="736" y2="142" stroke="#a4238f" stroke-width="2.5" />
  <polygon points="730,137 740,142 730,147" fill="#a4238f" />
  <rect x="742" y="108" width="216" height="68" rx="12" fill="rgba(164,35,143,0.1)" stroke="#a4238f" stroke-width="1.8" />
  <text x="850" y="134" text-anchor="middle" class="svg-kicker" fill="#a4238f">VERIFIED WAVES</text>
  <text x="850" y="156" text-anchor="middle" class="svg-h3" fill="#0d1030">Clean Rebuild</text>
  <line x1="958" y1="142" x2="1022" y2="142" stroke="#a4238f" stroke-width="2.5" />
  <polygon points="1016,137 1026,142 1016,147" fill="#a4238f" />
  <circle cx="1076" cy="142" r="46" fill="#a4238f" />
  <text x="1076" y="136" text-anchor="middle" class="svg-kicker" fill="rgba(255,255,255,0.85)">T3 SLA</text>
  <text x="1076" y="154" text-anchor="middle" class="svg-label" fill="#ffffff">Omnia</text>
</svg>
</div>
<div class="ed-bottom-prose-row">
<div>

# Keep the criteria.<br>Drop the code.

</div>
<div>
<p class="ed-prose">Hardening a prototype in place forces engineers to own months of generated code nobody understands. Starting over from a blank page loses everything the prototype discovered. <strong>The specification is the third option</strong>: Emery reads the working app's behaviour and scenarios into a versioned spec, and engineers rebuild from it cleanly.</p>
<p class="ed-prose">Admission to Tier 3 requires three pieces of evidence: the <strong>durable specification</strong>, a <strong>verified rebuild</strong> where every requirement is checked before merge, and an <strong>engineering SLA</strong> with the builder named as Product Owner.</p>
</div>
<div></div>
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

<div class="ed-split is-reversed">
<div class="ed-diagram-pane">
<svg viewBox="0 0 680 440" aria-hidden="true">
  <!-- Diagonal rising spine from T0 to T3 -->
  <line x1="64" y1="364" x2="616" y2="76" stroke="rgba(13,16,48,0.16)" stroke-width="2" />
  <!-- Tier Circles along the diagonal -->
  <circle cx="64" cy="364" r="24" fill="#ffffff" stroke="#b4234e" stroke-width="1.5" stroke-dasharray="4 4" />
  <text x="64" y="368" text-anchor="middle" class="svg-label" fill="#b4234e">T0</text>
  <circle cx="248" cy="268" r="26" fill="#e8f6fb" stroke="#0c7fa3" stroke-width="2" />
  <text x="248" y="273" text-anchor="middle" class="svg-label" fill="#0c7fa3">T1</text>
  <circle cx="432" cy="172" r="26" fill="#ffffff" stroke="#0d1030" stroke-width="2" />
  <text x="432" y="177" text-anchor="middle" class="svg-label" fill="#0d1030">T2</text>
  <circle cx="616" cy="76" r="26" fill="#a4238f" />
  <text x="616" y="81" text-anchor="middle" class="svg-label" fill="#ffffff">T3</text>
  <!-- Gate 01 Diamond + vertical leader down -->
  <polygon points="156,306 166,316 156,326 146,316" fill="#0c7fa3" />
  <line x1="156" y1="326" x2="156" y2="378" stroke="#0c7fa3" stroke-width="1.2" />
  <text x="156" y="396" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">01 · CLEARED TO SHARE (DAYS)</text>
  <text x="156" y="414" text-anchor="middle" class="svg-body" fill="#0d1030">100% pipeline self-service · 0 human sign-offs</text>
  <text x="156" y="430" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Template, secret scan, dep audit &amp; 1h onboarding</text>
  <!-- Gate 02 Diamond + vertical leader up/down -->
  <polygon points="340,210 350,220 340,230 330,220" fill="#0d1030" />
  <line x1="340" y1="112" x2="340" y2="210" stroke="#0d1030" stroke-width="1.2" />
  <text x="340" y="64" text-anchor="middle" class="svg-kicker" fill="#0d1030">02 · CLEARED FOR LIVE DATA (WEEKS)</text>
  <text x="340" y="82" text-anchor="middle" class="svg-body" fill="#0d1030">Human sign-off #1 — Data contract only</text>
  <text x="340" y="98" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Named business owner + scoped Omnia contract</text>
  <!-- Gate 03 Diamond + vertical leader down -->
  <polygon points="524,114 534,124 524,134 514,124" fill="#a4238f" />
  <line x1="524" y1="134" x2="524" y2="236" stroke="#a4238f" stroke-width="1.2" />
  <text x="524" y="256" text-anchor="middle" class="svg-kicker" fill="#a4238f">03 · CLEARED FOR PRODUCTION (FUNDED)</text>
  <text x="524" y="274" text-anchor="middle" class="svg-body" fill="#0d1030">Human sign-off #2 — Engineering &amp; security</text>
  <text x="524" y="290" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Emery spec, verified rebuild, least-privilege &amp; SLA</text>
</svg>
</div>
<div class="ed-prose-col">

# Three clearances.<br>Two human sign-offs.

<p class="ed-prose">A clearance is evidence, not a committee. The first clearance is produced entirely by the pipeline in days, so moving a prototype into the governed workspace never waits in a queue.</p>
<p class="ed-prose">Across an app's entire life, <strong>a person signs off at most twice</strong>: once at Tier 2 to approve the scoped data contract (never the code), and once at Tier 3 to accept the verified production rebuild under an engineering SLA.</p>
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

<div class="ed-band">
<div class="ed-band-top">
<div>

# Who owns<br>what.

</div>
<p class="ed-band-prose">The template is built once. Data contracts are published per domain. <strong>Hands-on engineering starts only at Tier 3</strong>, which is why Tiers 1 and 2 stay fast. Shell scaled citizen development past 4,000 builders under a three-zone model where embedded coaches — not review committees — did the enabling work.</p>
</div>
<OwnersGrid />
</div>

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

<div class="ed-band">
<div class="ed-band-top">
<div>

# The Codex guides.<br>The gates enforce.

</div>
<p class="ed-band-prose">In every assessment we have run, advice and enforcement had been mixed together — so either everything blocked in a review queue, or nothing did. <strong>Guidance belongs in the editor</strong>, where it is cheap to follow and free to ignore. <strong>Enforcement belongs in the pipeline</strong>, where three deterministic gates block what matters before merge.</p>
</div>
<div class="ed-band-canvas">
<svg viewBox="0 0 1140 360" aria-hidden="true">
  <!-- Left Zone: Three converging advisory streams in the editor -->
  <text x="40" y="34" class="svg-kicker" fill="#0c7fa3">IN THE EDITOR · ADVISORY (CHEAP TO FOLLOW, FREE TO IGNORE)</text>
  <path d="M 280 96 C 390 96, 410 185, 500 185" fill="none" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <path d="M 280 185 L 500 185" fill="none" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <path d="M 280 274 C 390 274, 410 185, 500 185" fill="none" stroke="#0c7fa3" stroke-width="1.5" stroke-dasharray="4 4" />
  <circle cx="280" cy="96" r="4" fill="#0c7fa3" />
  <text x="40" y="92" class="svg-label" fill="#0d1030">AI Assistant Rules</text>
  <text x="40" y="110" class="svg-caption" fill="rgba(13,16,48,0.58)">Conventions &amp; forbidden patterns on every prompt</text>
  <circle cx="280" cy="185" r="4" fill="#0c7fa3" />
  <text x="40" y="181" class="svg-label" fill="#0d1030">Pre-Wired Starter Skeleton</text>
  <text x="40" y="199" class="svg-caption" fill="rgba(13,16,48,0.58)">Auth, logging, config &amp; test harness ready on day one</text>
  <circle cx="280" cy="274" r="4" fill="#0c7fa3" />
  <text x="40" y="270" class="svg-label" fill="#0d1030">Embedded Business Coaches</text>
  <text x="40" y="288" class="svg-caption" fill="rgba(13,16,48,0.58)">Volunteers who answer in hours, not committees</text>
  <!-- Commit / Push convergence point -->
  <circle cx="500" cy="185" r="7" fill="#0d1030" />
  <text x="500" y="164" text-anchor="middle" class="svg-kicker" fill="#0d1030">GIT PUSH</text>
  <!-- Pipeline spine through 3 vertical gate planes -->
  <line x1="507" y1="185" x2="1030" y2="185" stroke="#0d1030" stroke-width="2.2" />
  <polygon points="1024,180 1036,185 1024,190" fill="#0d1030" />
  <text x="570" y="34" class="svg-kicker" fill="#a4238f">IN THE PIPELINE · MANDATORY (IMPOSSIBLE TO SKIP)</text>
  <!-- Gate Plane 1 -->
  <line x1="610" y1="68" x2="610" y2="260" stroke="#0d1030" stroke-width="2" />
  <circle cx="610" cy="185" r="6" fill="#ffffff" stroke="#0d1030" stroke-width="2.5" />
  <text x="610" y="286" text-anchor="middle" class="svg-kicker" fill="#0d1030">GATE 01 · EVERY PUSH</text>
  <text x="610" y="305" text-anchor="middle" class="svg-label" fill="#0d1030">Secret Scan</text>
  <text x="610" y="322" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Blocks any credential</text>
  <!-- Gate Plane 2 -->
  <line x1="775" y1="68" x2="775" y2="260" stroke="#0d1030" stroke-width="2" />
  <circle cx="775" cy="185" r="6" fill="#ffffff" stroke="#0d1030" stroke-width="2.5" />
  <text x="775" y="286" text-anchor="middle" class="svg-kicker" fill="#0d1030">GATE 02 · EVERY PUSH</text>
  <text x="775" y="305" text-anchor="middle" class="svg-label" fill="#0d1030">Audit &amp; Tests</text>
  <text x="775" y="322" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Blocks CVEs &amp; broken tests</text>
  <!-- Gate Plane 3 -->
  <line x1="940" y1="68" x2="940" y2="260" stroke="#a4238f" stroke-width="2" />
  <circle cx="940" cy="185" r="6" fill="#ffffff" stroke="#a4238f" stroke-width="2.5" />
  <text x="940" y="286" text-anchor="middle" class="svg-kicker" fill="#a4238f">GATE 03 · TIER 2+</text>
  <text x="940" y="305" text-anchor="middle" class="svg-label" fill="#0d1030">Contract Check</text>
  <text x="940" y="322" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">Blocks ungranted data calls</text>
  <!-- Final Merge / Deploy node -->
  <circle cx="1076" cy="185" r="34" fill="rgba(22,180,226,0.12)" stroke="#0c7fa3" stroke-width="2" />
  <text x="1076" y="182" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">MERGE</text>
  <text x="1076" y="197" text-anchor="middle" class="svg-caption" fill="#0d1030">&amp; Deploy</text>
</svg>
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

<div class="ed-band">
<div class="ed-band-top">
<div>

# Score any app<br>in five minutes.

</div>
<p class="ed-band-prose">Four questions about the app, never about the builder. <strong>The highest column any answer reaches sets the tier</strong> — so a one-person script that touches regulated records is a Tier 3 question, however few people use it. Use this blank scorecard in your first workshop to map what already exists and calibrate your thresholds.</p>
</div>
<Scorecard />
</div>

<!--
The scorecard, blank, for the workshop.

- Reach, data, the Monday 9am test, continuity. Mark one column per row.
- The highest column wins. A one-person app that touches regulated data is a Tier 3 question, however few use it.
- Change the thresholds to fit the organisation; the four questions stay.
-->
