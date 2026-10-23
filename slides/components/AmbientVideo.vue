<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useNav } from '@slidev/client'

defineProps<{
  src: string
  poster?: string
  variant?: 'hero' | 'approach' | 'split'
}>()

const videoRef = ref<HTMLVideoElement | null>(null)
const { currentSlideNo } = useNav()

function ensurePlaying() {
  const el = videoRef.value
  if (!el)
    return
  el.defaultMuted = true
  el.muted = true
  el.loop = true
  el.playsInline = true
  el.setAttribute('muted', '')
  el.setAttribute('playsinline', '')
  if (el.paused && el.offsetWidth > 0) {
    el.play().catch(() => {})
  }
}

watch(currentSlideNo, () => {
  requestAnimationFrame(ensurePlaying)
  setTimeout(ensurePlaying, 100)
  setTimeout(ensurePlaying, 350)
})

let timer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  ensurePlaying()
  timer = setInterval(ensurePlaying, 800)
  window.addEventListener('pointerdown', ensurePlaying, { passive: true })
  window.addEventListener('keydown', ensurePlaying, { passive: true })
})

onUnmounted(() => {
  if (timer)
    clearInterval(timer)
  window.removeEventListener('pointerdown', ensurePlaying)
  window.removeEventListener('keydown', ensurePlaying)
})
</script>

<template>
  <div
    class="ambient-video-wrap"
    :class="[
      variant === 'approach' ? 'is-approach' : '',
      variant === 'split' ? 'is-split' : 'is-hero',
    ]"
  >
    <video
      ref="videoRef"
      class="ambient-video-el"
      autoplay
      loop
      muted
      playsinline
      preload="auto"
      :poster="poster || ''"
      :src="src"
      @loadeddata="ensurePlaying"
      @canplay="ensurePlaying"
    />
    <div class="ambient-video-scrim" />
  </div>
</template>
