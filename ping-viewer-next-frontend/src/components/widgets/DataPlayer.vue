<template>
  <v-container class="pa-0">
    <v-row>
      <v-col class="mr-2">
        <input type="file" @change="loadFile" accept=".json" />
      </v-col>

      <template v-if="loadedData.length > 0">
        <v-col cols="auto" class="d-flex align-center">
          <v-btn color="success" @click="play" :disabled="isPlaying" density="compact" class="mr-1">
            <v-icon left>mdi-play</v-icon>
          </v-btn>

          <v-btn color="warning" @click="pause" :disabled="!isPlaying" density="compact" class="mr-1">
            <v-icon left>mdi-pause</v-icon>
          </v-btn>

          <v-btn color="error" @click="stop" density="compact" class="mr-1">
            <v-icon left>mdi-stop</v-icon>
          </v-btn>
        </v-col>

        <v-col cols="2" class="d-flex flex-column justify-center mr-4">
          <input type="range" v-model.number="playbackSpeed" min="0.1" max="10" step="0.1" class="w-100" />
          <div class="text-caption">Speed: {{ playbackSpeed }}x</div>
        </v-col>

        <v-col class="d-flex flex-column justify-center">
          <input type="range" v-model.number="currentFrame" :min="0" :max="loadedData.length - 1" class="w-100"
            @input="handleFrameChange" />
          <div class="d-flex justify-space-between">
            <span class="text-caption">Frame: {{ displayedFrame }} / {{ loadedData.length }}</span>
            <span class="text-caption">Time: {{ formatTime(loadedData[currentFrame]?.timestamp) }}</span>
          </div>
        </v-col>
      </template>
    </v-row>
  </v-container>
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
