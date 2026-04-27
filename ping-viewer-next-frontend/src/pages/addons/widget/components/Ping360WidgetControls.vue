<template>
  <div class="ping360-widget-controls">
    <div v-if="!isOpen" class="open-toggle-wrapper" :class="{ 'is-recording': isRecording }">
      <v-btn
        class="chevron-btn"
        icon
        variant="text"
        size="small"
        aria-label="Open controls"
        @click="isOpen = true"
      >
        <v-icon size="22">mdi-chevron-right</v-icon>
      </v-btn>
    </div>

    <div v-else class="control-card">
      <div class="controls-column">
        <div class="control-pill">
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Decrease range" @click="emitAction('step_range', 'down')">
            <v-icon size="18">mdi-minus</v-icon>
          </v-btn>
          <span class="pill-label">Range</span>
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Increase range" @click="emitAction('step_range', 'up')">
            <v-icon size="18">mdi-plus</v-icon>
          </v-btn>
        </div>

        <div class="control-pill">
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Decrease sector" @click="emitAction('step_sector', 'down')">
            <v-icon size="18">mdi-minus</v-icon>
          </v-btn>
          <span class="pill-label">Sector</span>
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Increase sector" @click="emitAction('step_sector', 'up')">
            <v-icon size="18">mdi-plus</v-icon>
          </v-btn>
        </div>

        <div class="control-pill">
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Decrease gain" @click="emitAction('decrease_gain')">
            <v-icon size="18">mdi-minus</v-icon>
          </v-btn>
          <span class="pill-label">Gain</span>
          <v-btn class="pill-btn" icon variant="text" size="small"
            aria-label="Increase gain" @click="emitAction('increase_gain')">
            <v-icon size="18">mdi-plus</v-icon>
          </v-btn>
        </div>

        <v-btn
          class="rec-pill"
          :class="{ 'is-recording': isRecording }"
          variant="text"
          @click="emitAction('toggle_recording')"
        >
          <span class="rec-label">REC</span>
          <span class="rec-circle" />
        </v-btn>
      </div>

      <v-btn
        class="chevron-btn collapse-btn"
        icon
        variant="text"
        size="small"
        aria-label="Collapse controls"
        @click="isOpen = false"
      >
        <v-icon size="22">mdi-chevron-left</v-icon>
      </v-btn>
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

const emit = defineEmits(['button-click']);

const isOpen = ref(false);

const emitAction = (action, value) => {
  emit('button-click', { action, value, id: action });
};
</script>

<style scoped>
.ping360-widget-controls {
  position: absolute;
  inset: 0;
  z-index: 200;
  pointer-events: none;
}

.open-toggle-wrapper {
  position: absolute;
  top: 8px;
  left: 8px;
  display: inline-block;
  pointer-events: auto;
}

.open-toggle-wrapper.is-recording::before {
  content: '';
  position: absolute;
  inset: -4px;
  border-radius: 50%;
  border: 2px solid #f44336;
  box-shadow: 0 0 8px rgba(244, 67, 54, 0.8);
  pointer-events: none;
  animation: open-toggle-flash 1s ease-in-out infinite;
  z-index: 1;
}

@keyframes open-toggle-flash {
  0% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.25; transform: scale(1.15); }
  100% { opacity: 1; transform: scale(1); }
}

.chevron-btn {
  width: 36px !important;
  height: 36px !important;
  min-width: 36px !important;
  border-radius: 50% !important;
  background-color: rgba(0, 0, 0, 0.55) !important;
  color: #fff !important;
  border: 1px solid rgba(255, 255, 255, 0.2) !important;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.35);
  pointer-events: auto !important;
}

.collapse-btn {
  align-self: flex-start;
  pointer-events: auto !important;
}

.control-card {
  position: absolute;
  top: 8px;
  left: 8px;
  display: inline-flex;
  align-items: flex-start;
  gap: 8px;
  background: transparent;
  pointer-events: auto;
  padding: 0;
}

.controls-column {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  pointer-events: auto;
}

.control-pill {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 160px;
  height: 44px;
  padding: 0 5px;
  background-color: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 999px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  pointer-events: auto;
}

.pill-btn {
  width: 32px !important;
  height: 32px !important;
  min-width: 32px !important;
  border-radius: 50% !important;
  background-color: rgba(255, 255, 255, 0.08) !important;
  color: #fff !important;
  border: 1px solid rgba(255, 255, 255, 0.25) !important;
  flex-shrink: 0;
  position: relative;
  z-index: 2;
  pointer-events: auto !important;
  cursor: pointer;
}

.pill-label {
  flex: 1;
  text-align: center;
  font-size: 14px;
  font-weight: 500;
  color: #fff;
  user-select: none;
  letter-spacing: 0.4px;
  pointer-events: none;
}

.rec-pill {
  width: 110px !important;
  height: 36px !important;
  min-width: 110px !important;
  padding: 0 14px !important;
  border-radius: 999px !important;
  background-color: rgba(0, 0, 0, 0.55) !important;
  color: #fff !important;
  border: 1px solid rgba(255, 255, 255, 0.25) !important;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.35);
  text-transform: none;
  letter-spacing: 0.5px;
  position: relative;
  z-index: 2;
  pointer-events: auto !important;
  cursor: pointer;
}

.rec-pill.is-recording {
  background-color: rgba(244, 67, 54, 0.55) !important;
  border-color: rgba(244, 67, 54, 0.85) !important;
}

.rec-pill :deep(.v-btn__content) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.rec-label {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.5px;
  pointer-events: none;
}

.rec-circle {
  width: 8px;
  height: 8px;
  margin-left: 8px;
  border-radius: 50%;
  background-color: rgba(255, 255, 255, 0.55);
  transition: background-color 0.2s ease, box-shadow 0.2s ease;
  pointer-events: none;
}

.rec-pill.is-recording .rec-circle {
  background-color: #fff;
  box-shadow: 0 0 6px rgba(255, 255, 255, 0.85);
  animation: rec-pulse 1.2s ease-in-out infinite;
}

@keyframes rec-pulse {
  0% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.45; transform: scale(0.8); }
  100% { opacity: 1; transform: scale(1); }
}
</style>
