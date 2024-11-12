<template>
	<div class="data-recorder">
	  <v-btn
		:color="isRecording ? 'error' : 'primary'"
		:class="{ 'pulse': isRecording }"
		icon
		elevation="2"
		size="large"
		@click="toggleRecording"
	  >
		<v-icon :class="{ 'rotate': isRecording }" size="large">
		  {{ isRecording ? 'mdi-movie' : 'mdi-movie-outline' }}
		</v-icon>
	  </v-btn>
	</div>
  </template>

  <script setup>
import { ref } from 'vue';

const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
});

const emit = defineEmits(['recording-complete']);

const isRecording = ref(false);
const recordedData = ref([]);
const recordingStartTime = ref(null);

const toggleRecording = () => {
  if (!isRecording.value) {
    startRecording();
  } else {
    stopRecording();
  }
};

const startRecording = () => {
  isRecording.value = true;
  recordedData.value = [];
  recordingStartTime.value = Date.now();
};

const stopRecording = () => {
  if (recordedData.value.length === 0) {
    isRecording.value = false;
    return;
  }

  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const fileName = `recorded_data_${props.device.device_type}_${props.device.id}_${timestamp}.json`;

  const recordingData = {
    id: Date.now(),
    fileName,
    timestamp: new Date().toISOString(),
    deviceType: props.device.device_type,
    deviceId: props.device.id,
    data: recordedData.value,
    downloaded: false,
  };

  emit('recording-complete', recordingData);
  isRecording.value = false;
  recordedData.value = [];
};

const recordData = (data) => {
  if (isRecording.value) {
    const frame = {
      timestamp: new Date().toISOString(),
      device: {
        id: props.device.id,
        device_type: props.device.device_type,
        source: props.device.source,
      },
      data: data,
    };
    recordedData.value.push(frame);
  }
};

defineExpose({ recordData });
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