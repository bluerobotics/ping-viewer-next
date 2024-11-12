<template>
  <div class="mb-4">
    <input type="file" @change="loadFile" accept=".json" class="mb-2" />
    <div v-if="loadedData.length > 0" class="mt-4">
      <div class="flex items-center space-x-2">
        <button
          @click="play"
          :disabled="isPlaying"
          class="bg-green-500 text-white px-4 py-2 rounded hover:bg-green-600 disabled:opacity-50"
        >
          Play
        </button>
        <button
          @click="pause"
          :disabled="!isPlaying"
          class="bg-yellow-500 text-white px-4 py-2 rounded hover:bg-yellow-600 disabled:opacity-50"
        >
          Pause
        </button>
        <button
          @click="stop"
          class="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600"
        >
          Stop
        </button>
        <input
          type="range"
          v-model.number="playbackSpeed"
          min="0.1"
          max="10"
          step="0.1"
          class="w-32"
        />
        <span>Speed: {{ playbackSpeed }}x</span>
      </div>
      <div class="mt-2">
        <input
          type="range"
          v-model.number="currentFrame"
          :min="0"
          :max="loadedData.length - 1"
          class="w-full"
          @input="handleFrameChange"
        />
        <div class="flex justify-between">
          <span>Frame: {{ displayedFrame }} / {{ loadedData.length }}</span>
          <span
            >Time: {{ formatTime(loadedData[currentFrame]?.timestamp) }}</span
          >
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue';

const loadedData = ref([]);
const currentFrame = ref(0);
const isPlaying = ref(false);
const playbackSpeed = ref(1);
let playTimer = null;
let startTime = 0;
let baseTimestamp = 0;

const emit = defineEmits(['update:currentFrame', 'loadedData']);

const displayedFrame = computed(() => {
  return Math.min(Math.max(1, currentFrame.value + 1), loadedData.value.length);
});

const loadFile = (event) => {
  const file = event.target.files?.[0];
  if (!file) return;

  const reader = new FileReader();
  reader.onload = (e) => {
    try {
      loadedData.value = JSON.parse(e.target.result);
      currentFrame.value = 0;
      baseTimestamp = new Date(loadedData.value[0].timestamp).getTime();
      emit('loadedData', loadedData.value);
      updateCurrentFrame();
    } catch (error) {
      console.error('Error parsing JSON file:', error);
      alert("Error loading file. Please ensure it's a valid JSON file.");
    }
  };
  reader.readAsText(file);
};

const play = () => {
  if (currentFrame.value >= loadedData.value.length - 1) {
    currentFrame.value = 0;
  }
  isPlaying.value = true;
  startTime =
    performance.now() -
    (new Date(loadedData.value[currentFrame.value].timestamp).getTime() - baseTimestamp);
  playNextFrame();
};

const pause = () => {
  isPlaying.value = false;
  if (playTimer) {
    clearTimeout(playTimer);
  }
};

const stop = () => {
  isPlaying.value = false;
  currentFrame.value = 0;
  if (playTimer) {
    clearTimeout(playTimer);
  }
  updateCurrentFrame();
};

const playNextFrame = () => {
  if (!isPlaying.value || currentFrame.value >= loadedData.value.length - 1) {
    isPlaying.value = false;
    return;
  }

  updateCurrentFrame();
  currentFrame.value++;

  if (currentFrame.value < loadedData.value.length) {
    const currentTime = performance.now();
    const actualTimestamp =
      new Date(loadedData.value[currentFrame.value].timestamp).getTime() - baseTimestamp;
    const targetElapsedTime = actualTimestamp / playbackSpeed.value;
    const timeToNextFrame = Math.max(0, targetElapsedTime - (currentTime - startTime));

    playTimer = setTimeout(playNextFrame, timeToNextFrame);
  } else {
    isPlaying.value = false;
  }
};

const updateCurrentFrame = () => {
  currentFrame.value = Math.min(Math.max(0, currentFrame.value), loadedData.value.length - 1);
  emit('update:currentFrame', loadedData.value[currentFrame.value]);
};

const handleFrameChange = () => {
  updateCurrentFrame();
  if (isPlaying.value) {
    pause();
    play();
  }
};

const formatTime = (timestamp) => {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  return date.toUTCString();
};

watch(currentFrame, updateCurrentFrame);

watch(playbackSpeed, () => {
  if (isPlaying.value) {
    pause();
    play();
  }
});

defineExpose({ loadFile, play, pause, stop });
</script>
