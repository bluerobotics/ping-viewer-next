<template>
	<div ref="containerRef" class="w-full h-full flex items-center justify-center bg-transparent p-4">
		<div class="relative" :style="{ width: `${size}px`, height: `${size}px` }">
			<Sonar360Mask :angle="angle" :lineColor="lineColor" :lineWidth="lineWidth" :maxDistance="maxDistance"
				:numMarkers="numMarkers" :showRadiusLines="showRadiusLines" :showMarkers="showMarkers"
				:radiusLineColor="radiusLineColor" :markerColor="markerColor"
				:markerBackgroundColor="markerBackgroundColor" :radiusLineWidth="radiusLineWidth"
				:startAngle="startAngle" :endAngle="endAngle">
				<Sonar360Shader :measurement="measurement" :numLines="400" :lineLength="1202"
					:color-palette="colorPalette" :get-color-from-palette="getColorFromPalette" :startAngle="startAngle"
					:endAngle="endAngle" :yaw_angle="yaw_angle" :debug=false />
			</Sonar360Mask>
		</div>

		<div v-if="debug" class="absolute top-0 right-0 bg-black bg-opacity-50 text-white p-2 text-xs">
			<div>Angle: {{ angle }}</div>
			<div>Show Radius Lines: {{ showRadiusLines }}</div>
			<div>Show Markers: {{ showMarkers }}</div>
			<div>Radius Line Color: {{ radiusLineColor }}</div>
			<div>Marker Color: {{ markerColor }}</div>
			<div>Radius Line Width: {{ radiusLineWidth }}</div>
			<div>Num Markers: {{ numMarkers }}</div>
			<div>Max Distance: {{ maxDistance }}</div>
			<div>Yaw Angle: {{ yaw_angle.toFixed(1) }}°</div>
		</div>
	</div>
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { getColorFromPalette } from '../SonarColorOptions.vue';
import Sonar360Mask from './Sonar360Mask.vue';
import Sonar360Shader from './Sonar360Shader.vue';

const props = defineProps({
  measurement: {
    type: Object,
    default: null,
  },
  angle: {
    type: Number,
    required: true,
  },
  colorPalette: {
    type: String,
    required: true,
  },
  lineColor: {
    type: String,
    default: 'red',
  },
  lineWidth: {
    type: Number,
    default: 0.5,
  },
  maxDistance: {
    type: Number,
    default: 300,
  },
  numMarkers: {
    type: Number,
    default: 5,
  },
  showRadiusLines: {
    type: Boolean,
    default: true,
  },
  showMarkers: {
    type: Boolean,
    default: true,
  },
  radiusLineColor: {
    type: String,
    default: 'green',
  },
  markerColor: {
    type: String,
    default: 'green',
  },
  radiusLineWidth: {
    type: Number,
    default: 0.5,
  },
  debug: {
    type: Boolean,
    default: false,
  },
  startAngle: {
    type: Number,
    default: 0,
  },
  endAngle: {
    type: Number,
    default: 360,
  },
  yaw_angle: {
    type: Number,
    default: 0,
  },
  markerBackgroundColor: {
    type: String,
    default: 'rgba(0, 0, 0, 0.5)',
  },
});

const containerRef = ref(null);
const size = ref(300);

const updateSize = () => {
  if (containerRef.value) {
    const rect = containerRef.value.getBoundingClientRect();
    size.value = Math.min(rect.width, rect.height);
  }
};

onMounted(() => {
  updateSize();
  window.addEventListener('resize', updateSize);
});

onUnmounted(() => {
  window.removeEventListener('resize', updateSize);
});
</script>