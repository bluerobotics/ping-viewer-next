<template>
  <template v-if="isWidgetRoute">
    <v-app class="h-screen w-screen bg-transparent " :theme="theme">
      <router-view />
    </v-app>
  </template>

  <template v-else>
    <v-app class="h-screen w-screen" :theme="theme">
      <ServerConnection v-if="!serverUrl" @serverConnected="onServerConnected" />

      <template v-else>
        <v-app-bar class="flex-none">
          <v-menu location="bottom">
            <template v-slot:activator="{ props }">
              <v-btn v-bind="props" icon>
                <v-icon>mdi-menu</v-icon>
              </v-btn>
            </template>

            <v-list>
              <v-list-item v-for="(item, index) in menuItems" :key="index" @click="selectView(item.value)">
                <template v-slot:prepend>
                  <v-icon>{{ item.icon }}</v-icon>
                </template>
                <v-list-item-title>{{ item.text }}</v-list-item-title>
              </v-list-item>
            </v-list>
          </v-menu>

          <v-app-bar-title>Ping Viewer Next</v-app-bar-title>

          <v-tooltip location="bottom">
            <template v-slot:activator="{ props }">
              <v-icon v-bind="props" :color="yawConnectionStatus === 'Connected' ? 'success' : 'error'" class="mr-2">
                mdi-compass
              </v-icon>
            </template>
            MAVLink: {{ yawConnectionStatus }}
          </v-tooltip>

          <v-menu location="bottom end">
            <template v-slot:activator="{ props }">
              <v-btn v-bind="props" icon class="mr-2">
                <v-badge :content="undownloadedRecordings.length.toString()"
                  :model-value="undownloadedRecordings.length > 0" color="error">
                  <v-icon>mdi-movie</v-icon>
                </v-badge>
              </v-btn>
            </template>

            <v-card min-width="300" class="recordings-menu">
              <v-card-title class="d-flex align-center py-2">
                <span class="text-h6">Recordings</span>
                <v-spacer></v-spacer>
                <v-btn v-if="recordings.length" density="comfortable" variant="text" @click="selectView('Replay')">
                  Open Replay View
                </v-btn>
              </v-card-title>

              <v-divider></v-divider>

              <v-list v-if="recordings.length" class="recordings-list">
                <v-list-item v-for="recording in recordings" :key="recording.id"
                  :class="{ 'new-recording': !recording.downloaded }">
                  <template v-slot:prepend>
                    <v-icon :icon="recording.deviceType === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'"></v-icon>
                  </template>

                  <v-list-item-title class="text-truncate">
                    {{ recording.fileName }}
                  </v-list-item-title>
                  <v-list-item-subtitle>
                    {{ new Date(recording.timestamp).toLocaleString() }}
                  </v-list-item-subtitle>

                  <template v-slot:append>
                    <div class="d-flex gap-2">
                      <v-btn icon="mdi-play" variant="text" size="small" @click="playRecording(recording)"
                        v-if="recording.downloaded"></v-btn>
                      <v-btn icon="mdi-download" variant="text" size="small" @click="downloadRecording(recording)"
                        v-if="!recording.downloaded"></v-btn>
                    </div>
                  </template>
                </v-list-item>
              </v-list>

              <v-card-text v-else class="text-center py-4">
                No recordings available
              </v-card-text>
            </v-card>
          </v-menu>

          <template v-slot:append>
            <v-menu location="bottom end">
              <template v-slot:activator="{ props }">
                <v-btn v-bind="props" icon>
                  <v-icon>mdi-dots-vertical</v-icon>
                </v-btn>
              </template>

              <v-list>

                <v-list-item @click="showSettings = true">
                  <template v-slot:prepend>
                    <v-icon>mdi-cog</v-icon>
                  </template>
                  <v-list-item-title>Visual Settings</v-list-item-title>
                </v-list-item>

                <v-list-item @click="toggleFullscreen">
                  <template v-slot:prepend>
                    <v-icon>{{ isFullscreen ? 'mdi-fullscreen-exit' : 'mdi-fullscreen' }}</v-icon>
                  </template>
                  <v-list-item-title>{{ isFullscreen ? 'Exit Fullscreen' : 'Fullscreen' }}</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </template>
        </v-app-bar>

        <v-main class="flex-1">
          <div class="h-full overflow-auto">
            <component :is="views[selectedView]" :serverUrl="serverUrl" :websocket="websocket"
              :websocketStatus="websocketStatus" :deviceData="deviceData" @send-message="sendWebSocketMessage"
              class="h-full w-full" />
          </div>
        </v-main>

        <v-dialog v-model="showSettings" max-width="600px">
          <v-card>
            <v-card-title class="text-h5 pb-2">VisualSettings</v-card-title>

            <v-card-text>
              <VisualSettings :common-settings="commonSettings" :ping1DSettings="ping1DSettings"
                :ping360Settings="ping360Settings" :is-dark-mode="isDarkMode"
                @update:common-settings="updateCommonSettings" @update:ping1D-settings="updatePing1DSettings"
                @update:ping360-settings="updatePing360Settings" @update:is-dark-mode="updateDarkMode"
                @save="saveSettings" @reset="resetSettings" @close="showSettings = false" />
            </v-card-text>

          </v-card>
        </v-dialog>
      </template>

      <v-snackbar v-model="showNotification" :timeout="3000" location="top right">
        New recording is ready to download
        <template v-slot:actions>
          <v-btn color="primary" variant="text" @click="showNotification = false">
            Close
          </v-btn>
        </template>
      </v-snackbar>
    </v-app>
  </template>


</template>
<script setup>
import { computed, onMounted, onUnmounted, provide, reactive, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import ServerConnection from './components/utils/ServerConnection.vue';
import VisualSettings from './components/utils/VisualSettings.vue';
import DevicesView from './components/views/DevicesView.vue';
import MultiView from './components/views/MultiView.vue';
import ReplayView from './components/views/ReplayView.vue';
import SettingsView from './components/views/SettingsView.vue';
import WebSocketAnalysisView from './components/views/WebsocketAnalysisView.vue';

const selectedView = ref('Devices');
const serverUrl = ref(null);
const websocket = ref(null);
const websocketStatus = ref('Disconnected');
const deviceData = reactive({});
const showSettings = ref(false);
const isFullscreen = ref(false);
const isDarkMode = ref(true);
const theme = ref('dark');
const recordings = ref([]);
const showNotification = ref(false);
const route = useRoute();

const yawAngle = ref(0);
const yawConnectionStatus = ref('Disconnected');
let yawWebSocket = null;
let reconnectTimeout = null;

const isWidgetRoute = computed(() => {
  return route.path.startsWith('/addons/widget/');
});

const undownloadedRecordings = computed(() => {
  return recordings.value.filter((recording) => !recording.downloaded);
});

const handleRecordingComplete = (recordingData) => {
  recordings.value.unshift(recordingData);
  showNotification.value = true;
};

const downloadRecording = (recording) => {
  const dataStr = JSON.stringify(recording.data, null, 2);
  const dataUri = `data:application/json;charset=utf-8,${encodeURIComponent(dataStr)}`;

  const linkElement = document.createElement('a');
  linkElement.setAttribute('href', dataUri);
  linkElement.setAttribute('download', recording.fileName);
  linkElement.click();

  const index = recordings.value.findIndex((r) => r.id === recording.id);
  if (index !== -1) {
    recordings.value[index] = { ...recordings.value[index], downloaded: true };
  }
};

const initializeYawConnection = () => {
  const savedUrl = localStorage.getItem('yawWebsocketUrl');
  if (savedUrl) {
    connectYawWebSocket(savedUrl);
  }
};

const connectYawWebSocket = (url) => {
  if (yawWebSocket?.readyState === WebSocket.OPEN) {
    return;
  }

  try {
    yawWebSocket = new WebSocket(url);

    yawWebSocket.onopen = () => {
      yawConnectionStatus.value = 'Connected';
    };

    yawWebSocket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.message && data.message.type === 'ATTITUDE') {
          yawAngle.value = 180 - (data.message.yaw * 180) / Math.PI;
        }
      } catch (error) {
        console.error('Error parsing yaw message:', error);
      }
    };

    yawWebSocket.onerror = (error) => {
      console.error('Yaw WebSocket error:', error);
      yawConnectionStatus.value = 'Error';
    };

    yawWebSocket.onclose = () => {
      yawConnectionStatus.value = 'Disconnected';
      yawWebSocket = null;
      if (reconnectTimeout) clearTimeout(reconnectTimeout);
      reconnectTimeout = setTimeout(() => {
        connectYawWebSocket(url);
      }, 5000);
    };
  } catch (error) {
    console.error('Failed to create Yaw WebSocket:', error);
    yawConnectionStatus.value = 'Error';
  }
};

const cleanupYawConnection = () => {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
    reconnectTimeout = null;
  }

  if (yawWebSocket) {
    yawWebSocket.close();
    yawWebSocket = null;
  }
};

provide('yawAngle', yawAngle);
provide('yawConnectionStatus', yawConnectionStatus);
provide('connectYawWebSocket', connectYawWebSocket);
provide('cleanupYawConnection', cleanupYawConnection);

const commonSettings = reactive({
  colorPalette: 'Thermal Blue',
  customPalette: [],
});

const ping1DSettings = reactive({
  columnCount: 100,
  tickCount: 5,
  depthLineColor: '#ffeb3b',
  depthTextColor: '#ffeb3b',
  currentDepthColor: '#ffeb3b',
  confidenceColor: '#4caf50',
  textBackground: 'rgba(0, 0, 0, 0.5)',
  debug: false,
  depthArrowColor: '#f44336',
});

const ping360Settings = reactive({
  lineColor: '#f44336',
  lineWidth: 0.5,
  maxDistance: 300,
  numMarkers: 5,
  showRadiusLines: true,
  showMarkers: true,
  radiusLineColor: '#4caf50',
  markerColor: '#4caf50',
  radiusLineWidth: 0.5,
  debug: false,
});

const views = {
  Devices: DevicesView,
  Replay: ReplayView,
  Settings: SettingsView,
  MultiView: MultiView,
  WebSocketAnalysis: WebSocketAnalysisView,
};

const menuItems = [
  { text: 'Devices', icon: 'mdi-devices', value: 'Devices' },
  { text: 'Replay', icon: 'mdi-video-marker-outline', value: 'Replay' },
  {
    text: 'MultiView',
    icon: 'mdi-hexagon-multiple-outline',
    value: 'MultiView',
  },
  { text: 'Settings', icon: 'mdi-cog', value: 'Settings' },
  {
    text: 'WebSocket Analysis',
    icon: 'mdi-webhook',
    value: 'WebSocketAnalysis',
  },
];

const loadSettings = () => {
  try {
    const savedCommon = localStorage.getItem('common-settings');
    const savedPing1D = localStorage.getItem('ping1d-settings');
    const savedPing360 = localStorage.getItem('ping360-settings');
    const savedCustomPalette = localStorage.getItem('customColorPalette');

    if (savedCommon) Object.assign(commonSettings, JSON.parse(savedCommon));
    if (savedPing1D) Object.assign(ping1DSettings, JSON.parse(savedPing1D));
    if (savedPing360) Object.assign(ping360Settings, JSON.parse(savedPing360));
    if (savedCustomPalette) {
      commonSettings.customPalette = JSON.parse(savedCustomPalette);
    }
  } catch (error) {
    console.error('Error loading settings:', error);
  }
};

const saveSettings = () => {
  try {
    localStorage.setItem('common-settings', JSON.stringify(commonSettings));
    localStorage.setItem('ping1d-settings', JSON.stringify(ping1DSettings));
    localStorage.setItem('ping360-settings', JSON.stringify(ping360Settings));
    if (commonSettings.customPalette?.length > 0) {
      localStorage.setItem('customColorPalette', JSON.stringify(commonSettings.customPalette));
    }
    showSettings.value = false;
  } catch (error) {
    console.error('Error saving settings:', error);
  }
};

const resetSettings = () => {
  Object.assign(commonSettings, {
    colorPalette: 'Ocean',
  });

  Object.assign(ping1DSettings, {
    columnCount: 100,
    tickCount: 5,
    depthLineColor: '#00FF00',
    depthTextColor: '#00FF00',
    currentDepthColor: '#00FF00',
    confidenceColor: '#00FF00',
    textBackground: 'rgba(0, 0, 0, 0.5)',
    debug: false,
    depthArrowColor: '#f44336',
  });

  Object.assign(ping360Settings, {
    lineColor: '#00FF00',
    lineWidth: 0.5,
    maxDistance: 300,
    numMarkers: 5,
    showRadiusLines: true,
    showMarkers: true,
    radiusLineColor: '#00FF00',
    markerColor: '#00FF00',
    radiusLineWidth: 0.5,
    debug: false,
  });
};

const updateCommonSettings = (newSettings) => {
  Object.assign(commonSettings, newSettings);
};

const updatePing1DSettings = (newSettings) => {
  Object.assign(ping1DSettings, newSettings);
};

const updatePing360Settings = (newSettings) => {
  Object.assign(ping360Settings, newSettings);
};

const updateDarkMode = (value) => {
  isDarkMode.value = value;
  toggleTheme();
};

const toggleTheme = () => {
  theme.value = isDarkMode.value ? 'dark' : 'light';
  localStorage.setItem('theme', theme.value);
};

const toggleFullscreen = () => {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen();
  } else {
    document.exitFullscreen();
  }
};

const handleFullscreenChange = () => {
  isFullscreen.value = !!document.fullscreenElement;
};

const onServerConnected = (url) => {
  serverUrl.value = url;
  localStorage.setItem('serverUrl', url);
  connectWebSocket();
};

const processWebSocketMessage = (data) => {
  if (!data) {
    console.warn('Received invalid data:', data);
    return;
  }

  if (data.DeviceInfo) {
    deviceData.DeviceInfo = data.DeviceInfo;
    return;
  }

  if (data.DeviceMessage) {
    const deviceId = data.DeviceMessage.device_id;
    if (!deviceId) {
      console.warn('Received DeviceMessage without device_id:', data);
      return;
    }

    const messageType = Object.keys(data.DeviceMessage.PingMessage)[0];
    if (!messageType) {
      console.warn('Received DeviceMessage without PingMessage type:', data);
      return;
    }

    if (!deviceData[deviceId]) {
      deviceData[deviceId] = {};
    }

    deviceData[deviceId][messageType] = data.DeviceMessage.PingMessage[messageType];
  }
};

const connectWebSocket = () => {
  if (websocket.value) {
    websocket.value?.close();
  }

  const wsUrl = `ws://${new URL(serverUrl.value).host}/ws`;
  websocket.value = new WebSocket(wsUrl);

  websocket.value.onopen = () => {
    websocketStatus.value = 'Connected';
  };

  websocket.value.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      processWebSocketMessage(data);
    } catch (error) {
      console.error('Error processing WebSocket message:', error);
    }
  };

  websocket.value.onclose = (event) => {
    websocketStatus.value = 'Disconnected';
    setTimeout(() => {
      if (serverUrl.value) {
        connectWebSocket();
      }
    }, 5000);
  };

  websocket.value.onerror = (error) => {
    console.error('WebSocket error:', error);
    websocketStatus.value = 'Error';
  };
};

const sendWebSocketMessage = (message) => {
  if (websocket.value && websocket.value.readyState === WebSocket.OPEN) {
    websocket.value.send(message);
  }
};

const selectView = (view) => {
  selectedView.value = view;
};

provide('deviceSettings', {
  commonSettings,
  ping1DSettings,
  ping360Settings,
});

const playRecording = (recording) => {
  selectView('Replay');
  nextTick(() => {
    const replayView = views.Replay;
    if (replayView?.loadRecording) {
      replayView.loadRecording(recording);
    }
  });
};

onMounted(() => {
  loadSettings();
  initializeYawConnection();

  const savedTheme = localStorage.getItem('theme');
  if (savedTheme) {
    theme.value = savedTheme;
    isDarkMode.value = savedTheme === 'dark';
  } else {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.value = prefersDark ? 'dark' : 'light';
    isDarkMode.value = prefersDark;
  }

  document.addEventListener('fullscreenchange', handleFullscreenChange);
});

onUnmounted(() => {
  if (websocket.value) {
    websocket.value.close();
  }
  document.removeEventListener('fullscreenchange', handleFullscreenChange);
  cleanupYawConnection();
});

watch(
  () => theme.value,
  (newTheme) => {
    document.documentElement.className = newTheme;
  }
);

provide('recordings', {
  recordings,
  handleRecordingComplete,
});
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  height: 100%;
}

.recordings-menu {
  max-height: 400px;
  overflow-y: auto;
}

.new-recording {
  background-color: rgba(var(--v-theme-primary), 0.1);
}

.recordings-list {
  max-height: 300px;
  overflow-y: auto;
}

.settings-scroll {
  overflow-y: auto;
  max-height: calc(80vh - 64px);
  padding: 16px;
}

.settings-scroll::-webkit-scrollbar {
  width: 8px;
}

.settings-scroll::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}

.settings-scroll::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
}

.settings-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.3);
}
</style>