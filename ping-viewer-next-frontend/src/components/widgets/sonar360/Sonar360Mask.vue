<template>
    <div class="relative w-full h-full">
      <div class="w-full h-full">
        <slot></slot>
      </div>
      <svg class="absolute top-0 left-0 w-full h-full pointer-events-none" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid meet">
        <defs>
          <mask id="circle-mask">
            <rect width="100%" height="100%" fill="black"/>
            <circle cx="50%" cy="50%" r="50%" fill="black" />
          </mask>
        </defs>
        <rect width="100%" height="100%" fill="rgba(0,0,0,0.5)" mask="url(#circle-mask)" />

        <g v-if="showRadiusLines || showMarkers" v-for="line in radiusLines" :key="line.distance">
          <circle
            v-if="showRadiusLines"
            cx="50%"
            cy="50%"
            :r="`${line.radius}%`"
            fill="none"
            :stroke="radiusLineColor"
            :stroke-width="radiusLineWidth"
          />
          <text
            v-if="showMarkers"
            :x="50 + '%'"
            :y="`${50 - line.radius - 1}%`"
            :fill="markerColor"
            font-size="3"
            text-anchor="middle"
            alignment-baseline="middle"
          >
            {{ line.distance }}
          </text>
        </g>

        <line
          x1="50%"
          y1="50%"
          :x2="`${50 + 50 * Math.cos(adjustedAngleRad)}%`"
          :y2="`${50 + 50 * Math.sin(adjustedAngleRad)}%`"
          :stroke="lineColor"
          :stroke-width="lineWidth"
        />
      </svg>
    </div>
  </template>

  <script setup lang="ts">
  import { computed } from 'vue';

  const props = withDefaults(defineProps<{
    angle: number;
    lineColor?: string;
    lineWidth?: number;
    maxDistance?: number;
    numMarkers?: number;
    showRadiusLines?: boolean;
    showMarkers?: boolean;
    radiusLineColor?: string;
    markerColor?: string;
    radiusLineWidth?: number;
  }>(), {
    lineColor: 'red',
    lineWidth: 0.5,
    maxDistance: 50,
    numMarkers: 5,
    showRadiusLines: true,
    showMarkers: true,
    radiusLineColor: 'rgba(255,255,255,0.3)',
    markerColor: 'yellow',
    radiusLineWidth: 0.5
  });

  const adjustedAngleRad = computed(() => {
    const normalizedAngle = (props.angle / 400) * 360;
    return ((normalizedAngle + 90) * Math.PI) / 180;
  });

  const radiusLines = computed(() => {
    const lines = [];
    for (let i = 1; i <= props.numMarkers; i++) {
      const distance = (i / props.numMarkers) * props.maxDistance;
      const radius = (i / props.numMarkers) * 50;
      lines.push({ distance: Math.round(distance), radius });
    }
    return lines;
  });
  </script>