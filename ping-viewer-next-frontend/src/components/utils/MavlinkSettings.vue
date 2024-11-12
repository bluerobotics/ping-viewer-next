<template>
  <v-card class="bg-gray-800">
    <v-card-text>
      <div class="mb-6">
        <h2 class="text-xl mb-4">Connection Status</h2>
        <div class="flex items-center gap-2 mb-4">
          <div class="w-3 h-3 rounded-full" :class="statusIndicatorClass"></div>
          <span>{{ yawConnectionStatus }}</span>
        </div>
      </div>

      <div class="mb-6">
        <h2 class="text-xl mb-4">WebSocket Configuration</h2>
        <v-text-field v-model="localWebsocketUrl" label="WebSocket URL"
          :hint="'Example: ws://192.168.1.241:6040/ws/mavlink?filter=ATTITUDE'" persistent-hint
          :error-messages="connectionError" :disabled="isConnected" class="mb-4" />

        <div class="flex gap-4">
          <v-btn :color="isConnected ? 'error' : 'success'" @click="toggleConnection"
            :loading="yawConnectionStatus === 'Connecting'" size="large">
            <v-icon start>
              {{ isConnected ? 'mdi-lan-disconnect' : 'mdi-lan-connect' }}
            </v-icon>
            {{ isConnected ? 'Disconnect' : 'Connect' }}
          </v-btn>

          <v-btn color="secondary" @click="resetYaw" :disabled="!isConnected" size="large">
            <v-icon start>mdi-refresh</v-icon>
            Reset Yaw
          </v-btn>
        </div>
      </div>

      <div class="flex gap-6">
        <div class="flex-1">
          <h2 class="text-xl mb-4">Current Yaw</h2>
          <div class="bg-gray-700 p-4 rounded-lg">
            <div class="text-2xl font-mono text-center">{{ yawAngle.toFixed(1) }}°</div>
          </div>
        </div>

        <div class="flex-1">
          <h2 class="text-xl mb-4">Update Rate</h2>
          <div class="bg-gray-700 p-4 rounded-lg">
            <div class="text-2xl font-mono text-center">{{ updateRate.toFixed(1) }} Hz</div>
          </div>
        </div>
      </div>
    </v-card-text>
  </v-card>
</template>

<script setup>
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue';

const yawAngle = inject('yawAngle');
const yawConnectionStatus = inject('yawConnectionStatus');
const connectYawWebSocket = inject('connectYawWebSocket');
const cleanupYawConnection = inject('cleanupYawConnection');

const localWebsocketUrl = ref(
  localStorage.getItem('yawWebsocketUrl') ||
    `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.hostname}:6040/ws/mavlink?filter=ATTITUDE`
);
const connectionError = ref('');
const updateCount = ref(0);
const updateRate = ref(0);
const lastMessageTimestamp = ref(Date.now());
let updateRateInterval = null;

const isConnected = computed(() => yawConnectionStatus.value === 'Connected');

const statusIndicatorClass = computed(() => {
  return {
    'bg-green-500': yawConnectionStatus.value === 'Connected',
    'bg-red-500': yawConnectionStatus.value === 'Disconnected',
  };
});

const lastUpdateTime = computed(() => {
  const timeDiff = Date.now() - lastMessageTimestamp.value;
  if (timeDiff < 1000) return 'Just now';
  if (timeDiff < 60000) return `${Math.floor(timeDiff / 1000)}s ago`;
  return `${Math.floor(timeDiff / 60000)}m ago`;
});

const toggleConnection = () => {
  if (isConnected.value) {
    cleanupYawConnection();
    connectionError.value = '';
  } else {
    localStorage.setItem('yawWebsocketUrl', localWebsocketUrl.value);
    connectYawWebSocket(localWebsocketUrl.value);
  }
};

const resetYaw = () => {
  yawAngle.value = 0;
};

const startUpdateRateCalculator = () => {
  updateRateInterval = setInterval(() => {
    updateRate.value = updateCount.value;
    updateCount.value = 0;
  }, 1000);
};

watch(yawAngle, () => {
  updateCount.value++;
  lastMessageTimestamp.value = Date.now();
});

watch(localWebsocketUrl, () => {
  connectionError.value = '';
});

onMounted(() => {
  startUpdateRateCalculator();
});

onUnmounted(() => {
  if (updateRateInterval) {
    clearInterval(updateRateInterval);
  }
});
</script>