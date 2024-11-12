<template>
	<div class="waterfall-display relative w-full h-full">
		<WaterfallShader ref="waterfallShader" :width="width" :height="height" :max-depth="maxDepth"
			:min-depth="minDepth" :column-count="columnCount" :sensor-data="sensorData" :color-palette="colorPalette"
			:get-color-from-palette="getColorFromPalette" @update:columnCount="$emit('update:columnCount', $event)"
			@mousemove="handleMouseMove" @mouseleave="handleMouseLeave" />
		<canvas ref="overlayCanvas" class="absolute top-0 left-0 w-full h-full pointer-events-none"></canvas>
		<div class="depth-line absolute top-0 right-0 h-full w-px" :style="{ backgroundColor: depthLineColor }">
			<div v-for="tick in depthTicks" :key="tick" class="tick absolute right-0 w-2 h-px" :style="{
				top: `${tickPosition(tick)}%`,
				backgroundColor: depthLineColor,
			}">
				<span class="absolute right-3 transform -translate-y-1/2 text-xs px-1 rounded"
					:style="{ color: depthTextColor, backgroundColor: textBackground }">
					{{ tick.toFixed(1) }}m
				</span>
			</div>
		</div>
		<div class="depth-arrow absolute right-0 w-0 h-0 border-solid border-transparent border-l-8" :style="{
			top: `${arrowPosition}%`,
			borderLeftColor: depthArrowColor,
			transform: 'translateY(-50%)',
		}"></div>
		<div class="current-depth absolute top-2 left-2 text-sm px-1 rounded"
			:style="{ color: currentDepthColor, backgroundColor: textBackground }">
			Depth: {{ currentDepth.toFixed(2) }}m ± {{ accuracy.toFixed(2) }}m
		</div>
		<div class="confidence absolute top-8 left-2 text-sm px-1 rounded"
			:style="{ color: confidenceColor, backgroundColor: textBackground }">
			Confidence: {{ confidence }}%
		</div>
		<div class="virtual-max-depth absolute top-14 left-2 text-sm px-1 rounded"
			:style="{ color: virtualMaxDepthColor, backgroundColor: textBackground }">
			Current Max Depth: {{ virtualMaxDepth.toFixed(2) }}m
		</div>
		<div v-if="hoveredColumn !== null"
			class="hovered-column-info absolute bottom-2 left-2 text-sm px-2 py-1 rounded flex flex-col space-y-1"
			:style="{ color: currentDepthColor, backgroundColor: textBackground }">
			<div class="font-bold">Selected Measurements</div>

			<div class="flex justify-between">
				<span>Depth:</span>
				<span>{{ historicalData[hoveredColumn]?.depth.toFixed(2) }}m</span>
			</div>

			<div class="flex justify-between">
				<span>Min Depth:</span>
				<span>{{ historicalData[hoveredColumn]?.minDepth.toFixed(2) }}m</span>
			</div>

			<div class="flex justify-between">
				<span>Max Depth:</span>
				<span>{{ historicalData[hoveredColumn]?.maxDepth.toFixed(2) }}m</span>
			</div>

			<div class="flex justify-between">
				<span>Virtual Max Depth:</span>
				<span>{{ historicalData[hoveredColumn]?.virtualMaxDepth.toFixed(2) }}m</span>
			</div>

			<div class="flex justify-between">
				<span>Confidence:</span>
				<span>{{ historicalData[hoveredColumn]?.confidence }}%</span>
			</div>
		</div>
	</div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import WaterfallShader from './WaterfallShader.vue';

const props = defineProps({
  width: { type: Number, required: true },
  height: { type: Number, required: true },
  maxDepth: { type: Number, required: true },
  minDepth: { type: Number, required: true },
  columnCount: { type: Number, default: 200 },
  sensorData: { type: Array, required: true },
  colorPalette: { type: String, required: true },
  getColorFromPalette: { type: Function, required: true },
  currentDepth: { type: Number, required: true },
  accuracy: { type: Number, required: true },
  confidence: { type: Number, required: true },
  depthLineColor: { type: String, default: 'yellow' },
  depthTextColor: { type: String, default: 'yellow' },
  currentDepthColor: { type: String, default: 'yellow' },
  confidenceColor: { type: String, default: '#00FF00' },
  virtualMaxDepthColor: { type: String, default: '#FF8C00' },
  textBackground: { type: String, default: 'rgba(0, 0, 0, 0.5)' },
  depthArrowColor: { type: String, default: 'yellow' },
  tickCount: { type: Number, default: 5 },
});

const emit = defineEmits(['update:columnCount']);

const waterfallShader = ref(null);
const overlayCanvas = ref(null);
const ctx = ref(null);
const virtualMaxDepth = ref(props.maxDepth);
const hoveredColumn = ref(null);
const historicalData = ref([]);

const updateVirtualMaxDepth = () => {
  if (historicalData.value.length === 0) {
    virtualMaxDepth.value = props.maxDepth;
    return;
  }

  const maxHistoricalDepth = Math.max(...historicalData.value.map((data) => data.maxDepth));

  if (maxHistoricalDepth > virtualMaxDepth.value) {
    virtualMaxDepth.value = maxHistoricalDepth;
  } else if (maxHistoricalDepth < virtualMaxDepth.value) {
    virtualMaxDepth.value = Math.max(props.maxDepth, maxHistoricalDepth);
  }
};

watch(
  () => props.sensorData,
  () => {
    historicalData.value.unshift({
      depth: props.currentDepth,
      confidence: props.confidence,
      maxDepth: props.maxDepth,
      minDepth: props.minDepth,
      accuracy: props.accuracy,
      virtualMaxDepth: virtualMaxDepth.value,
    });

    if (historicalData.value.length > props.columnCount) {
      historicalData.value.pop();
    }

    updateVirtualMaxDepth();
    drawOverlay();
  }
);

watch(
  () => historicalData.value,
  () => {
    updateVirtualMaxDepth();
  },
  { deep: true }
);

const depthTicks = computed(() => {
  const depthRange = virtualMaxDepth.value - props.minDepth;
  return Array.from(
    { length: props.tickCount },
    (_, i) => props.minDepth + (i / (props.tickCount - 1)) * depthRange
  );
});

const arrowPosition = computed(() => {
  const depthRange = virtualMaxDepth.value - props.minDepth;
  const relativeDepth = props.currentDepth - props.minDepth;
  return (relativeDepth / depthRange) * 100;
});

const tickPosition = (depth) => {
  const depthRange = virtualMaxDepth.value - props.minDepth;
  const relativeDepth = depth - props.minDepth;
  return (relativeDepth / depthRange) * 100;
};

const handleMouseMove = (event) => {
  const rect = event.target.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const columnWidth = rect.width / props.columnCount;
  hoveredColumn.value = props.columnCount - 1 - Math.floor(x / columnWidth);
  drawOverlay();
};

const handleMouseLeave = () => {
  hoveredColumn.value = null;
  drawOverlay();
};

const drawOverlay = () => {
  if (!ctx.value) return;

  ctx.value.clearRect(0, 0, overlayCanvas.value.width, overlayCanvas.value.height);

  if (hoveredColumn.value !== null) {
    const columnWidth = overlayCanvas.value.width / props.columnCount;
    const x = (props.columnCount - 1 - hoveredColumn.value) * columnWidth;

    ctx.value.strokeStyle = 'white';
    ctx.value.lineWidth = 2;
    ctx.value.strokeRect(x, 0, columnWidth, overlayCanvas.value.height);

    if (historicalData.value[hoveredColumn.value]) {
      const columnData = historicalData.value[hoveredColumn.value];
      const y =
        ((columnData.depth - props.minDepth) / (virtualMaxDepth.value - props.minDepth)) *
        overlayCanvas.value.height;

      ctx.value.fillStyle = 'rgba(255, 0, 0, 0.5)';
      ctx.value.fillRect(x, y - 5, columnWidth, 10);
      ctx.value.strokeStyle = 'red';
      ctx.value.strokeRect(x, y - 5, columnWidth, 10);
    }
  }
};

const resizeOverlayCanvas = () => {
  if (overlayCanvas.value) {
    overlayCanvas.value.width = overlayCanvas.value.offsetWidth;
    overlayCanvas.value.height = overlayCanvas.value.offsetHeight;
    drawOverlay();
  }
};

onMounted(() => {
  ctx.value = overlayCanvas.value.getContext('2d');
  resizeOverlayCanvas();
  window.addEventListener('resize', resizeOverlayCanvas);
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeOverlayCanvas);
});
</script>

<style scoped>
.waterfall-display {
	position: relative;
}

.selected-object-info {
	z-index: 10;
}
</style>