<script setup lang="ts">
withDefaults(defineProps<{
  dark?: boolean
}>(), {
  dark: false,
})

const TIERS = [
  { code: 'T0', name: 'Unmanaged' },
  { code: 'T1', name: 'Governed' },
  { code: 'T2', name: 'Connected' },
  { code: 'T3', name: 'Supported' },
]

const ROWS: { name: string, ask: string, icon: string, cells: string[] }[] = [
  {
    name: 'Builder',
    ask: 'The domain expert',
    icon: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0',
    cells: [
      'Explores alone',
      'Builds in the template and owns the trial',
      'Owns the workflow; is, or names, the business owner',
      'Becomes product owner. No pager',
    ],
  },
  {
    name: 'Platform & coaches',
    ask: 'The enabling team',
    icon: 'M4 7h16M4 12h16M4 17h10',
    cells: [
      'Invites. Never punishes',
      'Provides the workspace, the template, the gates',
      'Publishes data contracts; one lightweight review',
      'Runs the production platform',
    ],
  },
  {
    name: 'Engineering',
    ask: 'Senior engineers',
    icon: 'M8 9l-4 3 4 3M16 9l4 3-4 3M13 5l-2 14',
    cells: [
      '—',
      'Sets the template once',
      'Owns the contracts, not the app',
      'Rebuilds from the specification; on call under SLA',
    ],
  },
]
</script>

<template>
  <div class="stack is-owners" :class="{ 'is-dark': dark }">
    <div class="stack-corner" />
    <div v-for="(tier, i) in TIERS" :key="tier.code" class="stack-head is-plain" :class="`is-t${i}`">
      <span class="stack-head-code">{{ tier.code }}</span>
      <span class="stack-head-name">{{ tier.name }}</span>
    </div>

    <template v-for="row in ROWS" :key="row.name">
      <div class="stack-lane">
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
          <path :d="row.icon" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <div>
          <span class="stack-lane-name">{{ row.name }}</span>
          <span class="stack-lane-ask">{{ row.ask }}</span>
        </div>
      </div>
      <div
        v-for="(cell, i) in row.cells"
        :key="row.name + i"
        class="stack-cell is-plain"
        :class="[`is-t${i}`, { 'is-empty': cell === '—', 'is-shift': i === 3 && row.name !== 'Platform & coaches' }]"
      >
        <span class="stack-cell-text">{{ cell }}</span>
      </div>
    </template>
  </div>
</template>
