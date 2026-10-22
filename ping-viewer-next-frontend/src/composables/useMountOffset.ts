import { updateViewerSettings, viewerSettings } from '@/composables/useViewerSettings';
import { ref, watch } from 'vue';

// Firmware auto_transmit misbehaves when start_angle > stop_angle, so the sonar
// always scans the same sector and the mount offset only rotates the view. The
// service stores the value; this module mirrors it so Ping360Settings and
// Ping360Loader share it.
const mountOffset = ref(180);
// Last value known to the service, whether it was received or sent
let synced = 180;

export function useMountOffset() {
  return mountOffset;
}

watch(
  viewerSettings,
  (settings) => {
    if (settings) {
      synced = settings.mount_offset;
      mountOffset.value = settings.mount_offset;
    }
  },
  { immediate: true }
);

watch(mountOffset, (value) => {
  if (value === synced) return;
  synced = value;
  updateViewerSettings({ mount_offset: value }).catch((error) => {
    console.error('Failed to save mount offset:', error);
  });
});
