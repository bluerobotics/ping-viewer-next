<template>
  <div class="w-full h-full">
    <div class="relative w-full pb-[100%]">
      <div class="absolute inset-0">
        <template v-if="useMask">
          <Sonar360Mask
            :angle="currentMeasurement?.angle ?? 0"
            :lineColor="lineColor"
            :lineWidth="lineWidth"
            :maxDistance="maxDistance"
            :numMarkers="numMarkers"
            :showRadiusLines="showRadiusLines"
            :showMarkers="showMarkers"
            :radiusLineColor="radiusLineColor"
            :markerColor="markerColor"
            :radiusLineWidth="radiusLineWidth"
          >
            <Sonar360Shader
              :measurement="currentMeasurement"
              :numLines="400"
              :lineLength="1200"
            />
          </Sonar360Mask>
        </template>
        <template v-else>
          <Sonar360Shader
            :measurement="currentMeasurement"
            :numLines="400"
            :lineLength="1200"
          />
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import Sonar360Mask from './Sonar360Mask.vue';
import Sonar360Shader from './Sonar360Shader.vue';

interface SonarMeasurement {
  angle: number;
  data: Uint8Array;
}

const props = withDefaults(defineProps<{
  measurement: SonarMeasurement | null;
  useMask?: boolean;
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
  measurement: null,
  useMask: true,
  lineColor: 'red',
  lineWidth: 0.5,
  maxDistance: 300,
  numMarkers: 5,
  showRadiusLines: true,
  showMarkers: true,
  radiusLineColor: 'rgba(255,255,255,0.3)',
  markerColor: 'white',
  radiusLineWidth: 0.5
});

const currentMeasurement = ref<SonarMeasurement | null>(null);

watch(() => props.measurement, (newMeasurement) => {
  if (newMeasurement) {
    currentMeasurement.value = newMeasurement;
  }
});
</script>