<template>
  <div class="settings-container p-2">
    <v-tabs v-model="activeSettingsTab" class="border-b mb-2">
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
    </v-tabs>

    <v-window v-model="activeSettingsTab">
      <v-window-item value="general">
        <div class="px-4">
          <v-list-subheader class="px-0 ">Palette</v-list-subheader>
          <div>
            <SonarColorOptions :initial-palette="localCommonSettings.colorPalette"
              @update:colorPalette="updateColorPalette" />
          </div>
          <section>
            <v-list-subheader class="px-0 ">Display Options</v-list-subheader>
            <div class="d-flex align-center justify-space-between">
              <span class="text-body-2 px-3">Dark Mode</span>
              <v-switch class="px-3" :model-value="isDarkMode" @update:model-value="$emit('update:isDarkMode', $event)"
                hide-details></v-switch>
            </div>
          </section>
        </div>
      </v-window-item>

      <v-window-item value="ping1d">
        <div class="px-4">
          <section>
            <v-list-subheader class="px-0 ">Display Settings</v-list-subheader>
            <div class="d-flex flex-column gap-4 px-3">
              <v-text-field v-model.number="localPing1DSettings.columnCount" type="number" label="Column Count"
                hide-details density="compact"></v-text-field>
              <v-text-field v-model.number="localPing1DSettings.tickCount" type="number" label="Tick Count" hide-details
                density="compact"></v-text-field>
            </div>
          </section>

          <section>
            <v-list-subheader class="px-0 ">Display Options</v-list-subheader>
            <div class="d-flex align-center justify-space-between pe-3">
              <span class="text-body-2 px-3">Debug Mode</span>
              <v-switch v-model="localPing1DSettings.debug" hide-details></v-switch>
            </div>
          </section>

          <section>
            <v-list-subheader class="px-0 ">Colors</v-list-subheader>
            <div class="d-flex flex-column">
              <div class="d-flex align-center justify-space-between">
                <span class="text-body-2 px-3">Depth Line Color</span>
                <ColorPickerField v-model="localPing1DSettings.depthLineColor" :defaultValue="'#ffeb3b'" />
              </div>
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

      <v-window-item value="ping360">
        <div class="px-4">

          <section class="mb-2">
            <v-list-subheader class="px-0 ">Display Settings</v-list-subheader>
            <div class="d-flex flex-column gap-4 px-3">
              <v-text-field v-model.number="localPing360Settings.numMarkers" type="number" label="Number of Markers"
                hide-details density="compact"></v-text-field>
              <v-text-field v-model.number="localPing360Settings.lineWidth" type="number" step="0.1" label="Line Width"
                hide-details density="compact"></v-text-field>
              <v-text-field v-model.number="localPing360Settings.radiusLineWidth" type="number" step="0.1"
                label="Radius Line Width" hide-details density="compact"></v-text-field>
            </div>
          </section>

          <section class="mb-2">
            <v-list-subheader>Display Options</v-list-subheader>
            <div class="d-flex flex-column px-3">
              <div class="d-flex align-center justify-space-between -mb-5 ">
                <span class="text-body-2">Show Radius Lines</span>
                <v-switch v-model="localPing360Settings.showRadiusLines" hide-details></v-switch>
              </div>
              <div class="d-flex align-center justify-space-between -mb-5 ">
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
            <v-list-subheader class="px-0 ">Colors</v-list-subheader>
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

      <v-window-item value="presets">
        <div class="px-4">
          <section class="mb-2">
            <div class=" mb-4">Accessibility Presets</div>
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
    </v-window>

    <v-divider class="my-2"></v-divider>

    <div class="d-flex -mb-6 justify-space-between">
      <div class="d-flex gap-4">
        <v-btn variant="text" @click="$emit('close')">
          Close
        </v-btn>
      </div>
      <div class="d-flex gap-4">
        <v-btn color="error" variant="text" @click="handleReset">
          Reset
        </v-btn>
        <v-btn color="primary" variant="text" @click="saveSettings">
          Save
        </v-btn>
      </div>
    </div>
  </div>
</template>

<script setup>
import { reactive, ref, watch } from 'vue';
import SonarColorOptions from '../widgets/SonarColorOptions.vue';
import { colorPalettes } from '../widgets/SonarColorOptions.vue';
import ColorPickerField from './ColorPickerField.vue';

const props = defineProps({
  commonSettings: {
    type: Object,
    required: true,
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
});

const emit = defineEmits([
  'update:commonSettings',
  'update:ping1DSettings',
  'update:ping360Settings',
  'update:isDarkMode',
  'save',
  'reset',
  'close',
]);

const activeSettingsTab = ref('general');
const selectedPreset = ref('default');
const localCommonSettings = reactive({ ...props.commonSettings });
const localPing1DSettings = reactive({ ...props.ping1DSettings });
const localPing360Settings = reactive({ ...props.ping360Settings });

const accessibilityPresets = [
  { title: 'Default', value: 'default' },
  { title: 'Deuteranopia (Red-Green)', value: 'deuteranopia' },
  { title: 'Protanopia (Red-Green)', value: 'protanopia' },
  { title: 'Tritanopia (Blue-Yellow)', value: 'tritanopia' },
  { title: 'Monochromacy', value: 'monochromacy' },
  { title: 'High Contrast', value: 'highContrast' },
];

const presetConfigs = {
  default: {
    description: 'Default color settings',
    settings: {
      commonSettings: {
        colorPalette: 'Ocean',
      },
      ping1DSettings: {
        columnCount: 100,
        tickCount: 5,
        depthLineColor: '#ffeb3b',
        depthTextColor: '#ffeb3b',
        currentDepthColor: '#ffeb3b',
        confidenceColor: '#4caf50',
        textBackground: 'rgba(0, 0, 0, 0.5)',
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
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
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
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
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
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
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
      commonSettings: {
        colorPalette: 'Monochrome Black',
      },
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
      commonSettings: {
        colorPalette: 'Monochrome White',
      },
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
  localStorage.removeItem('selectedAccessibilityPreset');
};

const handlePresetChange = (preset) => {
  if (preset === 'default') {
    handleReset();
    return;
  }

  const config = presetConfigs[preset].settings;

  Object.assign(localCommonSettings, {
    ...localCommonSettings,
    ...config.commonSettings,
  });

  Object.assign(localPing1DSettings, {
    ...localPing1DSettings,
    ...config.ping1DSettings,
  });

  Object.assign(localPing360Settings, {
    ...localPing360Settings,
    ...config.ping360Settings,
  });

  emit('update:commonSettings', { ...localCommonSettings });
  emit('update:ping1DSettings', { ...localPing1DSettings });
  emit('update:ping360Settings', { ...localPing360Settings });

  localStorage.setItem('selectedAccessibilityPreset', preset);
};

const updateColorPalette = (newPalette) => {
  localCommonSettings.colorPalette = newPalette;
  if (newPalette === 'Custom') {
    localCommonSettings.customPalette = colorPalettes.Custom;
  }
  emit('update:commonSettings', { ...localCommonSettings });
};

const saveSettings = () => {
  localStorage.setItem('common-settings', JSON.stringify(localCommonSettings));
  localStorage.setItem('ping1d-settings', JSON.stringify(localPing1DSettings));
  localStorage.setItem('ping360-settings', JSON.stringify(localPing360Settings));

  if (localCommonSettings.customPalette?.length > 0) {
    localStorage.setItem('customColorPalette', JSON.stringify(localCommonSettings.customPalette));
  }

  emit('save');
};

watch(
  () => props.commonSettings,
  (newSettings) => {
    Object.assign(localCommonSettings, newSettings);
  },
  { deep: true }
);

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
  localCommonSettings,
  (newSettings) => {
    emit('update:commonSettings', { ...newSettings });
  },
  { deep: true }
);

watch(
  localPing1DSettings,
  (newSettings) => {
    emit('update:ping1DSettings', { ...newSettings });
  },
  { deep: true }
);

watch(
  localPing360Settings,
  (newSettings) => {
    emit('update:ping360Settings', { ...newSettings });
  },
  { deep: true }
);
</script>
<style>
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