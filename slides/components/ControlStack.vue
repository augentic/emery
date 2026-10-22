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

const LANES: { key: LaneKey, name: string, ask: string, icon: string }[] = [
  { key: 'build', name: 'Build', ask: 'Who builds it, where, with what', icon: 'M4 17l6-5-6-5M12 19h8' },
  { key: 'check', name: 'Check', ask: 'What runs on every change', icon: 'M12 3l8 3v6c0 4.5-3.4 8-8 9-4.6-1-8-4.5-8-9V6l8-3zM8.5 12l2.5 2.5 4.5-5' },
  { key: 'data', name: 'Data', ask: 'What it can touch', icon: 'M12 3c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3' },
  { key: 'run', name: 'Run', ask: 'Who answers when it breaks', icon: 'M3 12h4l2.5-6 4 12 2.5-6h5' },
]

const TIERS: {
  code: string
  name: string
  via?: string
  cells: Record<LaneKey, string>
  chips?: Partial<Record<LaneKey, string>>
}[] = [
  {
    code: 'T0',
    name: 'Unmanaged',
    cells: {
      build: 'Personal laptop, consumer AI chat',
      check: 'Nothing runs',
      data: 'Pasted extracts, personal keys',
      run: 'Runs when the builder runs it',
    },
  },
  {
    code: 'T1',
    name: 'Governed',
    via: 'Cleared to share',
    cells: {
      build: 'Approved AI editor, one template, Enterprise GitHub',
      check: 'Secret scan, dependency audit, tests on every change',
      data: 'Synthetic or local data only',
      run: 'Isolated sandbox, best effort',
    },
    chips: { build: 'The Codex', check: 'The Codex' },
  },
  {
    code: 'T2',
    name: 'Connected',
    via: 'Cleared for live data',
    cells: {
      build: 'Same workspace; data asked for by contract',
      check: 'Plus contract and schema checks',
      data: 'Live data through scoped, audited capabilities',
      run: 'Platform-hosted, named business owner',
    },
    chips: { data: 'Omnia' },
  },
  {
    code: 'T3',
    name: 'Supported',
    via: 'Cleared for production',
    cells: {
      build: 'Engineers rebuild from the specification',
      check: 'Plus every requirement verified before merge',
      data: 'Least-privilege grants; regulated data allowed',
      run: 'Production runtime, engineering on call',
    },
    chips: { build: 'Emery', check: 'Emery', run: 'Omnia' },
  },
]

function state(index: number, tier: number, focus: boolean) {
  if (!focus)
    return 'is-plain'
  if (index === tier)
    return 'is-current'
  return index < tier ? 'is-passed' : 'is-ahead'
}
</script>

<template>
  <div class="stack" :class="{ 'is-dark': dark, 'is-compact': compact }">
    <div class="stack-corner" />
    <div
      v-for="(t, i) in TIERS"
      :key="t.code"
      class="stack-head"
      :class="[state(i, tier, focus), `is-t${i}`]"
    >
      <span class="stack-head-code">{{ t.code }}</span>
      <span class="stack-head-name">{{ t.name }}</span>
      <span v-if="t.via" class="stack-head-via">{{ t.via }}</span>
      <span v-else class="stack-head-via">Where every app starts</span>
    </div>

    <template v-for="lane in LANES" :key="lane.key">
      <div class="stack-lane">
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
          <path :d="lane.icon" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <div>
          <span class="stack-lane-name">{{ lane.name }}</span>
          <span v-if="!compact" class="stack-lane-ask">{{ lane.ask }}</span>
        </div>
      </div>
      <div
        v-for="(t, i) in TIERS"
        :key="t.code + lane.key"
        class="stack-cell"
        :class="[state(i, tier, focus), `is-t${i}`]"
      >
        <span class="stack-cell-text">{{ t.cells[lane.key] }}</span>
        <span v-if="chips && t.chips?.[lane.key]" class="stack-chip">{{ t.chips[lane.key] }}</span>
      </div>
    </template>
  </div>
</template>
