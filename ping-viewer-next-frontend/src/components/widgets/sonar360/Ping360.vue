<template>
  <div ref="containerRef" class="w-full h-full flex items-center justify-center bg-transparent overflow-hidden">
    <div class="relative shrink-0" :style="containerStyle">
			<Sonar360Mask :angle="angle" :lineColor="lineColor" :lineWidth="lineWidth" :maxDistance="maxDistance"
				:numMarkers="numMarkers" :showRadiusLines="showRadiusLines" :showMarkers="showMarkers"
				:radiusLineColor="radiusLineColor" :markerColor="markerColor"
				:markerBackgroundColor="markerBackgroundColor" :radiusLineWidth="radiusLineWidth"
				:startAngle="startAngle" :endAngle="endAngle">
			<Sonar360Shader :measurement="measurement" :numLines="400"
				:color-palette="colorPalette" :get-color-from-palette="getColorFromPalette" :startAngle="startAngle"
				:endAngle="endAngle" :yaw_angle="yaw_angle" :max-radius="shaderMaxRadius" :debug=false />
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
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { getColorFromPalette } from '../SonarColorOptions.vue';
import Sonar360Mask from './Sonar360Mask.vue';
import Sonar360Shader from './Sonar360Shader.vue';
import { useHeadDown } from './useHeadDown';

// When headDown is on we mirror the whole sonar container horizontally.
// `Sonar360Mask` applies a second `scaleX(-1)` to its depth markers so their
// text stays readable. Both flips must stay in sync.
const headDown = useHeadDown();

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
    default: 'rgba(255, 255, 255, 0.7)',
  },
  markerColor: {
    type: String,
    default: 'white',
  },
  radiusLineWidth: {
    type: Number,
    default: 1,
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

// Mirror of `Sonar360Mask`'s `maxRadius` (50 - lineWidth/2) but expressed in
// the shader's normalized 0..1 space, so the WebGL data ends exactly at the
// outermost depth arc instead of overshooting past it by half a stroke width.
const shaderMaxRadius = computed(() => (50 - props.lineWidth / 2) / 50);

const sectorWidth = computed(() => {
  const diff = props.endAngle - props.startAngle;
  return diff >= 0 ? diff : diff + 360;
});

const isHalfCircleView = computed(() => {
  if (sectorWidth.value <= 180) return false;
  return props.startAngle >= 270 && props.endAngle <= 90;
});

function isAngleInSector(angle, start, end) {
  if (start <= end) {
    return angle >= start && angle <= end;
  }
  return angle >= start || angle <= end;
}

const sectorBoundingBox = computed(() => {
  if (sectorWidth.value >= 360 || sectorWidth.value === 0) {
    return { minX: 0, maxX: 1, minY: 0, maxY: 1 };
  }

  const toRad = (angle) => ((angle - 90) * Math.PI) / 180;
  const points = [{ x: 0.5, y: 0.5 }];

  points.push({
    x: 0.5 + 0.5 * Math.cos(toRad(props.startAngle)),
    y: 0.5 + 0.5 * Math.sin(toRad(props.startAngle)),
  });
  points.push({
    x: 0.5 + 0.5 * Math.cos(toRad(props.endAngle)),
    y: 0.5 + 0.5 * Math.sin(toRad(props.endAngle)),
  });

  for (const cardinal of [0, 90, 180, 270]) {
    if (isAngleInSector(cardinal, props.startAngle, props.endAngle)) {
      points.push({
        x: 0.5 + 0.5 * Math.cos(toRad(cardinal)),
        y: 0.5 + 0.5 * Math.sin(toRad(cardinal)),
      });
    }
  }

  return {
    minX: Math.min(...points.map((p) => p.x)),
    maxX: Math.max(...points.map((p) => p.x)),
    minY: Math.min(...points.map((p) => p.y)),
    maxY: Math.max(...points.map((p) => p.y)),
  };
});

const buildTransform = (...parts) => parts.filter(Boolean).join(' ');

const containerStyle = computed(() => {
  const flip = headDown.value ? 'scaleX(-1)' : '';

  if (isHalfCircleView.value) {
    return {
      width: `${size.value}px`,
      height: `${size.value}px`,
      transform: buildTransform('translate(-50%, 48%)', flip),
      position: 'fixed',
      left: '50%',
      bottom: '0',
    };
  }

  const bb = sectorBoundingBox.value;
  const bbCenterX = (bb.minX + bb.maxX) / 2;
  const bbCenterY = (bb.minY + bb.maxY) / 2;
  const offsetX = (0.5 - bbCenterX) * size.value;
  const offsetY = (0.5 - bbCenterY) * size.value;

  const style = {
    width: `${size.value}px`,
    height: `${size.value}px`,
  };

  const translate = offsetX !== 0 || offsetY !== 0 ? `translate(${offsetX}px, ${offsetY}px)` : '';
  const transform = buildTransform(translate, flip);
  if (transform) {
    style.transform = transform;
  }

  return style;
});

const updateSize = () => {
  if (!containerRef.value) return;
  const rect = containerRef.value.getBoundingClientRect();
  if (isHalfCircleView.value) {
    size.value = Math.min(rect.width, rect.height * 2);
    return;
  }
  // Scale the sector proportionally (no distortion, no clipping) until the
  // tighter of the two axes hits the container edge. The looser axis may
  // have a small empty band when the sector's bbox aspect doesn't match
  // the viewport aspect — that gap is geometrically unavoidable without
  // either stretching the arc or clipping the sector ends.
  const bb = sectorBoundingBox.value;
  const bbWidth = Math.max(bb.maxX - bb.minX, 0.0001);
  const bbHeight = Math.max(bb.maxY - bb.minY, 0.0001);
  size.value = Math.min(rect.width / bbWidth, rect.height / bbHeight);
};

let resizeObserver = null;

onMounted(() => {
  updateSize();
  window.addEventListener('resize', updateSize);
  if (containerRef.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => updateSize());
    resizeObserver.observe(containerRef.value);
  }
});

onUnmounted(() => {
  window.removeEventListener('resize', updateSize);
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
});

watch([() => props.startAngle, () => props.endAngle], () => {
  updateSize();
});
</script>
