import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { ApiService } from '../ping-device/services/api-service';
import { DeviceAgent } from '../ping-device/services/device-agent';
export { PolarModeService } from '../ping-device/services/polar-mode-service';
import type { PingDeviceAPI } from '../ping-device/types/common';
import type { DeviceSelection } from '../ping-device/types/requests';
import {
  MAX_NUMBER_OF_POINTS,
  MAX_SAMPLE_PERIOD,
  MAX_TRANSMIT_DURATION,
  MIN_NUMBER_OF_POINTS,
  MIN_SAMPLE_PERIOD,
  MIN_TRANSMIT_DURATION,
  SAMPLE_PERIOD_TICK_DURATION,
} from '../ping-device/utils/constants';
import { type DeviceSlot, deviceKey } from '../ping-device/utils/device-slot';
import {
  calculateRange,
  calculateSamplePeriod,
  calculateTransmitDuration,
  calculateTransmitDurationMax,
  degreesToGradians,
  gradiansToDegrees,
} from '../ping-device/utils/ping360-utils';

interface RecordingStatus {
  device_type: string;
  slot: number;
  is_active: boolean;
  file_path?: string;
  start_time?: string;
}

interface RecordingStatusData {
  device_type?: string;
  slot?: number;
  RecordingStatus?: RecordingStatus | null;
  AllRecordingStatus?: RecordingStatus[];
  is_active?: boolean;
  [key: string]: unknown;
}

export const usePingDeviceStore = defineStore('pingDevice', () => {
  const config = {
    serverUrl: ref(`${window.location.hostname}:6060`),
  };

  const agents = new Map<string, DeviceAgent>();
  const recordingState = ref(new Map<string, boolean>());
  const wsManager = ref<{
    addListener: (cb: (data: RecordingStatusData) => void) => void;
  } | null>(null);

  const apiService = new ApiService(config.serverUrl.value);

  function setServerUrl(url: string): void {
    config.serverUrl.value = url;
    apiService.setServerUrl(url);
  }

  function setWsManager(
    manager: { addListener: (cb: (data: RecordingStatusData) => void) => void } | null
  ) {
    wsManager.value = manager;
    if (manager) {
      manager.addListener((data: RecordingStatusData) => {
        if (!data) return;
        const sessionData = data.RecordingStatus || data;
        if (sessionData.device_type != null && sessionData.slot != null) {
          recordingState.value.set(deviceKey(sessionData as DeviceSlot), !!sessionData.is_active);
        }
      });
    }
  }

  async function fetchInitialRecordingStatus(device: DeviceSlot) {
    const key = deviceKey(device);
    try {
      const response = await apiService.sendHttpRequest(
        'GetRecordingStatus',
        'recordings_manager',
        { device_type: device.device_type, slot: device.slot }
      );
      const status = response as { RecordingStatus?: RecordingStatus | null };
      if (status?.RecordingStatus) {
        recordingState.value.set(key, status.RecordingStatus.is_active);
      } else {
        recordingState.value.set(key, false);
      }
    } catch (error) {
      console.error('Error fetching initial recording status:', error);
      recordingState.value.set(key, false);
    }
  }

  function getAgent(deviceType: string, slot: number): DeviceAgent {
    const device = { device_type: deviceType, slot };
    const key = deviceKey(device);
    if (!agents.has(key)) {
      const agent = new DeviceAgent(device, config.serverUrl.value);
      agents.set(key, agent);
      recordingState.value.set(key, false);
      fetchInitialRecordingStatus(device);
    }

    const agent = agents.get(key);
    if (!agent) {
      throw new Error(`Agent for ${key} should exist but was not found`);
    }
    return agent;
  }

  function usePingDevice(deviceType: DeviceSelection, slot: number): PingDeviceAPI {
    const device = { device_type: deviceType, slot };
    const key = deviceKey(device);
    const agent = getAgent(deviceType, slot);

    const connect = () => {
      agent.connect();
      fetchInitialRecordingStatus(device);
    };

    function disconnect() {
      const agent = agents.get(key);
      if (agent) {
        agent.disconnect();
        agents.delete(key);
        recordingState.value.delete(key);
      }
    }

    const reconnect = () => {
      agent.reconnect();
    };

    // Device type checks
    const isPing360 = computed(() => agent.deviceType.value === 'ping360');
    const isPing1D = computed(() => agent.deviceType.value === 'ping1d');

    // Recording functionality
    const isRecording = computed(() => recordingState.value.get(key) || false);

    const getRecordingStatus = async () => {
      try {
        const response = await apiService.sendHttpRequest(
          'GetRecordingStatus',
          'recordings_manager',
          { device_type: deviceType, slot }
        );
        const status = response as { RecordingStatus?: RecordingStatus | null };
        if (status?.RecordingStatus) {
          recordingState.value.set(key, status.RecordingStatus.is_active);
        }
        return status?.RecordingStatus || null;
      } catch (error) {
        console.error('Error getting recording status:', error);
        return null;
      }
    };

    const startRecording = async () => {
      try {
        const currentStatus = await getRecordingStatus();
        if (currentStatus?.is_active) {
          console.warn('Device is already recording');
          return false;
        }

        const response = await apiService.sendHttpRequest('StartRecording', 'recordings_manager', {
          device_type: deviceType,
          slot,
        });
        if (response) {
          recordingState.value.set(key, true);
          return true;
        }
        return false;
      } catch (error) {
        console.error('Error starting recording:', error);
        return false;
      }
    };

    const stopRecording = async () => {
      try {
        const currentStatus = await getRecordingStatus();
        if (!currentStatus?.is_active) {
          console.warn('Device is not recording');
          return false;
        }

        const response = await apiService.sendHttpRequest('StopRecording', 'recordings_manager', {
          device_type: deviceType,
          slot,
        });
        if (response) {
          recordingState.value.set(key, false);
          return true;
        }
        return false;
      } catch (error) {
        console.error('Error stopping recording:', error);
        return false;
      }
    };

    const toggleRecording = async () => {
      const currentStatus = await getRecordingStatus();
      return currentStatus?.is_active ? stopRecording() : startRecording();
    };

    // =================================================
    // PING360 SPECIFIC METHODS
    // =================================================

    const getPing360Settings = async (forceUpdate = false) => {
      return agent.getPing360Settings(forceUpdate);
    };

    const setSettings = async (settings) => {
      try {
        const response = await apiService.setPing360Settings(device, settings);

        if (agent.ping360Settings.value) {
          agent.ping360Settings.value = {
            ...agent.ping360Settings.value,
            ...settings,
          };
        }

        return !!response;
      } catch (error) {
        console.error('Error setting Ping360 settings:', error);
        return false;
      }
    };

    const startScan = async (parameters = {}) => {
      try {
        if (Object.keys(parameters).length > 0) {
          await setSettings(parameters);
        }

        const response = await apiService.enableContinuousMode(device);

        if (agent.ping360Settings.value && Object.keys(parameters).length > 0) {
          agent.ping360Settings.value = {
            ...agent.ping360Settings.value,
            ...parameters,
          };
        }

        return !!response;
      } catch (error) {
        console.error('Error starting Ping360 scan:', error);
        agent.error.value = `Failed to start scan: ${error.message}`;
        return false;
      }
    };

    const stopScan = async () => {
      try {
        const response = await apiService.disableContinuousMode(device);
        return !!response;
      } catch (error) {
        console.error('Error stopping Ping360 scan:', error);
        agent.error.value = `Failed to stop scan: ${error.message}`;
        return false;
      }
    };

    const setRange_ping360 = async (rangeTarget: string) => {
      if (!isPing360.value && agent.deviceType.value !== 'unknown') {
        agent.error.value = 'Device is not a Ping360';
        return false;
      }

      return agent.adjustRange(rangeTarget);
    };

    // =================================================
    // PING1D SPECIFIC METHODS
    // =================================================

    const getPing1DSettings = async (forceUpdate = false) => {
      return agent.getPing1DSettings(forceUpdate);
    };

    const setAutoMode = async (autoMode: boolean) => {
      try {
        await agent.sendPing1DCommand('SetModeAuto', {
          mode_auto: autoMode ? 1 : 0,
        });

        if (agent.ping1DSettings.value) {
          agent.ping1DSettings.value.mode_auto = autoMode ? 1 : 0;
        }

        return true;
      } catch (error) {
        return false;
      }
    };

    const setRange = async (scanStart: number, scanLength: number) => {
      try {
        await agent.sendPing1DCommand('SetRange', {
          scan_start: Math.round(scanStart * 1000), // Convert to mm
          scan_length: Math.round(scanLength * 1000), // Convert to mm
        });

        if (agent.ping1DSettings.value) {
          agent.ping1DSettings.value.scan_start = scanStart;
          agent.ping1DSettings.value.scan_length = scanLength;
        }

        return true;
      } catch (error) {
        return false;
      }
    };

    const setGainSetting = async (gainSetting: number) => {
      try {
        await agent.sendPing1DCommand('SetGainSetting', {
          gain_setting: gainSetting,
        });

        if (agent.ping1DSettings.value) {
          agent.ping1DSettings.value.gain_setting = gainSetting;
        }

        return true;
      } catch (error) {
        return false;
      }
    };

    const setSpeedOfSound = async (speedOfSound: number) => {
      try {
        await agent.sendPing1DCommand('SetSpeedOfSound', {
          speed_of_sound: Math.round(speedOfSound * 1000),
        });

        if (agent.ping1DSettings.value) {
          agent.ping1DSettings.value.speed_of_sound = speedOfSound;
        }

        return true;
      } catch (error) {
        return false;
      }
    };

    const getDistance = () => {
      if (!isPing1D.value && agent.deviceType.value !== 'unknown') {
        agent.error.value = 'Device is not a Ping1D';
        return false;
      }

      return agent.sendRequest({
        module: 'DeviceManager',
        command: 'Ping',
        payload: {
          device_type: deviceType,
          slot,
          device_request: { Ping1D: 'Distance' },
        },
      });
    };

    const getProfile = () => {
      if (!isPing1D.value && agent.deviceType.value !== 'unknown') {
        agent.error.value = 'Device is not a Ping1D';
        return false;
      }

      return agent.sendRequest({
        module: 'DeviceManager',
        command: 'Ping',
        payload: {
          device_type: deviceType,
          slot,
          device_request: { Ping1D: 'Profile' },
        },
      });
    };

    return {
      isConnected: agent.isConnected,
      error: agent.error,
      deviceType: agent.deviceType,
      reconnecting: agent.reconnecting,
      reconnectAttempts: agent.reconnectAttempts,

      isPing360,
      isPing1D,
      isRecording,

      data: {
        messages: agent.messages,
        ping360: agent.latestPing360Data,
        ping1D: agent.latestPing1DData,
        ping360Settings: agent.ping360Settings,
        ping1DSettings: agent.ping1DSettings,
        polarMode: agent.polarMode,
      },

      constants: {
        SAMPLE_PERIOD_TICK_DURATION,
        MIN_SAMPLE_PERIOD,
        MAX_SAMPLE_PERIOD,
        MIN_NUMBER_OF_POINTS,
        MAX_NUMBER_OF_POINTS,
        MIN_TRANSMIT_DURATION,
        MAX_TRANSMIT_DURATION,
      },

      common: {
        connect,
        disconnect,
        reconnect,
        toggleRecording,
        startRecording,
        stopRecording,
        getRecordingStatus,
      },

      ping360: {
        startScan,
        stopScan,

        getSettings: getPing360Settings,
        setSettings,
        setRange: setRange_ping360,

        calculateRange,
        calculateSamplePeriod,
        calculateTransmitDuration,
        calculateTransmitDurationMax,
        degreesToGradians,
        gradiansToDegrees,
      },

      ping1D: {
        getDistance,
        getProfile,

        getSettings: getPing1DSettings,
        setAutoMode,
        setRange,
        setGainSetting,
        setSpeedOfSound,
      },
    };
  }

  return {
    usePingDevice,
    setServerUrl,
    setWsManager,
  };
});
