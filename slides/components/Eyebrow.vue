<script setup lang="ts">
import { computed } from 'vue'
import { useSlideContext } from '@slidev/client'

defineProps<{
  text: string
}>()

const { $page, $nav } = useSlideContext()

const pad = (n: number) => String(n).padStart(2, '0')
const counter = computed(() => `${pad($page.value)} / ${pad($nav.value.total)}`)
</script>

<template>
  <h6 class="slide-eyebrow">
    {{ text }}
    <span class="eyebrow-num">{{ counter }}</span>
  </h6>
</template>
