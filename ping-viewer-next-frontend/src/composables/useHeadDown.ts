import { updateViewerSettings, viewerSettings } from '@/composables/useViewerSettings';
import { ref, watch } from 'vue';

// Shared across all Ping360 widgets. The service stores the value; this module
// mirrors it so the sonar view can flip without each widget talking to the API.
const headDown = ref(false);
// Last value known to the service, whether it was received or sent
let synced = false;

export function useHeadDown() {
  return headDown;
}

watch(
  viewerSettings,
  (settings) => {
    if (settings) {
      synced = settings.head_down;
      headDown.value = settings.head_down;
    }
  },
  { immediate: true }
);

watch(headDown, (value) => {
  if (value === synced) return;
  synced = value;
  updateViewerSettings({ head_down: value }).catch((error) => {
    console.error('Failed to save head-down setting:', error);
  });
});
