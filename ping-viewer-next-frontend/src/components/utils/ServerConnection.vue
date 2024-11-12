<template>
	<div class="fixed inset-0 backdrop-blur-sm flex items-center justify-center">
	  <div class="p-8 rounded-xl shadow-xl w-[450px] relative overflow-hidden">
		<div class="mb-6">
		  <h2 class="text-2xl font-bold">Ping Viewer Next</h2>
		  <p class="mt-1">Establishing connection to server...</p>
		</div>

		<div v-if="!serverInfo" class="space-y-4">
		  <div v-for="step in 4" :key="step"
			:class="['transition-all duration-300 py-3 px-4 rounded-lg',
			  currentStep === step ? 'bg-surface' : '']">
			<div class="flex items-center">
			  <div class="w-6 h-6 rounded-full flex items-center justify-center mr-3">
				<svg v-if="currentStep > step" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
				</svg>
				<span v-else class="text-sm">{{ step }}</span>
			  </div>
			  <span>{{ getStepText(step) }}</span>
			  <div v-if="currentStep === step && loading" class="ml-auto">
				<div class="w-5 h-5 border-2 border-t-transparent rounded-full animate-spin"></div>
			  </div>
			</div>
			<div v-if="currentStep === step && error" class="mt-2 ml-9 text-sm">
			  {{ error }}
			</div>

			<div v-if="step === 4 && currentStep === 4" class="mt-3 ml-9">
			  <input v-model="remoteAddress" type="text"
				class="w-full px-3 py-2 rounded border"
				placeholder="e.g. pingviewernext:8080"
				@keyup.enter="connectToRemote" />
			  <v-btn block class="mt-2" @click="connectToRemote">
				Connect to Remote Server
			  </v-btn>
			</div>
		  </div>
		</div>

		<div v-if="serverInfo" class="relative">
		  <div class="flex justify-center mb-6">
			<div class="relative">
			  <div class="w-16 h-16 rounded-full border-4 flex items-center justify-center">
				<svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
				</svg>
			  </div>
			  <div class="absolute inset-0 rounded-full border-4 border-t-transparent animate-spin"
				:style="{ animationDuration: '3s' }">
			  </div>
			</div>
		  </div>

		  <div class="text-center mb-6">
			<p class="font-bold text-lg mb-1">Connected Successfully</p>
			<p>
			  {{ serverInfo.name }} (v{{ serverInfo.version }})
			</p>
			<p v-if="autoConfirmCountdown > 0" class="text-sm mt-2">
			  Auto-continuing in {{ autoConfirmCountdown }}s...
			</p>
		  </div>

		  <div class="flex justify-between items-center">
			<v-btn variant="text" @click="editServer">
			  Change Server
			</v-btn>
			<v-btn color="primary" @click="confirmConnection">
			  Continue
			</v-btn>
		  </div>
		</div>
	  </div>
	</div>
  </template>

  <script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';

const currentStep = ref(1);
const loading = ref(false);
const error = ref(null);
const serverInfo = ref(null);
const remoteAddress = ref('');
const lastUsedServer = ref(null);
const autoConfirmCountdown = ref(0);
let countdownTimer = null;

const emit = defineEmits(['serverConnected']);

const CACHE_KEY = 'pingviewer-server';

const stepTexts = [
  'Checking last used server...',
  'Checking local server...',
  'Checking default remote server...',
  'Manual server configuration',
];

const getStepText = (step) => {
  return stepTexts[step - 1] || '';
};

const loadLastUsedServer = () => {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    if (cached) {
      const data = JSON.parse(cached);
      remoteAddress.value = data.address;
      return data.address;
    }
  } catch (e) {
    console.error('Error loading cached server:', e);
  }
  return null;
};

const saveLastUsedServer = (address) => {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ address }));
  } catch (e) {
    console.error('Error saving server to cache:', e);
  }
};

const startAutoConfirmCountdown = () => {
  stopAutoConfirmCountdown();
  autoConfirmCountdown.value = 5;
  countdownTimer = setInterval(() => {
    if (autoConfirmCountdown.value <= 1) {
      stopAutoConfirmCountdown();
      confirmConnection();
    } else {
      autoConfirmCountdown.value--;
    }
  }, 1000);
};

const stopAutoConfirmCountdown = () => {
  if (countdownTimer) {
    clearInterval(countdownTimer);
    countdownTimer = null;
  }
  autoConfirmCountdown.value = 0;
};

const editServer = () => {
  stopAutoConfirmCountdown();
  serverInfo.value = null;
  currentStep.value = 4;
};

const tryConnect = async (url) => {
  loading.value = true;
  error.value = null;
  try {
    const response = await fetch(url, {
      mode: 'cors',
      headers: {
        Accept: 'application/json',
      },
    });
    if (response.ok) {
      const contentType = response.headers.get('content-type');
      if (contentType?.includes('application/json')) {
        const data = await response.json();
        serverInfo.value = data;
        loading.value = false;

        startAutoConfirmCountdown();
        return true;
      }
      throw new Error('Invalid response format');
    }
  } catch (err) {
    console.error(`Error connecting to ${url}:`, err);
    error.value = err.message;
  }
  loading.value = false;
  return false;
};

const proceedToNextStep = () => {
  currentStep.value++;
  error.value = null;
};

const getServerUrl = (host) => {
  const isSecure = window.location.protocol === 'https:';
  const protocol = isSecure ? 'https:' : 'http:';
  return `${protocol}//${host}/register_service`;
};

onMounted(async () => {
  // Step 1: Try last used server
  const lastServer = loadLastUsedServer();
  if (lastServer) {
    if (await tryConnect(getServerUrl(lastServer))) {
      return;
    }
  }
  proceedToNextStep();

  // Step 2: Try local server
  if (await tryConnect(getServerUrl(window.location.host))) {
    remoteAddress.value = window.location.host;
    return;
  }
  proceedToNextStep();

  // Step 3: Try default remote server
  const defaultServer = 'pingviewernext:8080';
  if (await tryConnect(getServerUrl(defaultServer))) {
    remoteAddress.value = defaultServer;
    return;
  }
  proceedToNextStep();

  error.value = 'Could not connect to any known servers. Please enter a custom address.';
});

const connectToRemote = async () => {
  if (!remoteAddress.value) {
    error.value = 'Please enter a remote server address.';
    return;
  }

  const success = await tryConnect(`http://${remoteAddress.value}/register_service`);

  if (!success) {
    error.value = 'Could not connect to the specified remote server.';
    return;
  }

  saveLastUsedServer(remoteAddress.value);
};

const confirmConnection = () => {
  stopAutoConfirmCountdown();
  const url = `http://${remoteAddress.value}`;
  saveLastUsedServer(remoteAddress.value);
  emit('serverConnected', url);
};

// Watch for changes to serverInfo to handle cleanup
watch(serverInfo, (newValue, oldValue) => {
  if (!newValue && oldValue) {
    stopAutoConfirmCountdown();
  }
});

onUnmounted(() => {
  stopAutoConfirmCountdown();
});
</script>