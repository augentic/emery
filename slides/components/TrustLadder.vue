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
    line: 'Live systems, through capability contracts.',
    layers: ['Builder + AI', 'The Codex', 'Capability boundary'],
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

  <div v-else class="ed-band-canvas">
    <svg viewBox="0 0 1140 390" aria-hidden="true">
      <path d="M 10 340 L 275 340 L 275 266 L 560 266 L 560 192 L 845 192 L 845 118 L 1130 118 L 1130 368 L 10 368 Z" fill="rgba(22,180,226,0.045)" />
      <path d="M 10 340 L 275 340 L 275 266 L 560 266 L 560 192 L 845 192 L 845 118 L 1130 118" fill="none" stroke="#0c7fa3" stroke-width="2" />
      <line x1="275" y1="22" x2="275" y2="368" stroke="rgba(13,16,48,0.16)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="560" y1="22" x2="560" y2="368" stroke="rgba(13,16,48,0.16)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="845" y1="22" x2="845" y2="368" stroke="rgba(164,35,143,0.28)" stroke-width="1" stroke-dasharray="4 4" />
      <polygon points="275,14 281,20 275,26 269,20" fill="#0c7fa3" />
      <text x="275" y="8" text-anchor="middle" class="svg-pill" fill="#0c7fa3">CLEARED TO SHARE · DAYS</text>
      <polygon points="560,14 566,20 560,26 554,20" fill="#0c7fa3" />
      <text x="560" y="8" text-anchor="middle" class="svg-pill" fill="#0c7fa3">CLEARED FOR LIVE DATA · WEEKS</text>
      <polygon points="845,14 851,20 845,26 839,20" fill="#a4238f" />
      <text x="845" y="8" text-anchor="middle" class="svg-pill" fill="#a4238f">CLEARED FOR PRODUCTION · FUNDED</text>
      <text x="24" y="56" class="svg-kicker" fill="#b4234e">T0 · ~0% LONG-TERM</text>
      <text x="24" y="82" class="svg-h2" fill="#0d1030">Unmanaged</text>
      <text x="24" y="102" class="svg-body" fill="rgba(13,16,48,0.58)">One person, one laptop</text>
      <text x="300" y="56" class="svg-kicker" fill="#0c7fa3">T1 · ~75% OF ESTATE</text>
      <text x="300" y="82" class="svg-h2" fill="#0d1030">Governed</text>
      <text x="300" y="102" class="svg-body" fill="rgba(13,16,48,0.58)">Team trial inside the Codex</text>
      <text x="585" y="56" class="svg-kicker" fill="#0c7fa3">T2 · ~20% OF ESTATE</text>
      <text x="585" y="82" class="svg-h2" fill="#0d1030">Connected</text>
      <text x="585" y="102" class="svg-body" fill="rgba(13,16,48,0.58)">Live systems by capability contract</text>
      <text x="870" y="56" class="svg-kicker" fill="#a4238f">T3 · ~5% OF ESTATE</text>
      <text x="870" y="82" class="svg-h2" fill="#0d1030">Supported</text>
      <text x="870" y="102" class="svg-body" fill="rgba(13,16,48,0.58)">Verified rebuild &amp; SLA</text>
      <line x1="845" y1="155" x2="1130" y2="155" stroke="#a4238f" stroke-width="2.5" />
      <circle cx="845" cy="155" r="4" fill="#a4238f" />
      <text x="865" y="145" class="svg-label" fill="#a4238f">Specification + Engineering SLA (Emery)</text>
      <line x1="560" y1="229" x2="1130" y2="229" stroke="#0d1030" stroke-width="2" />
      <circle cx="560" cy="229" r="4" fill="#0d1030" />
      <text x="580" y="219" class="svg-label" fill="#0d1030">Host-Mediated Capability Boundary (Omnia)</text>
      <line x1="275" y1="303" x2="1130" y2="303" stroke="#0c7fa3" stroke-width="2" />
      <circle cx="275" cy="303" r="4" fill="#0c7fa3" />
      <text x="295" y="293" class="svg-label" fill="#0c7fa3">The Codex — Approved Template, Enterprise Git &amp; Automated Gates</text>
      <line x1="10" y1="368" x2="1130" y2="368" stroke="rgba(13,16,48,0.3)" stroke-width="1.5" stroke-dasharray="5 5" />
      <circle cx="10" cy="368" r="3.5" fill="#b4234e" />
      <text x="28" y="358" class="svg-body" fill="rgba(13,16,48,0.65)">Domain Builder + AI Assistant (All tiers T0–T3)</text>
    </svg>
  </div>
</template>
