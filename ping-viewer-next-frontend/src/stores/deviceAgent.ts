import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { ApiService } from '../ping-device/services/api-service';
import { DeviceAgent } from '../ping-device/services/device-agent';
export { PolarModeService } from '../ping-device/services/polar-mode-service';
import type { PingDeviceAPI } from '../ping-device/types/common';
import type { ConfigRequest, Ping1DRequest } from '../ping-device/types/requests';
import {
  MAX_NUMBER_OF_POINTS,
  MAX_SAMPLE_PERIOD,
  MAX_TRANSMIT_DURATION,
  MIN_NUMBER_OF_POINTS,
  MIN_SAMPLE_PERIOD,
  MIN_TRANSMIT_DURATION,
  SAMPLE_PERIOD_TICK_DURATION,
} from '../ping-device/utils/constants';
import {
  calculateRange,
  calculateSamplePeriod,
  calculateTransmitDuration,
  calculateTransmitDurationMax,
  degreesToGradians,
  gradiansToDegrees,
} from '../ping-device/utils/ping360-utils';

export const usePingDeviceStore = defineStore('pingDevice', () => {
  const config = {
    serverUrl: ref('blueos.local:6060'),
  };

  const agents = new Map<string, DeviceAgent>();

  const apiService = new ApiService(config.serverUrl.value);

  function setServerUrl(url: string): void {
    config.serverUrl.value = url;
    apiService.setServerUrl(url);
  }

  function getAgent(uuid: string): DeviceAgent {
    if (!agents.has(uuid)) {
      const agent = new DeviceAgent(uuid, config.serverUrl.value);
      agents.set(uuid, agent);
    }

    const agent = agents.get(uuid);
    if (!agent) {
      throw new Error(`Agent for uuid ${uuid} should exist but was not found`);
    }
    return agent;
  }

  function usePingDevice(uuid: string): PingDeviceAPI {
    const agent = getAgent(uuid);

    const sendConfigRequest = (key: string, value: number | string | boolean): boolean => {
      const request: ConfigRequest = { type: 'config', uuid, key, value };
      return agent.sendRequest(request);
    };

    const connect = () => {
      agent.connect();
    };

    function disconnect() {
      const agent = agents.get(uuid);
      if (agent) {
        agent.disconnect();
        agents.delete(uuid);
      }
    }

    const reconnect = () => {
      agent.reconnect();
    };

    // Device type checks
    const isPing360 = computed(() => agent.deviceType.value === 'ping360');
    const isPing1D = computed(() => agent.deviceType.value === 'ping1d');

    // =================================================
    // PING360 SPECIFIC METHODS
    // =================================================

    const getPing360Settings = async (forceUpdate = false) => {
      return agent.getPing360Settings(forceUpdate);
    };

    const setSettings = async (settings) => {
      try {
        const response = await apiService.setPing360Settings(uuid, settings);

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

        const response = await apiService.enableContinuousMode(uuid);

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
        const response = await apiService.disableContinuousMode(uuid);
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

      const request: Ping1DRequest = {
        type: 'ping1d',
        uuid,
        command: 'get_distance',
      };
      return agent.sendRequest(request);
    };

    const getProfile = () => {
      if (!isPing1D.value && agent.deviceType.value !== 'unknown') {
        agent.error.value = 'Device is not a Ping1D';
        return false;
      }

      const request: Ping1DRequest = {
        type: 'ping1d',
        uuid,
        command: 'get_profile',
      };
      return agent.sendRequest(request);
    };

    return {
      isConnected: agent.isConnected,
      error: agent.error,
      deviceType: agent.deviceType,
      reconnecting: agent.reconnecting,
      reconnectAttempts: agent.reconnectAttempts,

      isPing360,
      isPing1D,

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
        sendConfigRequest,
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
  };
});
