<template>
  <div class="waterfall-display relative w-full h-full">
    <WaterfallShader
      :width="width"
      :height="height"
      :max-depth="maxDepth"
      :min-depth="minDepth"
      :column-count="columnCount"
      :sensor-data="sensorData"
      :color-palette="colorPalette"
      @update:columnCount="$emit('update:columnCount', $event)"
    />
    <div class="depth-line absolute top-0 right-0 h-full w-px" :style="{ backgroundColor: depthLineColor }">
      <div
        v-for="tick in depthTicks"
        :key="tick"
        class="tick absolute right-0 w-2 h-px"
        :style="{ top: `${tickPosition(tick)}%`, backgroundColor: depthLineColor }"
      >
        <span
          class="absolute right-3 transform -translate-y-1/2 text-xs px-1 rounded"
          :style="{ color: depthTextColor, backgroundColor: textBackground }"
        >
          {{ tick.toFixed(1) }}m
        </span>
      </div>
    </div>
    <div
      class="depth-arrow absolute right-0 w-0 h-0 border-solid border-transparent border-l-8"
      :style="{ top: `${arrowPosition}%`, borderLeftColor: depthArrowColor, transform: 'translateY(-50%)' }"
    ></div>
    <div
      class="current-depth absolute top-2 left-2 text-sm px-1 rounded"
      :style="{ color: currentDepthColor, backgroundColor: textBackground }"
    >
      Depth: {{ currentDepth.toFixed(2) }}m ± {{ accuracy.toFixed(2) }}m
    </div>
    <div
      class="confidence absolute top-8 left-2 text-sm px-1 rounded"
      :style="{ color: confidenceColor, backgroundColor: textBackground }"
    >
      Confidence: {{ confidence }}%
    </div>
  </div>
</template>

<script>
import { computed } from "vue";
import WaterfallShader from "./WaterfallShader.vue";

export default {
	name: "WaterfallDisplay",
	components: {
		WaterfallShader,
	},
	props: {
		width: { type: Number, default: 500 },
		height: { type: Number, default: 400 },
		maxDepth: { type: Number, default: 200 },
		minDepth: { type: Number, default: 0 },
		columnCount: { type: Number, default: 200 },
		sensorData: { type: Array, default: () => [] },
		colorPalette: { type: String, default: "ocean" },
		currentDepth: { type: Number, default: 0 },
		accuracy: { type: Number, default: 0 },
		confidence: { type: Number, default: 0 },
		depthLineColor: { type: String, default: "#FFFFFF" },
		depthTextColor: { type: String, default: "#FFFFFF" },
		currentDepthColor: { type: String, default: "#FFFF00" },
		confidenceColor: { type: String, default: "#00FF00" },
		textBackground: { type: String, default: "rgba(0, 0, 0, 0.5)" },
		depthArrowColor: { type: String, default: "#FF0000" },
		tickCount: { type: Number, default: 5 },
	},
	emits: ["update:columnCount"],
	setup(props) {
		const depthTicks = computed(() => {
			const depthRange = props.maxDepth - props.minDepth;
			return Array.from(
				{ length: props.tickCount },
				(_, i) => props.minDepth + (i / (props.tickCount - 1)) * depthRange,
			);
		});

		const arrowPosition = computed(() => {
			const depthRange = props.maxDepth - props.minDepth;
			const relativeDepth = props.currentDepth - props.minDepth;
			return (relativeDepth / depthRange) * 100;
		});

		const tickPosition = (depth) => {
			const depthRange = props.maxDepth - props.minDepth;
			const relativeDepth = depth - props.minDepth;
			return (relativeDepth / depthRange) * 100;
		};

		return {
			depthTicks,
			arrowPosition,
			tickPosition,
		};
	},
};
</script>