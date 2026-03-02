<template>
  <div class="flex flex-col h-full relative">
    <FloatingControls v-if="showControls" :is-recording="isRecording">
      <DataRecorder :device="device" :server-url="serverUrl"
        @recording-started="handleRecordingStarted" @recording-stopped="handleRecordingStopped" />
      <v-btn icon @click="toggleFreeze" class="glass-button" size="x-large">
        <v-icon :color="isFreeze ? '#ef4444' : '#ffffff'" size="36">{{ isFreeze ? 'mdi-play' : 'mdi-pause' }}</v-icon>
      </v-btn>
      <v-btn icon @click="openSettings" class="glass-button" size="x-large">
        <v-icon color="#ffffff" size="36">mdi-cog</v-icon>
      </v-btn>
      <v-dialog v-model="isSettingsOpen" max-width="300px">
        <Ping1DSettings :isOpen="isSettingsOpen" :server-url="serverUrl" :device-id="device.id" />
      </v-dialog>
    </FloatingControls>

    <Ping1D v-bind="$props" :sensorData="displayData.sensorData" :currentDepth="displayData.currentDepth"
      :minDepth="displayData.minDepth" :maxDepth="displayData.maxDepth" :confidence="displayData.confidence"
      :accuracy="displayData.accuracy" class="flex-grow" />
  </div>
</template>

<script setup>
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue';
import DataRecorder from '../DataRecorder.vue';
import FloatingControls from '../FloatingControls.vue';
import Ping1D from './Ping1D.vue';
import Ping1DSettings from './Ping1DSettings.vue';

const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
  websocketUrl: {
    type: String,
    required: true,
  },
  width: {
    type: Number,
    default: 600,
  },
  height: {
    type: Number,
    default: 400,
  },
  colorPalette: {
    type: String,
    default: 'Thermal Blue',
  },
  debug: {
    type: Boolean,
    default: false,
  },
  depthLineColor: {
    type: String,
    default: 'yellow',
  },
  depthTextColor: {
    type: String,
    default: 'yellow',
  },
  currentDepthColor: {
    type: String,
    default: 'yellow',
  },
  confidenceColor: {
    type: String,
    default: 'green',
  },
  textBackground: {
    type: String,
    default: 'rgba(0, 0, 0, 0.5)',
  },
  depthArrowColor: {
    type: String,
    default: 'yellow',
  },
  tickCount: {
    type: Number,
    default: 5,
  },
  columnCount: {
    type: Number,
    default: 100,
  },
  showControls: {
    type: Boolean,
    default: true,
  },
  aScan: {
    type: Boolean,
    default: true,
  },
});

const socket = ref(null);
const isFreeze = ref(false);
const isSettingsOpen = ref(false);

const recordingSessions = inject('recordingSessions', ref(new Map()));
const isRecording = computed(() => {
  const session = recordingSessions.value.get(props.device.id);
  return session?.is_active ?? false;
});

const liveData = ref({
  sensorData: [],
  currentDepth: 0,
  minDepth: 0,
  maxDepth: 0,
  confidence: 0,
  accuracy: 0,
});

const displayData = ref({
  sensorData: [],
  currentDepth: 0,
  minDepth: 0,
  maxDepth: 0,
  confidence: 0,
  accuracy: 0,
});

const serverUrl = computed(() => {
  try {
    const url = new URL(props.websocketUrl);
    return `http${url.protocol === 'wss:' ? 's' : ''}://${url.host}`;
  } catch (error) {
    console.error('Error parsing WebSocket URL:', error);
    return '';
  }
});

const handleRecordingStarted = () => {};
const handleRecordingStopped = () => {};

const toggleFreeze = () => {
  isFreeze.value = !isFreeze.value;
  if (!isFreeze.value) {
    displayData.value = { ...liveData.value };
  }
};

const connectWebSocket = () => {
  if (socket.value) return;

  socket.value = new WebSocket(props.websocketUrl);

  socket.value.onopen = () => {};

  socket.value.onmessage = (event) => {
    try {
      const parsedData = JSON.parse(event.data);
      if (props.debug) {
        console.debug('Ping1D data:', parsedData);
      }

      const profile = parsedData?.DeviceMessage?.PingMessage?.Ping1D?.Profile;

      if (profile) {
        const newData = {
          sensorData: profile.profile_data,
          currentDepth: profile.distance / 1000,
          minDepth: profile.scan_start / 1000,
          maxDepth: profile.scan_length / 1000,
          confidence: profile.confidence,
          accuracy:
            ((100 - profile.confidence) / 100) *
            (profile.scan_length / 1000 - profile.scan_start / 1000) *
            0.1,
        };

        liveData.value = newData;

        if (!isFreeze.value) {
          displayData.value = { ...newData };
        }

        if (props.debug) {
          console.debug('Processed Ping1D data:', newData);
        }
      }
    } catch (error) {
      console.error('Error parsing Ping1D WebSocket data:', error);
    }
  };

  socket.value.onerror = (error) => {
    console.error('Ping1D WebSocket error:', error);
  };

  socket.value.onclose = () => {
    socket.value = null;
    setTimeout(connectWebSocket, 5000);
  };
};

const disconnectWebSocket = () => {
  if (socket.value) {
    socket.value.close();
    socket.value = null;
  }
};

const openSettings = async () => {
  isSettingsOpen.value = true;
};

onMounted(() => {
  connectWebSocket();
});

onUnmounted(() => {
  disconnectWebSocket();
});

watch(
  () => props.websocketUrl,
  (newUrl, oldUrl) => {
    if (newUrl !== oldUrl) {
      disconnectWebSocket();
      connectWebSocket();
    }
  }
);
</script>

<style scoped>
.h-full {
  height: 100%;
}

.glass-button {
  background-color: rgba(0, 0, 0, 0.10) !important;
  border: 1px solid rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(25px) !important;
  -webkit-backdrop-filter: blur(16px) !important;
  box-shadow: 0px 8px 8px 0px #00000033, 0px 8px 12px 6px #00000016 !important;
}
</style>