<template>
  <div class="absolute bottom-5 left-1/2 -translate-x-1/2 z-10">
    <div class="pill-container" :class="{ expanded: isExpanded, glass: isGlass }"
      @mouseenter="handleShowControls" @mouseleave="handleHideControls">
      <div class="pill-content" :class="{ expanded: isExpanded }">
        <slot></slot>
      </div>

      <div class="pill-hint" :class="{ hidden: isExpanded }">
        <v-icon size="small">mdi-chevron-up</v-icon>
      </div>

      <div v-if="isRecording" class="recording-indicator"></div>
    </div>
  </div>
</template>

<script setup>
import { computed, inject, ref } from 'vue';

defineProps({
  isRecording: {
    type: Boolean,
    default: false,
  },
});

const glass = inject('glass', ref(false));
const isGlass = computed(() => glass.value);

const isExpanded = ref(false);
const expansionTimeout = ref(null);

const handleShowControls = () => {
  if (expansionTimeout.value) clearTimeout(expansionTimeout.value);
  isExpanded.value = true;
};

const handleHideControls = () => {
  expansionTimeout.value = setTimeout(() => {
    isExpanded.value = false;
  }, 500);
};
</script>

<style scoped>
.pill-container {
  width: 3.25rem;
  height: 3.25rem;
  border-radius: 9999px;
  position: relative;
  cursor: pointer;
  transition: all 0.3s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgb(var(--v-theme-surface));
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.3), 0 4px 8px 3px rgba(0, 0, 0, 0.15);
}

.pill-container.glass {
  background-color: rgba(var(--v-theme-background), 0.5) !important;
  backdrop-filter: blur(25px) !important;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
}

.pill-container.expanded {
  width: auto;
  height: auto;
  padding: 0.375rem;
}

.pill-content {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  opacity: 0;
  width: 0;
  overflow: hidden;
  transition: all 0.3s ease;
}

.pill-content.expanded {
  opacity: 1;
  width: auto;
  overflow: visible;
}

.pill-hint {
  position: absolute;
  transition: all 0.3s ease;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.pill-hint.hidden {
  opacity: 0;
  transform: scale(0);
}

.recording-indicator {
  position: absolute;
  top: -0.125rem;
  right: -0.125rem;
  width: 0.5rem;
  height: 0.5rem;
  background-color: rgba(239, 68, 68, 0.6);
  border-radius: 9999px;
  animation: pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 0.6;
  }
  50% {
    opacity: 0.3;
  }
}
</style>
