<script setup lang="ts">
withDefaults(defineProps<{
  tier: number
  dark?: boolean
  compact?: boolean
  chips?: boolean
  focus?: boolean
}>(), {
  dark: false,
  compact: false,
  chips: true,
  focus: true,
})

type LaneKey = 'build' | 'check' | 'data' | 'run'

const LANES: { key: LaneKey, name: string, y: number }[] = [
  { key: 'build', name: '01 · BUILD', y: 120 },
  { key: 'check', name: '02 · CHECK', y: 196 },
  { key: 'data', name: '03 · DATA', y: 272 },
  { key: 'run', name: '04 · RUN', y: 348 },
]

const TIERS: {
  code: string
  name: string
  via: string
  x: number
  cells: Record<LaneKey, [string, string]>
}[] = [
  {
    code: 'T0',
    name: 'Unmanaged',
    via: 'Discovery',
    x: 110,
    cells: {
      build: ['Personal laptop & AI chat', 'No shared repo'],
      check: ['Nothing runs', 'Zero automated checks'],
      data: ['Pasted extracts', 'Spreadsheet copies'],
      run: ['Runs on builder laptop', 'Off when closed'],
    },
  },
  {
    code: 'T1',
    name: 'Governed',
    via: 'Cleared to share',
    x: 275,
    cells: {
      build: ['Approved AI editor + template', 'The Codex · Enterprise Git'],
      check: ['Secret scan, dep audit & tests', 'The Codex · Every push'],
      data: ['Synthetic or local data only', 'Air-gapped from live systems'],
      run: ['Isolated team sandbox', 'Best-effort uptime'],
    },
  },
  {
    code: 'T2',
    name: 'Connected',
    via: 'Cleared for live data',
    x: 440,
    cells: {
      build: ['Same workspace + capability contract', 'No raw credentials in code'],
      check: ['Plus contract & schema checks', 'Verified in pipeline'],
      data: ['Live systems via capability contract', 'WASI runtime · Host policy & telemetry'],
      run: ['WASI runtime on shared platform', 'Named business owner + backup'],
    },
  },
  {
    code: 'T3',
    name: 'Supported',
    via: 'Cleared for production',
    x: 605,
    cells: {
      build: ['Spec-driven wave rebuild', 'Keep the criteria · Drop the code'],
      check: ['Requirement coverage traced', 'Integrated-wave verification'],
      data: ['Regulated data by host policy', 'WASI runtime · Explicit capabilities'],
      run: ['Production WASI runtime & SLA', 'Service recipe · Engineers on call'],
    },
  },
]
</script>

<template>
  <div class="ed-diagram-pane">
    <svg viewBox="0 0 720 410" aria-hidden="true">
      <!-- Top Progression Track (T0 -> T1 -> T2 -> T3) -->
      <line x1="110" y1="38" x2="605" y2="38" stroke="rgba(255,255,255,0.2)" stroke-width="2" />
      <line v-if="tier > 0" x1="110" y1="38" :x2="TIERS[tier].x" y2="38" stroke="#78d8f2" stroke-width="2.5" />

      <template v-for="(t, i) in TIERS" :key="t.code">
        <circle
          :cx="t.x"
          cy="38"
          :r="i === tier ? 18 : 11"
          :fill="i === tier ? '#78d8f2' : (i < tier ? '#0d1030' : 'rgba(10,7,16,0.6)')"
          :stroke="i <= tier ? '#78d8f2' : 'rgba(255,255,255,0.28)'"
          stroke-width="2"
        />
        <text
          :x="t.x"
          y="42"
          text-anchor="middle"
          class="svg-pill"
          :fill="i === tier ? '#0d1030' : (i < tier ? '#78d8f2' : 'rgba(255,255,255,0.45)')"
        >
          {{ t.code }}
        </text>
        <text
          :x="t.x"
          y="72"
          text-anchor="middle"
          class="svg-label"
          :fill="i === tier ? '#ffffff' : 'rgba(255,255,255,0.45)'"
        >
          {{ t.name }}
        </text>
      </template>

      <!-- Four Horizontal Control Lanes for the Active Tier -->
      <template v-for="lane in LANES" :key="lane.key">
        <line x1="20" :y1="lane.y" x2="690" :y2="lane.y" stroke="rgba(255,255,255,0.14)" stroke-width="1" />
        <text x="20" :y="lane.y + 30" class="svg-kicker" fill="#78d8f2">{{ lane.name }}</text>

        <!-- Active Tier Setting -->
        <circle cx="156" :cy="lane.y + 32" r="5" fill="#78d8f2" />
        <text x="176" :y="lane.y + 28" class="svg-h3" fill="#ffffff">{{ TIERS[tier].cells[lane.key][0] }}</text>
        <text x="176" :y="lane.y + 48" class="svg-caption" fill="rgba(255,255,255,0.65)">{{ TIERS[tier].cells[lane.key][1] }}</text>

        <!-- Previous Tier Baseline (if tier > 0) -->
        <text
          v-if="tier > 0"
          x="690"
          :y="lane.y + 36"
          text-anchor="end"
          class="svg-caption"
          fill="rgba(255,255,255,0.42)"
        >
          ← T{{ tier - 1 }}: {{ TIERS[tier - 1].cells[lane.key][0] }}
        </text>
      </template>
    </svg>
  </div>
</template>
