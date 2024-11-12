<template>
  <v-list-item>
    <div class="d-flex align-center justify-space-between">
      <div class="d-flex align-center">
        <v-menu location="bottom end" :close-on-content-click="false">
          <template v-slot:activator="{ props }">
            <div class="d-flex align-center">
              <v-btn v-bind="props" size="small" :style="colorPreviewStyle">
                <v-icon size="small" color="white">mdi-palette</v-icon>
              </v-btn>
            </div>
          </template>

          <v-card min-width="300" class="pa-2">
            <v-card-text>
              <div class="d-flex justify-space-between mb-2">
                <v-btn-group variant="outlined" size="small">
                  <v-btn :color="showCanvas ? 'primary' : undefined" @click="showCanvas = !showCanvas"
                    prepend-icon="mdi-palette" size="x-small">
                    Canvas
                  </v-btn>
                  <v-btn :color="showInputs ? 'primary' : undefined" @click="showInputs = !showInputs"
                    prepend-icon="mdi-pencil" size="x-small">
                    Inputs
                  </v-btn>
                </v-btn-group>

                <v-select v-model="currentMode" :items="modes" density="compact" hide-details class="mode-select ml-2"
                  style="max-width: 100px;" />
              </div>

              <v-color-picker v-model="currentColor" :hide-canvas="!showCanvas" :hide-inputs="!showInputs"
                :mode="currentMode" elevation="0" :swatches="swatches" show-swatches></v-color-picker>

              <div class="d-flex justify-end mt-2">
                <v-btn size="small" color="error" variant="text" class="mr-2" @click="resetColor">
                  Reset
                </v-btn>
                <v-btn size="small" color="primary" variant="text" @click="applyColor">
                  Apply
                </v-btn>
              </div>
            </v-card-text>
          </v-card>
        </v-menu>
      </div>
    </div>
  </v-list-item>
</template>

<script setup>
import { computed, ref, watch } from 'vue';

const props = defineProps({
  modelValue: {
    type: String,
    required: true,
  },
  defaultValue: {
    type: String,
    default: '#000000',
  },
});

const emit = defineEmits(['update:modelValue']);

const showCanvas = ref(false);
const showInputs = ref(false);
const currentColor = ref(props.modelValue);
const currentMode = ref('hex');

const swatches = [
  ['#FF0000', '#FF4500', '#FF8C00', '#FFD700', '#FFFF00', '#9ACD32'],
  ['#00FF00', '#32CD32', '#00FA9A', '#00FFFF', '#00BFFF', '#0000FF'],
  ['#8A2BE2', '#9932CC', '#FF00FF', '#FF69B4', '#FFC0CB', '#FFFFFF'],
  ['#D3D3D3', '#A9A9A9', '#808080', '#696969', '#000000', '#8B4513'],
];

const modes = ['hex', 'hexa', 'rgba', 'hsla'];

const colorPreviewStyle = computed(() => ({
  backgroundColor: currentColor.value || props.modelValue,
  borderColor: currentColor.value || props.modelValue,
  width: '32px',
  height: '32px',
}));

watch(
  () => props.modelValue,
  (newValue) => {
    currentColor.value = newValue;
  }
);

const applyColor = () => {
  emit('update:modelValue', currentColor.value);
};

const resetColor = () => {
  currentColor.value = props.defaultValue;
  emit('update:modelValue', props.defaultValue);
};
</script>

<style scoped>
.mode-select :deep(.v-field__input) {
  min-height: 32px;
  padding-top: 0;
  padding-bottom: 0;
}

.mode-select :deep(.v-field__append-inner) {
  padding-top: 4px;
}

:deep(.v-color-picker) {
  max-width: none;
}

:deep(.v-color-picker__controls) {
  padding: 0;
}
</style>