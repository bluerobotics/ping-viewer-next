<template>
	<div class="h-full flex flex-col bg-gray-100">
		<div class="bg-gray-800 text-white py-4 px-6 flex justify-between items-center">
			<h1 class="text-2xl font-bold">WebSocket Analysis</h1>
			<div class="flex items-center space-x-4">
				<button @click="view = 'console'" :class="{
					'bg-blue-500': view === 'console',
					'bg-gray-600': view !== 'console',
				}" class="px-4 py-2 rounded text-white">
					Console
				</button>
				<button @click="view = 'tree'" :class="{
					'bg-blue-500': view === 'tree',
					'bg-gray-600': view !== 'tree',
				}" class="px-4 py-2 rounded text-white">
					JSON Tree
				</button>
			</div>
		</div>

		<div class="flex-grow overflow-hidden">
			<div v-if="view === 'console'" class="h-full flex flex-col p-4">
				<div class="mb-4 p-2 bg-white rounded shadow">
					<strong>Status:</strong> {{ status }}
					<span class="ml-4 text-sm text-gray-600">
						Messages: {{ messages.length }}/{{ maxMessages }}
					</span>
				</div>
				<div ref="messagesContainer" class="flex-grow bg-white p-4 rounded shadow overflow-y-auto mb-4"
					style="max-height: calc(60vh);">
					<div v-for="(message, index) in messages" :key="index" class="mb-2 text-sm font-mono">
						{{ message }}
					</div>
				</div>
				<div class="flex items-center mb-2">
					<input v-model="messageInput" @keyup.enter="sendMessage" placeholder="Type your message here"
						class="flex-grow p-2 border border-gray-300 rounded-l" />
					<button @click="sendMessage" class="bg-blue-500 text-white px-4 py-2 rounded-r">
						Send
					</button>
				</div>
				<label class="flex items-center">
					<input type="checkbox" v-model="autoScroll" class="mr-2" />
					<span>Enable Autoscroll</span>
				</label>
			</div>

			<div v-else-if="view === 'tree'" class="h-full p-4 overflow-y-auto">
				<div v-for="(device, deviceId) in organizedData" :key="deviceId"
					class="mb-4 bg-white rounded shadow p-4">
					<div @click="toggleDevice(deviceId)" class="cursor-pointer flex items-center justify-between">
						<div class="flex items-center">
							<span class="mr-2 text-xl">{{
								expandedDevices[deviceId] ? "−" : "+"
							}}</span>
							<h4 class="font-bold text-lg">
								Device ID: {{ deviceId }}
								<span class="text-gray-600 text-base ml-2">({{ Object.keys(device)[0] }})</span>
							</h4>
						</div>
						<div class="text-sm text-gray-500">
							{{ getTotalMessageTypes(device) }} message types
						</div>
					</div>
					<div v-if="expandedDevices[deviceId]" class="ml-6 mt-2">
						<div v-for="(deviceType, type) in device" :key="type" class="mb-2 border-t pt-2">
							<div v-for="(messageData, messageType) in deviceType" :key="messageType" class="mb-2">
								<div @click="toggleMessageContent(deviceId, type, messageType)"
									class="cursor-pointer flex items-center justify-between">
									<h6 class="font-medium text-sm">{{ messageType }}</h6>
									<div class="text-xs text-gray-500">
										{{ formatFrequency(messageData.frequency) }} |
										{{ messageData.messageCount }} messages
									</div>
								</div>
								<div v-if="isMessageContentExpanded(deviceId, type, messageType)" class="ml-4 mt-1">
									<pre v-if="messageData.latestMessage"
										class="text-xs bg-gray-100 p-2 rounded">{{ formatMessage(messageData.latestMessage) }}</pre>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</template>

<script>
import { nextTick, onMounted, onUnmounted, reactive, ref } from 'vue';

export default {
  props: {
    serverUrl: {
      type: String,
      required: true,
    },
  },
  setup(props) {
    const view = ref('tree');
    const status = ref('Connecting...');
    const socket = ref(null);
    const messages = ref([]);
    const messageInput = ref('');
    const messagesContainer = ref(null);
    const organizedData = reactive({});
    const expandedDevices = reactive({});
    const expandedMessageContents = reactive({});
    const autoScroll = ref(true);
    const maxMessages = 1000;

    const connectWebSocket = () => {
      socket.value = new WebSocket(`ws://${new URL(props.serverUrl).host}/ws`);

      socket.value.onopen = () => {
        status.value = 'Connected';
      };

      socket.value.onmessage = (event) => {
        messages.value.push(event.data);
        if (messages.value.length > maxMessages) {
          messages.value = messages.value.slice(-maxMessages);
        }

        parseAndOrganizeMessage(event.data);
        nextTick(() => {
          if (autoScroll.value && messagesContainer.value) {
            messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
          }
        });
      };

      socket.value.onclose = () => {
        status.value = 'Disconnected';
      };

      socket.value.onerror = () => {
        status.value = 'Error';
      };
    };

    const parseAndOrganizeMessage = (message) => {
      try {
        const data = JSON.parse(message);
        const deviceId = data.DeviceMessage.device_id;
        const pingMessage = data.DeviceMessage.PingMessage;

        if (!organizedData[deviceId]) {
          organizedData[deviceId] = {};
        }

        for (const [deviceType, messages] of Object.entries(pingMessage)) {
          if (!organizedData[deviceId][deviceType]) {
            organizedData[deviceId][deviceType] = {};
          }

          for (const [messageType, messageData] of Object.entries(messages)) {
            if (!organizedData[deviceId][deviceType][messageType]) {
              organizedData[deviceId][deviceType][messageType] = {
                latestMessage: null,
                messageHistory: [],
                messageCount: 0,
                frequency: 0,
              };
            }

            const typeData = organizedData[deviceId][deviceType][messageType];
            typeData.latestMessage = messageData;
            typeData.messageCount++;

            typeData.messageHistory.push({
              timestamp: Date.now(),
              message: messageData,
            });

            if (typeData.messageHistory.length > 10) {
              typeData.messageHistory.shift();
            }

            if (typeData.messageHistory.length > 1) {
              const oldestTimestamp = typeData.messageHistory[0].timestamp;
              const newestTimestamp =
                typeData.messageHistory[typeData.messageHistory.length - 1].timestamp;
              const timeDiff = (newestTimestamp - oldestTimestamp) / 1000;
              typeData.frequency = (typeData.messageHistory.length - 1) / timeDiff;
            } else {
              typeData.frequency = 0;
            }
          }
        }
      } catch (error) {
        console.error('Error parsing message:', error);
      }
    };

    const formatMessage = (message, expandArrays = false) => {
      const formatted = {};
      for (const [key, value] of Object.entries(message)) {
        if (Array.isArray(value)) {
          if (expandArrays) {
            formatted[key] = value;
          } else {
            formatted[key] = `[Array(${value.length})]`;
          }
        } else {
          formatted[key] = value;
        }
      }
      return JSON.stringify(formatted, null, 2);
    };

    const toggleDevice = (deviceId) => {
      expandedDevices[deviceId] = !expandedDevices[deviceId];
    };

    const toggleMessageContent = (deviceId, deviceType, messageType) => {
      const key = `${deviceId}-${deviceType}-${messageType}`;
      expandedMessageContents[key] = !expandedMessageContents[key];
    };

    const isMessageContentExpanded = (deviceId, deviceType, messageType) => {
      const key = `${deviceId}-${deviceType}-${messageType}`;
      return expandedMessageContents[key];
    };

    const getTotalMessageTypes = (device) => {
      return Object.values(device).reduce(
        (total, deviceType) => total + Object.keys(deviceType).length,
        0
      );
    };

    const formatFrequency = (frequency) => {
      if (frequency >= 1000000) {
        return `${(frequency / 1000000).toFixed(2)} MHz`;
      }
      if (frequency >= 1000) {
        return `${(frequency / 1000).toFixed(2)} kHz`;
      }
      return `${frequency.toFixed(2)} Hz`;
    };

    const toggleMessageType = (deviceId, type) => {
      const key = `${deviceId}-${type}`;
      expandedMessageTypes[key] = !expandedMessageTypes[key];
    };

    const sendMessage = () => {
      if (messageInput.value && socket.value && socket.value.readyState === WebSocket.OPEN) {
        socket.value.send(messageInput.value);
        messageInput.value = '';
      }
    };

    onMounted(() => {
      connectWebSocket();
    });

    onUnmounted(() => {
      if (socket.value) {
        socket.value.close();
      }
    });
    return {
      view,
      status,
      messages,
      messageInput,
      messagesContainer,
      organizedData,
      expandedDevices,
      autoScroll,
      maxMessages,
      sendMessage,
      formatMessage,
      toggleDevice,
      toggleMessageContent,
      isMessageContentExpanded,
      getTotalMessageTypes,
      formatFrequency,
    };
  },
};
</script>