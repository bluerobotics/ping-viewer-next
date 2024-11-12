<template>
  <div class="h-screen w-screen flex flex-col bg-black">
    <div class="flex-none bg-gray-800 p-4 flex justify-between items-center">
      <div class="flex items-center">
        <h1 class="text-2xl font-bold text-white">Multi-Device Viewer</h1>
        <v-chip v-if="selectedDevices.length > 0" color="primary" class="ml-4">
          {{ selectedDevices.length }} Device{{ selectedDevices.length !== 1 ? 's' : '' }} Selected
        </v-chip>
      </div>

      <div class="flex items-center">
        <v-btn color="primary" @click="showDeviceDialog = true">
          <v-icon icon="mdi-plus" class="mr-2" />
          Add Device
        </v-btn>
      </div>
    </div>

    <div class="flex-1 bg-black overflow-hidden">
      <div v-if="selectedDevices.length === 0" class="h-full flex items-center justify-center">
        <v-btn color="primary" size="large" @click="showDeviceDialog = true">
          Add Devices to View
        </v-btn>
      </div>

      <div v-else ref="contentContainer" class="h-full p-4 grid gap-4" :style="gridStyle">
        <div v-for="device in selectedDevices" :key="device.id"
          class="relative bg-gray-900 rounded-lg overflow-hidden flex items-center justify-center">
          <div class="absolute top-0 right-0 z-10 p-2 flex gap-2">
            <v-btn icon="mdi-close" size="small" color="error" variant="text" @click="removeDevice(device)" />
          </div>

          <div ref="componentContainer" class="relative" :style="containerStyle">
            <component :is="getDeviceComponent(device)" :device="device" :websocketUrl="getWebSocketUrl(device)"
              v-bind="getDeviceProps(device)" class="w-full h-full" />
          </div>
        </div>
      </div>
    </div>

    <v-dialog v-model="showDeviceDialog" max-width="800px" @click:outside="closeDialog">
      <v-card>
        <v-card-title class="text-h5 bg-gray-800 text-white">
          Select Devices
        </v-card-title>

        <v-card-text class="pa-6">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <DeviceCard v-for="device in availableDevices" :key="device.id" :device="device"
              :selected="isDeviceSelected(device)" :showActions="true" @toggle="toggleDevice(device)" />
          </div>
        </v-card-text>

        <v-card-actions class="pa-4">
          <v-spacer />
          <v-btn color="primary" @click="closeDialog">
            Done
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<script setup>
import { useDeviceFetching } from '@/composables/useDeviceFetching';
import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import DeviceCard from '../utils/DeviceCard.vue';
import Ping1DLoader from '../widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from '../widgets/sonar360/Ping360Loader.vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
});

const { commonSettings, ping1DSettings, ping360Settings } = inject('deviceSettings');

const { deviceInfo } = useDeviceFetching(props.serverUrl);
const contentContainer = ref(null);
const componentContainer = ref(null);
let resizeObserver = null;

const componentDimensions = ref({
  width: window.innerWidth * 0.7,
  height: window.innerHeight * 0.7,
});

const availableDevices = computed(() => {
  return deviceInfo.value.DeviceInfo?.filter((device) => device.status === 'ContinuousMode') || [];
});

const selectedDevices = ref([]);
const showDeviceDialog = ref(false);

const gridStyle = computed(() => {
  const count = selectedDevices.value.length;
  if (count <= 1) {
    return {
      'grid-template-columns': '1fr',
      'grid-template-rows': '1fr',
    };
  }
  if (count <= 2) {
    return {
      'grid-template-columns': '1fr 1fr',
      'grid-template-rows': '1fr',
    };
  }
  if (count <= 4) {
    return {
      'grid-template-columns': '1fr 1fr',
      'grid-template-rows': '1fr 1fr',
    };
  }
  const cols = Math.ceil(Math.sqrt(count));
  return {
    'grid-template-columns': `repeat(${cols}, 1fr)`,
    'grid-template-rows': `repeat(${Math.ceil(count / cols)}, 1fr)`,
  };
});

const containerStyle = computed(() => ({
  width: `${componentDimensions.value.width}px`,
  height: `${componentDimensions.value.height}px`,
  maxWidth: '100%',
  maxHeight: '100%',
}));

const updateComponentDimensions = () => {
  if (!contentContainer.value) return;

  const container = contentContainer.value;
  const containerRect = container.getBoundingClientRect();

  const cols =
    selectedDevices.value.length <= 1
      ? 1
      : selectedDevices.value.length <= 4
        ? 2
        : Math.ceil(Math.sqrt(selectedDevices.value.length));

  const rows =
    selectedDevices.value.length <= 2
      ? 1
      : selectedDevices.value.length <= 4
        ? 2
        : Math.ceil(selectedDevices.value.length / cols);

  const cellWidth = containerRect.width / cols;
  const cellHeight = containerRect.height / rows;

  componentDimensions.value = {
    width: Math.floor(cellWidth),
    height: Math.floor(cellHeight),
  };
};

const closeDialog = () => {
  showDeviceDialog.value = false;
  nextTick(() => {
    updateComponentDimensions();
    triggerChildrenResize();
  });
};

const toggleDevice = (device) => {
  const index = selectedDevices.value.findIndex((d) => d.id === device.id);
  if (index === -1) {
    selectedDevices.value.push(device);
  } else {
    selectedDevices.value.splice(index, 1);
  }
  nextTick(() => {
    updateComponentDimensions();
    triggerChildrenResize();
  });
};

const removeDevice = (device) => {
  const index = selectedDevices.value.findIndex((d) => d.id === device.id);
  if (index !== -1) {
    selectedDevices.value.splice(index, 1);
  }
  nextTick(() => {
    updateComponentDimensions();
    triggerChildrenResize();
  });
};

const isDeviceSelected = (device) => {
  return selectedDevices.value.some((d) => d.id === device.id);
};

const getDeviceComponent = (device) => {
  return device.device_type === 'Ping360' ? Ping360Loader : Ping1DLoader;
};

const getDeviceProps = (device) => {
  const settings = device.device_type === 'Ping360' ? ping360Settings : ping1DSettings;

  return {
    ...commonSettings,
    ...settings,
    width: componentDimensions.value.width,
    height: componentDimensions.value.height,
  };
};

const triggerChildrenResize = () => {
  setTimeout(() => {
    window.dispatchEvent(new Event('resize'));
  }, 100);
};

const getWebSocketUrl = (device) => {
  if (!device) return '';
  const url = new URL(props.serverUrl);
  const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${url.host}/ws?device_number=${device.id}`;
};

const handleResize = () => {
  updateComponentDimensions();
};

watch(
  selectedDevices,
  () => {
    nextTick(() => {
      updateComponentDimensions();
    });
  },
  { deep: true }
);

onMounted(() => {
  window.addEventListener('resize', handleResize);

  resizeObserver = new ResizeObserver(() => {
    updateComponentDimensions();
  });

  if (contentContainer.value) {
    resizeObserver.observe(contentContainer.value);
  }

  nextTick(() => {
    updateComponentDimensions();
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);

  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});
</script>

<style scoped>
.device-card {
  transition: all 0.3s ease;
}

.device-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}
</style>