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
  right: 152px;
  z-index: 2;
  width: 530px;
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
  letter-spacing: 0.06em;
  text-transform: uppercase;
  white-space: nowrap;
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

.live-card-head {
  min-height: 38px;
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  gap: 11px;
}

.live-card-head + h3 {
  margin-top: 0;
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
  padding: 14px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.09);
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.live-word-copy {
  min-width: 0;
}

.live-word-copy strong {
  display: block;
  font-size: 17px;
  line-height: 1.15;
  font-weight: 500;
}

.live-word-copy span {
  display: block;
  margin-top: 4px;
  font-size: 11.5px;
  line-height: 1.28;
  color: rgba(255, 255, 255, 0.68);
}

.live-panel-result {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.16);
  font-size: 21px;
  color: #ffffff;
}

.live-panel-result-label {
  display: block;
  margin-bottom: 5px;
  font-size: 11px;
  line-height: 1;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--phl-cyan-soft);
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
  border: 1px solid rgba(12, 127, 163, 0.35);
  background: rgba(22, 180, 226, 0.06);
  text-align: center;
  font-size: 18px;
  font-weight: 500;
  word-spacing: 10px;
  color: var(--phl-ink);
}

.slide-sandbox .live-rail {
  border-color: transparent;
  background: rgba(255, 255, 255, 0.09);
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

.live-feature-flow {
  margin-top: 22px;
  display: grid;
  grid-template-columns: 1.15fr 42px 1fr 42px 0.9fr;
  gap: 10px;
  align-items: stretch;
}

.live-feature-flow .live-card {
  min-height: 190px;
  padding: 20px;
}

.live-feature-flow .live-card h3 {
  font-size: 20px;
}

.live-feature-list {
  margin-top: 14px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.live-feature-list span {
  padding: 9px 10px;
  border: 1px solid rgba(13, 16, 48, 0.12);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.72);
  font-size: 12.5px;
  line-height: 1.2;
  color: var(--phl-ink-body);
}

.live-card.is-dark .live-feature-list span {
  border-color: rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.82);
}

.live-sdd-flow {
  width: min(100%, 1000px);
  margin-inline: auto;
  display: grid;
  grid-template-columns: 1.15fr 92px 0.55fr 24px 1fr;
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
  width: min(100%, 1000px);
  margin-inline: auto;
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
  margin: 12px auto 0;
  display: grid;
  grid-template-columns: 138px 1fr 138px;
  grid-template-rows: 36px 268px 36px;
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
  width: 72%;
  margin: 34px auto 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transform: translateY(12px);
}

.live-wasm-runtime {
  padding: 18px 20px 16px;
  border: 2px solid rgba(65, 145, 203, 0.62);
  border-radius: 12px;
  background: rgba(65, 145, 203, 0.08);
}

.live-wasi-components {
  width: 86%;
  margin: 0 auto;
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 42px;
}

.live-wasi-component {
  padding: 15px 12px 14px;
  border: 2px solid rgba(124, 92, 205, 0.64);
  border-radius: 9px;
  background: rgba(124, 92, 205, 0.1);
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
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  text-align: center;
  font-size: 13px;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-runtime-stack .live-runtime-capability {
  padding-block: 13px;
}

.live-capability-icon {
  width: 18px;
  height: 18px;
  color: var(--phl-cyan-ink);
}

.live-kicker-icon {
  width: 18px;
  height: 18px;
  margin-right: 7px;
  vertical-align: -4px;
}

.live-infrastructure {
  padding: 13px 13px 12px;
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

.live-trust-model {
  width: min(92%, 1020px);
  margin: 42px auto 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 28px;
}

.live-trust-state {
  display: grid;
  grid-template-rows: 39px 54px 60px 54px;
  overflow: hidden;
  border: 1.5px solid rgba(13, 16, 48, 0.22);
  border-radius: 12px;
  background: #ffffff;
}

.live-trust-state.is-explore {
  border-style: dashed;
}

.live-trust-state.is-govern {
  border-color: rgba(12, 127, 163, 0.52);
  background: rgba(22, 180, 226, 0.035);
}

.live-trust-state.is-connect {
  border-color: rgba(96, 93, 213, 0.5);
  background: rgba(96, 93, 213, 0.045);
}

.live-trust-state.is-support {
  border-color: rgba(164, 35, 143, 0.48);
  background: rgba(164, 35, 143, 0.035);
}

.live-trust-state-head,
.live-trust-band {
  padding: 10px 13px;
}

.live-trust-state-head {
  border-bottom: 1px solid rgba(13, 16, 48, 0.1);
  font-size: 11px;
  line-height: 1;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--phl-ink-muted);
}

.is-govern .live-trust-state-head {
  color: #0c7fa3;
}

.is-connect .live-trust-state-head {
  color: #605dd5;
}

.is-support .live-trust-state-head {
  color: #a4238f;
}

.live-trust-band {
  border-bottom: 1px solid rgba(13, 16, 48, 0.09);
}

.live-trust-band:last-child {
  border-bottom: 0;
}

.live-trust-band strong {
  display: block;
  font-size: 16px;
  line-height: 1.1;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-trust-band p {
  margin: 0;
  font-size: 13px;
  line-height: 1.35;
  color: var(--phl-ink-body);
}

.live-trust-band.is-boundary {
  background: rgba(255, 255, 255, 0.58);
}

.live-trust-band.is-boundary strong {
  font-size: 13px;
}

.live-trust-model-note {
  margin-top: 12px;
  text-align: center;
  font-size: 14px;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-radius-grid {
  width: min(92%, 1020px);
  margin: 32px auto 0;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 24px;
}

.live-radius-card {
  min-height: 120px;
  padding: 14px;
  border: 1.5px solid rgba(13, 16, 48, 0.22);
  border-radius: 12px;
  background: #ffffff;
}

.live-radius-card h3 {
  margin: 7px 0 8px;
  font-size: 18px;
  line-height: 1.12;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-radius-card p {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.35;
  color: var(--phl-ink-body);
}

.live-radius-scale {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  overflow: hidden;
  width: min(92%, 1020px);
  margin: 14px auto 0;
  border: 1px solid rgba(13, 16, 48, 0.18);
  border-radius: 10px;
}

.live-radius-scale div {
  padding: 10px 8px;
  border-right: 1px solid rgba(13, 16, 48, 0.12);
  text-align: center;
  font-size: 12px;
  color: var(--phl-ink);
}

.live-radius-scale div:last-child {
  border-right: 0;
}

.live-radius-scale div:nth-child(2) {
  background: rgba(22, 180, 226, 0.06);
}

.live-radius-scale div:nth-child(3) {
  background: rgba(96, 93, 213, 0.07);
}

.live-radius-scale div:nth-child(4) {
  background: rgba(164, 35, 143, 0.06);
}

.live-radius-scale strong {
  margin-right: 4px;
  font-weight: 700;
}

.live-loop {
  width: min(100%, 1040px);
  margin: 32px auto 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.live-loop-flow {
  display: grid;
  grid-template-columns: 1fr 1.15fr 1.45fr 1fr;
  gap: 10px;
  align-items: stretch;
}

.live-loop-flow .live-card {
  padding: 16px;
}

.live-loop-flow .live-card h3 {
  margin-top: 6px;
  font-size: 19px;
}

.live-loop-flow .live-card p {
  font-size: 13.5px;
}

.live-loop-lane {
  margin-top: 8px;
  padding: 8px 10px;
  border: 1px solid rgba(13, 16, 48, 0.16);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.65);
  font-size: 13px;
  font-weight: 500;
  color: var(--phl-ink);
}

.live-loop-feedback {
  padding: 13px 18px;
  border: 1.5px dashed rgba(164, 35, 143, 0.5);
  border-radius: 10px;
  background: rgba(164, 35, 143, 0.045);
  text-align: center;
  font-size: 14.5px;
  font-weight: 500;
  color: var(--phl-ink);
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

.section-kicker-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  border-radius: 999px;
  border: 1px solid rgba(120, 216, 242, 0.45);
  background: rgba(22, 180, 226, 0.12);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--phl-cyan-soft);
  margin-bottom: 6px;
}

.section-kicker-pill.is-magenta {
  border-color: rgba(245, 158, 245, 0.5);
  background: rgba(164, 35, 143, 0.2);
  color: #f59ef5;
}
</style>

<AmbientVideo src="/videos/hero-video-3.mp4" variant="hero" />

<Eyebrow text="PROPELLERHEAD · ACCOUNTABLE SOFTWARE DELIVERY" />

<div class="section-hero-grid">
<div class="section-hero-left">

# Code is cheap.<br>Delivery is hard.

<p class="section-subtitle">Use a delivery system that lets AI<br>deliver software — not just code.</p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Standardise delivery · One path from change to production</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Specify behaviour · Preserve what must remain true</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Sandbox execution · Limit what running code may access</span></div>
</div>

<!--
Opening:
- Generative AI makes code and test generation abundant. Trusted evidence remains scarce.
- Accountability has not moved to the model. The organisation still owns the outcome, access, evidence, and consequences.
- Our answer is a delivery system whose rules, record, and runtime boundary sit outside the model.
-->

---
title: Coding is no longer the bottleneck.
sync: shift
class: slide-light
---

<Eyebrow text="THE SHIFT · ABUNDANT CODE IS NOT A COHERENT SOLUTION" />

<div class="live-split">
<div class="live-copy">

# Coding is no longer<br>the bottleneck.

<p class="live-claim">AI can write code in minutes, but it cannot frame a problem, weigh trade-offs, or own consequences.</p>

<div class="live-compression">
<span>Weeks</span>
<div class="live-compression-line"></div>
<span>Minutes</span>
</div>
</div>

<div class="live-panel">
<div class="live-panel-title">More code ≠ a coherent solution</div>
<div class="live-word-grid">
<div class="live-word"><IconBadge tone="light" size="small"><ph:stack /></IconBadge><div class="live-word-copy"><strong>Cognitive load</strong><span>Overwhelms human review</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:puzzle-piece /></IconBadge><div class="live-word-copy"><strong>Hidden ambiguity</strong><span>Plausible code skips needed debate</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:eye-slash /></IconBadge><div class="live-word-copy"><strong>Lost comprehension</strong><span>No one understands it all</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:warning /></IconBadge><div class="live-word-copy"><strong>Accumulated slop</strong><span>Unmanageable over time</span></div></div>
</div>
<div class="live-panel-result"><span class="live-panel-result-label">The risk</span>An unmanageable application and codebase.</div>
</div>
</div>

<!--
Coding and test generation are no longer the bottlenecks they once were.
However, a lot of code does not equal a coherent, well-thought-through, useful business solution.
AI can write code in minutes, but it cannot frame a problem, weigh trade-offs, or own consequences.
Coding agents also smooth over ambiguity with plausible-looking code, skipping the design conversations the team needed to have.
Generated tests can repeat the implementation's assumptions. Trusted evidence still has to reject plausible but wrong code.
It can exacerbate cognitive load and overwhelm, and lead to an unmanageable application and codebase—no longer fully comprehensible and full of slop.
-->

---
title: Build the right thing. Build it to last.
sync: pivot
class: slide-light
---

<Eyebrow text="THE RESPONSE · STRATEGY AND DELIVERY" />

<div class="live-stage">

# Build the right thing.<br>Build it to last.

<p class="live-claim">Designing great software is a human process. While we can help with product design, <strong>our expertise is in delivering systems that last.</strong></p>

<div class="live-grid-2" style="width: 88%; margin: 28px auto 0; gap: 52px;">
<div class="live-card is-cyan" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="cyan"><ph:compass /></IconBadge>
<div class="live-kicker">01 · Good product strategy</div>
</div>
<h3>Building the right thing</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(12, 127, 163, 0.2);">Frame the problem · weigh trade-offs · test with prototypes</p>
</div>

<div class="live-card is-dark" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="light"><ph:gear-six /></IconBadge>
<div class="live-kicker">02 · A reliable delivery system</div>
</div>
<h3>Delivering maintainable software</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(255, 255, 255, 0.16);">Standardise delivery · specify behaviour · sandbox execution</p>
</div>
</div>

<div class="live-outcome is-punch" style="width: 88%; margin: 48px auto 0; box-sizing: border-box;">Decide what matters, deliver it safely, and keep it maintainable.</div>
</div>

<!--
Two things are increasingly important:
1) Good product strategy—framing the problem, weighing trade-offs, and using rapid prototypes to test assumptions without mistaking them for production systems. A prototype informs the decision; it does not make it.
2) A reliable delivery system—turning agreed intent into coherent, comprehensible, verified software without accumulating slop.
While we can help with both, our real expertise is in the second.
-->

---
title: A delivery system for AI-generated code.
sync: system
layout: default
class: slide-light
---

<Eyebrow text="THE DELIVERY SYSTEM · HUMAN-LED ENGINEERING" />

<div class="live-stage">

# A delivery system for AI-generated code.

<p class="live-claim">Human-led engineering — with agents working inside the system, not in charge of it.</p>

<div class="live-grid-3" style="margin-top: 24px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:shield-check /></IconBadge>
<div class="live-num">01 · Change path</div>
</div>
<h3>Standardise delivery</h3>
<p>How work is created, checked, shipped, and operated.</p>
</div>
<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:file-text /></IconBadge>
<div class="live-num">02 · Durable authority</div>
</div>
<h3>Specify behaviour</h3>
<p>What must remain true and how acceptance is proven.</p>
</div>
<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:lock-key /></IconBadge>
<div class="live-num">03 · Runtime boundary</div>
</div>
<h3>Sandbox execution</h3>
<p>What running code may access.</p>
</div>
</div>

<div class="live-rail">Specify → Build → Test → Analyse → Deploy → Observe → Recover</div>
</div>

<!--
This is the map for the next three sections.
- These three mechanisms keep engineering human-led: the model works inside the delivery path, while the authority to design and govern that path stays with the people outside it.
- Standardise delivery controls how a change moves from creation into operation.
- Specify behaviour controls what must remain true and what evidence accepts the result.
- Sandbox execution controls what the running application may access.
- Product judgement, acceptance, and release authority remain with people.
-->

---
title: Standardise delivery.
sync: section-standardise
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 01" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill">01 · Toolchain &amp; pipeline</div>

# Standardise delivery.

<p class="section-subtitle">One governed path from change to production—<strong>across how code is written and how it ships.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Isolate &amp; guide · Governed workspaces and shared engineering codex</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Enforce at compile time · Fail critical rules before review</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Automate promotion · Build once, test behaviour, and gate every change</span></div>
</div>

<!--
Section 01 divider: Standardise delivery.
-->

---
title: Standardise how code is written.
sync: authoring
class: slide-light
---

<Eyebrow text="01 · STANDARDISE DELIVERY · AUTHORING CONTROLS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Standardise how code is written.

<p class="live-claim">Give people and agents a safe workspace, shared standards, and compile-time guardrails.</p>

<div class="live-grid-3" style="margin-top: 22px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:shield-check /></IconBadge>
<div class="live-kicker">01 · Isolate</div>
</div>
<h3>Governed workspaces</h3>
<p>Non-production environments for people and agents.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:book-open /></IconBadge>
<div class="live-kicker">02 · Guide</div>
</div>
<h3>Shared engineering codex</h3>
<p>Standards, patterns, examples, and templates for people and agents.</p>
</div>

<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:gear-six /></IconBadge>
<div class="live-kicker">03 · Enforce</div>
</div>
<h3>Compiler-enforced standards</h3>
<p>Critical rules fail at compile time, before review.</p>
</div>
</div>

<div class="live-outcome">Guidance is shared. Non-negotiable rules are enforced.</div>
</div>

<!--
Authoring controls have three practical parts:
- Isolate work from production while an idea is still changing.
- Guide people and agents with one versioned engineering codex of standards, patterns, examples, and templates.
- Enforce critical coding rules as the code compiles, so people spend review time on design and trade-offs rather than policing mechanical rules.
-->

---
title: Automate the path to production.
sync: delivery-automation
class: slide-light
---

<Eyebrow text="01 · STANDARDISE DELIVERY · DELIVERY AUTOMATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Automate the path to production.

<p class="live-claim">Faster implementation matters only when evidence and delivery move at the same speed.</p>

<div class="live-pipeline" style="margin-top: 22px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:code /></IconBadge>
<div class="live-kicker">01 · Build</div>
</div>
<h3>Build once</h3>
<p>Reproducible artifact</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card">
<div class="live-card-head">
<IconBadge tone="ink"><ph:test-tube /></IconBadge>
<div class="live-kicker">02 · Test</div>
</div>
<h3>Test behaviour</h3>
<p>Unit · integration · replay · end-to-end</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:shield-check /></IconBadge>
<div class="live-kicker">03 · Analyse</div>
</div>
<h3>Gate every change</h3>
<p>Compiler · security · dependencies · secrets</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-magenta">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:rocket-launch /></IconBadge>
<div class="live-kicker">04 · Deploy</div>
</div>
<h3>Promote the artifact</h3>
<p>Provision · deploy · release</p>
</div>
</div>

<div class="live-outcome">The same accepted artifact moves forward with its evidence.</div>
</div>

<!--
These changes predate the current AI wave and make AI-assisted implementation operationally useful.
- Build produces one reproducible artifact.
- Automated tests exercise unit, integration, replay, and end-to-end behaviour.
- Compiler, dependency, secret, security, and review controls gate every change.
- The accepted artifact is promoted through provision, deployment, and release.
-->

---
title: Specify behaviour.
sync: section-intent
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 02" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill is-magenta">02 · Spec-driven development</div>

# Specify behaviour.

<p class="section-subtitle">People and agents share one source of truth: <strong>durable specifications, not code.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Surface ambiguity · Expose conflicts and gaps before coding</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Human review gate · Debate trade-offs and seal the spec</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Durable truth · Code is disposable; specifications remain</span></div>
</div>

<!--
Section 02 divider: Specify behaviour (Spec-driven development).
-->

---
title: AI coding moves right. Governance moves up.
sync: record-position
class: slide-light
---

<Eyebrow text="02 · SPECIFY BEHAVIOUR · SPEC-DRIVEN DEVELOPMENT" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="live-stage">

# AI coding moves right. Governance moves up.

<p class="live-claim">Adopting coding agents speeds up implementation — spec-driven development adds the governance above it.</p>

<div class="live-matrix">
<div class="live-matrix-axis is-top">Governed</div>
<div class="live-matrix-axis is-left">Hand-coded</div>
<div class="live-matrix-axis is-right">AI-assisted</div>
<div class="live-matrix-axis is-bottom">Ungoverned</div>
<div class="live-matrix-grid">
<div class="live-quadrant">
<h3>Traditional SDLC</h3>
<p>Governance around human implementation</p>
</div>
<div class="live-quadrant is-target">
<h3>Spec-driven development</h3>
<p>Durable intent around AI implementation</p>
</div>
<div class="live-quadrant">
<h3>Ad hoc development</h3>
<p>Local choices and human memory</p>
</div>
<div class="live-quadrant">
<h3>AI-assisted coding</h3>
<p>Conversation-led generation</p>
</div>
</div>
</div>

<div class="live-outcome" style="width: min(724px, calc(100% - 276px)); margin-inline: auto; box-sizing: border-box;">AI coding changes implementation. It does not create governance.</div>
</div>

<!--
Most AI adoption begins as a horizontal move: introduce an assistant or agent and produce code faster.
That does not create authority, an audit trail, or a durable statement of what must remain true.
Spec-driven development is the separate upward move. It puts governed intent around AI-assisted implementation.
-->

---
title: Specifications drive the loop.
sync: record-development
class: slide-light
---

<Eyebrow text="02 · SPECIFY BEHAVIOUR · THE DEVELOPMENT LOOP" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Specifications drive the loop.

<p class="live-claim">Surface conflicts and gaps for human judgement instead of burying them in generated code.</p>

<div class="live-sdd-flow" style="margin-top: 36px;">
<div class="live-card is-cyan">
<div class="live-kicker">01 · What + how</div>
<h3>Specify</h3>
<p>Intent · docs · tests · legacy source code<br>→ spec.md · design.md</p>
</div>
<div class="live-review-gate">
<strong>Review &amp;<br>seal</strong>
Human decision gate
</div>
<div class="live-card is-magenta">
<div class="live-kicker">02 · Derive</div>
<h3>Plan</h3>
<p>Units of work, ordered by dependency</p>
</div>
<div class="live-sdd-arrow">→</div>
<div class="live-card is-dark">
<div class="live-kicker">03 · Code + prove</div>
<h3>Build</h3>
<p>Build · integrate · verify against the specification</p>
</div>
</div>

<div class="live-outcome" style="width: min(100%, 1000px); margin-inline: auto; box-sizing: border-box;">Each change runs the loop. If the specification changes, it is reviewed and sealed again.</div>
</div>

<!--
This is the recognisable spec-driven development loop.
- Coding agents tend to smooth over ambiguity with plausible-looking code, short-circuiting the conversations a team needed to have.
- Specify reconciles intent, documentation, tests, and, where it exists, legacy source code into `spec.md` and `design.md`—surfacing conflicts and gaps explicitly.
- A person reviews, debates trade-offs, and seals that specification in the repository.
- Plan derives ephemeral work ordered by dependency from the reviewed specification. It can be discarded and generated again.
- Implement gives each unit of work the exact specification and plan entry it needs, builds and integrates it, and verifies the combined result against the specification.
-->

---
title: Code is disposable. Specifications are durable.
sync: record-truth
class: slide-light
---

<Eyebrow text="02 · SPECIFY BEHAVIOUR · DURABLE SPECIFICATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span>03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Code is disposable.<br><span class="live-title-accent">Specifications are durable.</span>

<p class="live-claim">The implementation can be regenerated. The reviewed specification preserves what the system must do and how it is designed.</p>

<div class="live-durability" style="margin-top: 36px;">
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

<div class="live-outcome is-punch" style="width: min(100%, 1000px); margin-inline: auto; box-sizing: border-box;">Regenerate the plan. Rebuild the code. The specification remains.</div>
</div>

<!--
This is the deliberate provocation at the end of the section.
- Disposable does not mean unimportant. It means the implementation is a derived output, not the authority over intent.
- Only the reviewed `spec.md` and `design.md` remain as the durable specification in the repository.
-->

---
title: Sandbox execution.
sync: section-constrain
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 03" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill">03 · Capability-based runtime</div>

# Sandbox execution.

<p class="section-subtitle">Grant generated code nothing by default—<strong>it reaches infrastructure only through host-mediated capabilities.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Wasmtime sandbox · No direct route to infrastructure</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Capability contracts · HTTP, Messaging, KeyValue, BlobStore</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Runtime recipes · Local, cloud, and on-premise</span></div>
</div>

<!--
Section 03 divider: Sandbox execution.
-->

---
title: Generated code cannot reach past its sandbox.
sync: runtime
class: slide-light
---

<Eyebrow text="03 · SANDBOX EXECUTION · CAPABILITY BOUNDARY" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Generated code cannot reach<br>past its sandbox.

<p class="live-claim">With the platform safe by construction, agents work within known bounds while engineers focus on the product.</p>

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

<div class="live-portable-link">Host-granted capabilities</div>

<div class="live-capability-row">
<div class="live-runtime-capability"><ph:globe class="live-capability-icon" /><span>HTTP</span></div>
<div class="live-runtime-capability"><ph:chat-circle class="live-capability-icon" /><span>Messaging</span></div>
<div class="live-runtime-capability"><ph:key class="live-capability-icon" /><span>KeyValue</span></div>
<div class="live-runtime-capability"><ph:database class="live-capability-icon" /><span>BlobStore</span></div>
</div>

<div class="live-portable-link">Host-selected infrastructure</div>

<div class="live-infrastructure">
<strong>Infrastructure</strong>
<em>Cloud/On-premise</em>
</div>
</div>
</div>

<!--
- Generated application code is packaged as one or more WASI components inside a Wasmtime sandbox.
- They reach HTTP, messaging, key-value storage, and blob storage only through capability interfaces supplied by the host.
- With the platform settled and safe by construction, agents implement within known bounds while engineers focus on the product.
-->

---
title: Different infrastructure. Same application.
sync: runtime-portability
class: slide-light
---

<Eyebrow text="03 · SANDBOX EXECUTION · PORTABLE INFRASTRUCTURE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardise delivery</span>
<span>02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Different infrastructure.<br>Same application.

<p class="live-claim">The component depends on capabilities, not products. A runtime recipe binds them to approved services in each environment.</p>

<div class="live-portability">
<div class="live-portable-app">
<div class="live-kicker">Application unchanged</div>
<strong>WASI Component</strong>
</div>

<div class="live-portable-link">Stable capability contract</div>

<div class="live-capability-row">
<div class="live-runtime-capability"><ph:globe class="live-capability-icon" /><span>HTTP</span></div>
<div class="live-runtime-capability"><ph:chat-circle class="live-capability-icon" /><span>Messaging</span></div>
<div class="live-runtime-capability"><ph:key class="live-capability-icon" /><span>KeyValue</span></div>
<div class="live-runtime-capability"><ph:database class="live-capability-icon" /><span>BlobStore</span></div>
</div>

<div class="live-portable-link">Same shape for each environment</div>

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
- The application component depends on stable capability contracts rather than directly on infrastructure products.
- Each environment supplies a reviewed runtime recipe: the runtime, capability bindings, identity and policy, configuration, and observability.
-->

---
title: Match safeguards to the stakes.
sync: section-trust
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · TRUST MODEL" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill">Contextual governance</div>

# Match safeguards to the stakes.

<p class="section-subtitle">Give prototypes room to learn—<strong>strengthen safeguards as people come to depend on the system.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Explore &amp; Govern · Keep discovery cheap and shared work clean</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Connect &amp; Support · Enforce runtime and full delivery boundaries</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Trust radius · Assess reach, data, dependence, and continuity</span></div>
</div>

<!--
Section divider: Trust Model and Trust Radius.
-->

---
title: Trust is contextual, not a maturity score.
sync: trust-model
class: slide-light
---

<Eyebrow text="TRUST MODEL · REQUIRED SAFEGUARDS FOLLOW CONSEQUENCE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="live-stage">

# Trust is contextual, not a maturity score.

<p class="live-claim">Explore, Govern, Connect, and Support are situations to match—not levels to climb.</p>

<div class="live-trust-model">
<div class="live-trust-state is-explore">
<div class="live-trust-state-head">State 0 · Explore</div>
<div class="live-trust-band">
<strong>Local discovery</strong>
</div>
<div class="live-trust-band">
<p>One builder · synthetic inputs</p>
</div>
<div class="live-trust-band is-boundary">
<strong>Isolate and keep local</strong>
</div>
</div>

<div class="live-trust-state is-govern">
<div class="live-trust-state-head">State 1 · Govern</div>
<div class="live-trust-band">
<strong>Shared work</strong>
</div>
<div class="live-trust-band">
<p>Named owner · protected changes</p>
</div>
<div class="live-trust-band is-boundary">
<strong>Authoring boundary</strong>
</div>
</div>

<div class="live-trust-state is-connect">
<div class="live-trust-state-head">State 2 · Connect</div>
<div class="live-trust-band">
<strong>Live capability</strong>
</div>
<div class="live-trust-band">
<p>Purpose + owner · live contracts</p>
</div>
<div class="live-trust-band is-boundary">
<strong>Runtime boundary</strong>
</div>
</div>

<div class="live-trust-state is-support">
<div class="live-trust-state-head">State 3 · Support</div>
<div class="live-trust-band">
<strong>Business dependence</strong>
</div>
<div class="live-trust-band">
<p>Reviewed intent · recovery evidence</p>
</div>
<div class="live-trust-band is-boundary">
<strong>Full delivery system</strong>
</div>
</div>
</div>

<p class="live-trust-model-note">Choose the least costly state that matches the actual consequence.</p>
</div>

<!--
These are operational states, not levels of organisational maturity.
There is no virtue in moving every application to the right. Keep it in the least costly state consistent with its actual consequence.
Read each state as its situation, its minimum evidence and ownership, then its required boundary:
- Explore: one builder, synthetic or local inputs, no shared dependence; isolate and keep local.
- Govern: named owner, approved workspace and tools, protected changes and checks; authoring boundary.
- Connect: explicit purpose and owner, typed live-system contracts, host policy and observability; runtime boundary.
- Support: durable reviewed intent, release and service ownership, recovery evidence; full delivery system.
-->

---
title: The highest consequence sets the trust radius.
sync: trust-radius
class: slide-light
---

<Eyebrow text="TRUST RADIUS · ASSESS FOUR DIMENSIONS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardise delivery</span>
<span class="is-active is-magenta">02 · Specify behaviour</span>
<span class="is-active">03 · Sandbox execution</span>
</div>

<div class="live-stage">

# The highest consequence sets the trust radius.

<p class="live-claim">Place each dimension on the same four-state scale. Take the furthest answer—not the average—and reassess whenever one changes.</p>

<div class="live-radius-grid">
<div class="live-radius-card">
<div class="live-kicker"><ph:globe class="live-kicker-icon" />01 · Reach</div>
<h3>What can it access?</h3>
<p>Local environment · shared systems · live capabilities</p>
</div>
<div class="live-radius-card">
<div class="live-kicker"><ph:database class="live-kicker-icon" />02 · Data</div>
<h3>What can it read or change?</h3>
<p>Synthetic data · project data · live or regulated data</p>
</div>
<div class="live-radius-card">
<div class="live-kicker"><ph:users-three class="live-kicker-icon" />03 · Dependence</div>
<h3>Who relies on it?</h3>
<p>One builder · a team · customers or operations</p>
</div>
<div class="live-radius-card">
<div class="live-kicker"><ph:lifebuoy class="live-kicker-icon" />04 · Continuity</div>
<h3>What must survive failure?</h3>
<p>Discard · restore · recover within an obligation</p>
</div>
</div>

<div class="live-radius-scale">
<div><strong>0</strong> Explore</div>
<div><strong>1</strong> Govern</div>
<div><strong>2</strong> Connect</div>
<div><strong>3</strong> Support</div>
</div>

</div>

<!--
Do not average the answers. The highest-consequence answer sets the application's required state and therefore its safeguards.
Reassess whenever reach, data, dependence, or continuity changes. The required state can move in either direction as those answers change.
-->

---
title: One delivery system. Four use cases.
sync: use-cases
class: slide-light
---

<Eyebrow text="THE DELIVERY SYSTEM · WHERE IT APPLIES" />

<div class="live-stage">

# One delivery system.<br>Four use cases.

<p class="live-claim">Whether starting from a prototype, a legacy codebase, or a new service, the same path applies.</p>

<div class="live-grid-2" style="margin-top: 20px; gap: 18px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:rocket-launch /></IconBadge>
<div class="live-kicker">01 · Prototype to production</div>
</div>
<h3>Take vibe-coded apps to production</h3>
<p>Extract intent from a rapid prototype and rebuild it as a supported service.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:file-text /></IconBadge>
<div class="live-kicker">02 · Legacy modernisation</div>
</div>
<h3>Retire technical debt</h3>
<p>Recover durable specifications from legacy code and regenerate a clean implementation.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="ink"><ph:globe /></IconBadge>
<div class="live-kicker">03 · Cloud portability</div>
</div>
<h3>Move across environments</h3>
<p>Build once against capability contracts and swap the runtime recipe per environment.</p>
</div>

<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:shield-check /></IconBadge>
<div class="live-kicker">04 · High-trust systems</div>
</div>
<h3>Deliver in regulated environments</h3>
<p>Move at AI speed with an auditable specification, compile-time rules, and sandboxed execution.</p>
</div>
</div>

</div>

<!--
Where the delivery system applies in practice:
1. Prototype to production: extract the real intent from a vibe-coded or citizen-built prototype and rebuild it through the governed path as a maintainable service.
2. Legacy modernisation: recover durable specifications (`spec.md` and `design.md`) from existing source code, tests, and docs, then regenerate a clean implementation.
3. Cloud portability: build once against capability contracts and move across local, cloud, and on-premise environments by changing the runtime recipe.
4. High-trust and regulated systems: gain AI delivery speed while preserving an auditable specification, compiler-enforced rules, and a zero-trust runtime sandbox.
-->

---
title: Narrow the target. Close the loop.
sync: future-system
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · THE FUTURE" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill is-magenta">Roadmap</div>

# Narrow the target. Close the loop.

<p class="section-subtitle">A constrained delivery system unlocks <strong>smaller models on a narrower target, and production signals returning as governed changes.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Narrow the target · Pre-scaffold structure, types, routes, and tests</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Run smaller models · Local and open-weight models inside the same gates</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Close the loop · Production signals return as governed changes</span></div>
</div>

<!--
Section divider: What comes next.
- So far the argument has covered the delivery system as it runs today: standardise delivery, specify behaviour, sandbox execution, and match safeguards to the stakes.
- That constrained shape unlocks two active directions on the roadmap:
  1. Smaller, local models on a narrower target—because the build adapter pre-scaffolds structure and tests and the runtime fixes the capability boundary.
  2. The path becoming a loop—where production signals are investigated and fixes proposed, then specified, built, and merged like any other change.
-->

---
title: A narrower target unlocks smaller models.
sync: future-models
class: slide-light
---

<Eyebrow text="THE FUTURE · PRE-SCAFFOLDED SLICES" />

<div class="live-stage">

# A narrower target unlocks smaller models.

<p class="live-claim">Frontier models are needed when an agent has to invent architecture—not when structure, contracts, and tests are already fixed.</p>

<div class="live-grid-2" style="width: 88%; margin: 28px auto 0; gap: 52px;">
<div class="live-card" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="ink"><ph:stack /></IconBadge>
<div class="live-kicker">Broad target · Open-ended coding</div>
</div>
<h3>Requires frontier models</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(13, 16, 48, 0.12);">Invent structure · choose libraries · hold the full codebase in context</p>
</div>

<div class="live-card is-dark" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="light"><ph:code /></IconBadge>
<div class="live-kicker">Narrow target · Pre-scaffolded slice</div>
</div>
<h3>Unlocks smaller, local models</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(255, 255, 255, 0.16);">Structure and tests pre-built · fill bounded logic · verify at compile time</p>
</div>
</div>

<div class="live-outcome" style="width: 88%; margin: 48px auto 0; box-sizing: border-box;">Today: frontier models in production · Next: open-weight and client-hosted models.</div>
</div>

<!--
Smaller models on a narrower target:
- A frontier model is necessary when a coding agent has to choose libraries, invent project structure, and hold an entire codebase in context.
- Our target build adapter removes that work before the model turn begins: it pre-scaffolds the crate, module layout, typed routes, message consumers, and scenario test skeletons from the sealed specification, then asks the model only to fill the behavioural markers and pass compiler, lint, and test checks.
- Today, that adapter runs with frontier LLMs in production engagements.
- In early proof-of-concept runs, the same scaffold-and-verify loop already builds working WebAssembly guest slices using an untuned open-weight model (`qwen3-coder-next`).
- Active roadmap focus: hardening small language model support so sensitive systems can be built and verified on local or client-hosted hardware without depending on a frontier API.
-->

---
title: Feedback drives the next change.
sync: future-loop
class: slide-light
---

<Eyebrow text="THE FUTURE · THE COMPLETE SYSTEM" />

<div class="live-stage">

# Feedback drives the next change.

<p class="live-claim">Nothing bypasses the path. A signal becomes a change; a change becomes new evidence.</p>

<div class="live-loop">
<svg viewBox="0 0 1140 480" aria-hidden="true" style="width: 100%; height: auto; display: block;">
  <defs>
    <marker id="loop-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="rgba(13,16,48,0.55)" />
    </marker>
    <marker id="loop-arrow-magenta" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="#7c5cc9" />
    </marker>
    <marker id="loop-arrow-red" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M0 0L10 5L0 10z" fill="#ff3b6b" />
    </marker>
    <linearGradient id="loop-gradient" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="#16b4e2" />
      <stop offset="100%" stop-color="#d63ad6" />
    </linearGradient>
  </defs>

  <!-- sources -->
  <rect x="10" y="36" width="176" height="54" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="98" y="69" text-anchor="middle" class="svg-label" fill="#0d1030">Security feeds</text>
  <rect x="10" y="354" width="176" height="54" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="98" y="387" text-anchor="middle" class="svg-label" fill="#0d1030">Dependency audit</text>

  <rect x="10" y="195" width="176" height="54" rx="9" fill="rgba(255,59,107,0.06)" stroke="rgba(255,59,107,0.8)" stroke-width="1.6" />
  <text x="98" y="215" text-anchor="middle" class="svg-label" fill="#0d1030">Vulnerability</text>
  <text x="98" y="233" text-anchor="middle" class="svg-label" fill="#0d1030">monitoring</text>
  <text x="98" y="246" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8px;">Analyse · security, dependencies</text>

  <line x1="98" y1="90" x2="98" y2="191" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <line x1="98" y1="354" x2="98" y2="253" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <line x1="186" y1="222" x2="228" y2="222" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />

  <!-- incident manager -->
  <rect x="232" y="150" width="304" height="144" rx="14" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.6" />
  <text x="252" y="178" class="svg-kicker" fill="rgba(13,16,48,0.58)">INCIDENT MANAGER</text>
  <rect x="252" y="196" width="122" height="58" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="313" y="231" text-anchor="middle" class="svg-label" fill="#0d1030">Investigate</text>
  <line x1="374" y1="225" x2="390" y2="225" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-end="url(#loop-arrow)" />
  <rect x="394" y="196" width="122" height="58" rx="9" fill="#ffffff" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" />
  <text x="455" y="231" text-anchor="middle" class="svg-label" fill="#0d1030">Propose Fix</text>
  <rect x="399" y="262" width="112" height="24" rx="12" fill="#0d1030" />
  <text x="455" y="278" text-anchor="middle" fill="#ffffff" style="font-size: 10.5px; font-weight: 600;">Review &amp; seal</text>

  <!-- forks to the two lanes -->
  <path d="M500 150 C 560 110, 560 92, 592 92" fill="none" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <path d="M500 294 C 560 334, 560 368, 592 368" fill="none" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <text x="512" y="118" class="svg-caption" fill="rgba(13,16,48,0.58)">investigate/fix</text>
  <text x="512" y="352" class="svg-caption" fill="rgba(13,16,48,0.58)">investigate/fix</text>

  <!-- infrastructure lane -->
  <rect x="596" y="28" width="528" height="142" rx="14" fill="rgba(22,180,226,0.04)" stroke="rgba(12,127,163,0.4)" stroke-width="1.5" />
  <text x="616" y="54" class="svg-kicker" fill="#0c7fa3">INFRASTRUCTURE</text>
  <rect x="616" y="59" width="168" height="4" rx="2" fill="url(#loop-gradient)" />
  <rect x="616" y="66" width="168" height="62" rx="9" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="700" y="96" text-anchor="middle" class="svg-label" fill="#0d1030">Specify–Build–Merge</text>
  <text x="700" y="116" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)" style="font-size: 8.5px;">Spec → Plan → Build → Merge → Verify</text>
  <line x1="784" y1="97" x2="808" y2="97" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-start="url(#loop-arrow)" marker-end="url(#loop-arrow)" />
  <path d="M812 76 a60 10 0 0 0 120 0 v42 a60 10 0 0 1 -120 0 z" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <ellipse cx="872" cy="76" rx="60" ry="10" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <text x="872" y="102" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="872" y="119" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8.5px;">spec.md · design.md</text>
  <rect x="972" y="66" width="136" height="62" rx="9" fill="rgba(124,92,205,0.1)" stroke="rgba(124,92,205,0.7)" stroke-width="1.6" />
  <text x="1040" y="99" text-anchor="middle" class="svg-label" fill="#0d1030">Terraform</text>
  <text x="1040" y="117" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">runtime recipe</text>
  <line x1="972" y1="97" x2="936" y2="97" stroke="rgba(124,92,205,0.8)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow-magenta)" />
  <line x1="1040" y1="128" x2="1040" y2="198" stroke="rgba(124,92,205,0.9)" stroke-width="2" marker-end="url(#loop-arrow-magenta)" />
  <text x="1052" y="168" class="svg-caption" fill="rgba(13,16,48,0.58)">update</text>

  <!-- shared infrastructure + monitoring -->
  <rect x="928" y="202" width="196" height="62" rx="10" fill="rgba(22,180,226,0.1)" stroke="#0c7fa3" stroke-width="1.8" />
  <text x="1026" y="236" text-anchor="middle" class="svg-h3" fill="#0d1030">Infrastructure</text>
  <text x="1026" y="255" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">Cloud/On-premise</text>
  <rect x="596" y="202" width="210" height="62" rx="10" fill="rgba(255,59,107,0.06)" stroke="rgba(255,59,107,0.8)" stroke-width="1.6" />
  <text x="701" y="228" text-anchor="middle" class="svg-label" fill="#0d1030">Infrastructure</text>
  <text x="701" y="248" text-anchor="middle" class="svg-label" fill="#0d1030">monitoring</text>
  <line x1="806" y1="233" x2="924" y2="233" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />
  <text x="865" y="223" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)">monitor</text>
  <line x1="596" y1="240" x2="540" y2="240" stroke="#ff3b6b" stroke-width="2" marker-end="url(#loop-arrow-red)" />
  <text x="568" y="230" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">observe</text>

  <!-- code lane -->
  <rect x="596" y="308" width="528" height="142" rx="14" fill="rgba(22,180,226,0.04)" stroke="rgba(12,127,163,0.4)" stroke-width="1.5" />
  <text x="616" y="334" class="svg-kicker" fill="#0c7fa3">CODE</text>
  <rect x="616" y="339" width="168" height="4" rx="2" fill="url(#loop-gradient)" />
  <rect x="616" y="346" width="168" height="62" rx="9" fill="rgba(22,180,226,0.08)" stroke="#0c7fa3" stroke-width="1.6" />
  <text x="700" y="376" text-anchor="middle" class="svg-label" fill="#0d1030">Specify–Build–Merge</text>
  <text x="700" y="396" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.58)" style="font-size: 8.5px;">Spec → Plan → Build → Merge → Verify</text>
  <line x1="784" y1="377" x2="808" y2="377" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" marker-start="url(#loop-arrow)" marker-end="url(#loop-arrow)" />
  <path d="M812 356 a60 10 0 0 0 120 0 v42 a60 10 0 0 1 -120 0 z" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <ellipse cx="872" cy="356" rx="60" ry="10" fill="#ffffff" stroke="rgba(13,16,48,0.4)" stroke-width="1.5" />
  <text x="872" y="382" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="872" y="399" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 8.5px;">spec.md · design.md</text>
  <rect x="972" y="346" width="136" height="62" rx="9" fill="rgba(13,16,48,0.05)" stroke="rgba(13,16,48,0.35)" stroke-width="1.5" />
  <text x="1040" y="374" text-anchor="middle" class="svg-label" fill="#0d1030">GitHub</text>
  <text x="1040" y="394" text-anchor="middle" class="svg-label" fill="#0d1030">Actions</text>
  <text x="1040" y="430" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">promote the artifact</text>
  <line x1="972" y1="377" x2="936" y2="377" stroke="rgba(13,16,48,0.45)" stroke-width="1.6" stroke-dasharray="5 4" marker-end="url(#loop-arrow)" />
  <line x1="1040" y1="346" x2="1040" y2="268" stroke="rgba(13,16,48,0.5)" stroke-width="2" marker-end="url(#loop-arrow)" />
  <text x="1052" y="305" class="svg-caption" fill="rgba(13,16,48,0.58)">deploy</text>

</svg>
</div>
</div>

<!--
The complete system, using vulnerability remediation as the worked example:
- Security feeds and dependency audits flow into vulnerability monitoring.
- An incident manager investigates and proposes a fix. It proposes; it does not ship.
- The fix enters at the production gate and runs the same loop as any change: specify, build, and merge — in a code lane and an infrastructure lane.
- The result is deployed and updated, and infrastructure monitoring watches it and raises the next signal.
- Nothing bypasses the path. The safeguards on each pass are the ones the trust radius requires.
-->

---
title: Thank you.
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

<div class="split-content-pane" style="justify-content: center;">
<div>
<Eyebrow text="PROPELLERHEAD · ACCOUNTABLE SOFTWARE DELIVERY" />

# Thank you.

</div>

</div>

<!--
The trust model gives the audience a way to judge how much safeguard a change needs.
Close by returning to the opening in personal terms: what is the first change they would trust AI to deliver? One change is enough to start; set the safeguards to match its stakes.
-->

