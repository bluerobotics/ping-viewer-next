<template>
  <div class="flex flex-col h-full bg-transparent p-4" ref="containerRef">
    <WaterfallDisplay
      :width="width"
      :height="height"
      :sensorData="sensorData"
      :currentDepth="currentDepth"
      :minDepth="minDepth"
      :maxDepth="maxDepth"
      :confidence="confidence"
      :accuracy="accuracy"
      :colorPalette="colorPalette"
      :depthLineColor="depthLineColor"
      :depthTextColor="depthTextColor"
      :currentDepthColor="currentDepthColor"
      :confidenceColor="confidenceColor"
      :textBackground="textBackground"
      :tickCount="tickCount"
      :columnCount="columnCount"
      :getColorFromPalette="getColorFromPalette"
      class="flex-grow"
    />
    <div
      v-if="debug"
      class="mt-2 bg-black bg-opacity-50 text-white p-2 text-xs"
    >
      <p>Current Depth: {{ currentDepth.toFixed(2) }} m</p>
      <p>Min Depth: {{ minDepth.toFixed(2) }} m</p>
      <p>Max Depth: {{ maxDepth.toFixed(2) }} m</p>
      <p>Confidence: {{ confidence }}%</p>
      <p>Accuracy: {{ accuracy.toFixed(2) }} m</p>
      <p>Data Points: {{ sensorData.length }}</p>
      <p>Width: {{ width }}px, Height: {{ height }}px</p>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { getColorFromPalette } from '../SonarColorOptions';
import WaterfallDisplay from './WaterfallMask.vue';

const props = defineProps({
  width: { type: Number, required: true },
  height: { type: Number, required: true },
  sensorData: { type: Array, required: true },
  currentDepth: { type: Number, required: true },
  minDepth: { type: Number, required: true },
  maxDepth: { type: Number, required: true },
  confidence: { type: Number, required: true },
  accuracy: { type: Number, required: true },
  colorPalette: { type: String, required: true },
  debug: { type: Boolean, default: false },
  depthLineColor: { type: String, default: 'yellow' },
  depthTextColor: { type: String, default: 'yellow' },
  currentDepthColor: { type: String, default: 'yellow' },
  confidenceColor: { type: String, default: '#00FF00' },
  textBackground: { type: String, default: 'rgba(0, 0, 0, 0.5)' },
  depthArrowColor: { type: String, default: 'yellow' },
  tickCount: { type: Number, default: 5 },
  columnCount: { type: Number, default: 100 },
});

const containerRef = ref(null);
</script>