<template>
	<div class="relative w-full h-full p4">
		<div class="absolute inset-0">
			<slot></slot>
		</div>

		<svg class="absolute top-0 left-0 w-full h-full pointer-events-none" viewBox="0 0 100 100"
			preserveAspectRatio="xMidYMid meet">
			<path v-if="!isFullCircle" :d="sectorPath" fill="none" :stroke="radiusLineColor" :stroke-width="radiusLineWidth" />

			<g v-if="showRadiusLines">
				<path v-for="line in radiusLines" :key="line.distance" :d="getRadiusLinePath(line.radius)"
					:stroke="radiusLineColor" :stroke-width="radiusLineWidth" fill="none" />
			</g>

			<line x1="50" y1="50" :x2="50 + 50 * Math.cos(adjustedAngleRad)" :y2="50 + 50 * Math.sin(adjustedAngleRad)"
				:stroke="lineColor" :stroke-width="lineWidth" />
		</svg>

		<template v-if="showMarkers">
			<div v-for="line in radiusLines" :key="line.distance"
				class="absolute text-xs px-1 rounded transform -translate-x-1/2 -translate-y-1/2" :style="{
					left: `${getMarkerPositionPercent(line.radius).x}%`,
					top: `${getMarkerPositionPercent(line.radius).y}%`,
					color: markerColor,
					backgroundColor: markerBackgroundColor
				}">
				{{ line.distance }}m
			</div>
		</template>
	</div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

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

const adjustedAngleRad = computed(() => {
  const normalizedAngle = (props.angle / 400) * 360;
  return ((normalizedAngle + 90) * Math.PI) / 180;
});

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

  const angleDifference = (props.endAngle - props.startAngle + 360) % 360;
  const largeArcFlag = angleDifference > 180 ? 1 : 0;
  const sweepFlag = 1;

  return `M 50 50 L ${startX} ${startY} A 50 50 0 ${largeArcFlag} ${sweepFlag} ${endX} ${endY} Z`;
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
  let angle: number;
  if (isFullCircle.value) {
    angle = 0;
  } else if (props.startAngle <= props.endAngle) {
    angle = (props.startAngle + props.endAngle) / 2;
  } else {
    angle = (props.startAngle + props.endAngle + 360) / 2;
    if (angle >= 360) angle -= 360;
  }
  const rad = (angle - 90) * (Math.PI / 180);
  return {
    x: 50 + radius * Math.cos(rad),
    y: 50 + radius * Math.sin(rad),
  };
};
</script>
