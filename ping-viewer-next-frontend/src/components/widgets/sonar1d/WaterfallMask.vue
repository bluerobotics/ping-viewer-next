<template>
	<div class="waterfall-display relative w-full h-full" :style="{ paddingRight: `${rightOffset}px` }">
		<WaterfallShader ref="waterfallShader" :width="width" :height="height" :max-depth="maxDepth"
			:min-depth="minDepth" :column-count="columnCount" :sensor-data="sensorData" :color-palette="colorPalette"
			:get-color-from-palette="getColorFromPalette" @update:columnCount="$emit('update:columnCount', $event)"
			@mousemove="handleMouseMove" @mouseleave="handleMouseLeave" />
		<canvas ref="overlayCanvas" class="absolute top-0 left-0 h-full pointer-events-none" :style="{ width: `calc(100% - ${rightOffset}px)` }"></canvas>
	<div class="depth-ruler absolute top-0 h-full pointer-events-none" :style="{
		right: showAScan ? '50px' : '0',
		width: `${RULER_WIDTH}px`,
	}">
		<template v-for="tick in depthTicks" :key="tick.value">
			<div class="absolute right-0 w-3 h-px bg-white/70" :style="{ top: `${tick.position}%` }"></div>
			<span class="absolute text-xl font-medium text-white whitespace-nowrap depth-label" :style="{
				right: '6px',
				top: `${tick.position}%`,
				transform: 'translateY(-100%)',
			}">{{ tick.label }}</span>
		</template>
	</div>
	<div v-if="hasValidMeasurement" class="absolute w-0 h-0 border-solid border-transparent border-l-[16px] border-y-[8px]" :style="{
		top: `${arrowPosition}%`,
		right: `${rightOffset}px`,
		borderLeftColor: depthArrowColor,
		transform: 'translateY(-50%)',
	}"/>
		<AScanLine
			v-if="showAScan"
			class="absolute top-0 right-0 h-full"
			:sensor-data="sensorData"
			:max-depth="maxDepth"
			:min-depth="minDepth"
			:virtual-max-depth="virtualMaxDepth"
			:width="50"
		/>

	<vue-draggable-resizable
		v-if="hasValidMeasurement"
		:x="boxPosition.x"
		:y="boxPosition.y"
		:w="boxPosition.w"
		:h="boxPosition.h"
		:min-width="130"
		:min-height="40"
		:parent="true"
		:resizable="true"
		:lock-aspect-ratio="true"
		:disableUserSelect="true"
		class="measurements-box glass-panel"
		@resizing="onResize"
		@dragging="onDrag"
		@dblclick="resetPosition"
	>
		<div class="measurements-content text-sm px-1 rounded" :style="{ fontSize: `${fontSize}px` }">
			<div class="text-left" :style="{ color: '#FFFFFF' }">
				Depth: {{ formatDepth(currentDepth) }}
			</div>
			<div class="text-left" :style="{ color: '#FFFFFF' }">
				Confidence: {{ confidence }}%
			</div>
		</div>
	</vue-draggable-resizable>

	<div v-if="hoveredColumn !== null && mousePosition && isValidColumnData(hoveredColumn)"
		class="hovered-column-info glass-panel px-2 py-1 rounded flex flex-col space-y-1 absolute" :style="{
			fontSize: `${fontSize}px`,
			...getHoveredBoxPosition()
		}">
		<div class="flex flex-col" :style="{ color: '#FFFFFF' }">
			<span :style="{ fontSize: `${fontSize * 0.4}px` }">Depth</span>
			<span>{{ formatDepth(historicalData[hoveredColumn]?.depth) }}</span>
		</div>
		<div class="flex flex-col" :style="{ color: '#FFFFFF' }">
			<span :style="{ fontSize: `${fontSize * 0.4}px` }">Confidence</span>
			<span>{{ historicalData[hoveredColumn]?.confidence }}%</span>
		</div>
	</div>
	</div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import VueDraggableResizable from 'vue-draggable-resizable';
import { useUnits } from '../../../composables/useUnits';
import AScanLine from './AScanLine.vue';
import WaterfallShader from './WaterfallShader.vue';

const { formatDepth, depthValue, depthUnit } = useUnits();

const RULER_WIDTH = 70;

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
  textBackground: { type: String, default: 'rgba(0, 0, 0, 0.8)' },
  depthArrowColor: { type: String, default: 'yellow' },
  tickCount: { type: Number, default: 5 },
  showAScan: { type: Boolean, default: true },
});

const emit = defineEmits(['update:columnCount']);

const rightOffset = computed(() => (props.showAScan ? 50 : 0));

const overlayCanvas = ref(null);
const ctx = ref(null);
const virtualMaxDepth = ref(props.maxDepth);
const hoveredColumn = ref(null);
const historicalData = ref([]);
const mousePosition = ref(null);
const containerHeight = ref(80);

const DEFAULT_BOX_SIZE = { w: 260, h: 80 };

function getDefaultPosition() {
  const container = document.querySelector('.waterfall-display');
  const containerWidth = container ? container.clientWidth : 600;
  return {
    x: Math.max(0, Math.floor((containerWidth - DEFAULT_BOX_SIZE.w) / 2)),
    y: 10,
    ...DEFAULT_BOX_SIZE,
  };
}

const boxPosition = ref(loadSavedPosition());

function loadSavedPosition() {
  const saved = localStorage.getItem('waterfall-measurements-position');
  const defaultPos = getDefaultPosition();
  const savedPosition = saved ? JSON.parse(saved) : defaultPos;
  const container = document.querySelector('.waterfall-display');
  if (!container) {
    return defaultPos;
  }

  const containerWidth = container.clientWidth;
  const containerHeight = container.clientHeight;

  const isOutOfBounds =
    savedPosition.x + savedPosition.w > containerWidth ||
    savedPosition.y + savedPosition.h > containerHeight ||
    savedPosition.x < 0 ||
    savedPosition.y < 0;

  return isOutOfBounds ? defaultPos : savedPosition;
}

function savePosition(position) {
  localStorage.setItem('waterfall-measurements-position', JSON.stringify(position));
}

function onDrag(x, y) {
  boxPosition.value.x = x;
  boxPosition.value.y = y;
  savePosition(boxPosition.value);
}

function onResize(x, y, width, height) {
  containerHeight.value = height;
  boxPosition.value = { x, y, w: width, h: height };
  savePosition(boxPosition.value);
}

function resetPosition() {
  boxPosition.value = getDefaultPosition();
  savePosition(boxPosition.value);
}

const hasValidMeasurement = computed(() => {
  return Number.isFinite(props.currentDepth) && Number.isFinite(props.confidence);
});

const isValidColumnData = (column) => {
  const data = historicalData.value[column];
  return data && Number.isFinite(data.depth) && Number.isFinite(data.confidence);
};

const fontSize = computed(() => {
  const scale = containerHeight.value / 42;
  return Math.floor(16 * scale);
});

const getHoveredBoxPosition = () => {
  if (!mousePosition.value) return {};
  const PADDING = 20;
  const left = mousePosition.value.x + PADDING;
  const top = mousePosition.value.y;
  return {
    left: `${left}px`,
    top: `${top}px`,
  };
};

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

const niceInterval = computed(() => {
  const minD = depthValue(props.minDepth);
  const maxD = depthValue(virtualMaxDepth.value);
  const range = maxD - minD;
  if (range <= 0) return 1;
  const roughInterval = range / 12;
  const mag = 10 ** Math.floor(Math.log10(roughInterval));
  const norm = roughInterval / mag;
  if (norm <= 1) return mag;
  if (norm <= 2) return 2 * mag;
  if (norm <= 5) return 5 * mag;
  return 10 * mag;
});

const depthTicks = computed(() => {
  const interval = niceInterval.value;
  const minD = depthValue(props.minDepth);
  const maxD = depthValue(virtualMaxDepth.value);
  const range = maxD - minD;
  if (range <= 0) return [];

  const ticks = [];
  const start = Math.ceil(minD / interval) * interval;
  for (let d = start; d <= maxD + interval * 0.01; d += interval) {
    if (d <= minD) continue;
    ticks.push({
      value: d,
      label: Math.abs(d - Math.round(d)) < 0.01 ? Math.round(d).toString() : d.toFixed(1),
      position: ((d - minD) / range) * 100,
    });
  }

  const lastTick = ticks.length > 0 ? ticks[ticks.length - 1] : null;
  if (!lastTick || maxD - lastTick.value > interval * 0.3) {
    ticks.push({
      value: maxD,
      label: maxD.toFixed(1),
      position: 100,
    });
  }

  return ticks;
});

const arrowPosition = computed(() => {
  const depthRange = virtualMaxDepth.value - props.minDepth;
  const relativeDepth = props.currentDepth - props.minDepth;
  return (relativeDepth / depthRange) * 100;
});

const handleMouseMove = (event) => {
  const rect = event.target.getBoundingClientRect();
  const x = event.clientX - rect.left;
  const y = event.clientY - rect.top;
  mousePosition.value = { x, y };

  const columnWidth = rect.width / props.columnCount;
  hoveredColumn.value = props.columnCount - 1 - Math.floor(x / columnWidth);
  drawOverlay();
};

const handleMouseLeave = () => {
  hoveredColumn.value = null;
  mousePosition.value = null;
  drawOverlay();
};

const drawOverlay = () => {
  if (!ctx.value) return;

  ctx.value.clearRect(0, 0, overlayCanvas.value.width, overlayCanvas.value.height);

  if (hoveredColumn.value !== null) {
    const columnWidth = overlayCanvas.value.width / props.columnCount;
    const x = (props.columnCount - 1 - hoveredColumn.value) * columnWidth;

    if (isValidColumnData(hoveredColumn.value)) {
      ctx.value.strokeStyle = 'white';
      ctx.value.lineWidth = 2;
      ctx.value.strokeRect(x, 0, columnWidth, overlayCanvas.value.height);

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

  // Add resize observer to handle container size changes
  const container = document.querySelector('.waterfall-display');
  if (container) {
    const observer = new ResizeObserver(() => {
      const newPosition = loadSavedPosition();
      boxPosition.value = newPosition;
      savePosition(newPosition);
    });
    observer.observe(container);
  }
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeOverlayCanvas);
});
</script>

<style>
@import "vue-draggable-resizable/style.css";
</style>

<style scoped>
.waterfall-display {
	position: relative;
	box-sizing: border-box;
}

.depth-ruler {
	z-index: 5;
}

.depth-label {
	-webkit-text-stroke: 1px rgba(0, 0, 0, 0.7);
	paint-order: stroke fill;
}

.measurements-box {
	position: absolute !important;
	top: 0;
	left: 0;
	cursor: move;
	border-radius: 8px;
}

.measurements-content {
	display: grid;
	line-height: 1.2;
	user-select: none;
}

:deep(.vdr) {
	border: 1px solid rgba(255, 255, 255, 0.15);
	border-radius: 8px;
	transition: border-color 0.2s ease;
}

:deep(.vdr:hover) {
	border-color: rgba(255, 255, 255, 0.3);
}

:deep(.handle) {
	background: none !important;
	border: none !important;
	width: 16px !important;
	height: 16px !important;
	opacity: 0;
	transition: opacity 0.2s ease;
}

:deep(.vdr:hover .handle) {
	opacity: 1;
}

:deep(.handle-tl) {
	top: -2px !important;
	left: -2px !important;
	border-top: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-left: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-radius: 6px 0 0 0 !important;
}

:deep(.handle-tr) {
	top: -2px !important;
	right: -2px !important;
	border-top: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-right: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-radius: 0 6px 0 0 !important;
}

:deep(.handle-bl) {
	bottom: -2px !important;
	left: -2px !important;
	border-bottom: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-left: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-radius: 0 0 0 6px !important;
}

:deep(.handle-br) {
	bottom: -2px !important;
	right: -2px !important;
	border-bottom: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-right: 2px solid rgba(255, 255, 255, 0.7) !important;
	border-radius: 0 0 6px 0 !important;
}

:deep(.handle-tl:hover),
:deep(.handle-tr:hover),
:deep(.handle-bl:hover),
:deep(.handle-br:hover) {
	filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.5));
}

:deep(.handle-tm),
:deep(.handle-bm),
:deep(.handle-ml),
:deep(.handle-mr) {
	display: none !important;
}

.shader-container {
	z-index: 10;
}

.glass-panel {
	background-color: rgba(0, 0, 0, 0.10);
	color: rgba(255, 255, 255, 1);
	border: 1px solid rgba(255, 255, 255, 0.15);
	backdrop-filter: blur(25px);
	-webkit-backdrop-filter: blur(16px);
	box-shadow: 0px 8px 8px 0px #00000033, 0px 8px 12px 6px #00000016;
}
</style>