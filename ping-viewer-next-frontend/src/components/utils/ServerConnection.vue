<template>
  <v-dialog v-model="dialog" persistent width="500" class="server-connection">
    <v-card>
      <v-card-title class="text-h5 pb-2">
        <v-icon start icon="mdi-connection" class="mr-2" />
        Ping Viewer Next
      </v-card-title>

      <v-card-text>
        <div v-if="!serverInfo">
          <v-stepper v-model="currentStep" class="">
            <v-stepper-items>
              <v-stepper-item v-for="step in 4" :key="step" :value="step">
                <div class="d-flex align-center py-2">
                  <div class="mr-4">
                    <v-icon v-if="currentStep > step" color="success">mdi-check-circle</v-icon>
                    <v-progress-circular v-else-if="currentStep === step && loading" indeterminate size="24" />
                    <span v-else>{{ step }}</span>
                  </div>
                  <div class="flex-grow-1">
                    <div>{{ getStepText(step) }}</div>
                    <div v-if="currentStep === step && error" class="text-error text-body-2 mt-1">
                      {{ error }}
                    </div>
                  </div>
                </div>

                <div v-if="step === 4 && currentStep === 4" class="mt-4">
                  <v-text-field v-model="remoteAddress" label="Server Address" placeholder="e.g. pingviewernext:8080"
                    hint="Enter the server address to connect" persistent-hint @keyup.enter="connectToRemote" />
                  <v-btn block color="primary" class="mt-4" @click="connectToRemote" :loading="loading">
                    Connect to Remote Server
                  </v-btn>
                </div>
              </v-stepper-item>
            </v-stepper-items>
          </v-stepper>
        </div>

        <div v-else>
          <v-row justify="center" class="mb-6">
            <v-col cols="auto">
              <v-progress-circular :model-value="autoConfirmCountdown * 20" color="primary" size="92">
                <v-icon size="40" color="primary">mdi-check</v-icon>
              </v-progress-circular>
            </v-col>
          </v-row>

          <div class="text-center mb-6">
            <div class="text-h6 mb-2">Connected Successfully</div>
            <div class="text-body-1">{{ serverInfo.name }} (v{{ serverInfo.version }})</div>
            <div v-if="autoConfirmCountdown > 0" class="text-body-2 mt-2">
              Auto-continuing in {{ autoConfirmCountdown }}s...
            </div>
          </div>

          <v-card-actions class="px-0">
            <v-spacer />
            <v-btn variant="text" @click="editServer">
              Change Server
            </v-btn>
            <v-btn color="primary" @click="confirmConnection">
              Continue
            </v-btn>
          </v-card-actions>
        </div>
      </v-card-text>
    </v-card>
  </v-dialog>
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';

const emit = defineEmits(['serverConnected']);

const dialog = ref(true);
const currentStep = ref(1);
const loading = ref(false);
const error = ref(null);
const serverInfo = ref(null);
const remoteAddress = ref('');
const autoConfirmCountdown = ref(0);
let countdownTimer = null;

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
  dialog.value = false;
  emit('serverConnected', url);
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

watch(serverInfo, (newValue, oldValue) => {
  if (!newValue && oldValue) {
    stopAutoConfirmCountdown();
  }
});

onUnmounted(() => {
  stopAutoConfirmCountdown();
});
</script>

<style scoped>
.server-connection :deep(.v-stepper) {
  box-shadow: none;
}

.server-connection :deep(.v-stepper-item) {
  margin-bottom: 16px;
}
</style>