<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: 'cyan' | 'magenta' | 'ink' | 'light'
    size?: 'small' | 'medium' | 'large'
  }>(),
  {
    tone: 'cyan',
    size: 'medium',
  },
)
</script>

<template>
  <span
    class="concept-icon"
    :class="[`is-${tone}`, `is-${size}`]"
    aria-hidden="true"
  >
    <slot />
  </span>
</template>

<style scoped>
.concept-icon {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(12, 127, 163, 0.26);
  border-radius: 10px;
  background: rgba(22, 180, 226, 0.08);
  color: var(--phl-cyan-ink);
}

.concept-icon.is-small {
  width: 30px;
  height: 30px;
  border-radius: 8px;
}

.concept-icon.is-medium {
  width: 38px;
  height: 38px;
}

.concept-icon.is-large {
  width: 46px;
  height: 46px;
  border-radius: 12px;
}

.concept-icon.is-magenta {
  border-color: rgba(164, 35, 143, 0.26);
  background: rgba(164, 35, 143, 0.08);
  color: #a4238f;
}

.concept-icon.is-ink {
  border-color: rgba(13, 16, 48, 0.16);
  background: var(--phl-bg-navy);
  color: #ffffff;
}

.concept-icon.is-light {
  border-color: rgba(255, 255, 255, 0.24);
  background: rgba(255, 255, 255, 0.1);
  color: var(--phl-cyan-soft);
}

.concept-icon :deep(svg) {
  width: 58%;
  height: 58%;
}
</style>
