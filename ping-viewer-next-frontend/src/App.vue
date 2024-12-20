<template>
  <template v-if="isWidgetRoute">
    <v-app class="h-screen w-screen bg-transparent">
      <router-view />
    </v-app>
  </template>
  <template v-else>
  <v-app>


    <v-main class="bg-gradient-to-r from-cyan-500 to-blue-500">
      <ServerConnection v-if="!serverUrl" @serverConnected="onServerConnected" />

      <div v-if="activeDevice" class="device-viewer" :class="{ 'glass disable-hover': glass }">
        <div class="device-header" :class="{ 'glass-inner disable-hover': glass }">
          <v-btn icon="mdi-close" variant="text" @click="activeDevice = null" />
          <div class="device-info">
            <span class="device-type">{{ activeDevice.device.device_type }}</span>
            <v-chip size="small" :color="getStatusColor(activeDevice.device.status)">
              {{ activeDevice.device.status }}
            </v-chip>
          </div>
        </div>

        <component :is="activeDevice.component" :device="activeDevice.device"
          :websocketUrl="getWebSocketUrl(activeDevice.device)" v-bind="deviceSettings" class="device-content" />
      </div>

      <div v-if="isReplayActive" class="device-viewer" :class="{ 'glass disable-hover': glass }">
        <div class="device-header" :class="{ 'glass-inner disable-hover': glass }">
          <v-btn icon="mdi-close" variant="text" @click="closeReplay" />
          <div class="device-info">
            <span class="device-type">{{ replayData.deviceType }} Replay</span>
            <v-chip size="small" color="info">
              Replay Mode
            </v-chip>
          </div>
          <div class="replay-controls" :class="{ 'glass-inner disable-hover': glass }">
            <DataPlayer ref="dataPlayer" @update:currentFrame="handleReplayFrame"
              @loadedData="handleReplayDataLoaded" />
          </div>
        </div>



        <ReplayView ref="replayViewRef" class="device-content" v-bind="deviceSettings" />
      </div>

      <div class="speed-dial-container" :class="{ 'speed-dial-open': isSpeedDialOpen, glass: glass }"
        :style="{ '--items-count': speedDialItems.length }">
        <v-btn class="main-trigger square-button" :class="{ 'glass-inner': glass }"
          @click="isSpeedDialOpen = !isSpeedDialOpen" variant="text">
          <v-icon :icon="isSpeedDialOpen ? 'mdi-menu-open' : 'mdi-menu'" :size="iconSize" :color="iconColor" />
        </v-btn>

        <transition-group name="speed-dial-items">
          <template v-for="(item, index) in speedDialItems" :key="item.icon + index">
            <v-btn v-show="isSpeedDialOpen" class="speed-dial-item" :class="{ 'glass-inner': glass }"
              :style="{ '--delay': `${index * 0.05}s` }" @click="item.action && item.action()">
              <transition name="icon-flip" mode="out-in">
                <v-icon :key="item.icon" :icon="typeof item.icon === 'function' ? item.icon() : item.icon"
                  :size="iconSize" :color="iconColor" />
              </transition>
            </v-btn>
          </template>
        </transition-group>


      </div>

      <v-card class="connection-menu-wrapper" :class="{ 'glass': glass }" v-if="isConnectionMenuOpen">
        <ConnectionManager v-if="serverUrl" :server-url="serverUrl" :glass="glass" :is-open="isConnectionMenuOpen"
          @update:is-open="isConnectionMenuOpen = $event" @select-device="handleDeviceSelection" />
      </v-card>

      <v-card class="connection-menu-wrapper" :class="{ 'glass': glass }" v-if="showSettings">
        <VisualSettings :is-open="showSettings" :glass="glass" :common-settings="commonSettings"
          :ping1DSettings="ping1DSettings" :ping360Settings="ping360Settings" :is-dark-mode="isDarkMode"
          :is-glass-mode="isGlassMode" :server-url="serverUrl" :yaw-connection-status="yawConnectionStatus"
          @update:is-open="showSettings = $event" @update:common-settings="updateCommonSettings"
          @update:ping1D-settings="updatePing1DSettings" @update:ping360-settings="updatePing360Settings"
          @update:is-dark-mode="updateDarkMode" @update:is-glass-mode="updateGlassMode"
          @update:server-url="handleServerUrlUpdate" @updateMavlink="handleMavlinkUpdate" @save="saveSettings"
          @reset="resetSettings" />
      </v-card>

      <div class="middle-section" :class="{ 'menu-open': isMenuOpen }">
        <v-btn class="middle-button square-button" :class="{ glass }" @click="toggleMenu">
          <v-icon :icon="isMenuOpen ? 'mdi-close' : 'mdi-wifi'" :size="iconSize" :color="iconColor"
            :class="{ 'rotate-180': !isMenuOpen }" />
        </v-btn>

        <div class="connection-menu" :class="{ 'glass disable-hover': glass }" v-show="isMenuOpen">
          <div :class="[{ 'glass-inner disable-hover': glass }]">


            <!-- Dynamic Device Settings -->
            <template v-if="activeDevice">
              <component :class="['menu-content', { 'glass-inner disable-hover': glass }]"
                :is="getDeviceSettingsComponent" :server-url="serverUrl" :device-id="activeDevice.device.id"
                :initial-angles="currentDeviceAngles" :is-open="isMenuOpen" @update:angles="handleAngleUpdate"
                @rangeChange="handleRangeChange" />
            </template>
            <template v-else>

              <div class="menu-content text-center pa-4 text-medium-emphasis">
                <v-icon size="48" class="mb-2">mdi-devices</v-icon>
                <div>No device selected.</div>
                <div class="text-caption">Try clicking 'Auto Create' to discover devices.</div>
              </div>
            </template>
          </div>
        </div>
      </div>

      <v-card class="recordings-menu-wrapper" :class="{ 'glass': glass }" v-if="showRecordingsMenu">
        <div :class="['menu-content', { 'glass-inner disable-hover': glass }]">
          <div class="d-flex justify-space-between align-center mb-4">
            <div class="text-h6">Recordings</div>
            <v-btn icon="mdi-close" variant="text" @click="showRecordingsMenu = false" />
          </div>

          <v-btn block color="primary" class="mb-4" @click="playRecording" prepend-icon="mdi-replay">
            Play records
          </v-btn>

          <div v-if="recordings.length === 0" class="text-center pa-4 text-medium-emphasis">
            <v-icon size="48" class="mb-2">mdi-video-off</v-icon>
            <div>No recordings available</div>
            <div class="text-caption mt-2">
              Records will appear here when you capture data from devices
            </div>
          </div>

          <v-list v-else :class="{ 'glass-inner': glass }">
            <v-list-subheader>Recent Recordings</v-list-subheader>

            <v-list-item v-for="recording in recordings" :key="recording.id"
              :class="{ 'new-recording': !recording.downloaded }">
              <template v-slot:prepend>
                <v-icon :icon="recording.deviceType === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'" />
              </template>

              <v-list-item-title class="text-truncate">
                {{ recording.fileName }}
              </v-list-item-title>

              <v-list-item-subtitle>
                {{ formatRecordingDate(recording.timestamp) }}
              </v-list-item-subtitle>

              <v-list-item-subtitle v-if="recording.settings" class="text-caption">
                {{ formatRecordingDetails(recording) }}
              </v-list-item-subtitle>

              <template v-slot:append>
                <div class="d-flex gap-2">
                  <v-tooltip location="top" text="Play Recording">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" icon="mdi-play" variant="text" size="small"
                        @click="playRecording(recording)" :disabled="isReplayActive" />
                    </template>
                  </v-tooltip>

                  <v-tooltip location="top" text="Download Recording">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" icon="mdi-download" variant="text" size="small"
                        @click="downloadRecording(recording)" />
                    </template>
                  </v-tooltip>

                  <v-tooltip location="top" text="Delete Recording">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" icon="mdi-delete" variant="text" size="small" color="error"
                        @click="deleteRecording(recording)" />
                    </template>
                  </v-tooltip>
                </div>
              </template>
            </v-list-item>
          </v-list>

          <v-card-actions v-if="recordings.length > 0" class="mt-4">
            <v-spacer />
            <v-btn color="error" variant="text" @click="clearRecordings">
              Clear All
            </v-btn>
          </v-card-actions>
        </div>
      </v-card>

      <v-btn class="bottom-button square-button" :class="{ glass }" @click="showRecordingsMenu = !showRecordingsMenu">
        <v-badge :content="undownloadedRecordings.length.toString()" :model-value="undownloadedRecordings.length > 0"
          color="error" location="top end" offset-x="-6" offset-y="-6">
          <v-icon icon="mdi-video-image" :size="iconSize" :color="iconColor" />
        </v-badge>
      </v-btn>

      <v-btn class="bottom-right-button square-button" :class="{ glass }">
        <v-icon icon="mdi-bell" :size="iconSize" :color="iconColor" />
      </v-btn>


    </v-main>



  </v-app>
</template>
</template>
<script setup>
import { computed, ref } from 'vue';
import { useDisplay, useTheme } from 'vuetify';
import ConnectionManager from './components/ConnectionManager.vue';
import Ping1DSettings from './components/widgets/sonar1d/Ping1DSettings.vue';
import Ping360Settings from './components/widgets/sonar360/Ping360Settings.vue';

const getDeviceSettingsComponent = computed(() => {
  if (!activeDevice.value) return null;
  return activeDevice.value.device.device_type === 'Ping360' ? Ping360Settings : Ping1DSettings;
});

// Add computed property for current device angles (for Ping360)
const currentDeviceAngles = computed(() => {
  if (!activeDevice.value || activeDevice.value.device.device_type !== 'Ping360') {
    return { startAngle: 0, endAngle: 360 };
  }
  // Return actual angles if they're stored in your state
  return { startAngle: 0, endAngle: 360 }; // Replace with actual angle values
});

// Add handlers for device-specific settings
const handleAngleUpdate = ({ startAngle, endAngle }) => {};

const handleRangeChange = (newRange) => {};

const getServerUrl = (wsUrl) => {
  try {
    const url = new URL(wsUrl);
    return `http${url.protocol === 'wss:' ? 's' : ''}://${url.host}`;
  } catch (error) {
    console.error('Error parsing WebSocket URL:', error);
    return '';
  }
};

const { name: breakpoint } = useDisplay();
const theme = useTheme();

const handleServerUrlUpdate = async (newUrl) => {
  serverUrl.value = newUrl;

  // Reconnect main websocket
  if (websocket.value) {
    websocket.value.close();
  }
  await nextTick();
  connectWebSocket();

  // Update MAVLink connection if auto-connect is enabled
  const autoConnectMavlink = localStorage.getItem('autoConnectMavlink') === 'true';
  if (autoConnectMavlink) {
    const mavlinkUrl = localStorage.getItem('mavlinkUrl');
    if (mavlinkUrl) {
      connectYawWebSocket(mavlinkUrl);
    }
  }
};

const isSpeedDialOpen = ref(false);
const isGlassMode = ref(false);
const glass = computed(() => isGlassMode.value);
const updateGlassMode = (value) => {
  isGlassMode.value = value;
  localStorage.setItem('glassMode', value.toString());
};

const updateConnectionIcon = () => (isConnectionMenuOpen.value ? 'mdi-close' : 'mdi-connection');
const updateVisualSettingsIcon = () =>
  showSettings.value ? 'mdi-close' : 'mdi-white-balance-sunny';

const getStatusColor = computed(() => (status) => {
  switch (status) {
    case 'ContinuousMode':
      return 'success';
    case 'Running':
      return 'info';
    case 'Error':
      return 'error';
    default:
      return 'warning';
  }
});

const speedDialItems = ref([
  {
    icon: 'mdi-information-outline',
    dialog: {
      isOpen: false,
      title: 'Information',
      content: 'Information content goes here',
    },
  },
  {
    icon: updateConnectionIcon,
    action: () => {
      isConnectionMenuOpen.value = !isConnectionMenuOpen.value;
    },
  },
  {
    icon: updateVisualSettingsIcon,
    action: () => {
      showSettings.value = !showSettings.value;
    },
  },
  { icon: 'mdi-memory-arrow-down' },
  {
    icon: 'mdi-settings',
    dialog: {
      isOpen: false,
      title: 'Settings',
      content: 'Settings content goes here',
    },
  },
]);

const iconColor = computed(() => {
  return theme.global.current.value.dark ? 'white' : 'black';
});

const iconSize = computed(() => {
  const sizes = {
    xs: 'default',
    sm: 'large',
    default: 'x-large',
  };
  return sizes[breakpoint.value] || sizes.default;
});

const dialogWidth = computed(() => {
  const sizes = {
    xs: '90vw',
    sm: '400px',
    md: '500px',
    lg: '600px',
    default: '600px',
  };
  return sizes[breakpoint.value] || sizes.default;
});

const isConnectionMenuOpen = ref(false);
const isVisualSettingsMenuOpen = ref(false);

const handleDeviceSelection = (device) => {
  selectDevice(device);
  isConnectionMenuOpen.value = false;
};

// Add this where your other functions are defined
const selectDevice = async (device) => {
  // Clear current device
  if (activeDevice.value) {
    // Clean up any existing websocket connections
    const oldWebSocket = `ws://${new URL(serverUrl.value).host}/ws?device_number=${activeDevice.value.device.id}`;
    const connections = [...(websocket.value?.clients || [])];
    for (const conn of connections) {
      if (conn.url === oldWebSocket) {
        conn.close();
      }
    }

    // Clear the active device first
    activeDevice.value = null;
    // Wait for cleanup
    await nextTick();
  }

  // Set new device
  const component = device.device_type === 'Ping360' ? Ping360Loader : Ping1DLoader;
  activeDevice.value = {
    device,
    component,
  };
  isConnectionMenuOpen.value = false;
};

// Add this to your reactive state
const activeDevice = ref(null);

const isMenuOpen = ref(false);
const config = ref({
  wifi: true,
  strength: 75,
  network: 'Home Network',
  autoConnect: true,
});

const networkOptions = ['Home Network', 'Office Network', 'Guest Network', 'Public WiFi'];

const toggleMenu = () => {
  isMenuOpen.value = !isMenuOpen.value;
};

const saveConfig = () => {
  isMenuOpen.value = false;
};

import { onMounted, onUnmounted, provide, reactive, watch } from 'vue';
import { useRoute } from 'vue-router';
import ServerConnection from './components/utils/ServerConnection.vue';
import VisualSettings from './components/utils/VisualSettings.vue';
import DevicesView from './components/views/DevicesView.vue';
import MultiView from './components/views/MultiView.vue';
import ReplayView from './components/views/ReplayView.vue';
import SettingsView from './components/views/SettingsView.vue';
import WebSocketAnalysisView from './components/views/WebsocketAnalysisView.vue';
import Ping1DLoader from './components/widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from './components/widgets/sonar360/Ping360Loader.vue';

const selectedView = ref('Devices');
const serverUrl = ref(null);
const websocket = ref(null);
const websocketStatus = ref('Disconnected');
const deviceData = reactive({});
const showSettings = ref(false);
const isFullscreen = ref(false);
const isDarkMode = ref(true);
const recordings = ref([]);
const showNotification = ref(false);
const route = useRoute();
const replayData = ref(null);
const isReplayActive = ref(false);

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

const deviceSettings = computed(() => {
  if (!activeDevice.value) return {};

  return {
    ...commonSettings,
    ...(activeDevice.value.device.device_type === 'Ping360' ? ping360Settings : ping1DSettings),
    width: activeDevice.value?.width || window.innerWidth,
    height: activeDevice.value?.height || window.innerHeight,
  };
});

const getWebSocketUrl = (device) => {
  if (!device || !serverUrl.value) return '';
  const url = new URL(serverUrl.value);
  const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${url.host}/ws?device_number=${device.id}`;
};

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
    yawConnectionStatus.value = 'Connecting';

    yawWebSocket.onopen = () => {
      yawConnectionStatus.value = 'Connected';
      localStorage.setItem('yawWebsocketUrl', url); // Save successful connection URL
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

      // Only attempt reconnection if auto-connect is enabled
      const autoConnectMavlink = localStorage.getItem('autoConnectMavlink') === 'true';
      if (autoConnectMavlink) {
        if (reconnectTimeout) clearTimeout(reconnectTimeout);
        reconnectTimeout = setTimeout(() => {
          connectYawWebSocket(url);
        }, 5000);
      }
    };
  } catch (error) {
    console.error('Failed to create Yaw WebSocket:', error);
    yawConnectionStatus.value = 'Error';
  }
};

const handleMavlinkUpdate = async ({ action, url, autoConnect }) => {
  if (action === 'disconnect') {
    cleanupYawConnection();
  } else if (action === 'connect' || action === 'reconnect') {
    if (action === 'reconnect') {
      cleanupYawConnection();
      await nextTick();
    }
    connectYawWebSocket(url);
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
    const savedGlassMode = localStorage.getItem('glassMode');
    if (savedGlassMode !== null) {
      isGlassMode.value = savedGlassMode === 'true';
    }
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
    localStorage.setItem('glassMode', isGlassMode.value.toString());
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
  theme.global.name.value = isDarkMode.value ? 'dark' : 'light';
  localStorage.setItem('theme', theme.global.name.value);
};

const toggleFullscreen = () => {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen();
  } else {
    document.exitFullscreen();
  }
};

const replayViewRef = ref(null);
const dataPlayer = ref(null);

const handleReplayFrame = (frame) => {
  replayViewRef.value?.updateCurrentDeviceData(frame);
};

const handleReplayDataLoaded = (data) => {
  replayViewRef.value?.onDataLoaded(data);
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
  showRecordingsMenu.value = false; // Close the menu
  isReplayActive.value = true; // Show the replay view
  replayData.value = recording; // Set the recording data

  // Optional: Close any active device view
  if (activeDevice.value) {
    activeDevice.value = null;
  }
};

const closeReplay = () => {
  isReplayActive.value = false;
  replayData.value = null;
};

onMounted(() => {
  loadSettings();
  initializeYawConnection();

  const savedTheme = localStorage.getItem('theme');
  if (savedTheme) {
    theme.global.name.value = savedTheme;
    isDarkMode.value = savedTheme === 'dark';
  } else {
    const prefersDark = window.matchMedia('(prefers-color-scheme: light)').matches;
    theme.global.name.value = prefersDark ? 'dark' : 'light';
    isDarkMode.value = prefersDark;
  }

  try {
    const savedRecordings = localStorage.getItem('sonar-recordings');
    if (savedRecordings) {
      recordings.value = JSON.parse(savedRecordings);
    }
  } catch (error) {
    console.error('Error loading saved recordings:', error);
  }

  const autoConnectMavlink = localStorage.getItem('autoConnectMavlink') === 'true';
  if (autoConnectMavlink) {
    const mavlinkUrl = localStorage.getItem('mavlinkUrl');
    if (mavlinkUrl) {
      connectYawWebSocket(mavlinkUrl);
    }
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
  () => theme.global.name.value,
  (newTheme) => {
    document.documentElement.className = newTheme;
  }
);

watch(isSpeedDialOpen, (newValue) => {
  if (!newValue) {
    isConnectionMenuOpen.value = false;
  }
});

provide('recordings', {
  recordings,
  handleRecordingComplete,
});

const showRecordingsMenu = ref(false);

const formatRecordingDate = (timestamp) => {
  return new Date(timestamp).toLocaleString();
};

const formatRecordingDetails = (recording) => {
  if (!recording.settings) return '';

  const details = [];
  if (recording.deviceType === 'Ping360') {
    if (recording.settings.startAngle !== undefined) {
      details.push(`${recording.settings.startAngle}° - ${recording.settings.endAngle}°`);
    }
    if (recording.settings.currentRange) {
      details.push(`${recording.settings.currentRange}m range`);
    }
  } else {
    if (recording.settings.maxDepth) {
      details.push(`${recording.settings.maxDepth}m depth`);
    }
  }
  return details.join(' | ');
};

const deleteRecording = (recording) => {
  const index = recordings.value.findIndex((r) => r.id === recording.id);
  if (index !== -1) {
    recordings.value.splice(index, 1);
    // Also remove from localStorage if you're storing them there
    saveRecordingsToStorage();
  }
};

const clearRecordings = () => {
  recordings.value = [];
  saveRecordingsToStorage();
};

const saveRecordingsToStorage = () => {
  try {
    localStorage.setItem('sonar-recordings', JSON.stringify(recordings.value));
  } catch (error) {
    console.error('Error saving recordings to storage:', error);
  }
};

// Add these watchers to handle menu state
watch(isSpeedDialOpen, (newValue) => {
  if (newValue) {
    showRecordingsMenu.value = false;
  }
});

watch(isMenuOpen, (newValue) => {
  if (newValue) {
    showRecordingsMenu.value = false;
  }
});

watch(showRecordingsMenu, (newValue) => {
  if (newValue) {
    isSpeedDialOpen.value = false;
    isMenuOpen.value = false;
  }
});
</script>
<style>
:root {
  --button-size: 3.25rem;
  --button-gap: 0.5rem;
  --border-radius: 0.5rem;
  /* Set border as global value */
  /* border: 1px solid rgba(203, 203, 203, 0.13) !important; */
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}

/* Glass effects */
.glass {
  background-color: rgba(var(--v-theme-background), 0.3) !important;
  backdrop-filter: blur(25px) !important;
  /* border: 1px solid rgba(203, 203, 203, 0.13) !important;
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important; */
}

.glass-inner {
  background-color: rgba(var(--v-theme-background), 0) !important;
  /* border: 1px solid rgba(203, 203, 203, 0.0) !important; */
}

.glass:not(.disable-hover):hover {
  background: rgba(var(--v-theme-background), 0.5) !important;
}

.glass-inner:not(.disable-hover):hover {
  background: rgba(var(--v-theme-background), 0.5) !important;
}
</style>

<style scoped>
/* Base button styles */
.square-button {
  width: var(--button-size) !important;
  height: var(--button-size) !important;
  min-width: var(--button-size) !important;
  padding: 0 !important;
  display: flex !important;
  align-items: center !important;
  justify-content: center !important;
}

/* Speed dial item styles */
.speed-dial-item {
  width: var(--button-size) !important;
  height: var(--button-size) !important;
  min-width: var(--button-size) !important;
  padding: 0 !important;
  display: flex !important;
  align-items: center !important;
  justify-content: center !important;
  opacity: 1;
  box-shadow: none !important;
  background: none !important;
}

.speed-dial-item:hover {
  opacity: 1;
}

.icon-flip-enter-active,
.icon-flip-leave-active {
  transition: all 0.3s ease;
}

.icon-flip-enter-from,
.icon-flip-leave-to {
  transform: rotateY(180deg);
  opacity: 0;
}

.connection-menu {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  left: calc(var(--button-size) + var(--button-gap));
  z-index: 999;
  border-radius: var(--border-radius);
}

.connection-menu-wrapper {
  position: fixed;
  top: calc(var(--button-size) + var(--button-gap));
  left: calc(var(--button-size) + var(--button-gap));
  z-index: 999;
  border-radius: var(--border-radius);
}

/* Container layout and positioning */
.speed-dial-container {
  position: fixed;
  top: 0;
  left: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--button-gap);
  z-index: 1000;
  padding: 0;
  height: var(--button-size);
  overflow: hidden;
  transition: all 0.3s ease;
  border-radius: 0 0 var(--border-radius) 0 !important;
  background: rgb(var(--v-theme-background));
  /* border: 1px solid rgba(203, 203, 203, 0.13) !important; */
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}

.speed-dial-container.speed-dial-open {
  height: calc((var(--button-size) * var(--items-count)) + (var(--button-gap) * (var(--items-count) - 1)));
}

.speed-dial-menu-section {
  display: flex;
  align-items: center;
  position: relative;
}

/* Middle section styles */
.middle-section {
  position: fixed;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  z-index: 1000;
  display: flex;
  align-items: center;
}

.middle-button {
  position: static;
  transform: none;
  border-radius: 0 var(--border-radius) var(--border-radius) 0 !important;
  border-left: none !important;
}

/* Bottom button positioning */
.bottom-button {
  position: fixed;
  left: 0;
  bottom: 0;
  z-index: 1000;
  border-radius: 0 var(--border-radius) 0 0 !important;
  border-left: none !important;
}

.bottom-right-button {
  position: fixed;
  right: 0;
  bottom: 0;
  z-index: 1000;
  border-radius: var(--border-radius) 0 0 0 !important;
  border-right: none !important;
}

/* Speed dial animations */
.speed-dial-items-enter-active,
.speed-dial-items-leave-active {
  transition: all 0.3s ease;
}

.speed-dial-items-enter-from,
.speed-dial-items-leave-to {
  opacity: 0;
  transform: translateX(calc(var(--button-size) * -1.2));
}

/* Menu styles */
.config-menu {
  width: 0;
  height: auto;
  overflow: hidden;
  transition: width 0.3s ease;
  border-radius: var(--border-radius);
  background: rgb(var(--v-theme-background));
  border: 1px solid rgba(203, 203, 203, 0.13) !important;
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}


/* Menu styles (continued) */
.speed-dial-container .config-menu {
  position: absolute;
  left: calc(var(--button-size) + var(--button-gap));
  top: 50%;
  transform: translateY(-50%);
  width: 0;
  height: auto;
  overflow: hidden;
  transition: width 0.3s ease;
  border-radius: var(--border-radius);
  background: rgb(var(--v-theme-background));
  border: 1px solid rgba(203, 203, 203, 0.13) !important;
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}

.speed-dial-container .config-menu,
.menu-open .config-menu {
  width: 300px;
}

.menu-content {
  width: 300px;
  padding: 1rem;
}

.v-list {
  background: inherit;
}

.menu-actions {
  margin-top: 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}



/* Icon transition */
.v-icon {
  transition: transform 0.3s ease;
}

.rotate-180 {
  transform: rotate(180deg);
}

/* Mobile responsive adjustments */
@media (max-width: 600px) {
  :root {
    --button-gap: 0.25rem;
  }

  .speed-dial-container,
  .middle-button,
  .bottom-button,
  .bottom-right-button {
    border-width: 0.5px !important;
  }

  .menu-open .config-menu {
    width: calc(100vw - var(--button-size));
    max-width: 300px;
  }
}

/* Animated background */
.animated-background {
  background: linear-gradient(-45deg,
      #0501ff,
      #004b92,
      #23a6d5,
      #23d5ab);
  background-size: 400% 400%;
  animation: gradient 5s ease infinite;
  min-height: 100vh;
  position: relative;
  width: 100%;
}

@keyframes gradient {
  0% {
    background-position: 0% 50%;
  }

  50% {
    background-position: 100% 50%;
  }

  100% {
    background-position: 0% 50%;
  }
}

/* Safe area handling for mobile devices */
@supports (padding: max(0px)) {

  .bottom-button,
  .bottom-right-button {
    bottom: max(0px, env(safe-area-inset-bottom));
  }
}

.device-viewer {
  padding: 0 3.5rem 0 3.5rem;
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 100;
  background: rgb(var(--v-theme-surface));
  display: flex;
  flex-direction: column;
}


.device-header {
  padding: 1rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

.device-header.glass-inner {
  background: rgba(var(--v-theme-background), 0);
}

.device-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.device-type {
  font-weight: 500;
}

.device-content {
  flex: 1;
  overflow: hidden;
}

@media (max-width: 600px) {
  .device-header {
    padding: 0.5rem;
  }
}

.recordings-menu-wrapper {
  position: fixed;
  bottom: calc(var(--button-size) + var(--button-gap));
  left: var(--button-gap);
  z-index: 999;
  border-radius: var(--border-radius);
  /* width: 400px;
  max-width: calc(100vw - var(--button-size) - var(--button-gap) * 2); */
}

.recordings-list {
  max-height: 20vh;
  overflow-y: auto;
}

.new-recording {
  background: rgba(var(--v-theme-primary), 0.1);
}

.replay-controls {
  /* padding: 1rem; */
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

/* .device-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
} */

@media (max-width: 600px) {
  .recordings-menu-wrapper {
    width: calc(100vw - var(--button-size) - var(--button-gap) * 2);
  }
}
</style>