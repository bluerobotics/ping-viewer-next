<template>
  <div class="flex flex-col h-full">
    <div class="flex-1 mx-10 my-10 min-h-0">
      <FloatingControls v-if="showControls" :is-recording="isRecording">
        <DataRecorder ref="dataRecorder" :device="device" @recording-complete="handleRecordingComplete"
          @recording-started="handleRecordingStarted" @recording-stopped="handleRecordingStopped" />
        <v-btn icon :color="isFreeze ? 'error' : 'primary'" @click="toggleFreeze" class="elevation-4" size="large">
          <v-icon>{{ isFreeze ? 'mdi-play' : 'mdi-pause' }}</v-icon>
        </v-btn>
        <v-btn icon color="primary" @click="openSettings" class="elevation-4" size="large">
          <v-icon>mdi-cog</v-icon>
        </v-btn>
        <v-dialog v-model="isSettingsOpen" max-width="300px">

          <Ping360Settings ref="settingsRef" :server-url="getServerUrl(websocketUrl)" :device-id="device.id"
            :initial-angles="{ startAngle, endAngle }" :isOpen="isSettingsOpen" @update:angles="handleAngleUpdate"
            @rangeChange="handleRangeChange" />
        </v-dialog>
      </FloatingControls>

      <Ping360 :measurement="displayMeasurement" :angle="displayAngle" :colorPalette="colorPalette"
        :lineColor="lineColor" :lineWidth="lineWidth" :maxDistance="currentRange" :numMarkers="numMarkers"
        :showRadiusLines="showRadiusLines" :showMarkers="showMarkers" :radiusLineColor="radiusLineColor"
        :markerColor="markerColor" :textBackgroundColor="markerBackgroundColor" :radiusLineWidth="radiusLineWidth"
        :debug="debug" :startAngle="startAngle" :endAngle="endAngle" :yaw_angle="yawAngle" v-bind="$attrs"
        class="h-full w-full" />
    </div>
  </div>
</template>

<script setup>
import { inject, onMounted, onUnmounted, ref, watch } from 'vue';
import DataRecorder from '../DataRecorder.vue';
import FloatingControls from '../FloatingControls.vue';
import Ping360 from './Ping360.vue';
import Ping360Settings from './Ping360Settings.vue';

const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
  websocketUrl: {
    type: String,
    required: true,
  },
  colorPalette: {
    type: String,
    required: true,
  },
  lineColor: {
    type: String,
    default: 'red',
  },
  lineWidth: {
    type: Number,
    default: 0.5,
  },
  maxDistance: {
    type: Number,
    default: 300,
  },
  numMarkers: {
    type: Number,
    default: 5,
  },
  showRadiusLines: {
    type: Boolean,
    default: true,
  },
  showMarkers: {
    type: Boolean,
    default: true,
  },
  radiusLineColor: {
    type: String,
    default: 'green',
  },
  markerColor: {
    type: String,
    default: 'green',
  },
  markerBackgroundColor: {
    type: String,
    default: 'rgba(0, 0, 0, 0.5)',
  },
  radiusLineWidth: {
    type: Number,
    default: 0.5,
  },
  debug: {
    type: Boolean,
    default: false,
  },
  showControls: {
    type: Boolean,
    default: true,
  },
});

const liveMeasurement = ref(null);
const liveAngle = ref(0);
const displayMeasurement = ref(null);
const displayAngle = ref(0);
const currentRange = ref(props.maxDistance);
const startAngle = ref(0);
const endAngle = ref(360);
const connectionStatus = ref('Disconnected');
const dataRecorder = ref(null);
const socket = ref(null);
const settingsRef = ref(null);
const isFreeze = ref(false);
const isRecording = ref(false);
const isSettingsOpen = ref(false);
const offset = ref(0);

const yawAngle = inject('yawAngle', ref(0));

const getServerUrl = (wsUrl) => {
  try {
    const url = new URL(wsUrl);
    return `http${url.protocol === 'wss:' ? 's' : ''}://${url.host}`;
  } catch (error) {
    console.error('Error parsing WebSocket URL:', error);
    return '';
  }
};

const toggleFreeze = () => {
  isFreeze.value = !isFreeze.value;
  if (!isFreeze.value) {
    displayMeasurement.value = liveMeasurement.value;
    displayAngle.value = liveAngle.value;
  }
};

const notifyRecording = inject('recordings', {
  handleRecordingComplete: null,
})?.handleRecordingComplete;

const handleRecordingComplete = (recordingData) => {
  if (notifyRecording) {
    const recordingWithSettings = {
      ...recordingData,
      settings: {
        startAngle: startAngle.value,
        endAngle: endAngle.value,
        currentRange: currentRange.value,
        yawAngle: yawAngle.value,
      },
    };
    notifyRecording(recordingWithSettings);
  }
};

const handleRecordingStarted = () => {
  isRecording.value = true;
};

const handleRecordingStopped = () => {
  isRecording.value = false;
};

function gradiansToDegrees(gradians) {
  if (gradians === 399) {
    return 360;
  }
  return Math.round((gradians * 360) / 400);
}

const sendGetConfigRequest = () => {
  if (!socket.value || socket.value.readyState !== WebSocket.OPEN) {
    console.error('WebSocket is not connected');
    return;
  }

  const configRequest = {
    command: 'ModifyDevice',
    module: 'DeviceManager',
    payload: {
      uuid: props.device.id,
      modify: 'GetPing360Config',
    },
  };

  socket.value.send(JSON.stringify(configRequest));
  if (props.debug) {
    console.debug('Sent GetPing360Config request:', configRequest);
  }
};

const connectWebSocket = () => {
  if (socket.value) return;

  socket.value = new WebSocket(props.websocketUrl);

  socket.value.onopen = () => {
    connectionStatus.value = 'Connected';
    sendGetConfigRequest();
  };

  socket.value.onmessage = (event) => {
    try {
      const parsedData = JSON.parse(event.data);
      if (props.debug) {
        console.debug('Ping360 data:', parsedData);
      }

      const config =
        parsedData.DeviceConfig?.ConfigAcknowledge?.modify?.SetPing360Config ||
        parsedData.DeviceConfig?.Ping360Config;

      if (config) {
        const SAMPLE_PERIOD_TICK_DURATION = 25e-9;
        currentRange.value = Math.round(
          (config.sample_period * SAMPLE_PERIOD_TICK_DURATION * config.number_of_samples * 1500) / 2
        );

        if (config.start_angle === 0 && config.stop_angle === 399) {
          startAngle.value = 0;
          endAngle.value = 360;
        } else {
          startAngle.value = (gradiansToDegrees(config.start_angle) + 180) % 360;
          endAngle.value = (gradiansToDegrees(config.stop_angle) + 180) % 360;
        }

        return;
      }

      const ping360Data = parsedData?.DeviceMessage?.PingMessage?.Ping360;
      if (!ping360Data) return;

      const messageData = ping360Data.DeviceData || ping360Data.AutoDeviceData;
      if (!messageData || messageData.angle === undefined || !messageData.data) return;

      const angleWithOffset = (messageData.angle + 400 + offset.value) % 400;

      liveMeasurement.value = {
        angle: angleWithOffset,
        data: new Uint8Array(messageData.data),
      };
      liveAngle.value = angleWithOffset;

      if (!isFreeze.value) {
        displayMeasurement.value = liveMeasurement.value;
        displayAngle.value = liveAngle.value;
      }

      dataRecorder.value?.recordData({
        angle: messageData.angle,
        data: new Uint8Array(messageData.data),
      });

      if (props.debug) {
        console.debug('Processed Ping360 data:', messageData);
      }
    } catch (error) {
      console.error('Error parsing Ping360 WebSocket data:', error);
    }
  };

  socket.value.onerror = (error) => {
    console.error('Ping360 WebSocket error:', error);
    connectionStatus.value = 'Error';
  };

  socket.value.onclose = () => {
    connectionStatus.value = 'Disconnected';
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

const handleAngleUpdate = ({ startAngle: newStart, endAngle: newEnd }) => {
  startAngle.value = newStart;
  endAngle.value = newEnd;
};

const handleRangeChange = (newRange) => {
  currentRange.value = newRange;
};

const openSettings = async () => {
  isSettingsOpen.value = true;
};

watch(
  () => props.websocketUrl,
  (newUrl, oldUrl) => {
    if (newUrl !== oldUrl) {
      disconnectWebSocket();
      connectWebSocket();
    }
  }
);

watch(yawAngle, (newYaw) => {
  if (props.debug) {
    console.debug('Yaw angle updated:', newYaw);
  }
});

onMounted(async () => {
  connectWebSocket();
});

onUnmounted(() => {
  disconnectWebSocket();
});
</script>

<style scoped>
.h-full {
  height: 100%;
}

.w-full {
  width: 100%;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
</style>