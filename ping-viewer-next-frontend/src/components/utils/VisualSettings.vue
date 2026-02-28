<template>
  <div class="settings-menu" :class="{ 'glass-inner disable-hover': glass }">
    <div :class="['menu-content', { 'glass-inner disable-hover': glass }]">
      <v-tabs v-model="activeTab" class="mb-2" grow>
        <v-tab value="display">
          <v-icon start>mdi-monitor</v-icon>
          Display
        </v-tab>
        <v-tab value="server">
          <v-icon start>mdi-server</v-icon>
          Server
        </v-tab>
      </v-tabs>

      <v-window v-model="activeTab" class="settings-window">
        <!-- Display Settings -->
        <v-window-item value="display">
          <div class="px-5 py-3">
            <div class="setting-row">
              <span class="setting-label">Units:</span>
              <v-select
                v-model="unitSystem"
                :items="[{ title: 'Metric', value: 'metric' }, { title: 'Imperial', value: 'imperial' }]"
                hide-details
                density="compact"
                variant="solo-filled"
                class="setting-select"
              />
            </div>

            <v-divider class="my-3" />

            <div class="setting-row">
              <span class="setting-label">Theme:</span>
              <v-select
                :model-value="isDarkMode ? 'Dark' : 'Light'"
                :items="['Light', 'Dark']"
                hide-details
                density="compact"
                variant="solo-filled"
                class="setting-select"
                @update:model-value="handleThemeChange"
              />
            </div>

            <v-divider class="my-3" />

            <v-checkbox
              v-model="localSettings.aScan"
              label="A-Scan"
              hide-details
              density="compact"
              class="mt-4"
            />

            <v-checkbox
              v-model="localSettings.debugMode"
              label="Debug Mode"
              hide-details
              density="compact"
            />

            <v-divider class="my-3" />

            <div class="setting-row">
              <span class="setting-label">Plot Theme:</span>
              <v-select
                v-model="localSettings.colorPalette"
                :items="paletteOptions"
                hide-details
                density="compact"
                variant="solo-filled"
                class="setting-select"
              />
            </div>

          </div>
        </v-window-item>

        <!-- Server Settings -->
        <v-window-item value="server">
          <div class="px-4">
            <!-- Server Configuration -->
            <section class="mb-6">
              <v-list-subheader class="px-0">Server Configuration</v-list-subheader>
              <v-text-field
                v-model="serverSettings.url"
                label="Server URL"
                placeholder="http://localhost:8080"
                hint="Enter the server URL"
                persistent-hint
                class="mb-4"
              />
            </section>

            <!-- MAVLink Configuration -->
            <section class="mb-6">
              <v-list-subheader class="px-0">MAVLink Connection</v-list-subheader>
              <v-text-field
                v-model="serverSettings.mavlinkUrl"
                label="MAVLink WebSocket URL"
                placeholder="ws://localhost:6040/ws/mavlink"
                hint="WebSocket URL for MAVLink connection"
                persistent-hint
                class="mb-4"
              />

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

                <v-btn
                  variant="elevated"
                  :color="mavlinkStatus === 'Connected' ? 'error' : 'success'"
                  size="small"
                  @click="toggleMavlinkConnection"
                  :loading="mavlinkStatus === 'Connecting'"
                >
                  {{ mavlinkStatus === 'Connected' ? 'Disconnect' : 'Connect' }}
                </v-btn>
              </div>
            </section>
          </div>
        </v-window-item>
      </v-window>

      <v-divider class="my-2" />

      <div class="d-flex justify-end pa-2">
        <v-btn variant="tonal" @click="saveSettings" class="px-8">
          SAVE
        </v-btn>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { colorPalettes } from '../widgets/SonarColorOptions.vue';
import { useUnits } from '../../composables/useUnits';

const { unitSystem } = useUnits();

const props = defineProps({
  isOpen: {
    type: Boolean,
    required: true,
  },
  glass: {
    type: Boolean,
    default: false,
  },
  displaySettings: {
    type: Object,
    required: true,
  },
  isDarkMode: {
    type: Boolean,
    required: true,
  },
  serverUrl: {
    type: String,
    default: '',
  },
  yawConnectionStatus: {
    type: String,
    default: 'Disconnected',
  },
});

const emit = defineEmits([
  'update:isOpen',
  'update:displaySettings',
  'update:isDarkMode',
  'update:serverUrl',
  'updateMavlink',
  'save',
]);

const activeTab = ref('display');
const localSettings = reactive({ ...props.displaySettings });
const mavlinkStatus = ref('Disconnected');

const paletteOptions = Object.keys(colorPalettes);

const serverSettings = reactive({
  url: props.serverUrl,
  mavlinkUrl: localStorage.getItem('mavlinkUrl') || 'ws://localhost:6040/ws/mavlink',
  autoConnectMavlink: localStorage.getItem('autoConnectMavlink') === 'true',
});

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

const handleThemeChange = (value) => {
  emit('update:isDarkMode', value === 'Dark');
};

const toggleMavlinkConnection = async () => {
  if (mavlinkStatus.value === 'Connected') {
    emit('updateMavlink', { action: 'disconnect', url: serverSettings.mavlinkUrl });
  } else {
    mavlinkStatus.value = 'Connecting';
    emit('updateMavlink', {
      action: 'connect',
      url: serverSettings.mavlinkUrl,
      autoConnect: serverSettings.autoConnectMavlink,
    });
  }
};

const saveSettings = () => {
  localStorage.setItem('display-settings', JSON.stringify(localSettings));

  if (serverSettings.url !== props.serverUrl) {
    emit('update:serverUrl', serverSettings.url);
  }

  // Save server settings
  localStorage.setItem('serverUrl', serverSettings.url);
  localStorage.setItem('mavlinkUrl', serverSettings.mavlinkUrl);
  localStorage.setItem('autoConnectMavlink', serverSettings.autoConnectMavlink.toString());

  if (mavlinkStatus.value === 'Connected') {
    emit('updateMavlink', {
      action: 'reconnect',
      url: serverSettings.mavlinkUrl,
      autoConnect: serverSettings.autoConnectMavlink,
    });
  } else if (serverSettings.autoConnectMavlink) {
    emit('updateMavlink', {
      action: 'connect',
      url: serverSettings.mavlinkUrl,
      autoConnect: true,
    });
  }

  emit('update:displaySettings', { ...localSettings });
  emit('save');
  emit('update:isOpen', false);
};

watch(
  () => props.displaySettings,
  (newSettings) => {
    Object.assign(localSettings, newSettings);
  },
  { deep: true }
);

watch(
  localSettings,
  (newSettings) => {
    emit('update:displaySettings', { ...newSettings });
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

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.setting-label {
  min-width: 100px;
  white-space: nowrap;
}

.setting-select {
  flex: 1;
  max-width: 220px;
}
</style>
