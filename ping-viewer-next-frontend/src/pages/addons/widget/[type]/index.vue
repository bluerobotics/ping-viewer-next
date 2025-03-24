<template>
  <div class="h-screen w-screen bg-transparent" ref="containerRef">

    <div v-if="isLoading" class="flex items-center justify-center text-white">
      <div class="text-center">
        <v-progress-circular indeterminate color="primary" size="64" class="mb-4" />
        <div>Connecting to device...</div>
      </div>
    </div>

    <div v-else-if="error" class="h-full w-full flex items-center justify-center">
      <div class="text-center p-4 max-w-md text-white">
        <v-icon color="error" size="48" class="mb-4">mdi-alert-circle</v-icon>
        <h2 class="text-xl mb-2">Error Loading Widget</h2>
        <p class="text-gray-400">{{ error }}</p>
        <div class="mt-4 text-left text-sm bg-gray-800 p-4 rounded">
          <div><strong>type:</strong> {{ route.params.type }}</div>
          <div><strong>server:</strong> {{ serverUrl }}</div>
          <div><strong>uuid:</strong> {{ deviceId }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="widgetComponent && deviceData" class="widget-container h-full w-full">
      <component :is="widgetComponent" v-bind="widgetProps" class="h-full w-full bg-transparent" ref="widgetRef" />
      <SonarMask :width="dimensions.width" :height="dimensions.height" :type="widgetType" :polar_mode="polarMode"
        class="widget-mask" @button-click="handleMaskButtonClick" />
    </div>
  </div>
</template>

<script>
import { usePingDeviceStore } from '@/stores/deviceAgent';
import { listenToDatalakeVariable } from '@bluerobotics/cockpit-api';
import Ping1DLoader from '@components/widgets/sonar1d/Ping1DLoader.vue';
import Ping360Loader from '@components/widgets/sonar360/Ping360Loader.vue';
import { computed, defineComponent, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import SonarMask from '../components/SonarMask.vue';

export default defineComponent({
  name: 'WidgetView',
  components: {
    SonarMask,
  },
  setup() {
    const route = useRoute();
    const containerRef = ref(null);
    const widgetRef = ref(null);
    const serverUrl = ref('');
    const deviceId = ref('');
    const error = ref('');
    const isLoading = ref(true);
    const deviceData = ref(null);
    const dimensions = ref({ width: 0, height: 0 });
    const yawAngle = ref(0);

    let resizeObserver = null;

    const updateDimensions = () => {
      if (!containerRef.value) return;
      const rect = containerRef.value.getBoundingClientRect();
      dimensions.value = {
        width: rect.width,
        height: rect.height,
      };
    };

    const widgetType = computed(() => route.params.type?.toLowerCase());

    const widgetComponent = computed(() => {
      switch (widgetType.value) {
        case 'ping360':
          return Ping360Loader;
        case 'ping1d':
          return Ping1DLoader;
        default:
          return null;
      }
    });

    const websocketUrl = computed(() => {
      if (!serverUrl.value || !deviceId.value) return '';
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = new URL(serverUrl.value).host;
      return `${wsProtocol}//${host}/ws?device_number=${deviceId.value}`;
    });

    const commonProps = {
      width: window.innerWidth,
      height: window.innerHeight,
      colorPalette: 'Thermal Blue',
    };

    const ping360Props = {
      ...commonProps,
      lineColor: '#f44336',
      lineWidth: 0.5,
      maxDistance: 300,
      numMarkers: 5,
      showRadiusLines: true,
      showMarkers: true,
      radiusLineColor: '#4caf50',
      markerColor: '#4caf50',
      radiusLineWidth: 0.5,
    };

    const ping1DProps = {
      ...commonProps,
      columnCount: 100,
      tickCount: 5,
      depthLineColor: '#ffeb3b',
      depthTextColor: '#ffeb3b',
      currentDepthColor: '#ffeb3b',
      confidenceColor: '#4caf50',
      textBackground: 'rgba(0, 0, 0, 0.5)',
      depthArrowColor: '#f44336',
    };

    const widgetProps = computed(() => {
      if (!deviceData.value) return {};

      const baseProps = {
        device: deviceData.value,
        websocketUrl: websocketUrl.value,
        width: dimensions.value.width,
        height: dimensions.value.height,
        showControls: false,
      };

      if (widgetType.value === 'ping360') {
        return {
          ...baseProps,
          ...ping360Props,
          width: dimensions.value.width,
          height: dimensions.value.height,
          yaw_angle: yawAngle.value,
        };
      }
      return {
        ...baseProps,
        ...ping1DProps,
        width: dimensions.value.width,
        height: dimensions.value.height,
        columnCount: Math.floor(dimensions.value.width / 20),
      };
    });

    const pingDeviceStore = usePingDeviceStore();
    const deviceInstance = ref(null);

    const handleMaskButtonClick = async (buttonEvent) => {
      if (!deviceInstance.value && deviceId.value) {
        deviceInstance.value = pingDeviceStore.usePingDevice(deviceId.value);
      }

      if (!deviceInstance.value) {
        console.error('No device instance available');
        return;
      }

      try {
        const { action, value, id } = buttonEvent;

        switch (action) {
          case 'increase_range':
            if (widgetType.value === 'ping360') {
              const rangeTarget = value || '+10%';
              const result = await deviceInstance.value.ping360.setRange(rangeTarget);
              if (result) {
              }
            } else if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                if (settings.mode_auto === 1) {
                  console.warn(`Disabling auto_range action: ${action}`);
                  await deviceInstance.value.ping1D.setAutoMode(false);
                }
                const newLength = Math.min(75, settings.scan_length + Number(value));
                await deviceInstance.value.ping1D.setRange(settings.scan_start, newLength);
              }
            }
            break;

          case 'decrease_range':
            if (widgetType.value === 'ping360') {
              const rangeTarget = value || '+10%';
              const result = await deviceInstance.value.ping360.setRange(rangeTarget);
              if (result) {
              }
            } else if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                if (settings.mode_auto === 1) {
                  console.warn(`Disabling auto_range action: ${action}`);
                  await deviceInstance.value.ping1D.setAutoMode(false);
                }
                const newLength = Math.max(1, settings.scan_length + Number(value)); // Decrease by 10%, min 1m
                await deviceInstance.value.ping1D.setRange(settings.scan_start, newLength);
              }
            }
            break;

          case 'set_range':
            if (widgetType.value === 'ping360') {
              const rangeTarget = `${value}m`;
              const result = await deviceInstance.value.ping360.setRange(rangeTarget);
              if (result) {
              }
            } else if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                if (settings.mode_auto === 1) {
                  console.warn(`Disabling auto_range action: ${action}`);
                  await deviceInstance.value.ping1D.setAutoMode(false);
                }
                await deviceInstance.value.ping1D.setRange(settings.scan_start, value);
              }
            }
            break;

          case 'sequence_range':
            if (deviceInstance.value) {
              const rangeSequence = [1, 2, 5, 10, 15, 20, 30, 40, 50, 60];

              try {
                let currentRange = 0;
                let currentIndex = 0;

                if (widgetType.value === 'ping360') {
                  const settings = await deviceInstance.value.ping360.getSettings();
                  if (settings) {
                    currentRange = deviceInstance.value.ping360.calculateRange(settings);

                    currentIndex = findClosestValueIndex(currentRange, rangeSequence);

                    let newIndex;
                    if (value === 'up') {
                      newIndex = Math.min(rangeSequence.length - 1, currentIndex + 1);
                    } else {
                      newIndex = Math.max(0, currentIndex - 1);
                    }

                    if (newIndex !== currentIndex) {
                      const newRange = rangeSequence[newIndex];
                      const result = await deviceInstance.value.ping360.setRange(`${newRange}m`);
                      if (result) {
                      }
                    }
                  }
                } else if (widgetType.value === 'ping1d') {
                  const settings = await deviceInstance.value.ping1D.getSettings();
                  if (settings) {
                    currentRange = settings.scan_length;

                    currentIndex = findClosestValueIndex(currentRange, rangeSequence);

                    let newIndex;
                    if (value === 'up') {
                      newIndex = Math.min(rangeSequence.length - 1, currentIndex + 1);
                    } else {
                      newIndex = Math.max(0, currentIndex - 1);
                    }

                    if (newIndex !== currentIndex) {
                      const newRange = rangeSequence[newIndex];
                      await deviceInstance.value.ping1D.setRange(settings.scan_start, newRange);
                    }
                  }
                }
              } catch (err) {
                console.error('Error adjusting sequence range:', err);
              }
            }
            break;

          case 'set_sector':
            if (widgetType.value === 'ping360' && deviceInstance.value.ping360) {
              const settings = await deviceInstance.value.ping360.getSettings();
              if (settings) {
                const sectorSpan = value;
                const centerAngle = 180;

                const halfSector = sectorSpan / 2;

                const startAngleDegrees = (centerAngle - halfSector + 360) % 360;
                const stopAngleDegrees = (centerAngle + halfSector) % 360;

                const startAngle =
                  deviceInstance.value.ping360.degreesToGradians(startAngleDegrees);
                const stopAngle =
                  stopAngleDegrees === 0
                    ? 399
                    : deviceInstance.value.ping360.degreesToGradians(stopAngleDegrees);

                const updatedSettings = {
                  ...settings,
                  start_angle: startAngle,
                  stop_angle: stopAngle,
                };

                const result = await deviceInstance.value.ping360.setSettings(updatedSettings);
                if (result) {
                }
              }
            }
            break;

          case 'increase_gain':
            if (widgetType.value === 'ping360' && deviceInstance.value.ping360) {
              const settings = await deviceInstance.value.ping360.getSettings();
              if (settings) {
                const newGain = Math.min(2, settings.gain_setting + 1);
                const updatedSettings = {
                  ...settings,
                  gain_setting: newGain,
                };
                await deviceInstance.value.ping360.setSettings(updatedSettings);
              }
            } else if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                if (settings.mode_auto === 1) {
                  console.warn(`Disabling auto_gain action: ${action}`);
                  await deviceInstance.value.ping1D.setAutoMode(false);
                }

                const newGain = Math.min(6, settings.gain_setting + 1);
                await deviceInstance.value.ping1D.setGainSetting(newGain);
              }
            }
            break;

          case 'decrease_gain':
            if (widgetType.value === 'ping360' && deviceInstance.value.ping360) {
              const settings = await deviceInstance.value.ping360.getSettings();
              if (settings) {
                const newGain = Math.max(0, settings.gain_setting - 1);
                const updatedSettings = {
                  ...settings,
                  gain_setting: newGain,
                };
                await deviceInstance.value.ping360.setSettings(updatedSettings);
              }
            } else if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                if (settings.mode_auto === 1) {
                  console.warn(`Disabling auto_gain action: ${action}`);
                  await deviceInstance.value.ping1D.setAutoMode(false);
                }

                const newGain = Math.max(0, settings.gain_setting - 1);
                await deviceInstance.value.ping1D.setGainSetting(newGain);
              }
            }
            break;

          case 'toggle_auto_gain':
            if (widgetType.value === 'ping1d') {
              const settings = await deviceInstance.value.ping1D.getSettings();
              if (settings) {
                const autoMode = settings.mode_auto === 0;
                await deviceInstance.value.ping1D.setAutoMode(autoMode);
              }
            }
            break;

          default:
            console.warn(`Unknown action: ${action}`);
        }
      } catch (err) {
        console.error('Error handling button action:', err);
      }
    };

    const polarMode = computed(() => {
      if (deviceInstance.value && widgetType.value === 'ping360') {
        if (deviceInstance.value.data.polarMode) {
          return deviceInstance.value.data.polarMode;
        }

        if (deviceInstance.value.polarMode) {
          return deviceInstance.value.polarMode.value;
        }
      }
      return 'full';
    });

    onMounted(async () => {
      updateDimensions();

      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          if (entry.target === containerRef.value) {
            updateDimensions();
          }
        }
      });

      if (containerRef.value) {
        resizeObserver.observe(containerRef.value);
      }

      window.addEventListener('resize', updateDimensions);
      await nextTick();
      updateDimensions();

      try {
        const params = new URLSearchParams(window.location.search);
        serverUrl.value = params.get('server') || `${location.protocol}//${location.host}`;
        deviceId.value = params.get('uuid') || '';

        if (!deviceId.value) {
          throw new Error('Missing required parameters: uuid');
        }

        const requestBody = {
          command: 'List',
          module: 'DeviceManager',
        };

        const response = await fetch(`${serverUrl.value}/device_manager/request`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Accept: 'application/json',
            'Access-Control-Allow-Origin': '*',
          },
          mode: 'cors',
          body: JSON.stringify({
            command: 'List',
            module: 'DeviceManager',
          }),
        }).catch((err) => {
          return {
            ok: true,
            json: () =>
              Promise.resolve({
                DeviceInfo: [
                  {
                    id: deviceId.value,
                    device_type: route.params.type?.toUpperCase() || 'Ping360',
                    status: 'ContinuousMode',
                    source: {
                      UdpStream: {
                        ip: new URL(serverUrl.value).hostname,
                        port: new URL(serverUrl.value).port,
                      },
                    },
                  },
                ],
              }),
          };
        });

        if (!response.ok) {
          throw new Error(`Failed to connect to server: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        let device = data.DeviceInfo?.find((d) => d.id === deviceId.value);

        if (!device) {
          device = {
            id: deviceId.value,
            device_type: route.params.type.toUpperCase(),
            status: 'ContinuousMode',
            source: {
              UdpStream: {
                ip: new URL(serverUrl.value).hostname,
                port: new URL(serverUrl.value).port,
              },
            },
          };
        }

        if (device.status !== 'ContinuousMode') {
          try {
            const setContinuousModeResponse = await fetch(
              `${serverUrl.value}/device_manager/request`,
              {
                method: 'POST',
                headers: {
                  'Content-Type': 'application/json',
                  Accept: 'application/json',
                  'Access-Control-Allow-Origin': '*',
                },
                body: JSON.stringify({
                  command: 'EnableContinuousMode',
                  module: 'DeviceManager',
                  payload: { uuid: deviceId.value },
                }),
              }
            );

            if (!setContinuousModeResponse.ok) {
              console.warn('Failed to set continuous mode:', setContinuousModeResponse.statusText);
            } else {
              device.status = 'ContinuousMode';
            }
          } catch (err) {
            console.warn('Failed to set continuous mode:', err);
          }
        }

        if (device.device_type.toLowerCase() !== widgetType.value) {
          throw new Error(
            `Device type mismatch: expected ${widgetType.value} but got ${device.device_type}`
          );
        }

        deviceData.value = device;
        isLoading.value = false;
      } catch (err) {
        console.error('Widget initialization error:', err);
        error.value = err.message;
        isLoading.value = false;
      }

      if (widgetType.value === 'ping360') {
        datalakeUnsubscribe = listenToDatalakeVariable(
          'ATTITUDE/yaw',
          (data) => {
            yawAngle.value = -(data * 180) / Math.PI;
          },
          10
        );
      }

      if (!isLoading.value && deviceData.value && deviceId.value) {
        deviceInstance.value = pingDeviceStore.usePingDevice(deviceId.value);

        const wsHost = new URL(serverUrl.value).host;
        pingDeviceStore.setServerUrl(wsHost);

        deviceInstance.value.common.connect();
      }
    });

    function findClosestValueIndex(value, array) {
      let closest = 0;
      let minDiff = Math.abs(value - array[0]);

      for (let i = 1; i < array.length; i++) {
        const diff = Math.abs(value - array[i]);
        if (diff < minDiff) {
          minDiff = diff;
          closest = i;
        }
      }

      return closest;
    }

    onUnmounted(() => {
      if (resizeObserver) {
        resizeObserver.disconnect();
      }

      if (deviceInstance.value) {
        deviceInstance.value.common.disconnect();
      }

      window.removeEventListener('resize', updateDimensions);
    });

    return {
      containerRef,
      widgetRef,
      error,
      isLoading,
      deviceData,
      widgetComponent,
      widgetProps,
      route,
      serverUrl,
      deviceId,
      websocketUrl,
      dimensions,
      widgetType,
      handleMaskButtonClick,
      polarMode,
    };
  },
});
</script>

<style>
html,
body {
  overflow: hidden !important;
}

.h-full {
  height: 100%;
}

.w-full {
  width: 100%;
}

.widget-container {
  position: relative;
  overflow: visible;
}

.widget-mask {
  pointer-events: auto !important;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
</style>