<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'full' | 'strip'
  current?: number | number[]
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
    line: 'Live systems, through capability contracts.',
    layers: ['Builder + AI', 'The Codex', 'Capability boundary'],
    share: '~20% of apps',
  },
  {
    code: 'T3',
    name: 'Supported',
    line: 'Production, with engineers on call.',
    layers: ['Builder + AI', 'The Codex', 'Capability boundary', 'Specification + SLA'],
    share: '~5% of apps',
  },
]

const GATES = [
  { name: 'Cleared to share', time: 'Days' },
  { name: 'Cleared for live data', time: 'Weeks' },
  { name: 'Cleared for production', time: 'A funded project' },
]

function state(index: number, current: number | number[] | undefined) {
  if (current === undefined)
    return ''
  if (Array.isArray(current))
    return current.includes(index) ? 'is-current' : 'is-dim'
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

  <div v-else class="ed-band-canvas">
    <svg viewBox="0 0 1140 390" aria-hidden="true">
      <!-- Stepped background silhouette -->
      <path d="M 10 326 L 275 326 L 275 262 L 560 262 L 560 198 L 845 198 L 845 134 L 1130 134 L 1130 378 L 10 378 Z" fill="rgba(22,180,226,0.04)" />

      <!-- Clearance Gate Dividers -->
      <line x1="275" y1="32" x2="275" y2="378" stroke="rgba(13,16,48,0.16)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="560" y1="32" x2="560" y2="378" stroke="rgba(13,16,48,0.16)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="845" y1="32" x2="845" y2="378" stroke="rgba(164,35,143,0.28)" stroke-width="1" stroke-dasharray="4 4" />

      <polygon points="275,20 281,26 275,32 269,26" fill="#0c7fa3" />
      <text x="275" y="14" text-anchor="middle" class="svg-pill" fill="#0c7fa3">CLEARED TO SHARE · DAYS</text>
      <polygon points="560,20 566,26 560,32 554,26" fill="#0c7fa3" />
      <text x="560" y="14" text-anchor="middle" class="svg-pill" fill="#0c7fa3">CLEARED FOR LIVE DATA · WEEKS</text>
      <polygon points="845,20 851,26 845,32 839,26" fill="#a4238f" />
      <text x="845" y="14" text-anchor="middle" class="svg-pill" fill="#a4238f">CLEARED FOR PRODUCTION · FUNDED</text>

      <!-- Tier Column Headers -->
      <text x="24" y="62" class="svg-kicker" fill="#b4234e">T0 · ~0% LONG-TERM</text>
      <text x="24" y="88" class="svg-h2" fill="#0d1030">Unmanaged</text>
      <text x="24" y="108" class="svg-body" fill="rgba(13,16,48,0.58)">One person, one laptop</text>

      <text x="300" y="62" class="svg-kicker" fill="#0c7fa3">T1 · ~75% OF ESTATE</text>
      <text x="300" y="88" class="svg-h2" fill="#0d1030">Governed</text>
      <text x="300" y="108" class="svg-body" fill="rgba(13,16,48,0.58)">Team trial inside the Codex</text>

      <text x="585" y="62" class="svg-kicker" fill="#0c7fa3">T2 · ~20% OF ESTATE</text>
      <text x="585" y="88" class="svg-h2" fill="#0d1030">Connected</text>
      <text x="585" y="108" class="svg-body" fill="rgba(13,16,48,0.58)">Live systems via capability contract</text>

      <text x="870" y="62" class="svg-kicker" fill="#a4238f">T3 · ~5% OF ESTATE</text>
      <text x="870" y="88" class="svg-h2" fill="#0d1030">Supported</text>
      <text x="870" y="108" class="svg-body" fill="rgba(13,16,48,0.58)">Spec-driven rebuild &amp; SLA</text>

      <!-- Cumulative Foundation Bands -->
      <rect x="845" y="138" width="285" height="46" rx="8" fill="rgba(164,35,143,0.1)" stroke="#a4238f" stroke-width="1.8" />
      <text x="865" y="166" class="svg-label" fill="#a4238f">+ Spec-Driven Rebuild &amp; SLA</text>

      <rect x="560" y="202" width="570" height="46" rx="8" fill="#0d1030" />
      <text x="580" y="230" class="svg-label" fill="#ffffff">+ WASI Runtime · Host-Mediated Capability Boundary</text>

      <rect x="275" y="266" width="855" height="46" rx="8" fill="rgba(22,180,226,0.1)" stroke="#0c7fa3" stroke-width="1.6" />
      <text x="295" y="294" class="svg-label" fill="#0c7fa3">+ The Codex · Approved Template, Enterprise Git &amp; Automated Gates</text>

      <rect x="10" y="330" width="1120" height="44" rx="8" fill="#ffffff" stroke="rgba(13,16,48,0.22)" stroke-width="1.4" stroke-dasharray="5 4" />
      <text x="30" y="357" class="svg-body" fill="rgba(13,16,48,0.72)">Domain Builder + AI Assistant (All tiers T0–T3)</text>
    </svg>
  </div>
</template>
