<template>
  <v-container class="pa-0">
    <v-row>
      <v-col class="mr-2" v-if="!mcapData">
        <input type="file" @change="loadFile" accept=".mcap" />
      </v-col>

      <template v-if="loadedData.length > 0">
        <v-col cols="auto" class="d-flex align-center justify-center">
          <v-btn
            class="player-btn"
            @click="togglePlayPause"
            icon
            variant="outlined"
            size="48"
          >
            <v-icon size="24">{{ isPlaying ? 'mdi-pause' : 'mdi-play' }}</v-icon>
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
import { onMounted, onUnmounted, ref, watch } from 'vue';

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

const emit = defineEmits([
  'update:currentFrame',
  'update:frames',
  'update:heading',
  'loadedData',
  'parsingProgress',
  'error',
]);

// Vehicle poses stay on their own timeline. Heading is sampled from the playback
// clock so it is not stuck waiting for the next sonar line to be drawn.
let yawSamples = [];
let lastHeading = null;
let applyingPlaybackFrame = false;

const angleDelta = (from, to) => ((to - from + 540) % 360) - 180;

const headingAtTime = (timeMs) => {
  const count = yawSamples.length;
  if (count === 0) return null;
  if (timeMs < yawSamples[0].t) return null;
  if (timeMs >= yawSamples[count - 1].t) return yawSamples[count - 1].deg;

  let lo = 0;
  let hi = count - 1;
  while (lo + 1 < hi) {
    const mid = (lo + hi) >> 1;
    if (yawSamples[mid].t <= timeMs) lo = mid;
    else hi = mid;
  }

  const start = yawSamples[lo];
  const end = yawSamples[hi];
  const span = end.t - start.t;
  const mix = span > 0 ? (timeMs - start.t) / span : 0;
  return start.deg + angleDelta(start.deg, end.deg) * mix;
};

const emitHeading = (timeMs) => {
  const degrees = headingAtTime(timeMs);
  if (degrees == null) return;
  if (lastHeading != null && Math.abs(angleDelta(lastHeading, degrees)) < 0.01) return;
  lastHeading = degrees;
  emit('update:heading', degrees);
};

const frameTimeMs = (index) => new Date(loadedData.value[index].timestamp).getTime();

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

    yawSamples = [];
    lastHeading = null;

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

    const decodeMessage = (msg) => {
      try {
        if (msg.data instanceof Uint8Array) {
          const decoded = new TextDecoder().decode(msg.data);
          try {
            return JSON.parse(decoded);
          } catch {
            return { raw: decoded };
          }
        }
        if (typeof msg.data === 'object' && msg.data !== null) {
          return msg.data;
        }
        return { raw: msg.data };
      } catch (decodeError) {
        console.warn('Error decoding message data:', decodeError);
        return { raw: Array.from(msg.data ?? []) };
      }
    };

    const topicOf = (msg) => reader.channelsById.get(msg.channelId)?.topic;

    // Poses are kept separate from sonar frames so playback can move the heading
    // between pings. VehicleData.heading (VFR_HUD, degrees) streams faster than
    // yaw (ATTITUDE, radians), but older recordings only have yaw.
    for (const msg of messages) {
      const topic = topicOf(msg);
      if (!topic || !topic.endsWith('/VehicleData')) continue;
      const pose = decodeMessage(msg);
      let deg = null;
      if (Number.isFinite(pose?.heading)) deg = pose.heading;
      else if (Number.isFinite(pose?.yaw)) deg = (pose.yaw * 180) / Math.PI;
      if (deg == null) continue;
      yawSamples.push({ t: Number(msg.logTime / 1_000_000n), deg });
    }
    yawSamples.sort((a, b) => a.t - b.t);
    const collapsed = [];
    for (const sample of yawSamples) {
      const previous = collapsed[collapsed.length - 1];
      if (previous && previous.t === sample.t) previous.deg = sample.deg;
      else collapsed.push(sample);
    }
    yawSamples = collapsed;

    loadedData.value = messages.flatMap((msg) => {
      const parsedData = decodeMessage(msg);

      const timestamp = new Date(Number(msg.logTime / 1_000_000n)).toISOString();

      const topic = topicOf(msg);

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

    applyingPlaybackFrame = true;
    currentFrame.value = 0;
    applyingPlaybackFrame = false;
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
    emit('error', error.message || 'Unknown error while parsing MCAP file');
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

const stopPlaybackLoop = () => {
  if (playTimer) {
    cancelAnimationFrame(playTimer);
    playTimer = null;
  }
};

const play = () => {
  if (!loadedData.value.length) return;
  if (currentFrame.value >= loadedData.value.length - 1) {
    currentFrame.value = 0;
  }
  isPlaying.value = true;
  const offset = frameTimeMs(currentFrame.value) - baseTimestamp;
  startTime = performance.now() - offset / playbackSpeed.value;
  stopPlaybackLoop();
  playbackLoop();
};

const pause = () => {
  isPlaying.value = false;
  stopPlaybackLoop();
};

const togglePlayPause = () => {
  if (isPlaying.value) {
    pause();
  } else {
    play();
  }
};

const playbackLoop = () => {
  if (!isPlaying.value) return;
  playTimer = requestAnimationFrame(() => {
    if (!isPlaying.value) return;

    const mediaOffset = (performance.now() - startTime) * playbackSpeed.value;
    const target = baseTimestamp + mediaOffset;
    const lastIndex = loadedData.value.length - 1;
    let index = currentFrame.value;
    const due = [];

    while (index < lastIndex && frameTimeMs(index + 1) <= target) {
      index += 1;
      due.push(loadedData.value[index]);
    }

    if (due.length) {
      applyingPlaybackFrame = true;
      currentFrame.value = index;
      applyingPlaybackFrame = false;
      emit('update:frames', due);
    }

    emitHeading(target);

    if (index >= lastIndex) {
      isPlaying.value = false;
      stopPlaybackLoop();
      return;
    }

    playbackLoop();
  });
};

const updateCurrentFrame = () => {
  if (!loadedData.value.length) return;
  currentFrame.value = Math.min(Math.max(0, currentFrame.value), loadedData.value.length - 1);
  emit('update:currentFrame', loadedData.value[currentFrame.value]);
  if (!isPlaying.value) emitHeading(frameTimeMs(currentFrame.value));
};

const handleFrameChange = () => {
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

watch(
  currentFrame,
  () => {
    if (applyingPlaybackFrame) return;
    updateCurrentFrame();
  },
  { flush: 'sync' }
);

watch(playbackSpeed, () => {
  if (isPlaying.value) {
    pause();
    play();
  }
});

let wasPlayingBeforeHidden = false;

const handleVisibilityChange = () => {
  if (document.hidden) {
    if (isPlaying.value) {
      wasPlayingBeforeHidden = true;
      pause();
    }
  } else if (wasPlayingBeforeHidden) {
    wasPlayingBeforeHidden = false;
    play();
  }
};

onMounted(() => {
  document.addEventListener('visibilitychange', handleVisibilityChange);
});

onUnmounted(() => {
  document.removeEventListener('visibilitychange', handleVisibilityChange);
  stopPlaybackLoop();
});

defineExpose({ loadFile, play, pause, togglePlayPause });
</script>

<style scoped>
.player-btn {
  background: rgba(255, 255, 255, 0.1) !important;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.2) !important;
  color: inherit !important;
}

.player-btn:hover {
  background: rgba(255, 255, 255, 0.2) !important;
}
</style>
