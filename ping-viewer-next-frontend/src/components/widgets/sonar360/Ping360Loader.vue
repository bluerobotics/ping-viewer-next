<template>
  <div class="flex bg-gray-100 p-6">
    <div class="max-w-4xl mx-auto bg-white shadow-lg rounded-lg overflow-hidden">
      <div class="bg-gray-800 text-white py-4 px-6">
        <h1 class="text-2xl font-bold">Ping Viewer Next - Sonar Display</h1>
      </div>

      <div class="p-6">
        <h2 class="text-xl font-semibold mb-4">WebSocket Connection</h2>

        <div class="flex items-center mb-6">
          <input v-model="websocketUrl" placeholder="Enter WebSocket URL"
            class="text-black flex-grow mr-2 p-2 border border-gray-300 rounded" />
          <button @click="toggleConnection"
            class="bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded">
            {{ isConnected ? 'Disconnect' : 'Connect' }}
          </button>
        </div>

        <h2 class="text-xl font-semibold mb-4">Sonar Display</h2>

        <div class="p-4 rounded">
          <Ping360
            :measurement="currentMeasurement"
            :maxDistance="300"
            :showMarkers="true"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import Ping360 from "./Ping360.vue";

interface SonarMeasurement {
  angle: number;
  data: Uint8Array;
}


const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const host = window.location.host;
const websocketUrl = ref(`${protocol}//${host}/ws?device_number=00000000-0000-0000-0000-000000000000`);
const socket = ref<WebSocket | null>(null);
const isConnected = ref(false);
const currentMeasurement = ref<SonarMeasurement | null>(null);

const toggleConnection = () => {
  if (isConnected.value) {
    disconnectWebSocket();
  } else {
    connectWebSocket();
  }
};

const connectWebSocket = () => {
  if (socket.value) {
    socket.value.close();
  }

  socket.value = new WebSocket(websocketUrl.value);

  socket.value.addEventListener('open', () => {
    console.log('WebSocket connected.');
    isConnected.value = true;
  });

  socket.value.addEventListener('close', () => {
    console.log('WebSocket closed.');
    isConnected.value = false;
  });

  socket.value.addEventListener('error', (error) => {
    console.error('WebSocket error:', error);
    isConnected.value = false;
  });

  socket.value.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    const deviceData = message.DeviceMessage.PingMessage.Ping360.DeviceData;
    currentMeasurement.value = {
      angle: deviceData.angle,
      data: new Uint8Array(deviceData.data)
    };
  });
};

const disconnectWebSocket = () => {
  if (socket.value) {
    socket.value.close();
  }
};
</script>