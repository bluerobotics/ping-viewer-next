<template>
  <div class="data-recorder">
    <v-btn
      :color="isRecording ? 'error' : 'primary'"
      :class="{ 'pulse': isRecording }"
      icon
      elevation="2"
      size="large"
      :loading="isLoading"
      @click="toggleRecording"
    >
      <v-icon :class="{ 'rotate': isRecording }" size="large">
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