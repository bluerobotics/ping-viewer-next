<template>
  <div v-if="showSplashScreen">
    <img
      class="fixed w-[100vw] h-[100vh] top-0 left-0 z-[9990] filter brightness-[60%]"
      :src="splashBackground"
      alt="Background"
    />
    <div
  class="fixed w-[70vw] h-[30vw] top-1/2 left-1/2 rounded-[20px] transform -translate-x-1/2 -translate-y-1/2 z-[9992]"
  :style="{
    backgroundColor: 'rgba(0, 0, 0, 0.10)',
    color: 'rgba(255, 255, 255, 1)',
    border: '1px solid rgba(255, 255, 255, 0.25)',
    backdropFilter: 'blur(25px)',            
    WebkitBackdropFilter: 'blur(16px)',     
    boxShadow: '0px 8px 8px 0px #00000033, 0px 8px 12px 6px #00000016'
  }"
>
      <div
        class="relative flex flex-col w-full h-[80%] rounded-tr-[20px] rounded-tl-[20px] items-center justify-center elevation-7 border-b-[1px] border-[#ffffff33] bg-[#FFFFFF11] px-8"
      >
        <img
          class="h-[55%] -mb-10 z-[9993] object-contain opacity-80 w-5/6"
          :src="pingViewerLogo"
          alt="Ping Viewer Logo"
        />
        <img
          class="absolute top-[2vh] left-[2vh] w-[15%] z-[9993] opacity-80"
          :src="blueRoboticsWhiteNameLogo"
          alt="Blue Robotics Logo"
        />
      </div>

      <div class="flex flex-col w-full h-[20%] items-center justify-center text-center opacity-80">
        <p class="text-[1.7vw]">{{ randomLightHeartedMessage }}</p>
      </div>
    </div>
    <div class="fixed top-4 right-4 z-[9993]">
      <button
        class="bg-[#00000088] text-white p-1 rounded-full elevation-1 focus:outline-none"
        @click="showSplashScreen = false"
      >
        <v-icon icon="mdi-close" class="text-2xl -mt-[2px] -mr-[1px]" />
      </button>
    </div>
    <div class="fixed bottom-4 right-4 z-[9993]">
      <button
        class="bg-[#00000088] text-white p-1 rounded-full elevation-3 focus:outline-none"
        @click="toggleFullscreen"
      >
        <v-icon :icon="isFullscreen ? 'mdi-fullscreen-exit' : 'mdi-fullscreen'" class="text-2xl -mt-[2px] -mr-[1px]" />
      </button>
    </div>
  </div>
</template>
<script lang="ts" setup>
import blueRoboticsWhiteNameLogo from '../../assets/blue-robotics-white-name-logo.png';
import pingViewerLogo from '../../assets/ping-2-logo.png';
import splashBackground from '../../assets/splash-background.png';

const { isFullscreen, toggle: toggleFullscreen } = useFullscreen();

import { useFullscreen } from '@vueuse/core';
import { onBeforeUnmount, onMounted, ref } from 'vue';

const showSplashScreen = ref(true);
const randomLightHeartedMessage = ref<string>('');
let timerId: ReturnType<typeof setTimeout>;

const startupLightHeartedMessages: string[] = [
  'Pinging plankton for signal clarity...',
  'Listening for whispers from the deep...',
  'Rendering reflections of rebellious rays...',
  'Teaching sonar beams to stay in their lanes...',
  'Cross-checking echoes for existential questions...',
  'Calibrating the ocean’s mood swings...',
  'Counting bubbles per second for accuracy...',
  'Decoding dolphin dialects into raw data...',
  'Tuning sonar chirps to the key of C-sea-major...',
  'Scanning abyssal plains for misplaced pixels...',
  'Asking manta rays to stop photobombing scans...',
  'Collecting echoes of long-lost legends...',
  'Reverberating responsibly — eco-friendly sonar engaged...',
  'Smoothing out waveforms for that silky sonar look...',
  'Analyzing echoes from yesterday’s tides...',
  'Summoning sub-surface spirits for calibration...',
  'Measuring murkiness — scientifically and emotionally...',
  'Refactoring sonar pings to sound 12% friendlier...',
  'Locating the legendary Lag Monster below 200 meters...',
  'Optimizing ping intervals to impress the squids...',
  'Searching for Atlantis... again...',
  'Synchronizing sonar with whale lullabies...',
  'Distinguishing fish schools from coding schools...',
  'Verifying that reflections are indeed reflective...',
  'Cleaning sonar domes — one bubble at a time...',
  'Detecting anomalies in the abyss (or just shrimp)...',
  'Negotiating data packets with passing jellyfish...',
  'Drawing depth lines with artistic flare (and flares)...',
  'Echo-locating enthusiasm across all channels...',
  'Waiting for the sea to answer our last ping...',
];

const remainingMessages = ref<string[]>([...startupLightHeartedMessages]);

const scheduleNextMessage = (): void => {
  const randomIndex = Math.floor(Math.random() * remainingMessages.value.length);
  const delay = Math.random() * 5000 + 3000;

  if (remainingMessages.value.length === 0) {
    remainingMessages.value = [...startupLightHeartedMessages];
  }
  randomLightHeartedMessage.value = remainingMessages.value.splice(randomIndex, 1)[0];
  timerId = setTimeout(scheduleNextMessage, delay);
};

const handleKeydown = (event: KeyboardEvent): void => {
  if (event.key === 'Escape') {
    showSplashScreen.value = false;
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
  scheduleNextMessage();
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown);
  clearTimeout(timerId);
});
</script>

<style scoped>
.glassMenuBlack {
      background-color: rgba(0, 0, 0, 0.15);
      color: rgba(255, 255, 255, 1);
      backdrop-filter: 25px blur;
      border: 1px solid rgba(255, 255, 255, 0.04);
      box-shadow: 0px 4px 4px 0px #00000033, 0px 8px 12px 6px #00000016;
}

@keyframes descend {
  from {
    transform: translateY(-150px);
  }
  to {
    transform: translateY(calc(100vh + 150px));
  }
}

.animate-descend {
  top: 0;
  animation: descend 40s linear forwards;
}

@keyframes tetherGrow {
  from {
    height: 0;
  }
  /* 100vh + 300px = total drop from -150px → (100vh +150px) */
  to {
    height: calc(100vh + 300px);
  }
}
.animate-tether-grow {
  animation: tetherGrow 40s linear forwards;
}

@keyframes left {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(-10vw);
  }
}
.animate-left {
  animation: left 30s linear forwards;
}

@keyframes ascend {
  from {
    transform: translateY(calc(0 + 150px));
  }
  to {
    transform: translateY(-350px);
  }
}
.animate-ascend {
  bottom: 0;
  animation: ascend 25s ease-in-out forwards;
  animation-delay: 5s;
}

@keyframes ascend {
  0% {
    transform: translate(0, 50px);
  }
  20% {
    transform: rotate(-6deg);
  }
  25% {
    transform: translate(-8px, -100px) rotate(6deg);
  }
  40% {
    transform: translate(6px, -170px);
  }

  100% {
    transform: translate(0, -400px);
  }
}

@keyframes tetherAscend {
  0% {
    transform: translate(0, 50px);
  }
  20% {
    transform: rotate(-6deg);
  }
  25% {
    transform: translate(-8px, -100px);
  }
  40% {
    transform: translate(6px, -170px) rotate(-6deg);
  }

  100% {
    transform: translate(20px, -350px) rotate(-26deg) scaleY(3);
  }
}

.animate-tether-ascend {
  bottom: 0;
  animation: tetherAscend 25s ease-in-out forwards;
  animation-delay: 5s;
}
</style>
