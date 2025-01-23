<template>
  <div class="absolute bottom-5 left-1/2 -translate-x-1/2 z-10">
    <div class="square-button-container" :class="{ 'expanded': isExpanded, 'glass': true }"
      @mouseenter="handleShowControls" @mouseleave="handleHideControls">
      <div class="button-content" :class="{ 'expanded': isExpanded }">
        <slot></slot>
      </div>

      <div class="expand-icon" :class="{ 'hidden': isExpanded }">
        <v-icon size="small">mdi-chevron-up</v-icon>
      </div>

      <div v-if="isRecording" class="recording-indicator"></div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';

defineProps({
  isRecording: {
    type: Boolean,
    default: false,
  },
});

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
.square-button-container {
  --button-size: 3.25rem;
  --button-gap: 0.5rem;
  --border-radius: 0.5rem;

  width: var(--button-size);
  height: var(--button-size);
  border-radius: var(--border-radius);
  position: relative;
  cursor: pointer;
  transition: all 0.3s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(var(--v-theme-background), 0.1) !important;
  backdrop-filter: blur(1px) !important;
  border: 1px solid rgba(203, 203, 203, 0.25);
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3), 0px 8px 12px 6px rgba(0, 0, 0, 0.15);
}

.square-button-container:hover {
  background-color: rgba(var(--v-theme-background), 0.3) !important;
}

.square-button-container.expanded {
  width: auto;
  height: auto;
  padding: 0.375rem;
}

.button-content {
  display: flex;
  align-items: center;
  gap: var(--button-gap);
  opacity: 0;
  width: 0;
  overflow: hidden;
  transition: all 0.3s ease;
}

.button-content.expanded {
  opacity: 1;
  width: auto;
  overflow: visible;
}

.expand-icon {
  position: absolute;
  transition: all 0.3s ease;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.expand-icon.hidden {
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