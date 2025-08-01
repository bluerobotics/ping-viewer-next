<template>
  <v-container class="pa-0">
    <v-row>
      <v-col class="mr-2" v-if="!mcapData">
        <input type="file" @change="loadFile" accept=".mcap" />
      </v-col>

      <template v-if="loadedData.length > 0">
        <v-col cols="2">
          <v-btn
            :color="isPlaying ? 'warning' : 'success'"
            @click="togglePlayPause"
            density="compact"
            icon
          >
            <v-icon>{{ isPlaying ? 'mdi-pause' : 'mdi-play' }}</v-icon>
          </v-btn>
        </v-col>

        <v-col cols="2" class="d-flex flex-column justify-center mr-4 pr-0">
          <input type="range" v-model.number="playbackSpeed" min="0.1" max="10" step="0.1" class="w-100" />
          <div class="text-caption">Speed: {{ playbackSpeed }}x</div>
        </v-col>

        <v-col class="d-flex flex-column justify-center">
          <input type="range" v-model.number="currentFrame" :min="0" :max="loadedData.length - 1" class="w-100"
            @input="handleFrameChange" />
          <div class="d-flex justify-space-between">
            <span class="text-caption">{{ formatTime(loadedData[currentFrame]?.timestamp) }}</span>
          </div>
        </v-col>
      </template>
    </v-row>
  </v-container>
</template>
<script setup>
import { BlobReadable } from '@mcap/browser';
import { McapIndexedReader } from '@mcap/core';
import { loadDecompressHandlers } from '@mcap/support';
import { ref, watch } from 'vue';

const props = defineProps({
  mcapData: {
    type: ArrayBuffer,
    default: null,
  },
  autoPlay: {
    type: Boolean,
    default: true,
  },
});

const loadedData = ref([]);
const currentFrame = ref(0);
const isPlaying = ref(false);
const playbackSpeed = ref(1);
let playTimer = null;
let startTime = 0;
let baseTimestamp = 0;

const emit = defineEmits(['update:currentFrame', 'loadedData', 'parsingProgress']);

const loadMcapFromBuffer = async (arrayBuffer) => {
  try {
    let decompressHandlers = {};
    try {
      decompressHandlers = await loadDecompressHandlers();
    } catch (error) {
      console.warn('Could not load decompression handlers:', error);
    }

    const blob = new Blob([arrayBuffer], { type: 'application/octet-stream' });
    const reader = await McapIndexedReader.Initialize({
      readable: new BlobReadable(blob),
      decompressHandlers,
    });

    const messages = [];
    let messageCount = 0;
    let totalMessages = 0;
    if (
      reader.messageIndex &&
      Array.isArray(reader.messageIndex) &&
      reader.messageIndex.length > 0
    ) {
      totalMessages = Number(reader.messageIndex.length);
    } else if (reader.statistics?.messageCount) {
      totalMessages = Number(reader.statistics.messageCount);
    }

    for await (const msg of reader.readMessages()) {
      messages.push(msg);
      messageCount++;
      if (totalMessages > 0) {
        if (messageCount % 100 === 0 || messageCount === totalMessages) {
          emit('parsingProgress', Math.floor((messageCount / totalMessages) * 100));
        }
      }
    }

    loadedData.value = messages.flatMap((msg, index) => {
      let parsedData;

      try {
        if (msg.data instanceof Uint8Array) {
          const decoded = new TextDecoder().decode(msg.data);
          try {
            parsedData = JSON.parse(decoded);
          } catch {
            parsedData = { raw: decoded };
          }
        } else if (typeof msg.data === 'object') {
          parsedData = msg.data;
        } else {
          parsedData = { raw: msg.data };
        }
      } catch (decodeError) {
        console.warn('Error decoding message data:', decodeError);
        parsedData = { raw: Array.from(msg.data) };
      }

      const timestamp = new Date(Number(msg.logTime / 1_000_000n)).toISOString();

      const channelInfo = reader.channelsById.get(msg.channelId);
      const topic = channelInfo?.topic;

      if (!topic) {
        return [];
      }

      const deviceMatch = topic.match(/device_([^\/]+)\/(.+)$/);
      if (!deviceMatch) {
        return [];
      }

      const deviceId = deviceMatch[1];
      const rawDeviceType = deviceMatch[2];

      // Normalize device type to expected format
      let deviceType;
      switch (rawDeviceType.toLowerCase()) {
        case 'ping1d':
          deviceType = 'Ping1D';
          break;
        case 'ping360':
          deviceType = 'Ping360';
          break;
        default:
          return [];
      }

      // Transform to match your expected JSON structure based on device type
      if (deviceType === 'Ping1D') {
        return [
          {
            timestamp,
            device: {
              id: deviceId,
              device_type: deviceType,
            },
            data: {
              sensorData: parsedData.profile_data,
              currentDepth: parsedData.distance / 1000,
              minDepth: parsedData.scan_start / 1000,
              maxDepth: parsedData.scan_length / 1000,
              confidence: parsedData.confidence,
              accuracy:
                ((100 - parsedData.confidence) / 100) *
                (parsedData.scan_length / 1000 - parsedData.scan_start / 1000) *
                0.1,
            },
          },
        ];
      }
      if (deviceType === 'Ping360') {
        return [
          {
            timestamp,
            device: {
              id: deviceId,
              device_type: deviceType,
            },
            data: {
              angle: parsedData.angle,
              data: parsedData.data,
              sample_period: parsedData.sample_period,
              number_of_samples: parsedData.number_of_samples,
              start_angle: parsedData.start_angle,
              stop_angle: parsedData.stop_angle,
            },
          },
        ];
      }
    });

    currentFrame.value = 0;
    if (loadedData.value.length > 0) {
      baseTimestamp = new Date(loadedData.value[0].timestamp).getTime();
    }

    emit('loadedData', loadedData.value);
    updateCurrentFrame();

    // Auto-play if enabled and we have data
    if (props.autoPlay && loadedData.value.length > 0) {
      // Small delay to ensure everything is ready
      setTimeout(() => {
        play();
      }, 100);
    }
  } catch (error) {
    console.error('Detailed error loading MCAP file:', error);
    console.error('Error stack:', error.stack);
    const errorMessage = error.message;
    alert(`Error loading MCAP file: ${errorMessage}\nCheck console for details.`);
  }
};

const loadFile = async (event) => {
  const file = event.target.files?.[0];
  if (!file) return;

  if (file.name.endsWith('.mcap')) {
    const arrayBuffer = await file.arrayBuffer();
    await loadMcapFromBuffer(arrayBuffer);
  } else {
    alert('Unsupported file type. Please select a .mcap file.');
  }
};

watch(
  () => props.mcapData,
  async (newData) => {
    if (newData) {
      await loadMcapFromBuffer(newData);
    }
  },
  { immediate: true }
);

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

const togglePlayPause = () => {
  if (isPlaying.value) {
    pause();
  } else {
    play();
  }
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

defineExpose({ loadFile, play, pause, togglePlayPause });
</script>
