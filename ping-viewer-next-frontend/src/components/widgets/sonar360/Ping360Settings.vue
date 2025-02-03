<template>
  <v-card>
    <v-card-title class="text-h5 pb-2">Sonar Settings</v-card-title>

    <v-card-text>
      <div v-if="isLoading" class="d-flex justify-center my-4">
        <v-progress-circular indeterminate></v-progress-circular>
      </div>

      <div v-else class="mb-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <v-tooltip text="Analog gain setting (0 = low, 1 = normal, 2 = high)" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Gain Setting
              </span>
            </template>
          </v-tooltip>
        </div>
        <div class="d-flex align-center gap-2">
          <v-slider v-model="settings.gain_setting" :min="0" :max="2" :step="1" show-ticks="always" tick-size="3"
            :ticks="{ 0: 'low', 1: 'normal', 2: 'high' }" density="compact" hide-details class="flex-grow-1" />
        </div>

        <div class="d-flex align-center justify-space-between mb-1 mt-4">
          <v-tooltip text="Scanning range in meters" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Range
              </span>
            </template>
          </v-tooltip>
          <span class="text-caption text-medium-emphasis mr-1">meters</span>
        </div>
        <div class="d-flex align-center gap-2">
          <v-slider v-model="range" :min="1.8" :max="60" :step="0.1" density="compact" hide-details class="flex-grow-1"
            @update:modelValue="handleRangeChange" />
          <v-text-field v-model.number="range" type="number" :min="1.8" :max="60" :step="0.1" density="compact"
            hide-details style="width: 82px !important; flex: 0 0 auto" @update:modelValue="handleRangeChange" />
        </div>
      </div>

      <div class="mb-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <v-tooltip text="Scanning angle range" location="left">
            <template v-slot:activator="{ props }">
              <span v-bind="props" class="text-body-2 text-medium-emphasis">
                Angle Range
              </span>
            </template>
          </v-tooltip>
          <div class="d-flex align-center">
            <v-tooltip text="Mirror angles" location="bottom">
              <template v-slot:activator="{ props }">
                <v-btn v-bind="props" icon="mdi-mirror" size="small" :color="isMirrorEnabled ? 'primary' : undefined"
                  class="mr-2" @click="mirrorAngles">
                  <v-icon>mdi-mirror</v-icon>
                </v-btn>
              </template>
            </v-tooltip>
          </div>
        </div>
        <div class="d-flex align-center gap-2">
          <v-range-slider v-model="angleRange" :min="0" :max="360" step="1" show-ticks="always" tick-size="4"
            thumb-label :ticks="{ 0: '0°', 180: '180°', 360: '360°' }" density="compact" hide-details
            class="flex-grow-1" @update:modelValue="handleAngleChange" />
        </div>
      </div>

      <v-divider class="mb-4"></v-divider>

      <v-btn block variant="tonal" @click="showAdvanced = !showAdvanced" class="mb-4">
        <v-icon :icon="showAdvanced ? 'mdi-chevron-up' : 'mdi-chevron-down'" class="mr-2"></v-icon>
        {{ showAdvanced ? 'Hide Advanced Settings' : 'Show Advanced Settings' }}
      </v-btn>

      <v-expand-transition>
        <div v-if="showAdvanced">
          <div class="mb-4">
            <div class="d-flex align-center justify-space-between">
              <v-tooltip text="Enable automatic parameter adjustment based on range" location="left">
                <template v-slot:activator="{ props }">
                  <span v-bind="props" class="text-body-2 text-medium-emphasis">
                    Auto Mode
                  </span>
                </template>
              </v-tooltip>
            </div>
            <v-switch v-model="autoMode" density="compact" hide-details />
          </div>

          <div class="d-flex align-center justify-space-between mb-2 mt-4">
            <v-tooltip text="Speed of sound in water" location="left">
              <template v-slot:activator="{ props }">
                <span v-bind="props" class="text-body-2 text-medium-emphasis">
                  Speed of Sound
                </span>
              </template>
            </v-tooltip>
            <span class="text-caption text-medium-emphasis mr-1">m/s</span>
          </div>
          <div class="d-flex align-center gap-2 mb-8">
            <v-slider v-model="settings.speed_of_sound" :min="1400" :max="1600" :step="1" density="compact" hide-details
              class="flex-grow-1" @update:modelValue="handleSpeedOfSoundChange" />
            <v-text-field v-model.number="settings.speed_of_sound" type="number" :min="1400" :max="1600" :step="1"
              density="compact" hide-details style="width: 80px" @update:modelValue="handleSpeedOfSoundChange" />
          </div>

          <div class="d-flex align-center justify-space-between mb-1 mt-4">
            <v-tooltip text="Time interval between samples (25ns units)" location="left">
              <template v-slot:activator="{ props }">
                <span v-bind="props" class="text-body-2 text-medium-emphasis">
                  Sample Period
                </span>
              </template>
            </v-tooltip>
            <span class="text-caption text-medium-emphasis mr-1">25ns</span>
          </div>
          <div class="d-flex align-center gap-2 mb-8">
            <v-slider v-model="settings.sample_period" :min="MIN_SAMPLE_PERIOD" :max="MAX_SAMPLE_PERIOD" :step="1"
              density="compact" hide-details class="flex-grow-1" :disabled="autoMode"
              @update:modelValue="handleSamplePeriodChange" />
            <v-text-field v-model.number="settings.sample_period" type="number" :min="MIN_SAMPLE_PERIOD"
              :max="MAX_SAMPLE_PERIOD" :step="1" density="compact" hide-details style="width: 80px" :disabled="autoMode"
              @update:modelValue="handleSamplePeriodChange" />
          </div>

          <div class="d-flex align-center justify-space-between mb-1 mt-4">
            <v-tooltip text="Number of samples per scan" location="left">
              <template v-slot:activator="{ props }">
                <span v-bind="props" class="text-body-2 text-medium-emphasis">
                  Number of Samples
                </span>
              </template>
            </v-tooltip>
          </div>
          <div class="d-flex align-center gap-2 mb-8">
            <v-slider v-model="settings.number_of_samples" :min="MIN_NUMBER_OF_POINTS" :max="MAX_NUMBER_OF_POINTS"
              :step="1" density="compact" hide-details class="flex-grow-1" :disabled="autoMode"
              @update:modelValue="handleNumberOfSamplesChange" />
            <v-text-field v-model.number="settings.number_of_samples" type="number" :min="MIN_NUMBER_OF_POINTS"
              :max="MAX_NUMBER_OF_POINTS" :step="1" density="compact" hide-details style="width: 80px"
              :disabled="autoMode" @update:modelValue="handleNumberOfSamplesChange" />
          </div>

          <div class="d-flex align-center justify-space-between mb-1 mt-4">
            <v-tooltip text="Duration of acoustic transmission" location="left">
              <template v-slot:activator="{ props }">
                <span v-bind="props" class="text-body-2 text-medium-emphasis">
                  Transmit Duration
                </span>
              </template>
            </v-tooltip>
            <span class="text-caption text-medium-emphasis mr-1">µs</span>
          </div>
          <div class="d-flex align-center gap-2 mb-8">
            <v-slider v-model="settings.transmit_duration" :min="MIN_TRANSMIT_DURATION" :max="transmitDurationMax"
              :step="1" density="compact" hide-details class="flex-grow-1" :disabled="autoMode"
              @update:modelValue="handleTransmitDurationChange" />
            <v-text-field v-model.number="settings.transmit_duration" type="number" :min="MIN_TRANSMIT_DURATION"
              :max="transmitDurationMax" :step="1" density="compact" hide-details style="width: 80px"
              :disabled="autoMode" @update:modelValue="handleTransmitDurationChange" />
          </div>

          <div class="d-flex align-center justify-space-between mb-1 mt-4">
            <v-tooltip text="Operating frequency" location="left">
              <template v-slot:activator="{ props }">
                <span v-bind="props" class="text-body-2 text-medium-emphasis">
                  Transmit Frequency
                </span>
              </template>
            </v-tooltip>
            <span class="text-caption text-medium-emphasis mr-1">kHz</span>
          </div>
          <div class="d-flex align-center gap-2 mb-8">
            <v-slider v-model="settings.transmit_frequency" :min="500" :max="1000" :step="1" density="compact"
              hide-details class="flex-grow-1" />
            <v-text-field v-model.number="settings.transmit_frequency" type="number" :min="500" :max="1000" :step="1"
              density="compact" hide-details style="width: 80px" />
          </div>
        </div>
      </v-expand-transition>

      <v-divider class="my-4"></v-divider>

      <div class="d-flex justify-end -mb-1">
        <v-btn color="primary" @click="saveSettings" :loading="isSaving" :disabled="isSaving || isLoading">
          {{ isSaving ? 'Applying...' : 'Apply Settings' }}
        </v-btn>
      </div>
    </v-card-text>
  </v-card>
</template>

<script setup>
import { computed, ref, watch } from 'vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
  deviceId: {
    type: String,
    required: true,
  },
  initialAngles: {
    type: Object,
    default: () => ({ startAngle: 0, endAngle: 360 }),
  },
  isOpen: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(['update:range', 'rangeChange', 'update:angles', 'update:isOpen']);

// Constants from Ping360 specs
const SAMPLE_PERIOD_TICK_DURATION = 25e-9;
const MIN_SAMPLE_PERIOD = 80;
const MAX_SAMPLE_PERIOD = 40000;
const MIN_NUMBER_OF_POINTS = 200;
const MAX_NUMBER_OF_POINTS = 1200;
const MIN_TRANSMIT_DURATION = 1;
const MAX_TRANSMIT_DURATION = 1000;

const isSaving = ref(false);
const isLoading = ref(false);
const showAdvanced = ref(false);
const autoMode = ref(true);
const range = ref(10);
const angleRange = ref([0, 360]);
const isMirrorEnabled = ref(false);

function mirrorAngles() {
  isMirrorEnabled.value = !isMirrorEnabled.value;
  if (isMirrorEnabled.value && !(angleRange.value[0] === 0 && angleRange.value[1] === 360)) {
    const startAngle = angleRange.value[0];
    const complementAngle = (360 - startAngle) % 360;
    angleRange.value = [startAngle, complementAngle];
  }
  handleAngleChange(angleRange.value);
}

const settings = ref({
  gain_setting: 0,
  transmit_duration: 32,
  sample_period: MIN_SAMPLE_PERIOD,
  transmit_frequency: 740,
  number_of_samples: MAX_NUMBER_OF_POINTS,
  speed_of_sound: 1500,
});

const transmitDurationMax = computed(() => {
  return Math.min(
    MAX_TRANSMIT_DURATION,
    Math.floor(settings.value.sample_period * SAMPLE_PERIOD_TICK_DURATION * 64e6)
  );
});

function degreesToGradians(degrees) {
  if (degrees === 360) {
    return 399;
  }
  return Math.round((degrees * 400) / 360);
}

function gradiansToDegrees(gradians) {
  if (gradians === 399) {
    return 360;
  }
  return Math.round((gradians * 360) / 400);
}
const fetchCurrentSettings = async () => {
  isLoading.value = true;
  try {
    const requestBody = {
      command: 'ModifyDevice',
      module: 'DeviceManager',
      payload: {
        uuid: props.deviceId,
        modify: 'GetPing360Config',
      },
    };

    const response = await fetch(`${props.serverUrl}/v1/device_manager/request`, {
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

    const data = await response.json();
    if (data?.DeviceConfig?.Ping360Config) {
      const config = data.DeviceConfig.Ping360Config;

      settings.value = {
        gain_setting: config.gain_setting,
        transmit_duration: config.transmit_duration,
        sample_period: config.sample_period,
        transmit_frequency: config.transmit_frequency,
        number_of_samples: config.number_of_samples,
        speed_of_sound: 1500,
      };

      const startAngleDegrees = gradiansToDegrees(config.start_angle);
      const stopAngleDegrees = gradiansToDegrees(config.stop_angle);

      angleRange.value = [startAngleDegrees, stopAngleDegrees];

      handleAngleChange(angleRange.value);
      range.value = calculateRange();
    }
  } catch (error) {
    console.error('Error fetching settings:', error);
  } finally {
    isLoading.value = false;
  }
};

defineExpose({ fetchCurrentSettings });

function calculateRange() {
  const samplePeriod = settings.value.sample_period * SAMPLE_PERIOD_TICK_DURATION;
  return (
    Math.round(
      ((samplePeriod * settings.value.number_of_samples * settings.value.speed_of_sound) / 2) * 10
    ) / 10
  );
}

function calculateSamplePeriod(desiredRange) {
  return Math.ceil(
    (2 * desiredRange) /
      (settings.value.number_of_samples *
        settings.value.speed_of_sound *
        SAMPLE_PERIOD_TICK_DURATION)
  );
}

function adjustTransmitDuration() {
  if (!autoMode.value) return;

  let autoDuration = Math.round((8000 * range.value) / settings.value.speed_of_sound);

  autoDuration = Math.round(
    Math.max(
      Math.ceil(2.5 * settings.value.sample_period * SAMPLE_PERIOD_TICK_DURATION * 1e6),
      autoDuration
    )
  );

  settings.value.transmit_duration = Math.round(
    Math.max(MIN_TRANSMIT_DURATION, Math.min(transmitDurationMax.value, autoDuration))
  );
}

function handleAngleChange(newAngles) {
  if (newAngles[0] === 0 && newAngles[1] === 360) {
    emit('update:angles', { startAngle: 0, endAngle: 360 });
    return;
  }

  const rotateAngle = (angle) => (angle + 180) % 360;

  const effectiveAngles = {
    startAngle: rotateAngle(newAngles[0]),
    endAngle: rotateAngle(newAngles[1]),
  };

  emit('update:angles', effectiveAngles);
}

function swapAngles() {
  isSwapEnabled.value = !isSwapEnabled.value;
  handleAngleChange(angleRange.value);
}

function handleRangeChange(newRange) {
  if (!autoMode.value) {
    range.value = Number(newRange.toFixed(1));
    return;
  }

  const newSamplePeriod = calculateSamplePeriod(newRange);

  if (newSamplePeriod < MIN_SAMPLE_PERIOD) {
    settings.value.number_of_samples = Math.max(
      MIN_NUMBER_OF_POINTS,
      Math.floor(
        (2 * newRange) /
          (MIN_SAMPLE_PERIOD * SAMPLE_PERIOD_TICK_DURATION * settings.value.speed_of_sound)
      )
    );
    settings.value.sample_period = MIN_SAMPLE_PERIOD;
  } else if (newSamplePeriod > MAX_SAMPLE_PERIOD) {
    settings.value.sample_period = MAX_SAMPLE_PERIOD;
    settings.value.number_of_samples = Math.min(
      MAX_NUMBER_OF_POINTS,
      Math.ceil(
        (2 * newRange) /
          (MAX_SAMPLE_PERIOD * SAMPLE_PERIOD_TICK_DURATION * settings.value.speed_of_sound)
      )
    );
  } else {
    settings.value.sample_period = newSamplePeriod;
  }

  adjustTransmitDuration();
  range.value = Number(newRange.toFixed(1));
  emit('rangeChange', newRange);
  emit('update:range', newRange);
}

function handleSpeedOfSoundChange() {
  if (autoMode.value) {
    handleRangeChange(range.value);
  }
}

function handleSamplePeriodChange() {
  if (!autoMode.value) {
    range.value = calculateRange();
  }
}

function handleNumberOfSamplesChange() {
  if (!autoMode.value) {
    range.value = calculateRange();
  }
}

function handleTransmitDurationChange() {
  if (!autoMode.value && settings.value.transmit_duration > transmitDurationMax.value) {
    settings.value.transmit_duration = transmitDurationMax.value;
  }
}

const sendCommand = async (command, payload = null) => {
  try {
    const requestBody = {
      command: 'Ping',
      module: 'DeviceManager',
      payload: {
        device_request: {
          Ping360: payload ? { [command]: payload } : command,
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

const saveSettings = async () => {
  isSaving.value = true;
  try {
    let startGradians;
    let endGradians;

    if (angleRange.value[0] === 0 && angleRange.value[1] === 360) {
      startGradians = 0;
      endGradians = 399;
    } else {
      startGradians = degreesToGradians(angleRange.value[0]);
      endGradians = degreesToGradians(angleRange.value[1]);
    }

    const modifyCommand = {
      command: 'ModifyDevice',
      module: 'DeviceManager',
      payload: {
        uuid: props.deviceId,
        modify: {
          SetPing360Config: {
            mode: 1,
            gain_setting: settings.value.gain_setting,
            transmit_duration: settings.value.transmit_duration,
            sample_period: settings.value.sample_period,
            transmit_frequency: settings.value.transmit_frequency,
            number_of_samples: settings.value.number_of_samples,
            start_angle: startGradians,
            stop_angle: endGradians,
            num_steps: 1,
            delay: 10,
          },
        },
      },
    };

    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify(modifyCommand),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    emit('update:isOpen', false);
  } catch (error) {
    console.error('Error saving settings:', error);
  } finally {
    isSaving.value = false;
  }
};

watch(autoMode, (newValue) => {
  if (newValue) {
    handleRangeChange(range.value);
  }
});

watch(
  [
    () => settings.value.sample_period,
    () => settings.value.number_of_samples,
    () => settings.value.speed_of_sound,
  ],
  () => {
    if (autoMode.value) {
      handleRangeChange(range.value);
    }
  },
  { deep: true }
);

watch(
  angleRange,
  (newValue, oldValue) => {
    if (newValue[0] === oldValue?.[0] && newValue[1] === oldValue?.[1]) {
      return;
    }

    const [start, end] = newValue;

    if (start === 0 && end === 360) {
      handleAngleChange(newValue);
      return;
    }

    if (!isMirrorEnabled.value && end <= start) {
      angleRange.value = [end, start];
      return;
    }

    if (isMirrorEnabled.value) {
      if (start === 0) {
        angleRange.value = [start, 360];
        handleAngleChange(angleRange.value);
        return;
      }

      const complementAngle = (360 - start) % 360;
      if (Math.abs(end - complementAngle) > 1) {
        angleRange.value = [start, complementAngle];
        return;
      }
    }

    handleAngleChange(angleRange.value);
  },
  { deep: true, immediate: true }
);

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

<style>
.v-card.v-theme--dark {
  scrollbar-width: none;
  scrollbar-color: rgba(255, 255, 255, 0.2) rgba(255, 255, 255, 0.1);
}
</style>