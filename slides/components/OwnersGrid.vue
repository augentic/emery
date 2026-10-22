<script setup lang="ts">
withDefaults(defineProps<{
  dark?: boolean
}>(), {
  dark: false,
})
</script>

<template>
  <div class="ed-band-canvas">
    <svg viewBox="0 0 1140 365" aria-hidden="true">
      <defs>
        <linearGradient id="eng-ramp" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stop-color="#0c7fa3" stop-opacity="0.08" />
          <stop offset="68%" stop-color="#0c7fa3" stop-opacity="0.14" />
          <stop offset="100%" stop-color="#a4238f" stop-opacity="0.28" />
        </linearGradient>
      </defs>

      <!-- Vertical Tier Guide Lines -->
      <line x1="290" y1="36" x2="290" y2="340" stroke="rgba(13,16,48,0.12)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="555" y1="36" x2="555" y2="340" stroke="rgba(13,16,48,0.12)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="820" y1="36" x2="820" y2="340" stroke="rgba(13,16,48,0.12)" stroke-width="1" stroke-dasharray="4 4" />
      <line x1="1085" y1="36" x2="1085" y2="340" stroke="rgba(164,35,143,0.28)" stroke-width="1.2" stroke-dasharray="4 4" />

      <!-- Tier Column Headers -->
      <text x="290" y="20" text-anchor="middle" class="svg-kicker" fill="#b4234e">T0 · UNMANAGED</text>
      <text x="555" y="20" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T1 · GOVERNED (~75%)</text>
      <text x="820" y="20" text-anchor="middle" class="svg-kicker" fill="#0c7fa3">T2 · CONNECTED (~20%)</text>
      <text x="1085" y="20" text-anchor="middle" class="svg-kicker" fill="#a4238f">T3 · SUPPORTED (~5%)</text>

      <!-- Swimlane 01: Builder -->
      <text x="12" y="84" class="svg-kicker" fill="#0c7fa3">01 · DOMAIN BUILDER</text>
      <text x="12" y="104" class="svg-h3" fill="#0d1030">Never loses the app</text>
      <line x1="290" y1="92" x2="1085" y2="92" stroke="#0c7fa3" stroke-width="2" />
      <circle cx="290" cy="92" r="5" fill="#ffffff" stroke="#b4234e" stroke-width="2" />
      <text x="290" y="118" text-anchor="middle" class="svg-label" fill="#0d1030">Explores alone</text>
      <text x="290" y="134" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Personal prototype</text>

      <circle cx="555" cy="92" r="5" fill="#0c7fa3" />
      <text x="555" y="118" text-anchor="middle" class="svg-label" fill="#0d1030">Owns the team trial</text>
      <text x="555" y="134" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Builds inside template</text>

      <circle cx="820" cy="92" r="5" fill="#0c7fa3" />
      <text x="820" y="118" text-anchor="middle" class="svg-label" fill="#0d1030">Business owner</text>
      <text x="820" y="134" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Owns workflow + backup</text>

      <circle cx="1085" cy="92" r="7" fill="#a4238f" />
      <text x="1085" y="118" text-anchor="middle" class="svg-label" fill="#a4238f">Product Owner</text>
      <text x="1085" y="134" text-anchor="middle" class="svg-caption" fill="#0d1030">Owns backlog · No pager</text>

      <!-- Swimlane 02: Platform & Coaches -->
      <text x="12" y="192" class="svg-kicker" fill="#0c7fa3">02 · PLATFORM &amp; COACHES</text>
      <text x="12" y="212" class="svg-h3" fill="#0d1030">Enables &amp; coaches</text>
      <line x1="290" y1="200" x2="1085" y2="200" stroke="#0d1030" stroke-width="1.8" stroke-dasharray="5 4" />
      <circle cx="290" cy="200" r="4.5" fill="#0d1030" />
      <text x="290" y="226" text-anchor="middle" class="svg-label" fill="#0d1030">Invites in</text>
      <text x="290" y="242" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Never punishes</text>

      <circle cx="555" cy="200" r="4.5" fill="#0d1030" />
      <text x="555" y="226" text-anchor="middle" class="svg-label" fill="#0d1030">Runs the Codex</text>
      <text x="555" y="242" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Workspace, template &amp; gates</text>

      <circle cx="820" cy="200" r="4.5" fill="#0d1030" />
      <text x="820" y="226" text-anchor="middle" class="svg-label" fill="#0d1030">One contract review</text>
      <text x="820" y="242" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Publishes domain contracts</text>

      <circle cx="1085" cy="200" r="4.5" fill="#0d1030" />
      <text x="1085" y="226" text-anchor="middle" class="svg-label" fill="#0d1030">Runs Omnia platform</text>
      <text x="1085" y="242" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Runtime &amp; audit plane</text>

      <!-- Swimlane 03: Senior Engineering (Stepped Effort Wedge) -->
      <text x="12" y="300" class="svg-kicker" fill="#a4238f">03 · SENIOR ENGINEERING</text>
      <text x="12" y="320" class="svg-h3" fill="#0d1030">Where the risk is</text>
      <path d="M 555 308 L 820 298 L 1085 272 L 1085 308 Z" fill="url(#eng-ramp)" />
      <line x1="290" y1="308" x2="555" y2="308" stroke="rgba(13,16,48,0.2)" stroke-width="1.5" stroke-dasharray="3 4" />
      <path d="M 555 308 L 820 298 L 1085 272" fill="none" stroke="#a4238f" stroke-width="2.2" />

      <circle cx="290" cy="308" r="3.5" fill="rgba(13,16,48,0.28)" />
      <text x="290" y="334" text-anchor="middle" class="svg-body" fill="rgba(13,16,48,0.45)">Zero involvement</text>

      <circle cx="555" cy="308" r="5" fill="#0c7fa3" />
      <text x="555" y="334" text-anchor="middle" class="svg-label" fill="#0d1030">Sets template once</text>
      <text x="555" y="350" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Zero per-app time</text>

      <circle cx="820" cy="298" r="5.5" fill="#0d1030" />
      <text x="820" y="334" text-anchor="middle" class="svg-label" fill="#0d1030">Owns data contracts</text>
      <text x="820" y="350" text-anchor="middle" class="svg-caption" fill="rgba(13,16,48,0.55)">Once per domain, not per app</text>

      <circle cx="1085" cy="272" r="8" fill="#a4238f" />
      <text x="1085" y="334" text-anchor="middle" class="svg-label" fill="#a4238f">Rebuilds &amp; takes pager</text>
      <text x="1085" y="350" text-anchor="middle" class="svg-caption" fill="#0d1030">Verified Emery build + SLA</text>
    </svg>
  </div>
</template>
