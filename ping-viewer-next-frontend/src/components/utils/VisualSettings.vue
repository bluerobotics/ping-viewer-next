<template>
  <div class="settings-menu" :class="{ 'glass-inner disable-hover': glass }">
    <div :class="['menu-content', { 'glass-inner disable-hover': glass }]">
      <v-tabs v-model="activeSettingsTab" class="mb-2">
        <v-tab value="general">
          <v-icon start>mdi-cog</v-icon>
          General
        </v-tab>
        <v-tab value="ping1d">
          <v-icon start>mdi-altimeter</v-icon>
          Ping1D
        </v-tab>
        <v-tab value="ping360">
          <v-icon start>mdi-radar</v-icon>
          Ping360
        </v-tab>
        <v-tab value="presets">
          <v-icon start>mdi-palette-swatch</v-icon>
          Presets
        </v-tab>
        <v-tab value="server">
          <v-icon start>mdi-server</v-icon>
          Server
        </v-tab>
      </v-tabs>

      <v-window v-model="activeSettingsTab" class="settings-window">
        <!-- General Settings -->
        <v-window-item value="general">
          <div class="px-4">
            <section>
              <v-list-subheader class="px-0">Display Options</v-list-subheader>
              <div class="d-flex align-center justify-space-between">
                <span class="text-body-2 px-3">Dark Mode</span>
                <v-switch class="px-3" :model-value="isDarkMode"
                  @update:model-value="$emit('update:isDarkMode', $event)" hide-details></v-switch>
              </div>
              <div class="d-flex align-center justify-space-between">
                <span class="text-body-2 px-3">Glass Effect</span>
                <v-switch class="px-3" :model-value="isGlassMode"
                  @update:model-value="$emit('update:isGlassMode', $event)" hide-details></v-switch>
              </div>
            </section>
          </div>
        </v-window-item>

        <!-- Ping1D Settings -->
        <v-window-item value="ping1d">
          <div class="px-4">
            <section>
              <v-list-subheader class="px-0">Palette</v-list-subheader>
              <div>
                <SonarColorOptions :initial-palette="localPing1DSettings.colorPalette"
                  @update:colorPalette="updatePing1DColorPalette" />
              </div>
            </section>
            <section>
              <v-list-subheader class="px-0">Display Settings</v-list-subheader>
              <div class="d-flex flex-column gap-4 px-3">
                <v-text-field v-model.number="localPing1DSettings.columnCount" type="number" label="Column Count"
                  hide-details density="compact"></v-text-field>
                <v-text-field v-model.number="localPing1DSettings.tickCount" type="number" label="Tick Count"
                  hide-details density="compact"></v-text-field>
              </div>
            </section>

            <section>
              <v-list-subheader class="px-0">Display Options</v-list-subheader>
              <div class="d-flex align-center justify-space-between pe-3">
                <span class="text-body-2 px-3">Debug Mode</span>
                <v-switch v-model="localPing1DSettings.debug" hide-details></v-switch>
              </div>
            </section>

            <section>
              <v-list-subheader class="px-0">Colors</v-list-subheader>
              <div class="d-flex flex-column">
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Depth Line Color</span>
                  <ColorPickerField v-model="localPing1DSettings.depthLineColor" :defaultValue="'#ffeb3b'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Depth Text Color</span>
                  <ColorPickerField v-model="localPing1DSettings.depthTextColor" :defaultValue="'#ffeb3b'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Current Depth Color</span>
                  <ColorPickerField v-model="localPing1DSettings.currentDepthColor" :defaultValue="'#ffeb3b'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Confidence Color</span>
                  <ColorPickerField v-model="localPing1DSettings.confidenceColor" :defaultValue="'#4caf50'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Depth Arrow Color</span>
                  <ColorPickerField v-model="localPing1DSettings.depthArrowColor" :defaultValue="'#f44336'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Text Background Color</span>
                  <ColorPickerField v-model="localPing1DSettings.textBackground" :defaultValue="'rgba(0, 0, 0, 0.5)'" />
                </div>
              </div>
            </section>
          </div>
        </v-window-item>

        <!-- Ping360 Settings -->
        <v-window-item value="ping360">
          <div class="px-4">
            <section>
              <v-list-subheader class="px-0">Palette</v-list-subheader>
              <div>
                <SonarColorOptions :initial-palette="localPing360Settings.colorPalette"
                  @update:colorPalette="updatePing360ColorPalette" />
              </div>
            </section>
            <section class="mb-2">
              <v-list-subheader class="px-0">Display Settings</v-list-subheader>
              <div class="d-flex flex-column gap-4 px-3">
                <v-text-field v-model.number="localPing360Settings.numMarkers" type="number" label="Number of Markers"
                  hide-details density="compact"></v-text-field>
                <v-text-field v-model.number="localPing360Settings.lineWidth" type="number" step="0.1"
                  label="Line Width" hide-details density="compact"></v-text-field>
                <v-text-field v-model.number="localPing360Settings.radiusLineWidth" type="number" step="0.1"
                  label="Radius Line Width" hide-details density="compact"></v-text-field>
              </div>
            </section>

            <section class="mb-2">
              <v-list-subheader>Display Options</v-list-subheader>
              <div class="d-flex flex-column px-3">
                <div class="d-flex align-center justify-space-between -mb-5">
                  <span class="text-body-2">Show Radius Lines</span>
                  <v-switch v-model="localPing360Settings.showRadiusLines" hide-details></v-switch>
                </div>
                <div class="d-flex align-center justify-space-between -mb-5">
                  <span class="text-body-2">Show Markers</span>
                  <v-switch v-model="localPing360Settings.showMarkers" hide-details></v-switch>
                </div>
                <div class="d-flex align-center justify-space-between -mb-5">
                  <span class="text-body-2">Debug Mode</span>
                  <v-switch v-model="localPing360Settings.debug" hide-details></v-switch>
                </div>
              </div>
            </section>

            <section>
              <v-list-subheader class="px-0">Colors</v-list-subheader>
              <div class="d-flex flex-column">
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Line Color</span>
                  <ColorPickerField v-model="localPing360Settings.lineColor" :defaultValue="'#f44336'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Marker Color</span>
                  <ColorPickerField v-model="localPing360Settings.markerColor" :defaultValue="'#4caf50'" />
                </div>
                <div class="d-flex align-center justify-space-between">
                  <span class="text-body-2 px-3">Radius Line Color</span>
                  <ColorPickerField v-model="localPing360Settings.radiusLineColor" :defaultValue="'#4caf50'" />
                </div>
              </div>
            </section>
          </div>
        </v-window-item>

        <!-- Presets -->
        <v-window-item value="presets">
          <div class="px-4">
            <section class="mb-2">
              <div class="mb-4">Accessibility Presets</div>
              <v-select v-model="selectedPreset" :items="accessibilityPresets" label="Color Vision Mode"
                @update:model-value="handlePresetChange" variant="outlined" density="comfortable" />
            </section>

            <v-alert v-if="selectedPreset !== 'default'" color="info" variant="tonal">
              {{ getPresetDescription(selectedPreset) }}
              <div class="text-caption mt-2">
                You can still adjust individual settings in other tabs.
              </div>
            </v-alert>
          </div>
        </v-window-item>

        <!-- Server Settings -->
        <v-window-item value="server">
          <div class="px-4">
            <!-- Server Configuration -->
            <section class="mb-6">
              <v-list-subheader class="px-0">Server Configuration</v-list-subheader>
              <v-text-field v-model="serverSettings.url" label="Server URL" placeholder="http://localhost:8080"
                hint="Enter the server URL" persistent-hint class="mb-4" />
            </section>

            <!-- MAVLink Configuration -->
            <section class="mb-6">
              <v-list-subheader class="px-0">MAVLink Connection</v-list-subheader>
              <v-text-field v-model="serverSettings.mavlinkUrl" label="MAVLink WebSocket URL"
                placeholder="ws://localhost:6040/ws/mavlink" hint="WebSocket URL for MAVLink connection" persistent-hint
                class="mb-4" />

              <div class="d-flex align-center mb-4">
                <v-switch v-model="serverSettings.autoConnectMavlink" hide-details class="mr-2" />
                <span class="text-body-2">Auto-connect to MAVLink on startup</span>
              </div>

              <div class="d-flex align-center mb-4">
                <div class="mr-4 d-flex align-center">
                  <div class="mr-2">Status:</div>
                  <v-chip variant="elevated" :color="mavlinkStatusColor" size="small">
                    {{ mavlinkStatus }}
                  </v-chip>
                </div>

                <v-btn variant="elevated" :color="mavlinkStatus === 'Connected' ? 'error' : 'success'" size="small"
                  @click="toggleMavlinkConnection" :loading="mavlinkStatus === 'Connecting'">
                  {{ mavlinkStatus === 'Connected' ? 'Disconnect' : 'Connect' }}
                </v-btn>
              </div>
            </section>
          </div>
        </v-window-item>

      </v-window>

      <v-divider class="my-2"></v-divider>

      <div class="d-flex justify-end pa-2">
        <v-btn variant="tonal" @click="handleReset" class="mr-2">
          Reset
        </v-btn>
        <v-btn variant="tonal" @click="saveSettings">
          Save
        </v-btn>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, reactive, ref, watch } from 'vue';
import SonarColorOptions from '../widgets/SonarColorOptions.vue';
import { colorPalettes } from '../widgets/SonarColorOptions.vue';
import ColorPickerField from './ColorPickerField.vue';

const props = defineProps({
  isOpen: {
    type: Boolean,
    required: true,
  },
  glass: {
    type: Boolean,
    default: false,
  },
  ping1DSettings: {
    type: Object,
    required: true,
  },
  ping360Settings: {
    type: Object,
    required: true,
  },
  isDarkMode: {
    type: Boolean,
    required: true,
  },
  isGlassMode: {
    type: Boolean,
    required: true,
  },
  serverUrl: {
    type: String,
    required: true,
  },
  yawConnectionStatus: {
    type: String,
    required: true,
  },
});

const emit = defineEmits([
  'update:isOpen',
  'update:ping1DSettings',
  'update:ping360Settings',
  'update:isDarkMode',
  'update:isGlassMode',
  'update:serverUrl',
  'updateMavlink',
  'save',
  'reset',
]);

const activeSettingsTab = ref('general');
const selectedPreset = ref('default');
const localPing1DSettings = reactive({ ...props.ping1DSettings });
const localPing360Settings = reactive({ ...props.ping360Settings });
const mavlinkStatus = ref('Disconnected');
const serverSettings = reactive({
  url: props.serverUrl,
  mavlinkUrl: localStorage.getItem('mavlinkUrl') || 'ws://localhost:6040/ws/mavlink',
  autoConnectMavlink: localStorage.getItem('autoConnectMavlink') === 'true',
});

const accessibilityPresets = [
  { title: 'Default', value: 'default' },
  { title: 'Deuteranopia (Red-Green)', value: 'deuteranopia' },
  { title: 'Protanopia (Red-Green)', value: 'protanopia' },
  { title: 'Tritanopia (Blue-Yellow)', value: 'tritanopia' },
  { title: 'Monochromacy', value: 'monochromacy' },
  { title: 'High Contrast', value: 'highContrast' },
];

const mavlinkStatusColor = computed(() => {
  switch (mavlinkStatus.value) {
    case 'Connected':
      return 'success';
    case 'Connecting':
      return 'warning';
    case 'Error':
      return 'error';
    default:
      return 'grey';
  }
});

const presetConfigs = {
  default: {
    description: 'Default color settings',
    settings: {
      ping1DSettings: {
        columnCount: 500,
        tickCount: 5,
        depthLineColor: '#ffeb3b',
        depthTextColor: '#ffeb3b',
        currentDepthColor: '#ffeb3b',
        confidenceColor: '#4caf50',
        textBackground: 'rgba(0, 0, 0, 0.8)',
        depthArrowColor: '#f44336',
        debug: false,
      },
      ping360Settings: {
        lineColor: '#f44336',
        lineWidth: 0.5,
        numMarkers: 5,
        showRadiusLines: true,
        showMarkers: true,
        radiusLineColor: '#4caf50',
        markerColor: '#4caf50',
        radiusLineWidth: 0.5,
        debug: false,
      },
    },
  },
  deuteranopia: {
    description: 'Optimized for red-green color blindness (deuteranopia)',
    settings: {
      ping1DSettings: {
        depthLineColor: '#0077BB',
        depthTextColor: '#0077BB',
        currentDepthColor: '#0077BB',
        confidenceColor: '#EE7733',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#EE7733',
      },
      ping360Settings: {
        lineColor: '#EE7733',
        radiusLineColor: '#0077BB',
        markerColor: '#0077BB',
      },
    },
  },
  protanopia: {
    description: 'Optimized for red-green color blindness (protanopia)',
    settings: {
      ping1DSettings: {
        depthLineColor: '#0077BB',
        depthTextColor: '#0077BB',
        currentDepthColor: '#0077BB',
        confidenceColor: '#CCBB44',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#CCBB44',
      },
      ping360Settings: {
        lineColor: '#CCBB44',
        radiusLineColor: '#0077BB',
        markerColor: '#0077BB',
      },
    },
  },
  tritanopia: {
    description: 'Optimized for blue-yellow color blindness',
    settings: {
      ping1DSettings: {
        depthLineColor: '#FF99AA',
        depthTextColor: '#FF99AA',
        currentDepthColor: '#FF99AA',
        confidenceColor: '#44BB99',
        textBackground: 'rgba(0, 0, 0, 0.7)',
        depthArrowColor: '#44BB99',
      },
      ping360Settings: {
        lineColor: '#FF99AA',
        radiusLineColor: '#44BB99',
        markerColor: '#44BB99',
      },
    },
  },
  monochromacy: {
    description: 'Monochrome mode using high-contrast patterns',
    settings: {
      ping1DSettings: {
        depthLineColor: '#FFFFFF',
        depthTextColor: '#FFFFFF',
        currentDepthColor: '#FFFFFF',
        confidenceColor: '#CCCCCC',
        textBackground: 'rgba(0, 0, 0, 0.9)',
        depthArrowColor: '#FFFFFF',
      },
      ping360Settings: {
        lineColor: '#FFFFFF',
        radiusLineColor: '#CCCCCC',
        markerColor: '#FFFFFF',
      },
    },
  },
  highContrast: {
    description: 'High contrast mode for better visibility',
    settings: {
      ping1DSettings: {
        depthLineColor: '#FFFFFF',
        depthTextColor: '#FFFFFF',
        currentDepthColor: '#FFFFFF',
        confidenceColor: '#FFFFFF',
        textBackground: 'rgba(0, 0, 0, 1)',
        depthArrowColor: '#FFFFFF',
      },
      ping360Settings: {
        lineColor: '#FFFFFF',
        radiusLineColor: '#FFFFFF',
        markerColor: '#FFFFFF',
        lineWidth: 1.0,
        radiusLineWidth: 1.0,
      },
    },
  },
};

const getPresetDescription = (preset) => {
  return presetConfigs[preset]?.description || '';
};

const handleReset = () => {
  selectedPreset.value = 'default';
  emit('reset');

  // Reset server settings
  serverSettings.url = props.serverUrl;
  serverSettings.mavlinkUrl = 'ws://localhost:6040/ws/mavlink';
  serverSettings.autoConnectMavlink = false;
  localStorage.removeItem('mavlinkUrl');
  localStorage.removeItem('autoConnectMavlink');

  localStorage.removeItem('selectedAccessibilityPreset');
};

const toggleMavlinkConnection = async () => {
  if (mavlinkStatus.value === 'Connected') {
    emit('updateMavlink', {
      action: 'disconnect',
      url: serverSettings.mavlinkUrl,
    });
  } else {
    mavlinkStatus.value = 'Connecting';
    emit('updateMavlink', {
      action: 'connect',
      url: serverSettings.mavlinkUrl,
      autoConnect: serverSettings.autoConnectMavlink,
    });
  }
};

const handlePresetChange = (preset) => {
  if (preset === 'default') {
    handleReset();
    return;
  }

  const config = presetConfigs[preset].settings;

  Object.assign(localPing1DSettings, {
    ...localPing1DSettings,
    ...config.ping1DSettings,
  });

  Object.assign(localPing360Settings, {
    ...localPing360Settings,
    ...config.ping360Settings,
  });

  emit('update:ping1DSettings', { ...localPing1DSettings });
  emit('update:ping360Settings', { ...localPing360Settings });

  localStorage.setItem('selectedAccessibilityPreset', preset);
};

const saveSettings = () => {
  // Save visual settings
  localStorage.setItem('ping1d-settings', JSON.stringify(localPing1DSettings));
  localStorage.setItem('ping360-settings', JSON.stringify(localPing360Settings));

  if (localPing1DSettings.customPalette?.length > 0) {
    localStorage.setItem(
      'ping1d-custom-palette',
      JSON.stringify(localPing1DSettings.customPalette)
    );
  }
  if (localPing360Settings.customPalette?.length > 0) {
    localStorage.setItem(
      'ping360-custom-palette',
      JSON.stringify(localPing360Settings.customPalette)
    );
  }

  if (serverSettings.url !== props.serverUrl) {
    emit('update:serverUrl', serverSettings.url);
  }

  // Save server settings
  localStorage.setItem('serverUrl', serverSettings.url);
  localStorage.setItem('mavlinkUrl', serverSettings.mavlinkUrl);
  localStorage.setItem('autoConnectMavlink', serverSettings.autoConnectMavlink.toString());

  if (mavlinkStatus.value === 'Connected') {
    // Reconnect with new settings if already connected
    emit('updateMavlink', {
      action: 'reconnect',
      url: serverSettings.mavlinkUrl,
      autoConnect: serverSettings.autoConnectMavlink,
    });
  } else if (serverSettings.autoConnectMavlink) {
    // Connect if auto-connect is enabled
    emit('updateMavlink', {
      action: 'connect',
      url: serverSettings.mavlinkUrl,
      autoConnect: true,
    });
  }

  emit('save');
  emit('update:isOpen', false);
};

const updatePing1DColorPalette = (newPalette) => {
  localPing1DSettings.colorPalette = newPalette;
  if (newPalette === 'Custom') {
    localPing1DSettings.customPalette = colorPalettes.Custom;
  }
  emit('update:ping1DSettings', { ...localPing1DSettings });
};

const updatePing360ColorPalette = (newPalette) => {
  localPing360Settings.colorPalette = newPalette;
  if (newPalette === 'Custom') {
    localPing360Settings.customPalette = colorPalettes.Custom;
  }
  emit('update:ping360Settings', { ...localPing360Settings });
};

watch(
  () => props.ping1DSettings,
  (newSettings) => {
    Object.assign(localPing1DSettings, newSettings);
  },
  { deep: true }
);

watch(
  () => props.ping360Settings,
  (newSettings) => {
    Object.assign(localPing360Settings, newSettings);
  },
  { deep: true }
);

watch(
  () => props.yawConnectionStatus,
  (newStatus) => {
    mavlinkStatus.value = newStatus;
  },
  { immediate: true }
);

onMounted(() => {
  // Load saved settings
  if (serverSettings.autoConnectMavlink && mavlinkStatus.value === 'Disconnected') {
    toggleMavlinkConnection();
  }
});
</script>

<style>

.settings-window {
  /* flex: 1; */
  overflow-y: auto;
  overflow-x: hidden;
  max-height: calc(70vh - 2 * (var(--button-size) + var(--button-gap)));
}
</style>
