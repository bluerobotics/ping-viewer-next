<template>
  <div class="mb-4">
    <v-list-item>
      <v-list-item-title class="text-subtitle-2 mb-2">Color Palette</v-list-item-title>

      <div class="mt-2">
        <v-select v-model="selectedPalette" :items="Object.keys(allPalettes)" hide-details density="compact"
          class="mb-2" @update:model-value="updateColorPalette" />

        <v-list-item-title class="text-subtitle-2 mb-2">Gradient Preview</v-list-item-title>
        <div class="mt-2">
          <div class="h-8 w-full rounded-md overflow-hidden" :style="gradientPreviewStyle"></div>
        </div>
      </div>

      <div v-if="selectedPalette === 'Custom'" class="mt-4">
        <v-list-item-title class="text-subtitle-2 mb-2 d-flex justify-space-between align-center">
          <span>Custom Palette Points</span>
          <v-btn size="small" color="primary" @click="addColorPoint" :disabled="customPalette.length >= 10"
            density="compact">
            Add Point
          </v-btn>
        </v-list-item-title>

        <TransitionGroup name="point-list" tag="div">
          <div v-for="(point, index) in customPalette" :key="index"
            class="mb-2 pa-2 bg-surface rounded-md flex items-center gap-4 point-item">
            <div class="flex flex-col items-start" style="min-width: 120px;">
              <label class="text-caption mb-1">Position</label>
              <v-text-field v-model.number="point.pos" type="number" min="0" max="1" step="0.1" hide-details
                density="compact" class="flex-grow-0" style="width: 100%;" @input="debouncedValidateAndUpdate(index)" />
            </div>

            <div class="flex flex-col items-start" style="min-width: 120px;">
              <label class="text-caption mb-1">Transparency</label>
              <v-text-field v-model.number="point.color[3]" type="number" min="0" max="255" hide-details
                density="compact" class="flex-grow-0" style="width: 100%;" @input="debouncedValidateAndUpdate(index)" />
            </div>

            <input type="color" :value="rgbaToHex(point.color)" @input="(e) => updatePointColor(index, e.target.value)"
              class="w-10 h-8 rounded cursor-pointer flex-shrink-0" />

            <v-btn icon="mdi-delete" size="small" color="error" variant="text" @click="removeColorPoint(index)"
              :disabled="customPalette.length <= 2" density="compact" class="flex-shrink-0" />
          </div>
        </TransitionGroup>
      </div>
    </v-list-item>
  </div>
</template>

<script>
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue';

export const colorPalettes = {
  Transparent: [
    { pos: 0, color: [255, 255, 255, 0] },
    { pos: 1, color: [255, 255, 255, 255] },
  ],
  Heatmap: [
    { pos: 0, color: [0, 0, 0, 255] },
    { pos: 0.25, color: [255, 0, 0, 255] },
    { pos: 0.5, color: [255, 255, 0, 255] },
    { pos: 0.75, color: [255, 255, 255, 255] },
    { pos: 1, color: [255, 255, 255, 255] },
  ],
  Grayscale: [
    { pos: 0, color: [0, 0, 0, 255] },
    { pos: 1, color: [255, 255, 255, 255] },
  ],
  Ocean: [
    { pos: 0, color: [0, 0, 60, 255] },
    { pos: 0.5, color: [0, 63, 255, 255] },
    { pos: 1, color: [0, 255, 255, 255] },
  ],
  'Thermal Blue': [
    { pos: 0, color: [5, 34, 95, 255] },
    { pos: 0.25, color: [106, 168, 79, 255] },
    { pos: 0.5, color: [255, 255, 0, 255] },
    { pos: 0.75, color: [127, 96, 0, 255] },
    { pos: 1, color: [92, 15, 8, 255] },
  ],
  'Thermal Black': [
    { pos: 0, color: [0, 0, 0, 255] },
    { pos: 0.25, color: [106, 168, 79, 255] },
    { pos: 0.5, color: [255, 255, 0, 255] },
    { pos: 0.75, color: [127, 96, 0, 255] },
    { pos: 1, color: [92, 15, 8, 255] },
  ],
  'Thermal White': [
    { pos: 0, color: [255, 255, 255, 255] },
    { pos: 0.25, color: [106, 168, 79, 255] },
    { pos: 0.5, color: [255, 255, 0, 255] },
    { pos: 0.75, color: [127, 96, 0, 255] },
    { pos: 1, color: [92, 15, 8, 255] },
  ],
  'Monochrome Black': [
    { pos: 0, color: [0, 0, 0, 255] },
    { pos: 1, color: [255, 255, 255, 255] },
  ],
  'Monochrome White': [
    { pos: 0, color: [255, 255, 255, 255] },
    { pos: 1, color: [0, 0, 0, 255] },
  ],
  'Monochrome Sepia': [
    { pos: 0, color: [48, 33, 19, 255] },
    { pos: 1, color: [232, 201, 67, 255] },
  ],
  Custom: [],
};

export const getColorFromPalette = (value, palette) => {
  const intensity = value / 255;
  const gradient = colorPalettes[palette] || colorPalettes['Thermal Blue'];

  for (let i = 1; i < gradient.length; i++) {
    if (intensity <= gradient[i].pos) {
      const t = (intensity - gradient[i - 1].pos) / (gradient[i].pos - gradient[i - 1].pos);
      const c1 = gradient[i - 1].color;
      const c2 = gradient[i].color;
      return [
        Math.round(c1[0] + t * (c2[0] - c1[0])),
        Math.round(c1[1] + t * (c2[1] - c1[1])),
        Math.round(c1[2] + t * (c2[2] - c1[2])),
        Math.round(c1[3] + t * (c2[3] - c1[3])),
      ];
    }
  }
  return gradient[gradient.length - 1].color;
};

export default {
  name: 'SonarColorOptions',
  props: {
    initialPalette: {
      type: String,
      default: 'Ocean',
    },
  },
  emits: ['update:colorPalette'],

  setup(props, { emit }) {
    const selectedPalette = ref(props.initialPalette);
    const customPalette = shallowRef([
      { pos: 0, color: [0, 0, 0, 255] },
      { pos: 1, color: [255, 255, 255, 255] },
    ]);

    const colorCache = new Map();
    const gradientCache = new Map();
    let updateTimeout = null;

    onMounted(() => {
      try {
        const savedCustomPalette = localStorage.getItem('customColorPalette');
        if (savedCustomPalette) {
          customPalette.value = JSON.parse(savedCustomPalette);
        }
      } catch (error) {
        console.error('Error loading custom palette:', error);
      }
    });

    const allPalettes = computed(() => ({
      ...colorPalettes,
      Custom: customPalette.value,
    }));

    const gradientPreviewStyle = computed(() => {
      const palette =
        selectedPalette.value === 'Custom'
          ? customPalette.value
          : colorPalettes[selectedPalette.value];

      const cacheKey = JSON.stringify(palette);
      if (gradientCache.has(cacheKey)) {
        return gradientCache.get(cacheKey);
      }

      const stops = palette
        .map((point) => {
          const [r, g, b, a] = point.color;
          return `rgba(${r}, ${g}, ${b}, ${a / 255}) ${point.pos * 100}%`;
        })
        .join(', ');

      const style = {
        background: `linear-gradient(to right, ${stops})`,
      };

      gradientCache.set(cacheKey, style);
      return style;
    });

    const debouncedUpdate = (fn) => {
      if (updateTimeout) {
        clearTimeout(updateTimeout);
      }
      updateTimeout = setTimeout(fn, 100);
    };

    const updateColorPalette = () => {
      debouncedUpdate(() => {
        if (selectedPalette.value === 'Custom') {
          colorPalettes.Custom = customPalette.value;
        }
        emit('update:colorPalette', selectedPalette.value);
      });
    };

    const validateAndUpdatePoint = (index) => {
      const point = customPalette.value[index];
      point.pos = Math.max(0, Math.min(1, point.pos));
      point.color[3] = Math.max(0, Math.min(255, point.color[3]));

      customPalette.value.sort((a, b) => a.pos - b.pos);

      gradientCache.clear();

      debouncedUpdate(() => {
        localStorage.setItem('customColorPalette', JSON.stringify(customPalette.value));
        updateColorPalette();
      });
    };

    const debouncedValidateAndUpdate = (index) => {
      debouncedUpdate(() => validateAndUpdatePoint(index));
    };

    const addColorPoint = () => {
      if (customPalette.value.length >= 10) return;

      const lastPoint = customPalette.value[customPalette.value.length - 1];
      const newPos = Math.min(1, lastPoint.pos + 0.1);

      customPalette.value = [...customPalette.value, { pos: newPos, color: [255, 255, 255, 255] }];

      validateAndUpdatePoint(customPalette.value.length - 1);
    };

    const removeColorPoint = (index) => {
      if (customPalette.value.length <= 2) return;

      customPalette.value = customPalette.value.filter((_, i) => i !== index);

      debouncedUpdate(() => {
        localStorage.setItem('customColorPalette', JSON.stringify(customPalette.value));
        updateColorPalette();
      });
    };

    const rgbaToHex = (rgba) => {
      const key = rgba.join(',');
      if (colorCache.has(key)) {
        return colorCache.get(key);
      }
      const [r, g, b] = rgba;
      const hex = `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
      colorCache.set(key, hex);
      return hex;
    };

    const hexToRgba = (hex) => {
      const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
      return result
        ? [
            Number.parseInt(result[1], 16),
            Number.parseInt(result[2], 16),
            Number.parseInt(result[3], 16),
            255,
          ]
        : null;
    };

    const updatePointColor = (index, hexColor) => {
      const rgba = hexToRgba(hexColor);
      if (rgba) {
        requestAnimationFrame(() => {
          rgba[3] = customPalette.value[index].color[3];
          customPalette.value[index].color = rgba;
          validateAndUpdatePoint(index);
        });
      }
    };

    watch(
      () => props.initialPalette,
      (newPalette) => {
        if (newPalette && colorPalettes[newPalette]) {
          selectedPalette.value = newPalette;
        }
      }
    );

    onUnmounted(() => {
      colorCache.clear();
      gradientCache.clear();
      if (updateTimeout) {
        clearTimeout(updateTimeout);
      }
    });

    return {
      allPalettes,
      selectedPalette,
      customPalette,
      gradientPreviewStyle,
      updateColorPalette,
      addColorPoint,
      removeColorPoint,
      validateAndUpdatePoint,
      debouncedValidateAndUpdate,
      updatePointColor,
      rgbaToHex,
    };
  },
};
</script>