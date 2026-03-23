<template>
	<div class="relative w-full h-full p4">
		<div class="absolute inset-0">
			<slot></slot>
		</div>

		<svg class="absolute top-0 left-0 w-full h-full pointer-events-none" viewBox="0 0 100 100"
			preserveAspectRatio="xMidYMid meet">
			<defs>
				<linearGradient :id="sweepGradientId" gradientUnits="userSpaceOnUse"
					x1="50" y1="50"
					:x2="50 + maxRadius * Math.cos(adjustedAngleRad)"
					:y2="50 + maxRadius * Math.sin(adjustedAngleRad)">
					<stop offset="0%" stop-color="white" stop-opacity="0" />
					<stop offset="100%" stop-color="white" stop-opacity="0.9" />
				</linearGradient>
			</defs>

			<path v-if="!isFullCircle" :d="sectorPath" fill="none" :stroke="radiusLineColor"
				:stroke-width="radiusLineWidth" vector-effect="non-scaling-stroke" />

			<g v-if="showRadiusLines">
				<path v-for="line in radiusLines" :key="line.distance" :d="getRadiusLinePath(line.radius)"
					:stroke="radiusLineColor" :stroke-width="radiusLineWidth" fill="none"
					vector-effect="non-scaling-stroke" />
			</g>

			<line v-for="deg in angleLines" :key="deg"
				x1="50" y1="50"
				:x2="50 + maxRadius * Math.cos((deg - 90) * Math.PI / 180)"
				:y2="50 + maxRadius * Math.sin((deg - 90) * Math.PI / 180)"
				:stroke="radiusLineColor" :stroke-width="radiusLineWidth"
				vector-effect="non-scaling-stroke" />

			<line x1="50" y1="50"
				:x2="50 + maxRadius * Math.cos(adjustedAngleRad)"
				:y2="50 + maxRadius * Math.sin(adjustedAngleRad)"
				:stroke="`url(#${sweepGradientId})`" stroke-width="2"
				vector-effect="non-scaling-stroke" />
		</svg>

		<template v-if="showMarkers">
			<div v-for="line in radiusLines" :key="line.distance"
				class="absolute text-4xl font-medium text-white depth-label transform -translate-x-1/2 -translate-y-1/2" :style="{
					left: `calc(${getMarkerPositionPercent(line.radius).x}% + 50px)`,
					top: `${getMarkerPositionPercent(line.radius).y}%`,
				}">
				{{ depthValue(line.distance).toFixed(1) }}{{ depthUnit }}
			</div>
		</template>
	</div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useUnits } from '../../../composables/useUnits';

const instanceId = ref(Math.random().toString(36).slice(2, 8));
const sweepGradientId = computed(() => `sweep-grad-${instanceId.value}`);

const { depthValue, depthUnit } = useUnits();

const props = defineProps<{
  angle: number;
  lineColor: string;
  lineWidth: number;
  maxDistance: number;
  numMarkers: number;
  showRadiusLines: boolean;
  showMarkers: boolean;
  radiusLineColor: string;
  markerColor: string;
  radiusLineWidth: number;
  startAngle: number;
  endAngle: number;
  markerBackgroundColor: string;
}>();

const isFullCircle = computed(() => {
  return props.startAngle === 0 && props.endAngle === 360;
});

const angleLines = computed(() => {
  const all = [0, 45, 90, 135, 180, 225, 270, 315];
  if (isFullCircle.value) return all;
  return all.filter((deg) => {
    if (props.startAngle <= props.endAngle) {
      return deg >= props.startAngle && deg <= props.endAngle;
    }
    return deg >= props.startAngle || deg <= props.endAngle;
  });
});

const adjustedAngleRad = computed(() => {
  const normalizedAngle = ((props.angle + 1) / 400) * 360;
  return ((normalizedAngle + 90) * Math.PI) / 180;
});

const maxRadius = computed(() => 50 - props.lineWidth / 2);

const radiusLines = computed(() => {
  const lines = [];
  for (let i = 1; i <= props.numMarkers; i++) {
    const distance = (i / props.numMarkers) * props.maxDistance;
    const radius = (i / props.numMarkers) * (50 - props.lineWidth / 2);
    lines.push({ distance: Number.parseFloat(distance.toFixed(1)), radius });
  }
  return lines;
});

const sectorPath = computed(() => {
  if (isFullCircle.value) return '';

  const startRad = (props.startAngle - 90) * (Math.PI / 180);
  const endRad = (props.endAngle - 90) * (Math.PI / 180);
  const startX = 50 + 50 * Math.cos(startRad);
  const startY = 50 + 50 * Math.sin(startRad);
  const endX = 50 + 50 * Math.cos(endRad);
  const endY = 50 + 50 * Math.sin(endRad);

  return `M 50 50 L ${startX} ${startY} M 50 50 L ${endX} ${endY}`;
});

const getRadiusLinePath = (radius: number) => {
  if (isFullCircle.value) {
    return `M 50 ${50 - radius} A ${radius} ${radius} 0 1 1 50 ${
      50 + radius
    } A ${radius} ${radius} 0 1 1 50 ${50 - radius}`;
  }

  const startRad = (props.startAngle - 90) * (Math.PI / 180);
  const endRad = (props.endAngle - 90) * (Math.PI / 180);
  const startX = 50 + radius * Math.cos(startRad);
  const startY = 50 + radius * Math.sin(startRad);
  const endX = 50 + radius * Math.cos(endRad);
  const endY = 50 + radius * Math.sin(endRad);

  let largeArcFlag: number;
  let sweepFlag: number;
  if (props.startAngle <= props.endAngle) {
    largeArcFlag = props.endAngle - props.startAngle <= 180 ? 0 : 1;
    sweepFlag = 1;
  } else {
    largeArcFlag = 360 - props.startAngle + props.endAngle <= 180 ? 0 : 1;
    sweepFlag = 1;
  }

  return `M ${startX} ${startY} A ${radius} ${radius} 0 ${largeArcFlag} ${sweepFlag} ${endX} ${endY}`;
};

const getMarkerPositionPercent = (radius: number) => {
  const angle = 45;
  const rad = (angle - 90) * (Math.PI / 180);
  return {
    x: 50 + radius * Math.cos(rad),
    y: 50 + radius * Math.sin(rad),
  };
};
</script>

<style scoped>
.depth-label {
	-webkit-text-stroke: 1px rgba(0, 0, 0, 0.7);
	paint-order: stroke fill;
}
</style>
