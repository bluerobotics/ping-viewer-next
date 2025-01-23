<template>
  <div class="py-10 relative">
    <component v-if="isComponentReady" :is="deviceComponent" :device="currentDeviceData.device"
      v-bind="deviceSpecificProps" class="z-10"></component>
  </div>
</template>

<script setup>
import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import Ping1D from '../widgets/sonar1d/Ping1D.vue';
import Ping360 from '../widgets/sonar360/Ping360.vue';

const { commonSettings, ping1DSettings, ping360Settings } = inject('deviceSettings');

const currentDeviceData = ref(null);
const deviceView = ref(null);
const isComponentReady = ref(false);
const loadedDeviceType = ref(null);

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
    return {
      ...baseProps,
      ...commonSettings,
      ...ping360Settings,
      measurement: currentDeviceData.value.data.measurement,
      angle: currentDeviceData.value.data.measurement.angle,
      startAngle: 0,
      endAngle: 360,
    };
  }

  return {
    ...baseProps,
    ...commonSettings,
    ...ping1DSettings,
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

const updateCurrentDeviceData = async (frame) => {
  if (!frame || !frame.device) return;

  await handleDeviceTypeChange(frame.device.device_type);

  if (frame.device.device_type === 'Ping360') {
    const dataArray = Array.isArray(frame.data.data)
      ? frame.data.data
      : Object.values(frame.data.data);

    currentDeviceData.value = {
      device: frame.device,
      data: {
        measurement: {
          angle: frame.data.angle,
          data: new Uint8Array(dataArray),
        },
      },
    };
  } else {
    currentDeviceData.value = {
      device: frame.device,
      data: frame.data,
    };
  }

  nextTick(() => {
    updateComponentDimensions();
  });
};

const onDataLoaded = async (data) => {
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
  onDataLoaded,
});
</script>