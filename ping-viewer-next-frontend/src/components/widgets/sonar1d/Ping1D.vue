<template>
	<div class="flex flex-col h-full bg-gray-900 p-4">
		<WaterfallDisplay
		  :width="width"
		  :height="height"
		  :max-depth="maxDepth"
		  :min-depth="minDepth"
		  :column-count="columnCount"
		  :sensor-data="sensorData"
		  :color-palette="colorPalette"
		  :current-depth="currentDepth"
		  :accuracy="accuracy"
		  :confidence="confidence"
		  :tickCount="tickCount"
		  depth-line-color="yellow"
		  depth-text-color="yellow"
		  current-depth-color="yellow"
		  confidence-color="#00FF00"
		  text-background="rgba(0, 0, 0, 0.5)"
		  @update:columnCount="columnCount = $event"
		/>
	</div>
  </template>

  <script>
import { onUnmounted, reactive, ref, watch } from "vue";
import WaterfallDisplay from "./WaterfallDisplay.vue";

export default {
	components: {
		WaterfallDisplay,
	},
	props: {
		websocketUrl: {
			type: String,
			required: true,
		},
		colorPalette: {
			type: String,
			required: true,
		},
		isConnected: {
			type: Boolean,
			required: true,
		},
	},
	emits: ["update:lastUpdateTime", "update:isConnected"],
	setup(props, { emit }) {
		const columnCount = ref(226);
		const sensorData = reactive([]);
		const currentDepth = ref(0);
		const minDepth = ref(0);
		const maxDepth = ref(0);
		const confidence = ref(0);
		const accuracy = ref(0);
		let socket = null;

		watch(
			() => props.isConnected,
			(newIsConnected) => {
				if (newIsConnected) {
					connectWebSocket();
				} else {
					disconnectWebSocket();
				}
			},
		);

		function connectWebSocket() {
			if (socket) return;

			socket = new WebSocket(props.websocketUrl);

			socket.onopen = () => {
				console.log("WebSocket connected");
				emit("update:isConnected", true);
			};

			socket.onmessage = (event) => {
				try {
					const parsedData = JSON.parse(event.data);
					const profile = parsedData.DeviceMessage.PingMessage.Ping1D.Profile;

					sensorData.splice(0, sensorData.length, ...profile.profile_data);
					currentDepth.value = profile.distance / 1000;
					minDepth.value = profile.scan_start / 1000;
					maxDepth.value = profile.scan_length / 1000;
					confidence.value = profile.confidence;

					accuracy.value =
						((100 - confidence.value) / 100) *
						(maxDepth.value - minDepth.value) *
						0.1;

					emit("update:lastUpdateTime", new Date().toLocaleTimeString());

					console.log(
						"Received new sensor data:",
						profile.profile_data.length,
						"values, current depth:",
						currentDepth.value.toFixed(2),
						"m, min depth:",
						minDepth.value.toFixed(2),
						"m, max depth:",
						maxDepth.value.toFixed(2),
						"m, confidence:",
						confidence.value,
						"%, accuracy:",
						accuracy.value.toFixed(2),
						"m",
					);
				} catch (error) {
					console.error("Error parsing WebSocket data:", error);
				}
			};

			socket.onerror = (error) => {
				console.error("WebSocket error:", error);
			};

			socket.onclose = () => {
				console.log("WebSocket disconnected");
				emit("update:isConnected", false);
				socket = null;
			};
		}

		function disconnectWebSocket() {
			if (socket) {
				socket.close();
				socket = null;
			}
		}

		onUnmounted(() => {
			disconnectWebSocket();
		});

		return {
			columnCount,
			sensorData,
			currentDepth,
			minDepth,
			maxDepth,
			confidence,
			accuracy,
		};
	},
};
</script>