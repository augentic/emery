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

<p class="section-subtitle">A system for shipping software with AI —<br>not just generating code.</p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Standardising delivery · One path from change to production</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Specifying behaviour · Define requirements and verify code against them</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Sandboxing execution · Limit what running code may access</span></div>
</div>

<!--
Opening:
- Generative AI makes code and test generation abundant. Trusted evidence remains scarce.
- Accountability has not moved to the model. Your organisation still carries the pager, the audit risk, and the production consequences.
- Our answer is not a better prompt. It is an engineering delivery system whose rules, specification record, and runtime sandbox sit outside the model.
-->

---
title: Coding is no longer the bottleneck.
sync: shift
class: slide-light
---

<Eyebrow text="THE SHIFT · FROM GENERATION TO COMPREHENSION" />

<div class="live-split">
<div class="live-copy">

# Coding is no longer<br>the bottleneck.

<p class="live-claim">AI writes code in minutes, but without a delivery system it inflates PRs, shifts the bottleneck to review, and degrades delivery stability.</p>

<div class="live-compression">
<span>Weeks</span>
<div class="live-compression-line"></div>
<span>Minutes</span>
</div>
</div>

<div class="live-panel">
<div class="live-panel-title">More code ≠ faster delivery</div>
<div class="live-word-grid">
<div class="live-word"><IconBadge tone="light" size="small"><ph:stack /></IconBadge><div class="live-word-copy"><strong>Verification tax</strong><span>Larger PRs overwhelm human review</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:puzzle-piece /></IconBadge><div class="live-word-copy"><strong>Hidden assumptions</strong><span>Plausible code skips design trade-offs</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:copy /></IconBadge><div class="live-word-copy"><strong>Duplication &amp; churn</strong><span>Copy-paste growth replaces refactoring</span></div></div>
<div class="live-word"><IconBadge tone="light" size="small"><ph:warning /></IconBadge><div class="live-word-copy"><strong>Delivery instability</strong><span>More defects reach production</span></div></div>
</div>
<div class="live-panel-result"><span class="live-panel-result-label">The risk</span>A system no one can safely change.</div>
</div>
</div>

<!--
Ground this in industry telemetry (DORA, Faros AI, GitClear):
- DORA's State of AI-assisted Software Development report calls this "the verification tax": time saved writing code is re-spent auditing larger batches, and every 25% increase in AI adoption without automated safeguards correlates with a 7.2% drop in delivery stability.
- Faros AI telemetry across high-AI teams shows PRs are 51% larger, bugs per PR are up 54%, and PR review wait times are up 441%.
- GitClear's analysis of 623M lines of code shows refactoring is down 70% while duplicated code blocks are up 81%.
-->

---
title: Build the right thing. Build it to last.
sync: pivot
class: slide-light
---

<Eyebrow text="THE RESPONSE · STRATEGY AND DELIVERY" />

<div class="live-stage">

# Build the right thing.<br>Build it to last.

<p class="live-claim">Product strategy decides what is worth building. <strong>Our focus is delivering it as software that lasts.</strong></p>

<div class="live-grid-2" style="width: 88%; margin: 28px auto 0; gap: 52px;">
<div class="live-card is-cyan" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="cyan"><ph:compass /></IconBadge>
<div class="live-kicker">01 · Product strategy</div>
</div>
<h3>Build the right thing</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(12, 127, 163, 0.2);">Frame the problem · weigh trade-offs · test with prototypes</p>
</div>

<div class="live-card is-dark" style="min-height: 214px; padding: 26px 30px; display: flex; flex-direction: column; justify-content: space-between;">
<div>
<div class="live-card-head" style="margin-bottom: 14px;">
<IconBadge tone="light"><ph:gear-six /></IconBadge>
<div class="live-kicker">02 · Delivery system</div>
</div>
<h3>Build it to last</h3>
</div>
<p style="padding-top: 14px; border-top: 1px solid rgba(255, 255, 255, 0.16);">Standardising delivery · specifying behaviour · sandboxing execution</p>
</div>
</div>

<div class="live-outcome is-punch" style="width: 88%; margin: 48px auto 0; box-sizing: border-box;">Prototypes test assumptions. A delivery system ships production software.</div>
</div>

<!--
Draw a hard line between discovery prototypes and production systems:
1) Rapid AI prototypes are great for testing product assumptions cheaply—but a prototype informs the decision, it is not the production system.
2) Turning agreed intent into maintainable, auditable software without accumulating slop requires a disciplined delivery system. That second half is our core engineering focus.
-->

---
title: Three boundaries around AI code.
sync: system
layout: default
class: slide-light
---

<Eyebrow text="THE DELIVERY SYSTEM · HUMAN-LED ENGINEERING" />

<div class="live-stage">

# Three boundaries around AI code.

<p class="live-claim">Agents work inside the system. Engineers remain in charge of it.</p>

<div class="live-grid-3" style="margin-top: 24px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:shield-check /></IconBadge>
<div class="live-num">01 · Change path</div>
</div>
<h3>Standardising delivery</h3>
<p>How code is written, checked, and shipped.</p>
</div>
<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:file-text /></IconBadge>
<div class="live-num">02 · Behavioural contract</div>
</div>
<h3>Specifying behaviour</h3>
<p>What the software must do and how it is verified.</p>
</div>
<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:lock-key /></IconBadge>
<div class="live-num">03 · Runtime boundary</div>
</div>
<h3>Sandboxing execution</h3>
<p>What running code is permitted to touch.</p>
</div>
</div>

<div class="live-rail">Specify → Build → Test → Analyse → Deploy → Observe → Recover</div>
</div>

<!--
This is the architectural map for the deck:
- Instead of trusting the model to behave, we constrain it across three independent boundaries:
  1. The authoring and pipeline boundary (Standardising delivery).
  2. The behavioural contract and acceptance boundary (Specifying behaviour).
  3. The runtime execution boundary (Sandboxing execution).
- Product judgement, specification sign-off, and release authority stay with engineers.
-->

---
title: Standardising delivery.
sync: section-standardise
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 01" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill">01 · Toolchain &amp; pipeline</div>

# Standardising delivery.

<p class="section-subtitle">One governed path from change to production—<strong>across how code is written and how it ships.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Isolate &amp; guide · Isolated workspaces and shared engineering standards</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Enforce at compile time · Fail critical rules before review</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Automate deployment · Replay behaviour, gate the supply chain, and deploy</span></div>
</div>

<!--
Section 01 divider: Standardising delivery.
-->

---
title: Constrain code authoring.
sync: authoring
class: slide-light
---

<Eyebrow text="01 · STANDARDISING DELIVERY · AUTHORING CONTROLS" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardising delivery</span>
<span>02 · Specifying behaviour</span>
<span>03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Constrain code authoring.

<p class="live-claim">Isolate the workspace, share one set of engineering standards, and enforce architectural rules at compile time.</p>

<div class="live-grid-3" style="margin-top: 22px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:shield-check /></IconBadge>
<div class="live-kicker">01 · Isolate</div>
</div>
<h3>Isolated workspaces</h3>
<p>Non-production environments with no live access.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:book-open /></IconBadge>
<div class="live-kicker">02 · Guide</div>
</div>
<h3>Shared engineering standards</h3>
<p>One versioned rulebook for people and agents.</p>
</div>

<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:gear-six /></IconBadge>
<div class="live-kicker">03 · Enforce</div>
</div>
<h3>Compiler-enforced standards</h3>
<p>Architectural and safety rules enforced at compile time.</p>
</div>
</div>

<div class="live-outcome">Reviewers focus on design and trade-offs—not policing mechanical rules.</div>
</div>

<!--
Make "compiler-enforced standards" concrete for engineers:
- We don't just mean a style linter. During the agent's build turn, AST and compiler gates ban panics, ban direct network or filesystem calls, and enforce module layout.
- If generated code violates an architectural boundary, it fails to compile inside the agent's loop—long before a human engineer opens a pull request.
-->

---
title: Automate verification and deployment.
sync: delivery-automation
class: slide-light
---

<Eyebrow text="01 · STANDARDISING DELIVERY · DELIVERY AUTOMATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span class="is-active">01 · Standardising delivery</span>
<span>02 · Specifying behaviour</span>
<span>03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Automate verification and deployment.

<p class="live-claim">Agent-written unit tests often repeat the agent's own bugs—pipelines must replay behaviour against the spec, gate the supply chain, and automate deployment.</p>

<div class="live-pipeline is-three" style="margin-top: 22px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:test-tube /></IconBadge>
<div class="live-kicker">01 · Replay</div>
</div>
<h3>Hermetic behavioural replay</h3>
<p>Scripted doubles · deterministic scenarios · no live network</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:shield-check /></IconBadge>
<div class="live-kicker">02 · Gate</div>
</div>
<h3>Supply-chain &amp; secret gates</h3>
<p>Vetted dependencies · vulnerability scans · secret hygiene</p>
</div>
<div class="live-arrow">→</div>
<div class="live-card is-magenta">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:rocket-launch /></IconBadge>
<div class="live-kicker">03 · Deploy</div>
</div>
<h3>Automated review &amp; deployment</h3>
<p>AI review agents · immutable artifact · gated release</p>
</div>
</div>

<div class="live-outcome">One immutable artifact deployed with its verification evidence.</div>
</div>

<!--
Contrast traditional CI/CD with AI-era verification:
- Empirical studies (ACM ICSE 2026) show that when agents are prompted directly to write tests, they mirror their own implementation assumptions rather than testing contract boundaries and edge cases.
- Veracode's GenAI Code Security Report found that 45% of AI-generated code across 100+ models introduces known security flaws.
- The pipeline must prove behavioural scenarios against the spec over deterministic doubles, enforce dependency and secret hygiene, and gate deployment automatically.
-->

---
title: Specifying behaviour.
sync: section-intent
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 02" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill is-magenta">02 · Spec-driven development</div>

# Specifying behaviour.

<p class="section-subtitle">People and agents share one source of truth: <strong>durable specifications, not code.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Surface ambiguity · Expose conflicts and gaps before coding</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Human review gate · Debate trade-offs and seal the spec</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Durable truth · Code is disposable; specifications remain</span></div>
</div>

<!--
Section 02 divider: Specifying behaviour (Spec-driven development).
-->

---
title: Prompts are not specifications.
sync: record-position
class: slide-light
---

<Eyebrow text="02 · SPECIFYING BEHAVIOUR · SPEC-DRIVEN DEVELOPMENT" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardising delivery</span>
<span class="is-active is-magenta">02 · Specifying behaviour</span>
<span>03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Prompts are not specifications.

<p class="live-claim">Prompts disappear after a turn and leave agents to guess. Specifications persist in the repo as the contract both people and agents build against.</p>

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
<p>Versioned specs driving AI implementation</p>
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

<div class="live-outcome" style="width: min(724px, calc(100% - 276px)); margin-inline: auto; box-sizing: border-box;">Move right for speed. Move up for durability and auditability.</div>
</div>

<!--
Most teams adopt AI by moving horizontally on the bottom row: give developers a coding agent and generate code faster from chat prompts.
- Chat prompts evaporate after a turn, forcing agents to guess unstated constraints and rediscover context every session.
- Spec-driven development is the vertical move into the top-right quadrant: a persistent, versioned specification in the repo that raised functional test pass rates (+9.2 pp) and cut security defects by 73% in matched studies.
-->

---
title: The agentic development loop.
sync: record-development
class: slide-light
---

<Eyebrow text="02 · SPECIFYING BEHAVIOUR · THE DEVELOPMENT LOOP" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardising delivery</span>
<span class="is-active is-magenta">02 · Specifying behaviour</span>
<span>03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# The agentic development loop.

<p class="live-claim">Align on intent upfront, break work into bounded slices for agents, and verify code against the spec.</p>

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
<p>Implement · integrate · verify against the spec</p>
</div>
</div>

<div class="live-outcome" style="width: min(100%, 1000px); margin-inline: auto; box-sizing: border-box;">Every change runs the loop. Any spec change re-enters the human review gate.</div>
</div>

<!--
Give a concrete example at the "Review & seal" gate:
- Suppose legacy code has a 30-second timeout, an architecture doc says 10 seconds, and retry behaviour isn't specified anywhere.
- A normal coding agent silently picks one and buries that guess inside 400 lines of generated code.
- Specify extracts claims from your docs, tests, and code, flags the 30s vs 10s conflict, and marks the missing retry rule as `[unknown]`—halting at the Review & seal gate for a human decision before Plan and Build run.
-->

---
title: Code is disposable. Specifications are durable.
sync: record-truth
class: slide-light
---

<Eyebrow text="02 · SPECIFYING BEHAVIOUR · DURABLE SPECIFICATION" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardising delivery</span>
<span class="is-active is-magenta">02 · Specifying behaviour</span>
<span>03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Code is disposable.<br><span class="live-title-accent">Specifications are durable.</span>

<p class="live-claim">Code is generated from the spec. The sealed specification is what you keep.</p>

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
Land the provocation clearly:
- Calling code "disposable" does not mean production quality doesn't matter. It means code is a compiled artifact of the specification, not the sole place where business rules live.
- When models or target stacks improve, you can regenerate the plan and rebuild the implementation from `spec.md` and `design.md` because the behavioural truth is sealed in the repository.
-->

---
title: Sandboxing execution.
sync: section-constrain
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · FOUNDATION 03" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill">03 · Capability-based runtime</div>

# Sandboxing execution.

<p class="section-subtitle">Grant generated code nothing by default—<strong>it reaches infrastructure only through host-mediated capabilities.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Wasmtime sandbox · No direct route to infrastructure</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Capability contracts · HTTP, Messaging, KeyValue, BlobStore</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Runtime recipes · Local, cloud, and on-premise</span></div>
</div>

<!--
Section 03 divider: Sandboxing execution.
-->

---
title: Zero trust for generated code.
sync: runtime
class: slide-light
---

<Eyebrow text="03 · SANDBOXING EXECUTION · CAPABILITY BOUNDARY" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardising delivery</span>
<span>02 · Specifying behaviour</span>
<span class="is-active">03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Zero trust for generated code.

<p class="live-claim">Containers give every dependency ambient access to network, files, and secrets. WASI components start with zero authority and use only the capabilities the host links.</p>

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
Contrast the WASI sandbox directly against Docker containers:
- Every engineer in the room is thinking: "We already run in containers—aren't we sandboxed?"
- A container isolates the host OS, not your application's dependencies. If an AI agent pulls in a compromised package inside a container, that code shares the container's network socket, filesystem, and `DATABASE_URL` environment variable.
- Inside a Wasmtime sandbox, a WASI component has no ambient network socket, no filesystem, and no credentials—only the typed capability handles (`HTTP`, `Messaging`, `KeyValue`, `BlobStore`) the host explicitly injects.
-->

---
title: Infrastructure portability.
sync: runtime-portability
class: slide-light
---

<Eyebrow text="03 · SANDBOXING EXECUTION · PORTABLE INFRASTRUCTURE" />

<div class="foundation-progress" aria-label="Delivery foundations">
<span>01 · Standardising delivery</span>
<span>02 · Specifying behaviour</span>
<span class="is-active">03 · Sandboxing execution</span>
</div>

<div class="live-stage">

# Infrastructure portability.

<p class="live-claim">Write code once against capability contracts. Bind them to local, cloud, or on-premise services at deploy time.</p>

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

<div class="live-portable-link">Host-selected runtime recipe · bindings, policy, identity, observability</div>

<div class="live-recipe-grid">
<div class="live-recipe-card">
<div class="live-kicker">Local / trial</div>
<h3>In-memory + local services</h3>
<div class="live-recipe-template">
<span>Local Wasmtime host</span>
<span>In-memory KV &amp; blob</span>
<span>Local mock queues</span>
<span>Console telemetry</span>
</div>
</div>
<div class="live-recipe-card">
<div class="live-kicker">Cloud</div>
<h3>Managed services</h3>
<div class="live-recipe-template">
<span>Managed runtime</span>
<span>Cloud queues &amp; storage</span>
<span>IAM + KMS policy</span>
<span>Central observability</span>
</div>
</div>
<div class="live-recipe-card">
<div class="live-kicker">On-premise</div>
<h3>Approved internal services</h3>
<div class="live-recipe-template">
<span>Internal host cluster</span>
<span>Enterprise queues &amp; DB</span>
<span>Corporate PKI + Vault</span>
<span>SIEM + audit logs</span>
</div>
</div>
</div>
</div>
</div>

<!--
Why this matters to platform and security teams:
- Application code never imports an AWS, Azure, or vendor SDK directly—it calls the capability contract.
- Moving from a developer laptop to cloud or air-gapped on-premise infrastructure means swapping the host's runtime recipe, without rewriting or recompiling the application component.
-->

---
title: Use cases.
sync: use-cases
class: slide-light
---

<Eyebrow text="THE DELIVERY SYSTEM · WHERE IT APPLIES" />

<div class="live-stage">

# Use cases.

<p class="live-claim">Prototype, legacy codebase, or new service—every change runs the same governed path.</p>

<div class="live-grid-2" style="margin-top: 20px; gap: 18px;">
<div class="live-card is-cyan">
<div class="live-card-head">
<IconBadge tone="cyan"><ph:rocket-launch /></IconBadge>
<div class="live-kicker">01 · Prototype to production</div>
</div>
<h3>Take vibe-coded apps to production</h3>
<p>Extract intent from a prototype and rebuild it as a supported service.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="magenta"><ph:file-text /></IconBadge>
<div class="live-kicker">02 · Legacy modernisation</div>
</div>
<h3>Retire technical debt</h3>
<p>Recover specifications from legacy code and regenerate a clean implementation.</p>
</div>

<div class="live-card">
<div class="live-card-head">
<IconBadge tone="ink"><ph:globe /></IconBadge>
<div class="live-kicker">03 · Cloud portability</div>
</div>
<h3>Move across environments</h3>
<p>Build once against capability contracts and swap the runtime recipe.</p>
</div>

<div class="live-card is-dark">
<div class="live-card-head">
<IconBadge tone="light"><ph:shield-check /></IconBadge>
<div class="live-kicker">04 · High-trust systems</div>
</div>
<h3>Deliver in regulated environments</h3>
<p>Move at AI speed with auditable specs, compile-time rules, and sandboxing.</p>
</div>
</div>

</div>

<!--
Ask the room which of these four pressures is most acute for them today:
1. Vibe-coded prototypes that business stakeholders want in production tomorrow.
2. Legacy systems where the original authors left and the code is the only documentation.
3. Cloud/on-prem portability where vendor lock-in or data residency slows deployment.
4. Regulated services where auditability and blast-radius control are non-negotiable.
-->

---
title: Roadmap.
sync: future-system
class: slide-section
---

<AmbientVideo src="/videos/hero-video-3.mp4" variant="approach" />

<Eyebrow text="DELIVERY SYSTEM · ROADMAP" />

<div class="section-hero-grid">
<div class="section-hero-left">
<div class="section-kicker-pill is-magenta">What comes next</div>

# Roadmap.

<p class="section-subtitle">Tighter architectural constraints unlock <strong>smaller local models and automated remediation through the same gates.</strong></p>

</div>
</div>

<div class="hero-mechanisms">
<div class="hero-pillar"><span class="hero-pillar-num">01</span><span>Narrow the target · Pre-scaffold structure, types, routes, and tests</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">02</span><span>Run smaller models · Local and open-weight models inside the same gates</span></div>
<div class="hero-pillar"><span class="hero-pillar-num">03</span><span>Close the loop · Route production alerts through specification and review</span></div>
</div>

<!--
Section divider: What the constraints unlock next.
- Everything up to here is how the system operates today.
- Because the target architecture is strictly constrained and verified by compiler and test gates, it unlocks two major engineering advantages next:
  1. Running smaller, open-weight, client-hosted models instead of relying exclusively on frontier APIs.
  2. Feeding production security and incident alerts straight back into the same governed specification loop.
-->

---
title: Run smaller, local models.
sync: future-models
class: slide-light
---

<Eyebrow text="THE FUTURE · PRE-SCAFFOLDED SLICES" />

<div class="live-stage">

# Run smaller, local models.

<p class="live-claim">Frontier models are needed to invent architecture—not to fill in pre-scaffolded, compiler-checked slices.</p>

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
Why this resonates with security-conscious IT leaders:
- You only need a giant frontier model when the agent has to invent project structure, pick libraries, and hold the whole repo in context.
- Because our build step deterministically scaffolds the module layout, typed routes, message handlers, and scenario test skeletons from the sealed spec before the model turn starts, the model only fills in bounded business logic.
- In proof-of-concept runs, that narrower target already compiles and passes verification using untuned open-weight models—opening the path to air-gapped, client-hosted execution.
-->

---
title: Close the loop on issues.
sync: future-loop
class: slide-light
---

<Eyebrow text="THE FUTURE · THE COMPLETE SYSTEM" />

<div class="live-stage">

# Close the loop on issues.

<p class="live-claim">Detect an issue, analyse and propose a fix, approve the spec change, and build and deploy across code and infrastructure.</p>

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
  <text x="1040" y="430" text-anchor="middle" fill="rgba(13,16,48,0.55)" style="font-size: 9px;">deploy the artifact</text>
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
- Nothing bypasses the governed path.
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

<p class="live-claim" style="margin-top: 18px;">Questions and discussion.</p>

</div>

</div>

<!--
Close and open the floor for discussion:
- Ask: "Where in your portfolio is the gap between fast AI coding and accountable delivery most acute right now?"
- You don't need an enterprise-wide transformation to start—one bounded service or prototype is enough to prove the path.
-->

