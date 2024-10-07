<template>
  <div class="bg-gray-100 min-h-screen p-6">
    <div
      class="max-w-4xl mx-auto bg-white shadow-lg rounded-lg overflow-hidden"
    >
      <div class="bg-gray-800 text-white py-4 px-6">
        <h1 class="text-2xl font-bold">Ping Viewer Next</h1>
      </div>

      <div class="p-6">
        <button
          @click="redirectToDocs"
          class="mb-6 bg-blue-500 hover:bg-blue-600 text-white font-bold py-2 px-4 rounded"
        >
          Check API specifications
        </button>

        <h2 class="text-xl font-semibold mb-4">WebSocket Client</h2>

        <div id="status" class="mb-4 p-2 bg-gray-200 rounded">{{ status }}</div>

        <div
          ref="messagesContainer"
          class="messages-container bg-gray-100 p-4 rounded h-64 overflow-y-auto mb-4"
        >
          <div v-for="(message, index) in messages" :key="index" class="mb-2">
            {{ message }}
          </div>
        </div>

        <div class="flex items-center mb-4">
          <input
            type="text"
            v-model="messageInput"
            placeholder="Type your message here"
            @keyup.enter="sendMessage"
            class="flex-grow mr-2 p-2 border border-gray-300 rounded"
          />
          <button
            @click="sendMessage"
            class="bg-green-500 hover:bg-green-600 text-white font-bold py-2 px-4 rounded"
          >
            Send
          </button>
        </div>

        <label class="flex items-center">
          <input type="checkbox" v-model="autoScroll" class="mr-2" />
          <span>Enable Autoscroll</span>
        </label>
      </div>
    </div>
  </div>
</template>

<script>
export default {
	data() {
		return {
			status: "Connecting...",
			socket: null,
			messages: [],
			messageInput: "",
			autoScroll: true,
		};
	},
	methods: {
		redirectToDocs() {
			window.location.href = "/docs/";
		},
		connectWebSocket() {
			this.socket = new WebSocket(`ws://${window.location.host}/ws`);

			this.socket.onopen = () => {
				this.status = "Connected";
			};

			this.socket.onmessage = (event) => {
				this.messages.push(event.data);
				this.$nextTick(() => {
					if (this.autoScroll) {
						this.scrollToBottom();
					}
				});
			};

			this.socket.onclose = () => {
				this.status = "Disconnected";
			};

			this.socket.onerror = () => {
				this.status = "Error";
			};
		},
		sendMessage() {
			if (this.messageInput && this.socket.readyState === WebSocket.OPEN) {
				this.socket.send(this.messageInput);
				this.messageInput = "";
			}
		},
		scrollToBottom() {
			const container = this.$refs.messagesContainer;
			if (container) {
				container.scrollTop = container.scrollHeight;
			}
		},
	},
	mounted() {
		this.connectWebSocket();
	},
	beforeDestroy() {
		if (this.socket) {
			this.socket.close();
		}
	},
	updated() {
		this.$nextTick(() => {
			if (this.autoScroll) {
				this.scrollToBottom();
			}
		});
	},
};
</script>
