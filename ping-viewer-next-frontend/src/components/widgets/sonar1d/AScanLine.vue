<template>
	<canvas ref="canvas" class="a-scan-line h-full" />
</template>

<script setup>
import { onMounted, onUnmounted, ref, watch } from 'vue';

const props = defineProps({
  sensorData: { type: Array, default: () => [] },
  maxDepth: { type: Number, required: true },
  minDepth: { type: Number, required: true },
  virtualMaxDepth: { type: Number, required: true },
  width: { type: Number, default: 50 },
});

const canvas = ref(null);
let ctx = null;

const draw = () => {
  if (!ctx || !canvas.value) return;

  const w = canvas.value.width;
  const h = canvas.value.height;
  const data = props.sensorData;

  ctx.clearRect(0, 0, w, h);

  if (!data || data.length === 0) return;

  const depthRange = props.virtualMaxDepth - props.minDepth;
  const dataRange = props.maxDepth - props.minDepth;
  const scaleFactor = dataRange / depthRange;
  const centerX = w / 2;
  const maxAmplitude = centerX - 2;

  ctx.beginPath();
  for (let i = 0; i < data.length; i++) {
    const normalizedDepth = (i / data.length) * scaleFactor;
    const y = normalizedDepth * h;
    if (y > h) break;
    const intensity = (data[i] ?? 0) / 255;
    const x = centerX + intensity * maxAmplitude;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  for (let i = data.length - 1; i >= 0; i--) {
    const normalizedDepth = (i / data.length) * scaleFactor;
    const y = normalizedDepth * h;
    if (y > h) continue;
    const intensity = (data[i] ?? 0) / 255;
    const x = centerX - intensity * maxAmplitude;
    ctx.lineTo(x, y);
  }
  ctx.closePath();

  const gradient = ctx.createLinearGradient(0, 0, w, 0);
  gradient.addColorStop(0, 'rgba(0, 220, 80, 0.15)');
  gradient.addColorStop(0.5, 'rgba(0, 220, 80, 0.4)');
  gradient.addColorStop(1, 'rgba(0, 220, 80, 0.15)');
  ctx.fillStyle = gradient;
  ctx.fill();

  ctx.strokeStyle = 'rgba(0, 220, 80, 0.8)';
  ctx.lineWidth = 1.5;

  ctx.beginPath();
  for (let i = 0; i < data.length; i++) {
    const normalizedDepth = (i / data.length) * scaleFactor;
    const y = normalizedDepth * h;
    if (y > h) break;
    const intensity = (data[i] ?? 0) / 255;
    const x = centerX + intensity * maxAmplitude;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  ctx.beginPath();
  for (let i = 0; i < data.length; i++) {
    const normalizedDepth = (i / data.length) * scaleFactor;
    const y = normalizedDepth * h;
    if (y > h) break;
    const intensity = (data[i] ?? 0) / 255;
    const x = centerX - intensity * maxAmplitude;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  ctx.beginPath();
  ctx.strokeStyle = 'rgba(0, 220, 80, 0.3)';
  ctx.lineWidth = 1;
  ctx.moveTo(centerX, 0);
  ctx.lineTo(centerX, h);
  ctx.stroke();
};

const resize = () => {
  if (!canvas.value) return;
  canvas.value.width = props.width;
  canvas.value.height = canvas.value.offsetHeight;
  draw();
};

watch(() => props.sensorData, draw);

onMounted(() => {
  ctx = canvas.value.getContext('2d');
  resize();
  window.addEventListener('resize', resize);
});

onUnmounted(() => {
  window.removeEventListener('resize', resize);
});
</script>

<style scoped>
.a-scan-line {
	width: 50px;
	flex-shrink: 0;
	background: rgba(0, 0, 0, 0.3);
}
</style>
