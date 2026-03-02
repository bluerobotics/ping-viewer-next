<template>
  <div class="data-recorder">
    <v-btn
      class="glass-button"
      :class="{ 'pulse': isRecording }"
      icon
      variant="flat"
      size="x-large"
      :loading="isLoading"
      @click="toggleRecording"
    >
      <v-icon :class="{ 'rotate': isRecording }" size="36">
        {{ isRecording ? 'mdi-movie' : 'mdi-movie-outline' }}
      </v-icon>
    </v-btn>
  </div>
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
  .glass-button {
	background-color: rgba(0, 0, 0, 0.10) !important;
	border: 1px solid rgba(255, 255, 255, 0.15) !important;
	backdrop-filter: blur(25px) !important;
	-webkit-backdrop-filter: blur(16px) !important;
	box-shadow: 0px 8px 8px 0px #00000033, 0px 8px 12px 6px #00000016 !important;
  }

  .pulse {
	animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes pulse {
	0%, 100% {
	  opacity: 1;
	}
	50% {
	  opacity: 0.7;
	}
  }

  .rotate {
	animation: rotate 2s linear infinite;
  }

  @keyframes rotate {
	from {
	  transform: rotate(0deg);
	}
	to {
	  transform: rotate(360deg);
	}
  }
  </style>