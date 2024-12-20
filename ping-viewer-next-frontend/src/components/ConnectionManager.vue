<template>
  <div class="connection-manager">
    <div class="config-menu" :class="{ 'glass disable-hover': glass }" v-show="isOpen">
      <div :class="['menu-content', { 'glass-inner disable-hover': glass }]">
        <!-- Header Section -->
        <div class="menu-header d-flex justify-space-between align-center mb-4">
          <div class="text-h6">Device Management</div>
          <div class="d-flex gap-2">
            <v-btn color="primary" size="small" @click="autoCreateDevices" :loading="isAutoCreating">
              <v-icon start>mdi-plus-network</v-icon>
              Auto Create
            </v-btn>
            <v-btn color="primary" size="small" @click="refreshDevices" :loading="isRefreshing">
              <v-icon>mdi-refresh</v-icon>
            </v-btn>
          </div>
        </div>

        <!-- Device List -->
        <div class="device-list mb-4">
          <div v-if="isLoading" class="d-flex justify-center my-4">
            <v-progress-circular indeterminate />
          </div>

          <div v-else-if="devices.length === 0" class="text-center pa-4 text-medium-emphasis">
            <v-icon size="48" class="mb-2">mdi-devices</v-icon>
            <div>No devices found.</div>
            <div class="text-caption">Try clicking 'Auto Create' to discover devices.</div>
          </div>

          <v-list v-else :class="{ 'glass-inner': glass }" density="compact">
            <v-list-item v-for="device in devices" :key="device.id" :value="device" class="mb-2"
              :class="{ 'glass-inner': glass }">
              <template v-slot:prepend>
                <v-icon :icon="device.device_type === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'" />
              </template>

              <v-list-item-title>{{ device.device_type }}</v-list-item-title>
              <v-list-item-subtitle class="text-truncate">{{ device.id }}</v-list-item-subtitle>

              <template v-slot:append>
                <div class="d-flex align-center">
                  <v-chip :color="getStatusColor(device.status)" size="small" class="mr-2">
                    {{ device.status }}
                  </v-chip>
                  <v-btn color="primary" size="small" @click="selectDevice(device)">
                    Open
                  </v-btn>
                </div>
              </template>
            </v-list-item>
          </v-list>
        </div>

        <!-- Manual Creation Section -->
        <v-expand-transition>
          <div v-if="showManualCreate" :class="{ 'glass-inner': glass }" class="pa-4 rounded">
            <v-form @submit.prevent="createDevice">
              <v-select v-model="newDevice.device_selection" :items="deviceTypes" label="Device Type" class="mb-4" />
              <v-select v-model="newDevice.connectionType" :items="connectionTypes" label="Connection Type"
                class="mb-4" />

              <template v-if="newDevice.connectionType === 'UdpStream'">
                <v-text-field v-model="newDevice.udp.ip" label="IP Address" class="mb-4"
                  :rules="[v => !!v || 'IP is required']" />
                <v-text-field v-model.number="newDevice.udp.port" type="number" label="Port" class="mb-4"
                  :rules="[v => !!v || 'Port is required']" />
              </template>

              <template v-else-if="newDevice.connectionType === 'SerialStream'">
                <v-text-field v-model="newDevice.serial.path" label="Serial Path" class="mb-4"
                  :rules="[v => !!v || 'Path is required']" />
                <v-text-field v-model.number="newDevice.serial.baudrate" type="number" label="Baudrate" class="mb-4"
                  :rules="[v => !!v || 'Baudrate is required']" />
              </template>

              <div class="d-flex justify-end gap-2">
                <v-btn color="error" variant="text" @click="showManualCreate = false">Cancel</v-btn>
                <v-btn color="primary" :loading="isCreating" type="submit">Create</v-btn>
              </div>
            </v-form>
          </div>
        </v-expand-transition>

        <!-- Footer Actions -->
        <div class="menu-actions mt-4">
          <v-btn block :class="{ 'glass-inner': glass }" @click="showManualCreate = !showManualCreate">
            <v-icon start>{{ showManualCreate ? 'mdi-minus' : 'mdi-plus' }}</v-icon>
            {{ showManualCreate ? 'Cancel Manual Creation' : 'Manual Create' }}
          </v-btn>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
  glass: {
    type: Boolean,
    default: false,
  },
  isOpen: {
    type: Boolean,
    required: true,
  },
});

const emit = defineEmits(['update:isOpen', 'select-device']);

// State
const devices = ref([]);
const isLoading = ref(false);
const isRefreshing = ref(false);
const isAutoCreating = ref(false);
const isCreating = ref(false);
const showManualCreate = ref(false);
const error = ref(null);

// Form state
const newDevice = ref({
  device_selection: 'Auto',
  connectionType: 'UdpStream',
  udp: {
    ip: 'blueos.local',
    port: 12345,
  },
  serial: {
    path: '/dev/ttyUSB0',
    baudrate: 2500000,
  },
});

// Constants
const deviceTypes = [
  { title: 'Auto Detect', value: 'Auto' },
  { title: 'Ping1D', value: 'Ping1D' },
  { title: 'Ping360', value: 'Ping360' },
];

const connectionTypes = [
  { title: 'UDP', value: 'UdpStream' },
  { title: 'Serial', value: 'SerialStream' },
];

// Utility functions
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

// API functions
const fetchDevices = async () => {
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'List',
        module: 'DeviceManager',
      }),
    });

    if (!response.ok) throw new Error('Failed to fetch devices');

    const data = await response.json();
    devices.value = data.DeviceInfo || [];
    error.value = null;
  } catch (err) {
    console.error('Error fetching devices:', err);
    error.value = `Failed to fetch devices: ${err.message}`;
  }
};

const refreshDevices = async () => {
  isRefreshing.value = true;
  await fetchDevices();
  isRefreshing.value = false;
};

const autoCreateDevices = async () => {
  isAutoCreating.value = true;
  error.value = null;

  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'AutoCreate',
        module: 'DeviceManager',
      }),
    });

    if (!response.ok) throw new Error('Failed to auto-create devices');
    await refreshDevices();
  } catch (err) {
    console.error('Error auto-creating devices:', err);
    error.value = `Failed to auto-create devices: ${err.message}`;
  } finally {
    isAutoCreating.value = false;
  }
};

const createDevice = async () => {
  isCreating.value = true;
  error.value = null;

  try {
    const source =
      newDevice.value.connectionType === 'UdpStream'
        ? {
            UdpStream: {
              ip: newDevice.value.udp.ip,
              port: newDevice.value.udp.port,
            },
          }
        : {
            SerialStream: {
              path: newDevice.value.serial.path,
              baudrate: newDevice.value.serial.baudrate,
            },
          };

    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'Create',
        module: 'DeviceManager',
        payload: {
          device_selection: newDevice.value.device_selection,
          source,
        },
      }),
    });

    if (!response.ok) throw new Error('Failed to create device');

    await refreshDevices();
    showManualCreate.value = false;
  } catch (err) {
    console.error('Error creating device:', err);
    error.value = `Failed to create device: ${err.message}`;
  } finally {
    isCreating.value = false;
  }
};

const selectDevice = (device) => {
  emit('select-device', device);
  emit('update:isOpen', false);
};

// Lifecycle hooks
let refreshInterval;

onMounted(() => {
  isLoading.value = true;
  fetchDevices().finally(() => {
    isLoading.value = false;
  });
  refreshInterval = setInterval(fetchDevices, 5000);
});

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval);
  }
});
</script>

<style scoped>
.connection-manager {
  transition: all 0.3s ease;
  transform-origin: top left;
  animation: slideIn 0.3s ease;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(-20px);
  }

  to {
    opacity: 1;
    transform: translateX(0);
  }
}

/* Responsive adjustments */
@media (max-width: 600px) {
  .connection-menu-wrapper .config-menu {
    width: calc(100vw - var(--button-size) - var(--button-gap) * 2);
    max-width: 400px;
  }
}

.config-menu {
  width: 400px;
  max-width: calc(100vw - var(--button-size));
  border-radius: var(--border-radius);
  overflow: hidden;
  background: rgb(var(--v-theme-background));
}

.menu-content {
  padding: 1rem;
}

.device-list {
  max-height: 50vh;
  overflow-y: auto;
}

/* :deep(.v-list) {
  background: transparent;
} */

.menu-actions {
  border-top: 1px solid rgba(var(--v-border-color), 0.12);
  padding-top: 1rem;
}
</style>