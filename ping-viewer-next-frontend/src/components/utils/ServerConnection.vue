<template>
  <v-dialog v-if="!serverInfo" v-model="dialog" persistent width="500" class="glassMenu">
    <v-card class="glassMenu server-connection">
      <v-card-title class="text-h5 pt-2 pl-2">
        <div class="flex justify-between">
          <v-icon start icon="mdi-connection" size="30" class="mr-2" />
          <p class="-ml-4">Ping Viewer Next</p>
          <div></div>
        </div>
      </v-card-title>

      <v-card-text class="glassMenu server-connection">
        <div>
          <v-stepper v-model="currentStep" class="pl-6 glassMenuBlack elevation-3">
            <v-stepper-items>
              <v-stepper-item v-for="step in 4" :key="step" :value="step" >
                <div class="flex w-full justify-between items-center">
                  <div class="flex-grow-1">
                    <div>{{ getStepText(step) }}</div>
                    <div v-if="currentStep === step && error" class="bg-[#FF525277] text-white text-body-2 py-2 px-4 rounded-md mt-2 ml-2" style="border: 1px solid #FFFFFF44">
                      {{ error }}
                    </div>
                  </div>
                  <div class="ml-8">
                    <v-icon v-if="currentStep > step" color="success">mdi-check-circle</v-icon>
                    <v-progress-circular v-else-if="currentStep === step && loading" indeterminate size="24" class="opacity-80"/>
                  </div>
                </div>

                <div v-if="step === 4 && currentStep === 4" class="mt-4">
                  <v-text-field v-model="remoteAddress" label="Server Address" placeholder="e.g. pingviewernext:8080" class="w-[95%] mt-[18px]"
                    hint="Enter the server address to connect" persistent-hint @keyup.enter="connectToRemote" />
                  <v-btn block class="mt-6 -mb-4 -ml-4 glassButton" @click="connectToRemote" :loading="loading">
                    Connect to Remote Server
                  </v-btn>
                </div>
              </v-stepper-item>
            </v-stepper-items>
          </v-stepper>
        </div>
      </v-card-text>
    </v-card>
  </v-dialog>
  <SplashScreen v-if="showSplashScreen" />
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';
import SplashScreen from './SplashScreen.vue';

const emit = defineEmits(['serverConnected']);

const dialog = ref(true);
const currentStep = ref(1);
const loading = ref(false);
const error = ref(null);
const serverInfo = ref(null);
const remoteAddress = ref('');
const autoConfirmCountdown = ref(0);
const showSplashScreen = ref(true);
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
  showSplashScreen.value = false;
  emit('serverConnected', url);
};

const checkBlueOSVersion = async () => {
  try {
    const versionUrl = `${location.protocol}//${location.hostname}:8081/v1.0/version/current`;
    const response = await fetch(versionUrl, {
      method: 'GET',
      headers: {
        Accept: 'application/json',
      },
    });

    if (response.ok) {
      const versionData = await response.json();
      return {
        success: true,
        data: versionData,
      };
    }
    return { success: false };
  } catch (error) {
    console.error('Version endpoint check failed:', error);
    return { success: false };
  }
};

// Check if the there is an active connection; if not, keep the splash screen open for a maximum of 10 seconds.
onBeforeMount(async () => {
  const minSplashDuration = 5000;
  const maxSplashDuration = 10000;
  const startTime = Date.now();

  // Close splash screen no matter what, after 10 seconds
  setTimeout(() => {
    showSplashScreen.value = false;
  }, maxSplashDuration);

  const elapsed = Date.now() - startTime;
  if (elapsed < minSplashDuration) await sleep(minSplashDuration - elapsed);

  showSplashScreen.value = false;
});

onMounted(async () => {
  // BlueOS extension should skip connection menu
  const versionCheck = await checkBlueOSVersion();
  if (versionCheck.success) {
    serverInfo.value = versionCheck.data;
    remoteAddress.value = `${location.host}`;
    dialog.value = false;
    showSplashScreen.value = false;
    confirmConnection();
    return;
  }

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

  error.value = 'Could not connect to any known servers';
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
.main-window {
  border-radius: var(--border-radius);
  background-color: rgba(var(--v-theme-background), 0.5) !important;
  border: 1px solid rgba(203, 203, 203, 0.13) !important;
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
  0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}


.server-connection :deep(.v-stepper) {
  border-radius: var(--border-radius);
  background-color: rgba(var(--v-theme-background), 0.5) !important;
  border: 1px solid rgba(203, 203, 203, 0.13) !important;
  box-shadow: 0px 4px 4px 0px rgba(0, 0, 0, 0.3),
    0px 8px 12px 6px rgba(0, 0, 0, 0.15) !important;
}

.server-connection :deep(.v-stepper-item) {
  margin-bottom: 16px;
}
</style>