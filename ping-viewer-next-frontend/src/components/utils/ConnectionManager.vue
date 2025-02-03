<template>
  <div class="connection-manager">
    <div class="config-menu" :class="{ 'glass-inner disable-hover': glass }" v-show="isOpen">
      <div :class="['menu-content', { 'glass-inner disable-hover': glass }]">


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
              @click="selectDevice(device)">
              <template v-slot:prepend>
                <v-icon :icon="device.device_type === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'" />
              </template>

              <v-list-item-title>{{ device.device_type }}</v-list-item-title>
              <v-list-item-subtitle class="text-truncate">
                <v-tooltip v-if="device.source.SerialStream">
                  <template v-slot:activator="{ props }">
                    <div v-bind="props" class="d-flex align-center">
                      <v-icon size="small">mdi-usb-port</v-icon>
                      <span class="ml-1">{{ device.source.SerialStream.path }}</span>
                    </div>
                  </template>
                  Port: {{ device.source.SerialStream.path }}
                  Baudrate: {{ device.source.SerialStream.baudrate }}
                </v-tooltip>
                <v-tooltip v-else-if="device.source.UdpStream">
                  <template v-slot:activator="{ props }">
                    <div v-bind="props" class="d-flex align-center">
                      <v-icon size="small">mdi-ip-network</v-icon>
                      <span class="ml-1">{{ device.source.UdpStream.ip }}</span>
                    </div>
                  </template>
                  Ip: {{ device.source.UdpStream.ip }}
                  Port: {{ device.source.UdpStream.port }}
                </v-tooltip>
              </v-list-item-subtitle>

              <template v-slot:append>
                <div class="d-flex align-center gap-2">
                  <v-chip variant="elevated" :color="getStatusColor(device.status)" size="small">
                    {{ device.status }}
                  </v-chip>

                  <v-menu location="start" offset="5">
                    <template v-slot:activator="{ props }">
                      <v-btn v-bind="props" variant="tonal" icon size="small" density="compact">
                        <v-icon>mdi-dots-vertical</v-icon>
                      </v-btn>
                    </template>

                    <v-card :class="{ 'glass': glass }" min-width="200">
                      <v-list :class="{ 'glass-inner': glass }" density="compact">
                        <v-list-subheader>Device Actions</v-list-subheader>

                        <v-list-item @click="selectDevice(device)">
                          <template v-slot:prepend>
                            <v-icon variant="tonal">mdi-open-in-new</v-icon>
                          </template>
                          <v-list-item-title>Open Device</v-list-item-title>
                          <v-list-item-subtitle>View device data</v-list-item-subtitle>
                        </v-list-item>

                        <v-list-item v-if="device.status === 'ContinuousMode'" @click="disableContinuousMode(device.id)"
                          :disabled="loadingStates[device.id]">
                          <template v-slot:prepend>
                            <v-icon variant="tonal">mdi-pause</v-icon>
                          </template>
                          <v-list-item-title>Disable Continuous Mode</v-list-item-title>
                          <v-list-item-subtitle>Pause device data stream</v-list-item-subtitle>
                        </v-list-item>

                        <v-list-item v-else @click="enableContinuousMode(device.id)"
                          :disabled="loadingStates[device.id]">
                          <template v-slot:prepend>
                            <v-icon variant="tonal">mdi-play</v-icon>
                          </template>
                          <v-list-item-title>Enable Continuous Mode</v-list-item-title>
                          <v-list-item-subtitle>Start device data stream</v-list-item-subtitle>
                        </v-list-item>

                        <v-divider class="my-2"></v-divider>

                        <v-list-item @click="confirmDelete(device)" :disabled="loadingStates[device.id]">
                          <template v-slot:prepend>
                            <v-icon color="error">mdi-delete</v-icon>
                          </template>
                          <v-list-item-title>Delete Device</v-list-item-title>
                          <v-list-item-subtitle>Remove from manager's list</v-list-item-subtitle>
                        </v-list-item>
                      </v-list>
                    </v-card>
                  </v-menu>
                </div>
              </template>
            </v-list-item>
          </v-list>
        </div>

        <v-menu location="start" offset="10">
          <template v-slot:activator="{ props }">
            <v-btn v-bind="props" variant="tonal">
              <v-icon start>mdi-cog</v-icon>
              Advanced
            </v-btn>
          </template>

          <v-card :class="{ 'glass': glass }" min-width="200">
            <v-list :class="{ 'glass-inner': glass }">
              <v-list-subheader>Device Manager Actions</v-list-subheader>

              <v-list-item @click="autoCreateDevices" :disabled="isAutoCreating">
                <template v-slot:prepend>
                  <v-icon variant="tonal">mdi-plus-network</v-icon>
                </template>
                <v-list-item-title>Auto Create</v-list-item-title>
                <v-list-item-subtitle>Automatically run devices</v-list-item-subtitle>
              </v-list-item>

              <v-list-item @click="toggleManualCreate">
                <template v-slot:prepend>
                  <v-icon variant="tonal">mdi-plus</v-icon>
                </template>
                <v-list-item-title>Manual Create</v-list-item-title>
                <v-list-item-subtitle>Configure device manually</v-list-item-subtitle>
              </v-list-item>

              <v-list-item @click="refreshDevices" :disabled="isRefreshing">
                <template v-slot:prepend>
                  <v-icon variant="tonal">mdi-refresh</v-icon>
                </template>
                <v-list-item-title>Refresh List</v-list-item-title>
                <v-list-item-subtitle>Update device status</v-list-item-subtitle>
              </v-list-item>
            </v-list>
          </v-card>
        </v-menu>

        <!-- Manual Creation Dialog -->
        <v-dialog v-model="showManualCreate" max-width="500">
          <v-card :class="{ 'glass': glass }">
            <v-card-title>Create New Device</v-card-title>
            <v-card-text>
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
              </v-form>
            </v-card-text>
            <v-card-actions>
              <v-spacer />
              <v-btn color="error" variant="text" @click="showManualCreate = false">Cancel</v-btn>
              <v-btn color="success" :loading="isCreating" @click="createDevice">Create</v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>

        <!-- Delete Confirmation Dialog -->
        <v-dialog v-model="showDeleteDialog" max-width="400">
          <v-card :class="{ 'glass': glass }">
            <v-card-title>Confirm Delete</v-card-title>
            <v-card-text>
              Are you sure you want to delete this device?
              <div class="mt-2">
                <strong>Type:</strong> {{ deviceToDelete?.device_type }}<br>
                <strong>ID:</strong> {{ deviceToDelete?.id }}
              </div>
            </v-card-text>
            <v-card-actions>
              <v-spacer />
              <v-btn color="primary" variant="text" @click="showDeleteDialog = false">Cancel</v-btn>
              <v-btn color="error" :loading="isDeleting" @click="deleteDevice">Delete</v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>
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

const devices = ref([]);
const isLoading = ref(false);
const isRefreshing = ref(false);
const isAutoCreating = ref(false);
const isCreating = ref(false);
const isDeleting = ref(false);
const showManualCreate = ref(false);
const showDeleteDialog = ref(false);
const deviceToDelete = ref(null);
const error = ref(null);
const loadingStates = ref({});

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

const deviceTypes = [
  { title: 'Auto Detect', value: 'Auto' },
  { title: 'Ping1D', value: 'Ping1D' },
  { title: 'Ping360', value: 'Ping360' },
];

const connectionTypes = [
  { title: 'UDP', value: 'UdpStream' },
  { title: 'Serial', value: 'SerialStream' },
];

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

const toggleManualCreate = () => {
  showManualCreate.value = !showManualCreate.value;
};

const fetchDevices = async () => {
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
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
      headers: { 'Content-Type': 'application/json' },
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
      headers: { 'Content-Type': 'application/json' },
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

const enableContinuousMode = async (deviceId) => {
  loadingStates.value[deviceId] = true;
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        command: 'EnableContinuousMode',
        module: 'DeviceManager',
        payload: { uuid: deviceId },
      }),
    });

    if (!response.ok) throw new Error('Failed to enable continuous mode');
    await refreshDevices();
  } catch (err) {
    console.error('Error enabling continuous mode:', err);
    error.value = `Failed to enable continuous mode: ${err.message}`;
  } finally {
    loadingStates.value[deviceId] = false;
  }
};

const disableContinuousMode = async (deviceId) => {
  loadingStates.value[deviceId] = true;
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        command: 'DisableContinuousMode',
        module: 'DeviceManager',
        payload: { uuid: deviceId },
      }),
    });

    if (!response.ok) throw new Error('Failed to disable continuous mode');
    await refreshDevices();
  } catch (err) {
    console.error('Error disabling continuous mode:', err);
    error.value = `Failed to disable continuous mode: ${err.message}`;
  } finally {
    loadingStates.value[deviceId] = false;
  }
};

const confirmDelete = (device) => {
  deviceToDelete.value = device;
  showDeleteDialog.value = true;
};

const deleteDevice = async () => {
  if (!deviceToDelete.value) return;
  isDeleting.value = true;

  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        command: 'Delete',
        module: 'DeviceManager',
        payload: { uuid: deviceToDelete.value.id },
      }),
    });

    if (!response.ok) throw new Error('Failed to delete device');
    await refreshDevices();
    showDeleteDialog.value = false;
  } catch (err) {
    console.error('Error deleting device:', err);
    error.value = `Failed to delete device: ${err.message}`;
  } finally {
    isDeleting.value = false;
    deviceToDelete.value = null;
  }
};

const selectDevice = async (device) => {
  try {
    if (device.status !== 'ContinuousMode') {
      loadingStates.value[device.id] = true;
      await enableContinuousMode(device.id);
      await fetchDevices();
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
    emit('select-device', device);
    emit('update:isOpen', false);
  } catch (error) {
    console.error('Error selecting device:', error);
  } finally {
    loadingStates.value[device.id] = false;
  }
};

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

@media (max-width: 600px) {
  .config-menu {
    width: calc(100vw - var(--button-size) - var(--button-gap) * 2);
    max-width: 400px;
  }
}
</style>