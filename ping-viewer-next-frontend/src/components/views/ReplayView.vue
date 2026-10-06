<template>
  <div class="relative">
    <component v-if="isComponentReady" :is="deviceComponent" :device="currentDeviceData.device"
      v-bind="deviceSpecificProps" class="z-10"></component>
  </div>
</template>

<script setup>
import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { calculateRange, gradiansToDegrees } from '../../ping-device/utils/ping360-utils';
import Ping1D from '../widgets/sonar1d/Ping1D.vue';
import Ping360 from '../widgets/sonar360/Ping360.vue';

const { commonSettings, ping1DSettings, ping360Settings, displaySettings } =
  inject('deviceSettings');

const currentDeviceData = ref(null);
const deviceView = ref(null);
const isComponentReady = ref(false);
const loadedDeviceType = ref(null);
const heading = ref(null);

const componentDimensions = ref({
  width: 800,
  height: 600,
});

const deviceComponent = computed(() => {
  if (!currentDeviceData.value) return null;
  return currentDeviceData.value.device.device_type === 'Ping360' ? Ping360 : Ping1D;
});

const deviceSpecificProps = computed(() => {
  if (!currentDeviceData.value) return {};

  const baseProps = {
    width: componentDimensions.value.width,
    height: componentDimensions.value.height,
  };

  if (currentDeviceData.value.device.device_type === 'Ping360') {
    const d = currentDeviceData.value.data.measurementRaw;
    let maxRange = 300;
    let startAngle = 0;
    let endAngle = 360;
    if (d) {
      if (d.sample_period && d.number_of_samples) {
        maxRange = calculateRange({
          sample_period: d.sample_period,
          number_of_samples: d.number_of_samples,
          speed_of_sound: 1500,
        });
      }
      if (d.start_angle === 0 && d.stop_angle === 399) {
        startAngle = 0;
        endAngle = 360;
      } else {
        startAngle = (gradiansToDegrees(d.start_angle) + 180) % 360;
        endAngle = (gradiansToDegrees(d.stop_angle) + 180) % 360;
      }
    }
    return {
      ...baseProps,
      ...commonSettings,
      ...ping360Settings,
      measurement: currentDeviceData.value.data.measurement,
      measurements: currentDeviceData.value.data.measurements,
      angle: currentDeviceData.value.data.measurement.angle,
      maxDistance: maxRange,
      startAngle,
      endAngle,
      yaw_angle: heading.value ?? 0,
    };
  }

  return {
    ...baseProps,
    ...commonSettings,
    ...ping1DSettings,
    aScan: displaySettings?.aScan ?? true,
    sensorData: currentDeviceData.value.data.sensorData,
    currentDepth: currentDeviceData.value.data.currentDepth,
    minDepth: currentDeviceData.value.data.minDepth,
    maxDepth: currentDeviceData.value.data.maxDepth,
    confidence: currentDeviceData.value.data.confidence,
    accuracy: currentDeviceData.value.data.accuracy,
  };
});

const updateComponentDimensions = () => {
  if (!deviceView.value) return;

  const container = deviceView.value;
  const containerRect = container.getBoundingClientRect();
  const padding = 32;
  const availableWidth = containerRect.width - padding;
  const availableHeight = containerRect.height - padding;

  componentDimensions.value = {
    width: Math.floor(availableWidth),
    height: Math.floor(availableHeight),
  };
};

const handleDeviceTypeChange = async (newType) => {
  if (loadedDeviceType.value === newType) return;

  isComponentReady.value = false;
  await nextTick();

  loadedDeviceType.value = newType;
  isComponentReady.value = true;
};

const toMeasurement = (sample) => {
  const dataArray = Array.isArray(sample.data) ? sample.data : Object.values(sample.data);
  return {
    angle: sample.angle,
    data: new Uint8Array(dataArray),
  };
};

const applyPing360Frames = (frames) => {
  const lines = frames.map((frame) => toMeasurement(frame.data));
  const last = frames[frames.length - 1];
  currentDeviceData.value = {
    device: last.device,
    data: {
      measurement: lines[lines.length - 1],
      measurements: lines,
      measurementRaw: last.data,
    },
  };
};

const updateFrames = async (frames) => {
  if (!frames?.length) return;
  const last = frames[frames.length - 1];
  if (!last?.device) return;

  await handleDeviceTypeChange(last.device.device_type);

  if (last.device.device_type === 'Ping360') {
    applyPing360Frames(frames);
    return;
  }

  currentDeviceData.value = {
    device: last.device,
    data: last.data,
  };
};

const updateCurrentDeviceData = (frame) => updateFrames([frame]);

const setHeading = (degrees) => {
  heading.value = typeof degrees === 'number' && Number.isFinite(degrees) ? degrees : null;
};

const onDataLoaded = async (data) => {
  heading.value = null;
  if (data.length > 0) {
    isComponentReady.value = false;
    currentDeviceData.value = null;
    loadedDeviceType.value = null;
    await nextTick();

    updateCurrentDeviceData(data[0]);
  }
};

const handleResize = () => {
  updateComponentDimensions();
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
  nextTick(() => {
    updateComponentDimensions();
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  isComponentReady.value = false;
  currentDeviceData.value = null;
  loadedDeviceType.value = null;
});

defineExpose({
  updateCurrentDeviceData,
  updateFrames,
  setHeading,
  onDataLoaded,
});
</script>