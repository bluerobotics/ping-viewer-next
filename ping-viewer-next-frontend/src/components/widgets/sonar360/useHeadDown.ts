import { useLocalStorage } from '@vueuse/core';

// Shared across all Ping360 widgets; persisted so the user's view preference
// survives reloads. Currently a single global flag; if per-device scoping
// becomes necessary, key this by deviceId via a Pinia store instead.
const headDown = useLocalStorage('sonar360.headDown', false);

export function useHeadDown() {
  return headDown;
}
