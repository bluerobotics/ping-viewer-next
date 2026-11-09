import { listenToDatalakeVariable } from '@bluerobotics/cockpit-api';
import { Measure, meters, feet, nauticalMiles } from 'safe-units';
import { computed, ref, watch } from 'vue';

const STORAGE_KEY = 'unitSystem';

const DISTANCE_UNITS = {
  m: { label: 'meters', metersPerUnit: 1 },
  ft: { label: 'feet', metersPerUnit: Measure.of(1, feet).valueIn(meters) },
  nmi: { label: 'nautical miles', metersPerUnit: Measure.of(1, nauticalMiles).valueIn(meters) },
};

const DEPTH_UNITS = ['m', 'ft'];

const metricFallback = {
  distance: 'm',
  depth: 'm',
};

const imperialFallback = {
  distance: 'ft',
  depth: 'ft',
};

const storedUnitSystem = ref(localStorage.getItem(STORAGE_KEY) || 'metric');

// Null until Cockpit sends its own choice, so the stored preference above reads until then.
const cockpitDistanceUnit = ref(null);
const cockpitDepthUnit = ref(null);

const fallbackUnits = computed(() =>
  storedUnitSystem.value === 'imperial' ? imperialFallback : metricFallback
);

const distanceUnit = computed(() => cockpitDistanceUnit.value ?? fallbackUnits.value.distance);
const depthUnit = computed(() => cockpitDepthUnit.value ?? fallbackUnits.value.depth);

const isImperial = computed(() => depthUnit.value === 'ft');
const speedUnit = computed(() => (isImperial.value ? 'ft/s' : 'm/s'));

const distanceLabel = computed(() => DISTANCE_UNITS[distanceUnit.value].label);
const depthLabel = computed(() => DISTANCE_UNITS[depthUnit.value].label);

watch(storedUnitSystem, (value) => {
  localStorage.setItem(STORAGE_KEY, value);
});

// Inside Cockpit the widget follows the vehicle operator's units, which Cockpit only sends when
// they change. It publishes a unit per quantity, so nautical and custom preferences still read
// correctly. A unit the widget cannot render falls back to meters.
if (window.parent !== window) {
  listenToDatalakeVariable('UNITS/smallDistance', (value) => {
    if (DISTANCE_UNITS[value] === undefined) return;
    cockpitDistanceUnit.value = value;
  });

  listenToDatalakeVariable('UNITS/depth', (value) => {
    if (!DEPTH_UNITS.includes(value)) return;
    cockpitDepthUnit.value = value;
  });
}

function convertFromMeters(valueInMeters, unit) {
  return valueInMeters / DISTANCE_UNITS[unit].metersPerUnit;
}

// Nautical miles are small enough that a range would round to 0 at the decimals the caller asked
// for, so those keep just enough places to stay readable.
function precisionFor(value, decimals) {
  if (!Number.isFinite(value) || value === 0) return decimals;
  let places = decimals;
  while (places < 6 && Number(value.toFixed(places)) === 0) {
    places += 1;
  }
  return places;
}

function formatMeters(valueInMeters, unit, decimals) {
  const converted = convertFromMeters(valueInMeters, unit);
  return `${converted.toFixed(precisionFor(converted, decimals))}${unit}`;
}

function convertSpeedFromMps(valueInMps) {
  if (isImperial.value) {
    return valueInMps * Measure.of(1, meters).valueIn(feet);
  }
  return valueInMps;
}

export function useUnits() {
  function formatDistance(valueInMeters, decimals = 2) {
    return formatMeters(valueInMeters, distanceUnit.value, decimals);
  }

  function formatDepth(valueInMeters, decimals = 2) {
    return formatMeters(valueInMeters, depthUnit.value, decimals);
  }

  function formatSpeed(valueInMps, decimals = 0) {
    const converted = convertSpeedFromMps(valueInMps);
    return `${converted.toFixed(decimals)} ${speedUnit.value}`;
  }

  function distanceValue(valueInMeters) {
    return convertFromMeters(valueInMeters, distanceUnit.value);
  }

  function depthValue(valueInMeters) {
    return convertFromMeters(valueInMeters, depthUnit.value);
  }

  function depthToMeters(valueInDisplayUnit) {
    return valueInDisplayUnit * DISTANCE_UNITS[depthUnit.value].metersPerUnit;
  }

  return {
    storedUnitSystem,
    distanceUnit,
    depthUnit,
    isImperial,
    speedUnit,
    distanceLabel,
    depthLabel,
    formatDistance,
    formatDepth,
    formatSpeed,
    distanceValue,
    depthValue,
    depthToMeters,
  };
}
