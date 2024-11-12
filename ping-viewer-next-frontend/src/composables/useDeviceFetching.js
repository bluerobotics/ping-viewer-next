import { onMounted, onUnmounted, ref } from 'vue';

export function useDeviceFetching(serverUrl) {
  const deviceInfo = ref({ DeviceInfo: [] });
  const fetchingError = ref(null);
  let fetchInterval = null;

  const fetchDevices = async () => {
    try {
      const response = await fetch(`${serverUrl}/device_manager/List`, {
        mode: 'cors',
      });
      if (response.ok) {
        const data = await response.json();
        deviceInfo.value = data;
        fetchingError.value = null;
        return true;
      }
      fetchingError.value = 'Failed to fetch devices';
    } catch (err) {
      console.error(`Error connecting to ${serverUrl}:`, err);
      fetchingError.value = err.message;
    }
    return false;
  };

  const startPeriodicFetch = (interval = 5000) => {
    fetchDevices();
    fetchInterval = setInterval(fetchDevices, interval);
  };

  const stopPeriodicFetch = () => {
    if (fetchInterval) {
      clearInterval(fetchInterval);
      fetchInterval = null;
    }
  };

  onMounted(() => {
    startPeriodicFetch();
  });

  onUnmounted(() => {
    stopPeriodicFetch();
  });

  return {
    deviceInfo,
    fetchingError,
    fetchDevices,
    startPeriodicFetch,
    stopPeriodicFetch,
  };
}
