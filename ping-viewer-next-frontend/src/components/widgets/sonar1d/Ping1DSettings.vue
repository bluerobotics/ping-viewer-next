<template>
  <v-card>
    <v-card-title class="text-h5 pb-2">Ping1D Settings</v-card-title>
    <v-card-text>
      <div v-if="isLoading" class="d-flex justify-center my-4">
        <v-progress-circular indeterminate />
      </div>
      <div v-else class="mb-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <v-tooltip text="Enable automatic parameter adjustment" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Auto Mode
              </span>
            </template>
          </v-tooltip>
          <v-switch class="gap-2" v-model="isAutoMode" hide-details density="compact"
            @update:model-value="handleAutoModeChange"></v-switch>
        </div>

        <div class="d-flex align-center justify-space-between mb-1">
          <v-tooltip text="Scanning range in meters" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Range
              </span>
            </template>
          </v-tooltip>
          <span class="text-caption text-medium-emphasis mr-1">meters</span>
        </div>
        <div class="d-flex align-center gap-2 mb-4">
          <v-text-field v-model.number="settings.scan_start" type="number" label="Start" :disabled="isAutoMode"
            density="compact" hide-details style="width: 90px" @update:model-value="debouncedSaveSettings" />
          <v-text-field v-model.number="settings.scan_length" type="number" label="Length" :disabled="isAutoMode"
            density="compact" hide-details style="width: 90px" @update:model-value="debouncedSaveSettings" />
        </div>

        <div class="d-flex align-center justify-space-between mb-1">
          <v-tooltip text="Signal amplification level" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Gain Setting
              </span>
            </template>
          </v-tooltip>
        </div>
        <v-select v-model="settings.gain_setting" :items="gainOptions" :disabled="isAutoMode"
          density="compact" hide-details class="mb-4" @update:model-value="debouncedSaveSettings"></v-select>

        <div class="d-flex align-center justify-space-between">
          <v-tooltip text="Number of pings per second (Hz)" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Ping/s
              </span>
            </template>
          </v-tooltip>
        </div>
        <div class="d-flex align-center gap-2 mb-4">
          <v-slider v-model="pingsPerSecond" :min="0" :max="30" :step="1" density="compact" hide-details
            class="flex-grow-1" @update:model-value="debouncedSaveSettings"></v-slider>
          <v-text-field v-if="pingsPerSecond != 0" v-model.number="pingsPerSecond" type="number" :min="0" :max="30" :step="1"
            density="compact" hide-details style="width: 10px" @update:model-value="debouncedSaveSettings"
          ></v-text-field>
          <v-btn v-if="pingsPerSecond === 0" variant="tonal" @click="manualPing">
            Ping
          </v-btn>
        </div>

        <div class="d-flex align-center justify-space-between mb-1">
          <v-tooltip text="Speed of sound in water" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Speed of Sound
              </span>
            </template>
          </v-tooltip>
          <span class="text-caption text-medium-emphasis mr-1">m/s</span>
        </div>
        <div class="d-flex align-center gap-2">
          <v-slider v-model="settings.speed_of_sound" :min="1400" :max="1600" :step="1" density="compact" hide-details
            class="flex-grow-1" @update:model-value="debouncedSaveSettings"></v-slider>
          <v-text-field v-model.number="settings.speed_of_sound" type="number" :min="1400" :max="1600" :step="1"
            density="compact" hide-details style="width: 10px" @update:model-value="debouncedSaveSettings"></v-text-field>
        </div>
      </div>
    </v-card-text>
  </v-card>
</template>

<script setup>
import { useDebounceFn } from '@vueuse/core';
import { computed, onMounted, ref, watch } from 'vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
  deviceId: {
    type: String,
    required: true,
  },
  isOpen: {
    type: Boolean,
    default: false,
  },
});

const DEBOUNCE_VALUE_MS = 500;

const isLoading = ref(false);
const isInitializing = ref(true);
const rawAutoMode = ref(1);

const isAutoMode = computed({
  get: () => Boolean(rawAutoMode.value),
  set: (value) => {
    rawAutoMode.value = value ? 1 : 0;
  },
});

const settings = ref({
  scan_start: 0,
  scan_length: 10,
  gain_setting: 0,
  speed_of_sound: 1500,
  ping_interval: 25,
});

const pingsPerSecond = computed({
  get: () =>
    settings.value.ping_interval > 0 ? Math.round(1000 / settings.value.ping_interval) : 0,
  set: (value) => {
    if (value > 0) {
      settings.value.ping_interval = Math.round(1000 / Math.max(1, value));
    } else {
      settings.value.ping_interval = 0;
    }
  },
});

const gainOptions = [
  { title: '0.6', value: 0 },
  { title: '1.8', value: 1 },
  { title: '5.5', value: 2 },
  { title: '12.9', value: 3 },
  { title: '30.2', value: 4 },
  { title: '66.1', value: 5 },
  { title: '144', value: 6 },
];

const debouncedSaveSettings = useDebounceFn(async () => {
  if (isInitializing.value) return;

  try {
    await sendCommand('SetModeAuto', {
      mode_auto: rawAutoMode.value,
    });

    if (!isAutoMode.value) {
      await sendCommand('SetRange', {
        scan_start: Math.round(settings.value.scan_start * 1000),
        scan_length: Math.round(settings.value.scan_length * 1000),
      });

      await sendCommand('SetGainSetting', {
        gain_setting: settings.value.gain_setting,
      });
    }

    if (settings.value.ping_interval > 0) {
      await sendCommand('SetPingInterval', {
        ping_interval: settings.value.ping_interval,
      });
      await enableContinuousMode();
    } else {
      await disableContinuousMode();
    }

    await sendCommand('SetSpeedOfSound', {
      speed_of_sound: Math.round(settings.value.speed_of_sound * 1000),
    });
  } catch (error) {
    console.error('Error saving settings:', error);
  }
}, DEBOUNCE_VALUE_MS);

const enableContinuousMode = async () => {
  try {
    await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        command: 'EnableContinuousMode',
        module: 'DeviceManager',
        payload: {
          uuid: props.deviceId,
        },
      }),
    });
  } catch (error) {
    console.error('Failed to enable continuous mode:', error);
  }
};

const disableContinuousMode = async () => {
  try {
    await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        command: 'DisableContinuousMode',
        module: 'DeviceManager',
        payload: {
          uuid: props.deviceId,
        },
      }),
    });
  } catch (error) {
    console.error('Failed to disable continuous mode:', error);
  }
};

const manualPing = async () => {
  try {
    await sendCommand('Profile');
  } catch (error) {
    console.error('Failed to send manual ping:', error);
  }
};

const handleAutoModeChange = () => {
  debouncedSaveSettings();
};

const fetchCurrentSettings = async () => {
  isLoading.value = true;
  isInitializing.value = true;
  try {
    const settingsToFetch = ['ModeAuto', 'Range', 'GainSetting', 'SpeedOfSound', 'PingInterval'];

    for (const setting of settingsToFetch) {
      const response = await sendCommand(setting);
      if (response?.DeviceMessage?.PingMessage?.Ping1D) {
        const data = response.DeviceMessage.PingMessage.Ping1D[setting];

        switch (setting) {
          case 'ModeAuto':
            rawAutoMode.value = data.mode_auto;
            break;
          case 'Range':
            settings.value.scan_start = data.scan_start / 1000;
            settings.value.scan_length = data.scan_length / 1000;
            break;
          case 'GainSetting':
            settings.value.gain_setting = data.gain_setting;
            break;
          case 'SpeedOfSound':
            settings.value.speed_of_sound = Math.round(data.speed_of_sound / 1000);
            break;
          case 'PingInterval':
            settings.value.ping_interval = data.ping_interval;
            break;
        }
      }
    }
  } catch (error) {
    console.error('Error fetching settings:', error);
  } finally {
    isLoading.value = false;
    setTimeout(() => {
      isInitializing.value = false;
    }, DEBOUNCE_VALUE_MS);
  }
};

const sendCommand = async (command, payload = null) => {
  try {
    const requestBody = {
      command: 'Ping',
      module: 'DeviceManager',
      payload: {
        device_request: {
          Ping1D: payload ? { [command]: payload } : command,
        },
        uuid: props.deviceId,
      },
    };

    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    return await response.json();
  } catch (error) {
    console.error(`Error sending command ${command}:`, error);
    return null;
  }
};

watch(
  () => props.isOpen,
  async (newValue) => {
    if (newValue) {
      await fetchCurrentSettings();
    }
  }
);

onMounted(async () => {
  if (props.isOpen) {
    await fetchCurrentSettings();
  }
});
</script>