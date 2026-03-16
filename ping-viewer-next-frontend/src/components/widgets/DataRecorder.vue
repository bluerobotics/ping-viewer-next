<template>
  <button
    class="rec-pill"
    :class="{ 'rec-pill--active': isRecording, 'rec-pill--loading': isLoading }"
    :disabled="isLoading"
    @click="toggleRecording"
  >
    <span class="rec-pill__label">REC</span>
    <span class="rec-pill__dot" :class="{ 'rec-pill__dot--active': isRecording }"></span>
  </button>
</template>

<script setup>
import { computed, inject, ref } from 'vue';

const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
  serverUrl: {
    type: String,
    required: true,
  },
});

const emit = defineEmits(['recording-started', 'recording-stopped']);

const recordingSessions = inject('recordingSessions', ref(new Map()));
const isLoading = ref(false);

const isRecording = computed(() => {
  const session = recordingSessions.value.get(props.device.id);
  return session?.is_active ?? false;
});

const toggleRecording = async () => {
  if (isRecording.value) {
    await stopRecording();
  } else {
    await startRecording();
  }
};

const startRecording = async () => {
  isLoading.value = true;
  try {
    const response = await fetch(
      `${props.serverUrl}/v1/recordings_manager/${props.device.id}/StartRecording`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      }
    );
    if (!response.ok) {
      throw new Error('Failed to start recording');
    }
    emit('recording-started');
  } catch (err) {
    console.error('Error starting recording:', err);
  } finally {
    isLoading.value = false;
  }
};

const stopRecording = async () => {
  isLoading.value = true;
  try {
    const response = await fetch(
      `${props.serverUrl}/v1/recordings_manager/${props.device.id}/StopRecording`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      }
    );
    if (!response.ok) {
      throw new Error('Failed to stop recording');
    }
    emit('recording-stopped');
  } catch (err) {
    console.error('Error stopping recording:', err);
  } finally {
    isLoading.value = false;
  }
};
</script>

<style scoped>
.rec-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  box-sizing: border-box;
  height: 64px;
  padding: 0 1rem;
  border-radius: 9999px;
  border: 2px solid transparent;
  background-color: rgba(var(--v-theme-surface), 0.10);
  backdrop-filter: blur(25px);
  -webkit-backdrop-filter: blur(25px);
  box-shadow: 0px 8px 8px 0px rgba(0, 0, 0, 0.2), 0px 8px 12px 6px rgba(0, 0, 0, 0.09);
  cursor: pointer;
  transition: border-color 0.3s ease, background-color 0.3s ease;
  outline: none;
  user-select: none;
  white-space: nowrap;
}

.rec-pill:hover {
  border-color: rgba(239, 68, 68, 0.6);
  background-color: rgba(var(--v-theme-surface), 0.2);
}

.rec-pill--active {
  border-color: #ef4444;
  animation: pill-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.rec-pill--loading {
  opacity: 0.5;
  cursor: wait;
}

.rec-pill__label {
  font-size: 1.75rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgba(var(--v-theme-on-surface), 0.9);
  line-height: 1;
}

.rec-pill__dot {
  width: 0.75rem;
  height: 0.75rem;
  border-radius: 50%;
  background-color: rgba(var(--v-theme-on-surface), 0.35);
  transition: background-color 0.3s ease;
  flex-shrink: 0;
}

.rec-pill__dot--active {
  background-color: #ef4444;
  animation: dot-pulse 1.5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pill-pulse {
  0%, 100% {
    border-color: #ef4444;
  }
  50% {
    border-color: rgba(239, 68, 68, 0.4);
  }
}

@keyframes dot-pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}
</style>