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
.slidev-layout .slide-eyebrow {
  position: relative;
  padding-right: 72px;
  box-sizing: border-box;
}

.slidev-layout .eyebrow-num {
  position: absolute;
  top: 0;
  right: 0;
  white-space: nowrap;
}

.slidev-layout.slide-split .eyebrow-num {
  display: none;
}

.slidev-layout .foundation-progress {
  position: absolute;
  top: 28px;
  right: 160px;
  z-index: 2;
  width: 460px;
  margin: 0;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.slidev-layout .foundation-progress span {
  padding: 4px 8px 3px;
  border-top: 2px solid rgba(13, 16, 48, 0.12);
  font-size: 9px;
  line-height: 1.15;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--phl-ink-muted);
}

.slidev-layout .foundation-progress .is-active {
  border-color: var(--phl-cyan);
  background: rgba(22, 180, 226, 0.05);
  color: var(--phl-cyan-ink);
}

.slidev-layout .foundation-progress .is-active.is-magenta {
  border-color: var(--phl-magenta);
  background: rgba(164, 35, 143, 0.05);
  color: #a4238f;
}

.live-stage {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.live-split {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 0.9fr 1.1fr;
  gap: 64px;
  align-items: center;
}

.live-copy {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.slidev-layout .live-claim {
  margin: 0;
  max-width: 940px;
  font-size: 21px;
  line-height: 1.38;
  color: var(--phl-ink-body);
}

.slidev-layout.slide-sandbox .live-claim {
  color: rgba(255, 255, 255, 0.82);
}

.live-grid-2,
.live-grid-3,
.live-grid-4 {
  display: grid;
  gap: 20px;
}

.live-grid-2 {
  grid-template-columns: repeat(2, 1fr);
}

.live-grid-3 {
  grid-template-columns: repeat(3, 1fr);
}

.live-grid-4 {
  grid-template-columns: repeat(4, 1fr);
}

.live-card {
  min-width: 0;
  padding: 22px 24px;
  border: 1px solid rgba(13, 16, 48, 0.18);
  border-radius: 14px;
  background: #ffffff;
  box-shadow: var(--phl-card-shadow);
}

.live-card.is-cyan {
  border-color: rgba(12, 127, 163, 0.55);
  background: rgba(22, 180, 226, 0.06);
}

.live-card.is-magenta {
  border-color: rgba(164, 35, 143, 0.5);
  background: rgba(164, 35, 143, 0.06);
}

.live-card.is-dark {
  border-color: transparent;
  background: var(--phl-bg-navy);
  color: #ffffff;
}

.live-card.is-glass {
  border-color: rgba(255, 255, 255, 0.28);
  background: rgba(10, 7, 16, 0.72);
  color: #ffffff;
  box-shadow: none;
}

.live-num,
.live-kicker {
  font-size: 12px;
  line-height: 1.2;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--phl-cyan-ink);
}

.is-dark .live-num,
.is-dark .live-kicker,
.is-glass .live-num,
.is-glass .live-kicker {
  color: var(--phl-cyan-soft);
}

.live-card h3 {
  margin: 10px 0 8px;
  font-size: 23px;
  line-height: 1.12;
  font-weight: 500;
  letter-spacing: -0.02em;
}

.live-card p {
  margin: 0;
  font-size: 17px;
  line-height: 1.38;
  color: var(--phl-ink-body);
}

.live-card.is-dark p,
.live-card.is-glass p {
  color: rgba(255, 255, 255, 0.75);
}

.live-panel {
  padding: 30px;
  border-radius: 18px;
  background: var(--phl-bg-navy);
  color: #ffffff;
}

.live-panel-title {
  margin-bottom: 22px;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--phl-cyan-soft);
}

.live-word-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.live-word {
  padding: 18px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.09);
  font-size: 20px;
  font-weight: 500;
}

.live-panel-result {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.16);
  font-size: 21px;
  color: #ffffff;
}

.live-compression {
  margin-top: 36px;
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 16px;
  align-items: center;
  font-size: 16px;
  color: var(--phl-ink-muted);
}

.live-compression-line {
  position: relative;
  height: 2px;
  background: linear-gradient(90deg, var(--phl-cyan) 0%, var(--phl-magenta) 100%);
}

.live-compression-line::after {
  content: '';
  position: absolute;
  top: -5px;
  right: -1px;
  border-top: 6px solid transparent;
  border-bottom: 6px solid transparent;
  border-left: 10px solid var(--phl-magenta);
}

.live-outcome {
  margin-top: 22px;
  padding: 18px 24px;
  border: 1px solid rgba(164, 35, 143, 0.42);
  border-radius: 999px;
  background: rgba(164, 35, 143, 0.06);
  text-align: center;
  font-size: 21px;
  font-weight: 500;
  color: var(--phl-ink);
}

.slide-sandbox .live-outcome {
  border-color: rgba(245, 158, 245, 0.5);
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
}

.live-rail {
  margin-top: 20px;
  padding: 17px 24px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.09);
  text-align: center;
  font-size: 18px;
  font-weight: 500;
  word-spacing: 10px;
  color: #ffffff;
}

.live-versus {
  display: grid;
  grid-template-columns: 1fr 52px 1fr;
  gap: 18px;
  align-items: stretch;
}

.live-versus-mark {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 30px;
  color: var(--phl-ink-muted);
}

.live-list {
  margin: 18px 0 0;
  padding: 18px 0 0;
  border-top: 1px solid rgba(13, 16, 48, 0.12);
  display: flex;
  flex-direction: column;
  gap: 12px;
  font-size: 17px;
  color: var(--phl-ink);
}

.is-dark .live-list {
  border-color: rgba(255, 255, 255, 0.16);
  color: #ffffff;
}

.live-pipeline {
  display: grid;
  grid-template-columns: 1fr 42px 1fr 42px 1fr 42px 1fr;
  gap: 10px;
  align-items: center;
}

.live-arrow {
  text-align: center;
  font-size: 30px;
  color: var(--phl-magenta);
}

.live-pipeline .live-card {
  padding: 20px;
  min-height: 160px;
}

.live-pipeline .live-card h3 {
  font-size: 20px;
}

.live-host {
  padding: 24px 30px;
  border: 2px solid var(--phl-bg-navy);
  border-radius: 18px;
  background: rgba(13, 16, 48, 0.025);
}

.live-host-head {
  display: flex;
  justify-content: space-between;
  margin-bottom: 16px;
}

.live-component {
  padding: 18px 24px;
  border-radius: 12px;
  background: var(--phl-bg-navy);
  text-align: center;
  color: #ffffff;
}

.live-component strong {
  display: block;
  margin-top: 4px;
  font-size: 23px;
  font-weight: 500;
}

.live-capability {
  width: fit-content;
  margin: 14px auto;
  padding: 8px 22px;
  border: 1px solid rgba(164, 35, 143, 0.5);
  border-radius: 999px;
  background: rgba(164, 35, 143, 0.08);
  font-size: 15px;
  font-weight: 600;
  color: #a4238f;
}

.live-host .live-card {
  padding: 16px 20px;
  text-align: center;
}

.live-converge {
  margin: 18px auto;
  text-align: center;
  font-size: 28px;
  line-height: 1;
  color: var(--phl-magenta);
}

.live-route {
  display: grid;
  grid-template-columns: 1fr auto 1fr auto 1fr;
  gap: 14px;
  align-items: center;
}

.live-route-step {
  padding: 20px;
  border-radius: 12px;
  background: var(--phl-bg-navy);
  text-align: center;
  font-size: 19px;
  font-weight: 500;
  color: #ffffff;
}

.live-stat-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 22px;
  align-items: stretch;
}

.live-stat {
  padding: 26px;
  border: 1px solid rgba(13, 16, 48, 0.18);
  border-radius: 16px;
  text-align: center;
  background: #ffffff;
}

.live-stat.is-latest {
  border-color: transparent;
  background: var(--phl-brand-gradient);
  color: #ffffff;
}

.live-stat-value {
  font-size: 52px;
  line-height: 1;
  font-weight: 500;
  letter-spacing: -0.04em;
}

.live-stat-label {
  margin-top: 12px;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.live-stat-note {
  margin-top: 8px;
  font-size: 16px;
  color: var(--phl-ink-muted);
}

.is-latest .live-stat-note {
  color: rgba(255, 255, 255, 0.78);
}

.live-repairs {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.live-repair {
  display: grid;
  grid-template-columns: 1fr 54px 1.25fr;
  gap: 18px;
  align-items: center;
  padding: 17px 22px;
  border: 1px solid rgba(13, 16, 48, 0.14);
  border-radius: 12px;
  background: #ffffff;
  font-size: 18px;
  color: var(--phl-ink);
}

.live-repair-arrow {
  text-align: center;
  color: var(--phl-cyan-ink);
}

.live-repair strong {
  font-weight: 500;
  color: var(--phl-cyan-ink);
}

.live-footnote {
  margin-top: 16px;
  font-size: 13px;
  line-height: 1.35;
  color: var(--phl-ink-muted);
}
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
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Set rules · Bound how work is created</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Record intent · Preserve what must remain true</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Constrain code · Grant nothing by default</span></div>
</div>

<!--
Opening:
- Code generation is no longer the scarce part of software delivery.
- Accountability has not moved to the model. The organisation still owns the outcome, access, evidence, and consequences.
- Our answer is a delivery system whose rules, record, and runtime boundary sit outside the model.
-->

---
title: Implementation changed. Responsibility did not.
sync: shift
class: slide-light
---

<Eyebrow text="THE SHIFT · AI MOVES THE DELIVERY BOTTLENECK" />

<div class="live-split">
<div class="live-copy">

# Implementation changed.<br>Responsibility did not.

<p class="live-claim">AI compresses the work of turning an idea into plausible code.</p>

<div class="live-compression">
<span>Weeks</span>
<div class="live-compression-line"></div>
<span>Minutes</span>
</div>
</div>

<div class="live-panel">
<div class="live-panel-title">Responsibility remains</div>
<div class="live-word-grid">
<div class="live-word">Direction</div>
<div class="live-word">Authority</div>
<div class="live-word">Evidence</div>
<div class="live-word">Ownership</div>
</div>
<div class="live-panel-result">The bottleneck is an accepted change, safely in production.</div>
</div>
</div>

<!--
The model has compressed implementation effort, not organisational responsibility.
Plausibility is abundant too. More plausible output increases the need for clear authority and independent evidence.

Client question:
- Which of direction, authority, evidence, or ownership currently slows your changes most?
-->

---
title: Begin with the change
sync: outcome
class: slide-light
---

<Eyebrow text="THE STARTING POINT · OUTCOME BEFORE TECHNOLOGY" />

<div class="live-stage">

# Begin with the change.

<div class="live-grid-4">
<div class="live-card is-cyan">
<div class="live-num">01 · Benefit</div>
<h3>Who benefits?</h3>
</div>
<div class="live-card">
<div class="live-num">02 · Outcome</div>
<h3>What changes?</h3>
</div>
<div class="live-card">
<div class="live-num">03 · Evidence</div>
<h3>How will we know?</h3>
</div>
<div class="live-card is-magenta">
<div class="live-num">04 · Authority</div>
<h3>Who decides?</h3>
</div>
</div>

<div class="live-outcome">One useful production outcome—small enough to learn, meaningful enough to matter.</div>
</div>

<!--
We do not begin with “Where can we use AI?” We begin with a result that should be easier, safer, or faster.
This is enough direction to stop fast implementation from hardening a weak assumption, not a giant up-front specification.

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

<div class="live-stage">

# Set rules. Record intent. Constrain code.

<p class="live-claim">The model works inside the delivery system. It does not define the system.</p>

<div class="live-grid-3" style="margin-top: 24px;">
<div class="live-card is-glass">
<div class="live-num">01 · Toolchain controls</div>
<h3>Set rules</h3>
<p>How work may be created.</p>
</div>
<div class="live-card is-glass">
<div class="live-num">02 · Spec-driven delivery</div>
<h3>Record intent</h3>
<p>What must remain true.</p>
</div>
<div class="live-card is-glass">
<div class="live-num">03 · Sandboxed execution</div>
<h3>Constrain code</h3>
<p>What the result may touch.</p>
</div>
</div>

<div class="live-rail">Build → Test → Provision → Deploy → Release → Observe → Recover</div>
</div>

<!--
This is the map for the next three sections. Take each control in turn.
These are different controls over different questions.
- Toolchain controls govern how people and agents may work.
- Spec-driven delivery governs what the software must do and what evidence supports accepting it.
- Sandboxed execution governs what the running application may touch.
- Reusable delivery assets carry the result into operation.
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

<div class="live-stage">

# Isolate. Standardise. Automate.

<div class="live-grid-3">
<div class="live-card is-cyan">
<div class="live-kicker">01 · Isolate</div>
<h3>Governed workspaces</h3>
<p>Non-production environments for people and agents.</p>
</div>

<div class="live-card">
<div class="live-kicker">02 · Standardise</div>
<h3>Shared engineering codex</h3>
<p>Coding standards, approved patterns, examples, and templates—written for people and agents.</p>
</div>

<div class="live-card is-dark">
<div class="live-kicker">03 · Automate</div>
<h3>Compiler-enforced standards</h3>
<p>Critical coding rules are checked automatically, alongside security, dependency, quality, and test controls.</p>
</div>
</div>

<div class="live-outcome">Faster experimentation · Consistent controls · Auditable delivery</div>
</div>

<!--
Set rules has three practical parts.
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

<div class="live-stage">

# One safe path from build to trial.

<p class="live-claim">A versioned engineering codex, approved tools, policies, and deployment patterns follow every change.</p>

<div class="live-pipeline" style="margin-top: 20px;">
<div class="live-card is-cyan">
<div class="live-kicker">01 · Create</div>
<h3>AI-assisted build</h3>
<p>Governed workspace</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card">
<div class="live-kicker">02 · Record</div>
<h3>Enterprise source control</h3>
<p>Traceable change</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-dark">
<div class="live-kicker">03 · Assure</div>
<h3>Automated gates</h3>
<p>Compile-time rules · secrets · dependencies · tests</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-magenta">
<div class="live-kicker">04 · Trial</div>
<h3>Controlled non-production</h3>
<p>Access and review</p>
</div>
</div>

<div class="live-outcome">The codex guides people and agents. Gates enforce what cannot be optional.</div>
</div>

<!--
This is the operating path, not a prescribed vendor stack.
- The codex is human-readable and agent-usable: one reviewed source for standards, patterns, examples, and templates.
- The workspace carries the codex and approved tools into each change.
- Critical coding standards are enforced as the code compiles; mandatory controls also appear at source control, integration, and deployment boundaries.
- A change reaches a controlled trial environment before anyone considers production.

Client question:
- Which step in this path is currently missing, optional, or different for every team?
-->

---
title: Chat is not the record
sync: record
class: slide-light
---

<Eyebrow text="02 · RECORD INTENT · SPEC-DRIVEN DELIVERY" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span class="is-active is-magenta">02 · Record intent</span>
<span>03 · Constrain code</span>
</div>

<div class="live-stage">

# Chat is not the record.

<div class="live-pipeline">
<div class="live-card">
<div class="live-kicker">Sources</div>
<h3>Evidence</h3>
<p>Intent · docs · tests · code</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-dark">
<div class="live-kicker">Policy</div>
<h3>Authority</h3>
<p>Agreement · divergence · conflict</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-magenta">
<div class="live-kicker">Sealed</div>
<h3>Revision</h3>
<p>Specification · design · plan</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-cyan">
<div class="live-kicker">Bounded</div>
<h3>Build</h3>
<p>Isolate · integrate · verify</p>
</div>
</div>

<div class="live-outcome">Swap the model. The evidence, authority, and reviewed intent remain.</div>
</div>

<!--
A spec-driven delivery process sits around coding agents, not in place of them.
Sources stay distinct and claims stay anchored. Authority is explicit. Equal authority can remain a conflict.
A person seals the revision; the sealed revision, not the conversation, directs implementation and verification.

Client question:
- If the model, vendor, and chat disappeared tomorrow, where would accepted intent remain?
-->

---
title: Generated code gets no ambient authority
sync: runtime
class: slide-light
---

<Eyebrow text="03 · CONSTRAIN CODE · SANDBOXED EXECUTION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Set rules</span>
<span>02 · Record intent</span>
<span class="is-active">03 · Constrain code</span>
</div>

<div class="live-stage">

# Generated code gets no ambient authority.

<div class="live-host">
<div class="live-host-head">
<span class="live-kicker">WebAssembly runtime</span>
<span class="live-kicker">Rust on Wasmtime</span>
</div>

<div class="live-component">
Sandboxed WASI component
<strong>Differentiating business behaviour</strong>
</div>

<div class="live-capability">Typed capability contracts</div>

<div class="live-grid-3">
<div class="live-card"><h3>Data &amp; state</h3></div>
<div class="live-card"><h3>Flow &amp; calls</h3></div>
<div class="live-card"><h3>Trust &amp; insight</h3></div>
</div>
</div>

<div class="live-outcome">No grant means no reach.</div>
</div>

<!--
Application logic has no ambient filesystem, network, process, credential, or environment access.
The guest asks for a role such as state or messaging. The host chooses the backend, credentials, and policy.
This complements infrastructure security; it does not replace credential scope or network policy.

Client question:
- If generated code were compromised, what could it reach simply because the process environment can?
-->

---
title: One delivery system. Two starting points.
sync: entry
class: slide-light
---

<Eyebrow text="WHERE IT APPLIES · NEW SYSTEMS AND MODERNISATION" />

<div class="live-stage">

# One delivery system. Two starting points.

<div class="live-grid-2">
<div class="live-card is-cyan">
<div class="live-kicker">New system</div>
<h3>A useful idea or prototype</h3>
<p>Do not let a demo become accidental policy.</p>
</div>
<div class="live-card is-magenta">
<div class="live-kicker">Existing system</div>
<h3>Code nobody can safely change</h3>
<p>Recover the product, not just the code.</p>
</div>
</div>

<div class="live-converge">↓</div>

<div class="live-route">
<div class="live-route-step">Reviewed intent</div>
<div class="live-arrow">→</div>
<div class="live-route-step">Bounded delivery</div>
<div class="live-arrow">→</div>
<div class="live-route-step">Constrained operation</div>
</div>
</div>

<!--
New systems and modernisation have different starting risks.
Both converge on reviewed intent, bounded implementation, independent verification, and constrained operation.

Client question:
- Is the immediate pressure a new capability that cannot reach production, or an existing system that cannot safely change?
-->

---
title: Evidence must be able to say no
sync: proof-result
class: slide-light
---

<Eyebrow text="EVIDENCE · FIXED REPLAY SUITE, NOT A MODEL DEMO" />

<div class="live-stage">

# Evidence must be able to say no.

<div class="live-stat-grid">
<div class="live-stat">
<div class="live-stat-value">0 / 22</div>
<div class="live-stat-label">Early build</div>
<div class="live-stat-note">Plausible. Wrong.</div>
</div>
<div class="live-stat">
<div class="live-stat-value">7 / 22</div>
<div class="live-stat-label">System improved</div>
<div class="live-stat-note">Checks and guidance changed.</div>
</div>
<div class="live-stat is-latest">
<div class="live-stat-value">21 / 22</div>
<div class="live-stat-label">Latest build</div>
<div class="live-stat-note">12 / 12 production messages.</div>
</div>
</div>

<div class="live-outcome">One case remains open—and reported.</div>
<div class="live-footnote">Propellerhead internal evaluation, October 2026. Same fixed 22-case suite as the hand-written counterpart.</div>
</div>

<!--
We recovered a specification from an anonymised real-time rail service and rebuilt it as a WASI component.
The early output looked plausible line by line. The recorded cases showed it was not correct.
The open case remains visible because evidence is useful only when it can still say no.
-->

---
title: Fix the system, not the output
sync: proof-learning
class: slide-light
---

<Eyebrow text="COMPOUNDING DELIVERY · EVERY FAILURE IMPROVES THE NEXT BUILD" />

<div class="live-stage">

# Fix the system, not the output.

<div class="live-repairs">
<div class="live-repair">
<span>A consumer never registered</span>
<span class="live-repair-arrow">→</span>
<strong>Refuse unreachable publishers</strong>
</div>
<div class="live-repair">
<span>A rejection looked processed</span>
<span class="live-repair-arrow">→</span>
<strong>Refuse swallowed errors</strong>
</div>
<div class="live-repair">
<span>A type was laid out empty</span>
<span class="live-repair-arrow">→</span>
<strong>Repair the design-type reader</strong>
</div>
</div>

<div class="live-outcome">No defect was hand-patched. Every later build inherits the fix.</div>
</div>

<!--
Each defect became a reusable capability in the delivery system: a check, better extraction, clearer guidance, or repaired tooling.
The score is not the product claim. The compounding delivery system is.
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
<p>Useful, consequential, and harder to deliver than it should be.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">02</span>
<div>
<h4>Trace its full delivery path</h4>
<p>Direction, authority, evidence, runtime reach, release, and ownership.</p>
</div>
</div>
<div class="opportunity-row">
<span class="opportunity-row-num">03</span>
<div>
<h4>Leave with assets and a decision</h4>
<p>Deliver, repair a foundation, reframe the work—or stop.</p>
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
- Readiness establishes the first useful boundary and ends in an explicit decision.
- Sometimes the responsible answer is not to build yet.

Final question:
- What should be easier, safer, or faster than it is today?
-->
