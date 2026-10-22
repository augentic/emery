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
  { code: 'T0', name: 'Unmanaged' },
  { code: 'T1', name: 'Governed' },
  { code: 'T2', name: 'Connected' },
  { code: 'T3', name: 'Supported' },
]

const DIMS: { key: DimKey, num: string, name: string, ask: string, cells: [string, string][] }[] = [
  {
    key: 'reach',
    num: '01',
    name: 'Reach',
    ask: 'Who uses it?',
    cells: [
      ['1 person', 'The builder'],
      ['2–4 people', 'A team trial'],
      ['5–24 people', 'A department'],
      ['25+ or external', 'Cross-team or customers'],
    ],
  },
  {
    key: 'data',
    num: '02',
    name: 'Data & state',
    ask: 'What does it touch?',
    cells: [
      ['Personal files', 'Pasted extracts'],
      ['Synthetic or local', 'Nothing live'],
      ['Live internal data', 'Through scoped contracts'],
      ['Regulated or core records', 'PII, finance, safety'],
    ],
  },
  {
    key: 'criticality',
    num: '03',
    name: 'Monday 9am test',
    ask: 'If it stops working?',
    cells: [
      ['Nobody notices', 'A throwaway'],
      ['A mild annoyance', 'Wait for the builder'],
      ['A slow day', 'Manual fallback'],
      ['Operations stall', 'Revenue, safety, SLA'],
    ],
  },
  {
    key: 'continuity',
    num: '04',
    name: 'Continuity',
    ask: 'Who sustains it?',
    cells: [
      ['One laptop', 'No history'],
      ['Enterprise GitHub', 'Automated checks'],
      ['Named business owner', 'With a backup'],
      ['Engineering on call', 'Under an SLA'],
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
  <div class="score" :class="{ 'has-mark': mark }">
    <div class="score-corner" />
    <div v-for="(tier, i) in TIERS" :key="tier.code" class="score-head" :class="[`is-t${i}`, { 'is-result': result === i }]">
      <span class="score-head-code">{{ tier.code }}</span>
      <span class="score-head-name">{{ tier.name }}</span>
    </div>

    <template v-for="dim in DIMS" :key="dim.key">
      <div class="score-dim">
        <span class="score-dim-num">{{ dim.num }}</span>
        <span class="score-dim-name">{{ dim.name }}</span>
        <span class="score-dim-ask">{{ dim.ask }}</span>
      </div>
      <div
        v-for="(cell, i) in dim.cells"
        :key="dim.key + i"
        class="score-cell"
        :class="[`is-t${i}`, { 'is-mark': mark?.[dim.key] === i }]"
      >
        <strong>{{ cell[0] }}</strong>
        <span>{{ cell[1] }}</span>
      </div>
    </template>

    <div v-if="result !== undefined" class="score-result">
      <span class="score-result-rule">The highest column any row reaches sets the tier.</span>
      <span class="score-result-verdict">
        {{ subject }} → <strong>{{ TIERS[result].code }} {{ TIERS[result].name }}</strong>
        <template v-if="result > 0"> · {{ GATES[result] }}</template>
      </span>
    </div>
  </div>
</template>
