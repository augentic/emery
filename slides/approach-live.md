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

.live-pipeline.is-three {
  grid-template-columns: 1fr 42px 1fr 42px 1fr;
}

.live-sdd-flow {
  display: grid;
  grid-template-columns: 1.15fr 92px 0.85fr 24px 1fr;
  gap: 8px;
  align-items: stretch;
}

.live-sdd-flow .live-card {
  min-height: 176px;
  padding: 18px;
}

.live-sdd-flow .live-card h3 {
  margin-top: 8px;
  font-size: 20px;
}

.live-sdd-flow .live-card p {
  font-size: 15px;
}

.live-sdd-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 26px;
  color: var(--phl-magenta);
}

.live-review-gate {
  padding: 14px 8px;
  border-right: 2px solid rgba(164, 35, 143, 0.52);
  border-left: 2px solid rgba(164, 35, 143, 0.52);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  font-size: 11px;
  line-height: 1.25;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #a4238f;
}

.live-review-gate strong {
  margin-bottom: 6px;
  font-size: 15px;
  line-height: 1.15;
  letter-spacing: 0;
  text-transform: none;
  color: var(--phl-ink);
}

.live-title-accent {
  color: #a4238f;
}

.live-durability {
  display: grid;
  grid-template-columns: 0.72fr 1.28fr;
  gap: 28px;
  align-items: stretch;
}

.live-disposable-card,
.live-durable-card {
  min-height: 188px;
  padding: 22px;
  border-radius: 16px;
}

.live-disposable-card {
  border: 1.5px dashed rgba(13, 16, 48, 0.34);
  background: rgba(13, 16, 48, 0.025);
}

.live-durable-card {
  border: 2px solid rgba(164, 35, 143, 0.72);
  background: var(--phl-bg-navy);
  box-shadow: 0 18px 44px rgba(13, 16, 48, 0.16);
  color: #ffffff;
}

.live-durability h3 {
  margin: 8px 0 16px;
  font-size: 22px;
  line-height: 1.15;
}

.live-code-word {
  margin-top: 20px;
  text-align: center;
  font-size: 48px;
  line-height: 1;
  font-weight: 600;
  letter-spacing: -0.025em;
  color: var(--phl-ink);
}

.live-code-caption {
  margin-top: 22px;
  text-align: center;
  font-size: 15px;
  color: var(--phl-ink-muted);
}

.live-file-row {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 16px;
  align-items: baseline;
  padding: 13px 15px;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.08);
}

.live-file-row + .live-file-row {
  margin-top: 10px;
}

.live-file-row strong {
  font-size: 18px;
  color: #ffffff;
}

.live-file-row span {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.72);
}

.live-outcome.is-punch {
  border-color: #a4238f;
  background: var(--phl-bg-navy);
  box-shadow: 0 14px 36px rgba(13, 16, 48, 0.14);
  font-weight: 600;
  color: #ffffff;
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

.live-matrix {
  width: min(100%, 1000px);
  margin: 14px auto 0;
  display: grid;
  grid-template-columns: 138px 1fr 138px;
  grid-template-rows: 38px 300px 38px;
  align-items: center;
}

.live-matrix-axis {
  justify-self: center;
  padding: 8px 18px;
  border: 1px solid rgba(13, 16, 48, 0.24);
  border-radius: 999px;
  background: #ffffff;
  font-size: 12px;
  line-height: 1;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--phl-ink);
  white-space: nowrap;
}

.live-matrix-axis.is-top {
  grid-column: 2;
  grid-row: 1;
  border-color: rgba(164, 35, 143, 0.48);
  background: rgba(164, 35, 143, 0.06);
  color: #a4238f;
}

.live-matrix-axis.is-bottom {
  grid-column: 2;
  grid-row: 3;
}

.live-matrix-axis.is-left {
  grid-column: 1;
  grid-row: 2;
}

.live-matrix-axis.is-right {
  grid-column: 3;
  grid-row: 2;
  border-color: rgba(12, 127, 163, 0.48);
  background: rgba(22, 180, 226, 0.06);
  color: var(--phl-cyan-ink);
}

.live-matrix-grid {
  grid-column: 2;
  grid-row: 2;
  height: 100%;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  grid-template-rows: repeat(2, 1fr);
  border: 1px solid rgba(13, 16, 48, 0.34);
  border-radius: 14px;
  overflow: hidden;
  background: #ffffff;
  box-shadow: var(--phl-card-shadow);
}

.live-quadrant {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

.live-quadrant:nth-child(odd) {
  border-right: 1px solid rgba(13, 16, 48, 0.24);
}

.live-quadrant:nth-child(-n + 2) {
  border-bottom: 1px solid rgba(13, 16, 48, 0.24);
}

.live-quadrant:nth-child(n + 3) {
  background: rgba(13, 16, 48, 0.025);
}

.live-quadrant.is-target {
  background: rgba(164, 35, 143, 0.07);
  box-shadow: inset 0 0 0 2px rgba(164, 35, 143, 0.44);
}

.live-quadrant h3 {
  margin: 0;
  font-size: 25px;
  line-height: 1.14;
  font-weight: 500;
  letter-spacing: -0.02em;
}

.live-quadrant.is-target h3 {
  color: #a4238f;
}

.live-quadrant p {
  margin: 8px 0 0;
  font-size: 14px;
  line-height: 1.3;
  color: var(--phl-ink-muted);
}

.live-runtime-stack {
  width: 80%;
  margin: 34px auto 0;
  display: flex;
  flex-direction: column;
  gap: 11px;
  transform: translateY(12px);
}

.live-wasm-runtime {
  padding: 14px 18px 13px;
  border: 2px solid rgba(65, 145, 203, 0.72);
  border-radius: 12px;
  background: rgba(65, 145, 203, 0.14);
}

.live-wasi-components {
  width: 86%;
  margin: 0 auto;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 42px;
}

.live-wasi-component {
  padding: 11px 12px 10px;
  border: 2px solid rgba(124, 92, 205, 0.64);
  border-radius: 9px;
  background: rgba(124, 92, 205, 0.12);
  text-align: center;
  color: var(--phl-ink);
}

.live-wasi-component strong,
.live-wasi-component em,
.live-runtime-name strong,
.live-runtime-name em,
.live-infrastructure strong,
.live-infrastructure em {
  display: block;
}

.live-wasi-component strong {
  font-size: 15px;
  font-weight: 500;
}

.live-wasi-component em {
  margin-top: 2px;
  font-size: 13px;
  color: var(--phl-ink-body);
}

.live-runtime-name {
  margin-top: 11px;
  text-align: center;
}

.live-runtime-name strong {
  font-size: 20px;
  line-height: 1.05;
  font-weight: 550;
}

.live-runtime-name em {
  margin-top: 2px;
  font-size: 13px;
  color: var(--phl-ink-body);
}

.live-capability-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
}

.live-runtime-capability {
  padding: 10px 7px;
  border: 1.8px solid rgba(90, 90, 82, 0.28);
  border-radius: 8px;
  background: rgba(248, 248, 245, 0.94);
  text-align: center;
  font-size: 13px;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-infrastructure {
  padding: 10px 13px 9px;
  border: 2px solid rgba(13, 16, 48, 0.48);
  border-radius: 9px;
  background: #ffffff;
  text-align: center;
  color: var(--phl-ink);
}

.live-infrastructure strong {
  font-size: 17px;
  font-weight: 500;
}

.live-infrastructure em {
  margin-top: 1px;
  font-size: 12px;
  color: var(--phl-ink-body);
}

.live-portability {
  width: 92%;
  margin: 24px auto 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.live-portable-app {
  width: 42%;
  margin: 0 auto;
  padding: 9px 16px;
  border: 2px solid rgba(124, 92, 205, 0.64);
  border-radius: 10px;
  background: rgba(124, 92, 205, 0.12);
  text-align: center;
}

.live-portable-app .live-kicker {
  color: #7557c7;
}

.live-portable-app strong {
  display: block;
  margin-top: 3px;
  font-size: 18px;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-portable-link {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 12px;
  align-items: center;
  font-size: 11px;
  line-height: 1;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--phl-ink-muted);
}

.live-portable-link::before,
.live-portable-link::after {
  content: '';
  height: 1px;
  background: rgba(13, 16, 48, 0.18);
}

.live-portability .live-capability-row {
  width: 80%;
  margin: 0 auto;
}

.live-recipe-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.live-recipe-card {
  padding: 12px 12px 13px;
  border: 1.5px solid rgba(13, 16, 48, 0.2);
  border-radius: 11px;
  background: #ffffff;
}

.live-recipe-card:nth-child(2) {
  border-color: rgba(12, 127, 163, 0.48);
  background: rgba(22, 180, 226, 0.05);
}

.live-recipe-card:nth-child(3) {
  border-color: rgba(164, 35, 143, 0.42);
  background: rgba(164, 35, 143, 0.045);
}

.live-recipe-card:first-child .live-kicker {
  color: var(--phl-ink-muted);
}

.live-recipe-card:nth-child(3) .live-kicker {
  color: #a4238f;
}

.live-recipe-card h3 {
  margin: 4px 0 9px;
  font-size: 16px;
  line-height: 1.12;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-recipe-template {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 5px;
}

.live-recipe-template span {
  padding: 6px 4px;
  border: 1px solid rgba(13, 16, 48, 0.12);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.78);
  text-align: center;
  font-size: 10px;
  line-height: 1.15;
  color: var(--phl-ink-body);
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

<div class="live-stage">

# AI coding moves right. Governance moves up.

<div class="live-matrix">
<div class="live-matrix-axis is-top">Governed</div>
<div class="live-matrix-axis is-left">Manual coding</div>
<div class="live-matrix-axis is-right">AI-assisted coding</div>
<div class="live-matrix-axis is-bottom">Ungoverned</div>
<div class="live-matrix-grid">
<div class="live-quadrant">
<h3>Traditional SDLC</h3>
<p>Governance around human implementation</p>
</div>
<div class="live-quadrant is-target">
<h3>Spec-driven delivery</h3>
<p>Durable intent around AI implementation</p>
</div>
<div class="live-quadrant">
<h3>Ad hoc development</h3>
<p>Local choices and human memory</p>
</div>
<div class="live-quadrant">
<h3>AI-assisted coding alone</h3>
<p>Conversation-led generation</p>
</div>
</div>
</div>

<div class="live-outcome">AI coding changes implementation. It does not create governance.</div>
</div>

<!--
Most AI adoption begins as a horizontal move: introduce an assistant or agent and produce code faster.
That does not create authority, an audit trail, or a durable statement of what must remain true.
Spec-driven delivery is the separate upward move. It puts governed intent around AI-assisted implementation.

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

<div class="live-stage">

# The specification drives development.

<p class="live-claim">The specification stays in the development loop—from requirements and design through implementation and verification.</p>

<div class="live-sdd-flow" style="margin-top: 22px;">
<div class="live-card is-cyan">
<div class="live-kicker">01 · What + how</div>
<h3>Specify</h3>
<p>Intent · docs · tests · legacy source code<br>→ spec.md · design.md</p>
</div>
<div class="live-review-gate">
<strong>Review &amp;<br>seal</strong>
Versioned in repo
</div>
<div class="live-card is-magenta">
<div class="live-kicker">02 · Derive</div>
<h3>Plan</h3>
<p>Ephemeral work and dependencies</p>
</div>
<div class="live-sdd-arrow">→</div>
<div class="live-card is-dark">
<div class="live-kicker">03 · Build + prove</div>
<h3>Implement</h3>
<p>Build · integrate · verify against the specification</p>
</div>
</div>

<div class="live-outcome">The plan is disposable. The reviewed specification is not.</div>
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

<div class="live-stage">

# Code is disposable.<br><span class="live-title-accent">The specification is durable.</span>

<p class="live-claim">The implementation can be regenerated. The reviewed specification preserves what the system must do and how it is designed.</p>

<div class="live-durability" style="margin-top: 20px;">
<div class="live-disposable-card">
<div class="live-kicker">Disposable implementation</div>
<div class="live-code-word">Code</div>
<div class="live-code-caption">Rebuild from the specification</div>
</div>

<div class="live-durable-card">
<div class="live-kicker">Versioned · reviewed · in the repository</div>
<h3>Durable specification</h3>
<div class="live-file-row"><strong>spec.md</strong><span>Requirements and scenarios</span></div>
<div class="live-file-row"><strong>design.md</strong><span>Architecture and decisions</span></div>
</div>
</div>

<div class="live-outcome is-punch">Regenerate the plan. Rebuild the code. The specification remains.</div>
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
title: Give generated code less freedom to surprise you.
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

# Give generated code less freedom<br>to surprise you.

<p class="live-claim">Code reaches infrastructure only through capabilities supplied by the host.</p>

<div class="live-runtime-stack">
<div class="live-wasm-runtime">
<div class="live-wasi-components">
<div class="live-wasi-component">
<strong>WASI Component</strong>
<em>Generated code</em>
</div>
<div class="live-wasi-component">
<strong>WASI Component</strong>
<em>Generated code</em>
</div>
</div>
<div class="live-runtime-name">
<strong>Wasm Runtime</strong>
<em>Wasmtime sandbox</em>
</div>
</div>

<div class="live-capability-row">
<div class="live-runtime-capability">HTTP</div>
<div class="live-runtime-capability">Messaging</div>
<div class="live-runtime-capability">KeyValue</div>
<div class="live-runtime-capability">BlobStore</div>
</div>

<div class="live-infrastructure">
<strong>Infrastructure</strong>
<em>Cloud/On-premise</em>
</div>
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

<div class="live-stage">

# Different infrastructure.<br>The same application.

<p class="live-claim">The component depends on capabilities, not products. A runtime recipe binds them to approved services in each environment.</p>

<div class="live-portability">
<div class="live-portable-app">
<div class="live-kicker">Application unchanged</div>
<strong>WASI Component</strong>
</div>

<div class="live-portable-link">Stable capability contract</div>

<div class="live-capability-row">
<div class="live-runtime-capability">HTTP</div>
<div class="live-runtime-capability">Messaging</div>
<div class="live-runtime-capability">KeyValue</div>
<div class="live-runtime-capability">BlobStore</div>
</div>

<div class="live-portable-link">Host-selected runtime recipe</div>

<div class="live-recipe-grid">
<div class="live-recipe-card">
<div class="live-kicker">Local / trial</div>
<h3>In-memory + local services</h3>
<div class="live-recipe-template">
<span>Runtime</span>
<span>Capability bindings</span>
<span>Policy + identity</span>
<span>Config + observability</span>
</div>
</div>
<div class="live-recipe-card">
<div class="live-kicker">Cloud</div>
<h3>Managed services</h3>
<div class="live-recipe-template">
<span>Runtime</span>
<span>Capability bindings</span>
<span>Policy + identity</span>
<span>Config + observability</span>
</div>
</div>
<div class="live-recipe-card">
<div class="live-kicker">On-premise</div>
<h3>Approved internal services</h3>
<div class="live-recipe-template">
<span>Runtime</span>
<span>Capability bindings</span>
<span>Policy + identity</span>
<span>Config + observability</span>
</div>
</div>
</div>
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
