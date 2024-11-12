<template>
    <div
      class="absolute bottom-5 left-1/2 -translate-x-1/2 z-10"
      @mouseenter="handleShowControls"
      @mouseleave="handleHideControls"
    >
      <div
        class="rounded-full backdrop-blur-sm bg-white/5 border border-white/10 p-1.5 cursor-pointer transition-all duration-500 flex items-center justify-center group hover:bg-white/10 relative"
        :class="{
          'w-auto h-auto': isExpanded,
          'w-8 h-8': !isExpanded,
          'opacity-40 hover:opacity-100': !isExpanded && !isRecording
        }"
      >
        <div
          class="flex items-center gap-2 transition-all duration-500 overflow-hidden text-white/70"
          :class="{
            'opacity-100 w-auto': isExpanded,
            'opacity-0 w-0': !isExpanded
          }"
        >
          <slot></slot>
        </div>

        <div
          class="absolute transition-all duration-500 text-white/70"
          :class="{
            'opacity-0 scale-0': isExpanded,
            'opacity-100 scale-100': !isExpanded
          }"
        >
          <v-icon size="small">mdi-chevron-up</v-icon>
        </div>

        <div
          v-if="isRecording"
          class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-red-400/60 rounded-full animate-pulse shadow-sm"
        ></div>
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
  @keyframes pulse {
    0% {
      opacity: 0.6;
    }
    50% {
      opacity: 0.3;
    }
    100% {
      opacity: 0.6;
    }
  }

  .animate-pulse {
    animation: pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  .transition-all {
    transition: all 0.5s ease-in-out;
  }
  </style>