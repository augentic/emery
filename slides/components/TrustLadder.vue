<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'full' | 'strip'
  current?: number
  share?: boolean
}>(), {
  variant: 'full',
  current: undefined,
  share: true,
})

const TIERS = [
  {
    code: 'T0',
    name: 'Unmanaged',
    line: 'One person, one laptop. Discovery.',
    layers: ['Builder + AI'],
    share: 'None left',
  },
  {
    code: 'T1',
    name: 'Governed',
    line: 'Shared with a team, inside the Codex.',
    layers: ['Builder + AI', 'The Codex'],
    share: '~75% of apps',
  },
  {
    code: 'T2',
    name: 'Connected',
    line: 'Live data, through scoped contracts.',
    layers: ['Builder + AI', 'The Codex', 'Data plane'],
    share: '~20% of apps',
  },
  {
    code: 'T3',
    name: 'Supported',
    line: 'Production, with engineers on call.',
    layers: ['Builder + AI', 'The Codex', 'Data plane', 'Specification + SLA'],
    share: '~5% of apps',
  },
]

const GATES = [
  { name: 'Cleared to share', time: 'Days' },
  { name: 'Cleared for live data', time: 'Weeks' },
  { name: 'Cleared for production', time: 'A funded project' },
]

function state(index: number, current: number | undefined) {
  if (current === undefined)
    return ''
  return index === current ? 'is-current' : 'is-dim'
}
</script>

<template>
  <div v-if="variant === 'strip'" class="ladder-strip">
    <template v-for="(tier, i) in TIERS" :key="tier.code">
      <div class="ladder-pill" :class="state(i, current)">
        <span class="ladder-pill-code">{{ tier.code }}</span>
        <span class="ladder-pill-name">{{ tier.name }}</span>
      </div>
      <svg v-if="i < TIERS.length - 1" class="ladder-sep" width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
        <path d="M4 2l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </template>
  </div>

  <div v-else class="ladder" :class="{ 'has-share': share }">
    <div class="ladder-steps">
      <template v-for="(tier, i) in TIERS" :key="tier.code">
        <div class="ladder-col" :class="[`is-t${i}`, state(i, current)]">
          <div class="ladder-head">
            <span class="ladder-code">{{ tier.code }}</span>
            <h3>{{ tier.name }}</h3>
            <p>{{ tier.line }}</p>
          </div>
          <div class="ladder-stack">
            <div
              v-for="(layer, j) in [...tier.layers].reverse()"
              :key="layer"
              class="ladder-layer"
              :class="{ 'is-top': j === 0 && i > 0, 'is-bare': i === 0 }"
            >
              {{ layer }}
            </div>
          </div>
        </div>
        <div v-if="i < TIERS.length - 1" class="ladder-gate">
          <span class="ladder-gate-name">{{ GATES[i].name }}</span>
          <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 12h14M13 6l6 6-6 6" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          <span class="ladder-gate-time">{{ GATES[i].time }}</span>
        </div>
      </template>
    </div>
    <div v-if="share" class="ladder-share">
      <span class="ladder-share-label">A healthy estate</span>
      <div class="ladder-share-cells">
        <span v-for="(tier, i) in TIERS" :key="tier.code" :class="`is-t${i}`">{{ tier.share }}</span>
      </div>
    </div>
  </div>
</template>
