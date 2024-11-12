<template>
  <div class="h-full w-full flex flex-col bg-black">
    <div class="flex-1 flex flex-col overflow-hidden">
      <div v-if="!selectedDevice" class="flex-1 overflow-auto">
        <div class="flex-none p-4">
          <h1 class="text-2xl font-bold text-white">Sonar Device Dashboard</h1>
        </div>

        <div class="p-4">
          <DeviceSettings
            :server-url="serverUrl"
            @openDevice="selectDevice"
          />
        </div>
      </div>

      <div v-else class="flex-1 flex flex-col overflow-hidden" ref="deviceView">
        <div class="flex-none bg-gray-800 p-4 flex justify-between items-center">
          <button @click="backToDevices" class="flex items-center text-blue-400 hover:text-blue-300">
            <v-icon icon="mdi-arrow-left" class="mr-2" />
            Back to Devices
          </button>

          <div class="flex items-center gap-4">
            <div class="text-white">
              <span class="font-bold">{{ selectedDevice.device_type }}</span>
              <span class="text-gray-400 ml-2">{{ selectedDevice.id }}</span>
            </div>
            <v-chip :color="getStatusColor(selectedDevice.status)" size="small">
              {{ selectedDevice.status }}
            </v-chip>
          </div>
        </div>

        <div class="flex-1 overflow-hidden bg-black flex items-center justify-center p-4" ref="contentContainer">
          <Transition name="fade" mode="out-in">
            <div v-if="isComponentReady" ref="componentContainer" class="relative" :style="containerStyle">
              <component
                :is="deviceComponent"
                :device="selectedDevice"
                :websocketUrl="getWebSocketUrl(selectedDevice)"
                v-bind="deviceSettings"
                class="w-full h-full"
              />
            </div>
            <div v-else class="flex items-center justify-center">
              <v-progress-circular indeterminate />
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, inject, nextTick, onMounted, onUnmounted, ref } from 'vue';
import DeviceSettings from '../utils/DeviceManager.vue';
import Ping1DLoader from '../widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from '../widgets/sonar360/Ping360Loader.vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
});

const { commonSettings, ping1DSettings, ping360Settings } = inject('deviceSettings');
const componentContainer = ref(null);
const contentContainer = ref(null);
const deviceView = ref(null);
const selectedDevice = ref(null);
const isComponentReady = ref(false);

const componentDimensions = ref({
  width: window.innerWidth * 0.8,
  height: window.innerHeight * 0.7,
});

const deviceComponent = computed(() => {
  if (!selectedDevice.value) return null;
  return selectedDevice.value.device_type === 'Ping360' ? Ping360Loader : Ping1DLoader;
});

const deviceSettings = computed(() => {
  if (!selectedDevice.value) return {};

  const settings =
    selectedDevice.value.device_type === 'Ping360' ? ping360Settings : ping1DSettings;

  return {
    ...commonSettings,
    ...settings,
    width: componentDimensions.value.width,
    height: componentDimensions.value.height,
  };
});

const containerStyle = computed(() => ({
  width: `${componentDimensions.value.width}px`,
  height: `${componentDimensions.value.height}px`,
  maxWidth: '100%',
  maxHeight: '100%',
}));

const getStatusColor = (status) => {
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
};

const selectDevice = async (device) => {
  isComponentReady.value = false;
  selectedDevice.value = device;

  await nextTick();
  await new Promise((resolve) => setTimeout(resolve, 100));

  updateComponentDimensions();
  isComponentReady.value = true;
};

const backToDevices = () => {
  isComponentReady.value = false;
  selectedDevice.value = null;
};

const getWebSocketUrl = (device) => {
  if (!device) return '';
  const url = new URL(props.serverUrl);
  const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${url.host}/ws?device_number=${device.id}`;
};

const updateComponentDimensions = () => {
  if (!contentContainer.value) return;

  const container = contentContainer.value;
  const containerRect = container.getBoundingClientRect();
  const padding = 32;
  const availableWidth = containerRect.width - padding;
  const availableHeight = containerRect.height - padding;

  const targetAspectRatio = 4 / 3;

  let newWidth;
  let newHeight;

  if (availableWidth / availableHeight > targetAspectRatio) {
    newHeight = availableHeight;
    newWidth = availableHeight * targetAspectRatio;
  } else {
    newWidth = availableWidth;
    newHeight = availableWidth / targetAspectRatio;
  }

  componentDimensions.value = {
    width: Math.floor(newWidth),
    height: Math.floor(newHeight),
  };
};

const handleResize = () => {
  if (selectedDevice.value) {
    updateComponentDimensions();
  }
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.device-header {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.device-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

:deep(.v-chip.v-chip--size-small) {
  font-size: 0.75rem;
  height: 24px;
}
</style>