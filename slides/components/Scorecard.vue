<script setup lang="ts">
import { computed } from 'vue'

type DimKey = 'reach' | 'data' | 'criticality' | 'continuity'

const props = withDefaults(defineProps<{
  mark?: Partial<Record<DimKey, number>>
  subject?: string
}>(), {
  mark: undefined,
  subject: 'This app',
})

const TIERS = [
  { code: 'T0', name: 'Unmanaged', x: 300, color: '#b4234e' },
  { code: 'T1', name: 'Governed', x: 555, color: '#0c7fa3' },
  { code: 'T2', name: 'Connected', x: 810, color: '#0d1030' },
  { code: 'T3', name: 'Supported', x: 1065, color: '#a4238f' },
]

const DIMS: { key: DimKey, num: string, name: string, ask: string, y: number, cells: [string, string][] }[] = [
  {
    key: 'reach',
    num: '01 · REACH',
    name: 'Who uses it?',
    ask: 'Reach',
    y: 92,
    cells: [
      ['1 person', 'The builder alone'],
      ['2–4 people', 'A team trial'],
      ['5–24 people', 'A department'],
      ['25+ or external', 'Cross-team or customers'],
    ],
  },
  {
    key: 'data',
    num: '02 · DATA & STATE',
    name: 'What does it touch?',
    ask: 'Data',
    y: 168,
    cells: [
      ['Personal files', 'Pasted extracts'],
      ['Synthetic or local', 'Nothing live'],
      ['Live internal data', 'Scoped contracts'],
      ['Regulated records', 'PII, finance, safety'],
    ],
  },
  {
    key: 'criticality',
    num: '03 · MONDAY 9AM TEST',
    name: 'If it stops working?',
    ask: 'Criticality',
    y: 244,
    cells: [
      ['Nobody notices', 'A throwaway'],
      ['Mild annoyance', 'Wait for builder'],
      ['A slow day', 'Manual fallback'],
      ['Operations stall', 'Revenue, safety, SLA'],
    ],
  },
  {
    key: 'continuity',
    num: '04 · CONTINUITY',
    name: 'Who sustains it?',
    ask: 'Continuity',
    y: 320,
    cells: [
      ['One laptop', 'No history'],
      ['Enterprise GitHub', 'Automated CI'],
      ['Business owner', 'With a backup'],
      ['Engineers on call', 'Under an SLA'],
    ],
  },
]

const GATES = ['', 'Cleared to share', 'Cleared for live data', 'Cleared for production']

const result = computed(() => {
  if (!props.mark)
    return undefined
  const values = Object.values(props.mark).filter((v): v is number => typeof v === 'number')
  if (values.length === 0)
    return undefined
  return Math.max(...values)
})
</script>

<template>
  <div class="ed-band-canvas">
    <svg viewBox="0 0 1140 390" aria-hidden="true">
      <!-- High-water-mark vertical line when scored -->
      <template v-if="result !== undefined">
        <rect :x="TIERS[result].x - 76" y="4" width="152" height="346" rx="12" fill="rgba(164,35,143,0.06)" />
        <line :x1="TIERS[result].x" y1="38" :x2="TIERS[result].x" y2="348" stroke="#a4238f" stroke-width="1.8" stroke-dasharray="4 4" />
      </template>

      <!-- Tier Column Headers -->
      <template v-for="(tier, i) in TIERS" :key="tier.code">
        <text :x="tier.x" y="18" text-anchor="middle" class="svg-kicker" :fill="tier.color">{{ tier.code }}</text>
        <text :x="tier.x" y="38" text-anchor="middle" class="svg-h3" fill="#0d1030">{{ tier.name }}</text>
        <line v-if="result !== i" :x1="tier.x" y1="48" :x2="tier.x" y2="340" stroke="rgba(13,16,48,0.08)" stroke-width="1" />
      </template>

      <!-- Four Horizontal Gauge Tracks -->
      <template v-for="dim in DIMS" :key="dim.key">
        <text x="12" :y="dim.y - 8" class="svg-kicker" fill="#0c7fa3">{{ dim.num }}</text>
        <text x="12" :y="dim.y + 12" class="svg-h3" fill="#0d1030">{{ dim.name }}</text>

        <!-- Track baseline -->
        <line x1="300" :y1="dim.y - 4" x2="1065" :y2="dim.y - 4" stroke="rgba(13,16,48,0.16)" stroke-width="1.5" />

        <!-- Active progress segment when marked -->
        <line
          v-if="mark?.[dim.key] !== undefined"
          x1="300"
          :y1="dim.y - 4"
          :x2="TIERS[mark[dim.key]!].x"
          :y2="dim.y - 4"
          stroke="#a4238f"
          stroke-width="2.5"
        />

        <!-- 4 Stops along the gauge track -->
        <template v-for="(cell, i) in dim.cells" :key="dim.key + i">
          <circle
            :cx="TIERS[i].x"
            :cy="dim.y - 4"
            :r="mark?.[dim.key] === i ? 7 : 4.5"
            :fill="mark?.[dim.key] === i ? '#a4238f' : '#ffffff'"
            :stroke="mark?.[dim.key] === i ? '#a4238f' : TIERS[i].color"
            stroke-width="2"
          />
          <text
            :x="TIERS[i].x"
            :y="dim.y + 18"
            text-anchor="middle"
            class="svg-label"
            :fill="mark && mark[dim.key] !== i ? 'rgba(13,16,48,0.42)' : '#0d1030'"
          >
            {{ cell[0] }}
          </text>
          <text
            :x="TIERS[i].x"
            :y="dim.y + 34"
            text-anchor="middle"
            class="svg-caption"
            :fill="mark && mark[dim.key] !== i ? 'rgba(13,16,48,0.32)' : 'rgba(13,16,48,0.58)'"
          >
            {{ cell[1] }}
          </text>
        </template>
      </template>

      <!-- Bottom Verdict Line when scored -->
      <template v-if="result !== undefined">
        <line x1="12" y1="364" x2="1128" y2="364" stroke="rgba(13,16,48,0.16)" stroke-width="1" />
        <text x="12" y="384" class="svg-body" fill="rgba(13,16,48,0.58)">The furthest stop to the right sets the tier.</text>
        <text x="1128" y="384" text-anchor="end" class="svg-label" fill="#a4238f">
          {{ subject }} → {{ TIERS[result].code }} {{ TIERS[result].name }} ({{ GATES[result] }})
        </text>
      </template>
    </svg>
  </div>
</template>
