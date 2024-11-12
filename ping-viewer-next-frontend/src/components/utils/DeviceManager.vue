<template>
	<v-card flat class="device-management">
		<v-card-text class="pa-6">
			<!-- Desktop view -->
			<div class="d-none d-md-flex justify-space-between align-center mb-6">
				<h2 class="text-xl">Device Management</h2>
				<div class="d-flex gap-4">
					<v-btn color="primary" @click="showCreateDialog = true">
						<v-icon start>mdi-plus</v-icon>
						Create Device
					</v-btn>
					<v-btn color="primary" :loading="isAutoCreating" @click="autoCreateDevices">
						<v-icon start>mdi-plus-network</v-icon>
						Auto Create
					</v-btn>
					<v-btn color="primary" :loading="isRefreshing" @click="refreshDevices">
						<v-icon start>mdi-refresh</v-icon>
						Refresh
					</v-btn>
				</div>
			</div>

			<!-- Mobile view -->
			<div class="d-md-none mb-6">
				<div class="d-flex justify-space-between align-center">
					<h2 class="text-xl">Device Management</h2>
					<v-menu>
						<template v-slot:activator="{ props }">
							<v-btn v-bind="props" icon>
								<v-icon>mdi-menu</v-icon>
							</v-btn>
						</template>
						<v-list>
							<v-list-item @click="showCreateDialog = true">
								<template v-slot:prepend>
									<v-icon>mdi-plus</v-icon>
								</template>
								Create Device
							</v-list-item>
							<v-list-item @click="autoCreateDevices" :disabled="isAutoCreating">
								<template v-slot:prepend>
									<v-icon>mdi-plus-network</v-icon>
								</template>
								{{ isAutoCreating ? 'Creating...' : 'Auto Create' }}
							</v-list-item>
							<v-list-item @click="refreshDevices" :disabled="isRefreshing">
								<template v-slot:prepend>
									<v-icon>mdi-refresh</v-icon>
								</template>
								{{ isRefreshing ? 'Refreshing...' : 'Refresh' }}
							</v-list-item>
						</v-list>
					</v-menu>
				</div>
			</div>

			<div v-if="isLoading" class="d-flex justify-center my-4">
				<v-progress-circular indeterminate />
			</div>

			<div v-else class="table-container">
				<v-data-table :headers="headers" :items="devices" :items-per-page="-1"
					no-data-text="No devices found. Try clicking 'Auto Create' to discover devices." hover
					@click:row="openDevice" hide-default-footer class="devices-table" fixed-header>
					<template v-slot:header="{ props }">
						<tr>
							<th v-for="header in props.headers" :key="header.key" :style="{
								width: header.width,
								textAlign: header.align || 'left',
								padding: '12px 16px',
								whiteSpace: 'nowrap'
							}">
								{{ header.title }}
							</th>
						</tr>
					</template>

					<template v-slot:item="{ item: device }">
						<tr class="device-row" :class="{ 'cursor-pointer': true }" @click="openDevice(device)">
							<td>
								<div class="d-flex align-center">
									<v-icon :icon="device.device_type === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'"
										class="mr-2" />
									{{ device.device_type }}
								</div>
							</td>
							<td class="text-truncate" :title="device.id">
								{{ device.id }}
							</td>
							<td>
								<v-chip :color="getStatusColor(device.status)" size="small">
									{{ device.status }}
								</v-chip>
							</td>
							<td>
								<div v-if="device.source.SerialStream">
									<v-tooltip location="bottom">
										<template v-slot:activator="{ props }">
											<div v-bind="props" class="d-flex align-center">
												<v-icon start size="small">mdi-usb-port</v-icon>
												<span class="ml-1 text-truncate">{{ device.source.SerialStream.path
													}}</span>
											</div>
										</template>
										Baudrate: {{ device.source.SerialStream.baudrate }}
									</v-tooltip>
								</div>
								<div v-else-if="device.source.UdpStream">
									<v-tooltip location="bottom">
										<template v-slot:activator="{ props }">
											<div v-bind="props" class="d-flex align-center">
												<v-icon start size="small">mdi-ip-network</v-icon>
												<span class="ml-1">{{ device.source.UdpStream.ip }}</span>
											</div>
										</template>
										Port: {{ device.source.UdpStream.port }}
									</v-tooltip>
								</div>
							</td>
							<td class="text-center">
								<div class="d-flex justify-center gap-2" @click.stop>
									<v-btn color="primary" size="small" @click="openDevice(device)">
										<v-icon start>mdi-open-in-new</v-icon>
										Open
									</v-btn>
									<v-btn v-if="device.status === 'ContinuousMode'" color="warning" size="small"
										@click="disableContinuousMode(device.id)" :loading="loadingStates[device.id]">
										<v-icon start>mdi-pause</v-icon>
										Disable
									</v-btn>
									<v-btn v-else color="success" size="small" @click="enableContinuousMode(device.id)"
										:loading="loadingStates[device.id]">
										<v-icon start>mdi-play</v-icon>
										Enable
									</v-btn>
									<v-btn color="error" size="small" @click="confirmDelete(device)"
										:loading="loadingStates[device.id]">
										<v-icon>mdi-delete</v-icon>
									</v-btn>
								</div>
							</td>
						</tr>
					</template>
				</v-data-table>
			</div>

			<v-dialog v-model="showCreateDialog" max-width="500px">
				<v-card>
					<v-card-title>Create New Device</v-card-title>
					<v-card-text>
						<v-form @submit.prevent="createDevice">
							<v-select v-model="newDevice.device_selection" :items="deviceTypes" label="Device Type"
								class="mb-4" />
							<v-select v-model="newDevice.connectionType" :items="connectionTypes"
								label="Connection Type" class="mb-4" />

							<template v-if="newDevice.connectionType === 'UdpStream'">
								<v-text-field v-model="newDevice.udp.ip" label="IP Address" class="mb-4"
									:rules="[v => !!v || 'IP is required']" />
								<v-text-field v-model.number="newDevice.udp.port" type="number" label="Port"
									class="mb-4" :rules="[v => !!v || 'Port is required']" />
							</template>

							<template v-else-if="newDevice.connectionType === 'SerialStream'">
								<v-text-field v-model="newDevice.serial.path" label="Serial Path" class="mb-4"
									:rules="[v => !!v || 'Path is required']" />
								<v-text-field v-model.number="newDevice.serial.baudrate" type="number" label="Baudrate"
									class="mb-4" :rules="[v => !!v || 'Baudrate is required']" />
							</template>
						</v-form>
					</v-card-text>
					<v-card-actions>
						<v-spacer />
						<v-btn color="error" variant="text" @click="showCreateDialog = false">Cancel</v-btn>
						<v-btn color="success" :loading="isCreating" @click="createDevice">Create</v-btn>
					</v-card-actions>
				</v-card>
			</v-dialog>

			<v-dialog v-model="showDeleteDialog" max-width="400px">
				<v-card>
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

		</v-card-text>
	</v-card>
</template>

<script setup>
import { onMounted, onUnmounted, ref } from 'vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
});

const emit = defineEmits(['openDevice']);

const devices = ref([]);
const isLoading = ref(false);
const isRefreshing = ref(false);
const isAutoCreating = ref(false);
const isCreating = ref(false);
const isDeleting = ref(false);
const error = ref(null);
const loadingStates = ref({});
const showCreateDialog = ref(false);
const showDeleteDialog = ref(false);
const deviceToDelete = ref(null);
let refreshInterval = null;

const headers = [
  {
    title: 'Device Type',
    key: 'device_type',
    align: 'center',
    width: '15%',
  },
  {
    title: 'ID',
    key: 'id',
    align: 'center',
    width: '30%',
  },
  {
    title: 'Status',
    key: 'status',
    align: 'center',
    width: '15%',
  },
  {
    title: 'Connection',
    key: 'connection',
    align: 'center',
    width: '20%',
  },
  {
    title: 'Actions',
    key: 'actions',
    align: 'center',
    sortable: false,
    width: '20%',
  },
];

const deviceTypes = [
  { title: 'Auto Detect', value: 'Auto' },
  { title: 'Ping1D', value: 'Ping1D' },
  { title: 'Ping360', value: 'Ping360' },
];

const connectionTypes = [
  { title: 'UDP', value: 'UdpStream' },
  { title: 'Serial', value: 'SerialStream' },
];

const newDevice = ref({
  device_selection: 'Auto',
  connectionType: 'UdpStream',
  udp: {
    ip: '',
    port: 8080,
  },
  serial: {
    path: '/dev/ttyUSB0',
    baudrate: 2500000,
  },
});

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

    const data = await response.json();
    await refreshDevices();

    if (data.DeviceInfo?.length > 0) {
    }
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
    showCreateDialog.value = false;
  } catch (err) {
    console.error('Error creating device:', err);
    error.value = `Failed to create device: ${err.message}`;
  } finally {
    isCreating.value = false;
  }
};

const openDevice = (device) => {
  emit('openDevice', device);
};

const confirmDelete = (device) => {
  deviceToDelete.value = device;
  showDeleteDialog.value = true;
};

const deleteDevice = async () => {
  if (!deviceToDelete.value) return;

  isDeleting.value = true;
  error.value = null;

  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'Delete',
        module: 'DeviceManager',
        payload: {
          uuid: deviceToDelete.value.id,
        },
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

const enableContinuousMode = async (deviceId) => {
  loadingStates.value[deviceId] = true;
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'EnableContinuousMode',
        module: 'DeviceManager',
        payload: {
          uuid: deviceId,
        },
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
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        command: 'DisableContinuousMode',
        module: 'DeviceManager',
        payload: {
          uuid: deviceId,
        },
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
.device-management {
	height: fit-content;
	display: flex;
	flex-direction: column;
	max-width: 1200px;
	margin: 0 auto;
}
</style>