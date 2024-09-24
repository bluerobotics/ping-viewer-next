<template>
  <div class="flex flex-col h-full bg-gray-900 text-white overflow-hidden">
    <div class="flex-1 p-4 overflow-y-auto flex flex-col">
      <h2 class="text-2xl font-bold text-white mb-4">Ping1D Waterfall Display</h2>

      <div class="mb-4">
        <label for="websocketUrl" class="block text-sm font-medium text-gray-300">WebSocket URL:</label>
        <input
          id="websocketUrl"
          v-model="websocketUrl"
          type="text"
          class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md bg-gray-700 text-white"
          placeholder="Enter WebSocket URL"
        />
      </div>

      <div class="mb-4 flex items-center justify-between">
        <div class="w-1/2 mr-2">
          <label for="colorPalette" class="block text-sm font-medium text-gray-300">Color Palette:</label>
          <select
            id="colorPalette"
            v-model="selectedColorPalette"
            class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md bg-gray-700 text-white"
          >
            <option v-for="palette in colorPalettes" :key="palette.value" :value="palette.value">
              {{ palette.label }}
            </option>
          </select>
        </div>
        <div class="w-1/2 ml-2">
          <button @click="toggleWebSocket" class="w-full px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors">
            {{ isConnected ? 'Disconnect' : 'Connect' }} WebSocket
          </button>
        </div>
      </div>

      <div class="mb-4 text-white">
        Last update: {{ lastUpdateTime }}
      </div>

      <div class="flex-1 min-h-[100px] overflow-hidden">
        <Ping1D
          :websocket-url="websocketUrl"
          :color-palette="selectedColorPalette"
          :is-connected="isConnected"
          @update:lastUpdateTime="lastUpdateTime = $event"
          @update:isConnected="isConnected = $event"
        />
      </div>
    </div>
  </div>
</template>

<script>
import { onMounted, ref } from "vue";
import Ping1D from "./Ping1D.vue";

export default {
	components: {
		Ping1D,
	},
	setup() {
		const websocketUrl = ref("");

		onMounted(() => {
			const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
			const host = window.location.host;
			websocketUrl.value = `${protocol}//${host}/ws?device_number=00000000-0000-0000-1cc4-7b702224f52d`;
		});

		const colorPalettes = [
			{ value: "transparent", label: "Transparent" },
			{ value: "heatmap", label: "Heatmap" },
			{ value: "grayscale", label: "Grayscale" },
			{ value: "ocean", label: "Ocean" },
			{ value: "Thermal blue", label: "Thermal Blue" },
			{ value: "Thermal black", label: "Thermal Black" },
			{ value: "Thermal white", label: "Thermal White" },
			{ value: "Monochrome black", label: "Monochrome Black" },
			{ value: "Monochrome white", label: "Monochrome White" },
			{ value: "Monochrome sepia", label: "Monochrome Sepia" },
		];

		const selectedColorPalette = ref("ocean");
		const isConnected = ref(false);
		const lastUpdateTime = ref("Not updated yet");

		function toggleWebSocket() {
			if (isConnected.value) {
				isConnected.value = false;
			} else {
				if (websocketUrl.value.trim() !== "") {
					isConnected.value = true;
				} else {
					alert("Please enter a valid WebSocket URL");
				}
			}
		}

		return {
			websocketUrl,
			colorPalettes,
			selectedColorPalette,
			isConnected,
			lastUpdateTime,
			toggleWebSocket,
		};
	},
};
</script>
