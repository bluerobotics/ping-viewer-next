import { computed, ref, watch } from 'vue';
import { Measure, meters, feet } from 'safe-units';

const STORAGE_KEY = 'unitSystem';

const unitSystem = ref(localStorage.getItem(STORAGE_KEY) || 'metric');

watch(unitSystem, (value) => {
  localStorage.setItem(STORAGE_KEY, value);
});

const feetPerMeter = Measure.of(1, meters).valueIn(feet);

function convertFromMeters(valueInMeters) {
  if (unitSystem.value === 'imperial') {
    return valueInMeters * feetPerMeter;
  }
  return valueInMeters;
}

function convertSpeedFromMps(valueInMps) {
  if (unitSystem.value === 'imperial') {
    return valueInMps * feetPerMeter;
  }
  return valueInMps;
}

export function useUnits() {
  const isImperial = computed(() => unitSystem.value === 'imperial');
  const depthUnit = computed(() => (isImperial.value ? 'ft' : 'm'));
  const speedUnit = computed(() => (isImperial.value ? 'ft/s' : 'm/s'));
  const distanceLabel = computed(() => (isImperial.value ? 'feet' : 'meters'));

  function formatDepth(valueInMeters, decimals = 2) {
    const converted = convertFromMeters(valueInMeters);
    return `${converted.toFixed(decimals)}${depthUnit.value}`;
  }

  function formatSpeed(valueInMps, decimals = 0) {
    const converted = convertSpeedFromMps(valueInMps);
    return `${converted.toFixed(decimals)} ${speedUnit.value}`;
  }

  function depthValue(valueInMeters) {
    return convertFromMeters(valueInMeters);
  }

  function depthToMeters(valueInDisplayUnit) {
    if (unitSystem.value === 'imperial') {
      return valueInDisplayUnit / feetPerMeter;
    }
    return valueInDisplayUnit;
  }

  return {
    unitSystem,
    isImperial,
    depthUnit,
    speedUnit,
    distanceLabel,
    formatDepth,
    formatSpeed,
    depthValue,
    depthToMeters,
  };
}
