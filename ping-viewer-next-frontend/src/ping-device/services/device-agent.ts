import { ref, watch } from 'vue';
import type { Ref } from 'vue';
import type { DeviceAgentState, DeviceType } from '../types/common';
import type { Ping1DSettings } from '../types/ping1d';
import type { Ping360Settings } from '../types/ping360';
import type { DeviceRequest } from '../types/requests';
import type { DeviceResponse } from '../types/responses';
import type {
  GainSettingResponse,
  ModeAutoResponse,
  RangeResponse,
  SettingsResponse,
  SpeedOfSoundResponse,
} from '../types/responses';
import { DEFAULT_MAX_RECONNECT_ATTEMPTS, DEFAULT_RECONNECT_DELAY } from '../utils/constants';
import { ApiService } from './api-service';

import {
  calculateRange,
  calculateSamplePeriod,
  calculateTransmitDuration,
} from '../utils/ping360-utils';

import { MAX_NUMBER_OF_POINTS, MAX_SAMPLE_PERIOD, MIN_SAMPLE_PERIOD } from '../utils/constants';

/**
 * Creates and manages a WebSocket connection to a device
 */
export class DeviceAgent implements DeviceAgentState {
  ws: WebSocket | null = null;

  isConnected: Ref<boolean> = ref(false);
  messages: Ref<DeviceResponse[]> = ref([]);
  latestPing360Data = ref(null);
  latestPing1DData = ref(null);
  ping360Settings = ref(null);
  polarMode: Ref<string> = ref('full');
  ping1DSettings = ref(null);
  deviceType: Ref<DeviceType> = ref('unknown');
  error: Ref<string | null> = ref(null);
  reconnectAttempts: Ref<number> = ref(0);
  reconnecting: Ref<boolean> = ref(false);

  reconnectTimeout: number | null = null;
  maxReconnectAttempts: number = DEFAULT_MAX_RECONNECT_ATTEMPTS;
  reconnectDelay: number = DEFAULT_RECONNECT_DELAY;

  private apiService: ApiService;

  private settingsRequested = {
    ping360: false,
    ping1D: false,
  };

  constructor(
    private uuid: string,
    private serverUrl: string
  ) {
    this.apiService = new ApiService(serverUrl);
    this.connect();

    watch(this.ping360Settings, async (newSettings, oldSettings) => {
      if (newSettings) {
        const mode = await this.checkViewSector();
        this.polarMode.value = mode;
      }
    });
  }

  /**
   * Connect to the device via WebSocket
   */
  connect(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) return;

    if (this.reconnectTimeout !== null) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    const wsUrl = `ws://${this.serverUrl}/ws?device_number=${this.uuid}`;
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      this.isConnected.value = true;
      this.error.value = null;
      this.reconnectAttempts.value = 0;
      this.reconnecting.value = false;

      this.settingsRequested.ping360 = false;
      this.settingsRequested.ping1D = false;
    };

    this.ws.onmessage = (event) => {
      try {
        const parsedMessage = JSON.parse(event.data);

        this.messages.value.push(parsedMessage);

        if (parsedMessage.DeviceMessage?.PingMessage) {
          const deviceId = parsedMessage.DeviceMessage.device_id;

          if (deviceId === this.uuid) {
            const pingMessage = parsedMessage.DeviceMessage.PingMessage;

            if (pingMessage.Ping360) {
              this.deviceType.value = 'ping360';
              this.latestPing360Data.value = pingMessage.Ping360.AutoDeviceData;

              if (!this.settingsRequested.ping360) {
                this.settingsRequested.ping360 = true;
                this.getPing360Settings()
                  .then((settings) => {
                    if (settings) {
                      this.ping360Settings.value = settings;
                    }
                  })
                  .catch((err) => {
                    console.error('Failed to auto-request Ping360 settings:', err);
                  });
              }
            } else if (pingMessage.Ping1D) {
              this.deviceType.value = 'ping1d';
              this.latestPing1DData.value = pingMessage.Ping1D.AutoDeviceData;

              if (this.latestPing1DData.value) {
                const data = this.latestPing1DData.value;

                if (!this.ping1DSettings.value) {
                  this.ping1DSettings.value = {
                    scan_start: data.scan_start,
                    scan_length: data.scan_length,
                    gain_setting: data.gain_setting,
                    speed_of_sound: data.speed_of_sound,
                    mode_auto: data.mode_auto,
                  };
                } else {
                  this.ping1DSettings.value.scan_start = data.scan_start;
                  this.ping1DSettings.value.scan_length = data.scan_length;
                  this.ping1DSettings.value.gain_setting = data.gain_setting;
                  this.ping1DSettings.value.speed_of_sound = data.speed_of_sound;
                  this.ping1DSettings.value.mode_auto = data.mode_auto;
                }
              }

              if (!this.settingsRequested.ping1D) {
                this.settingsRequested.ping1D = true;
                this.getPing1DSettings()
                  .then((settings) => {
                    if (settings) {
                      this.ping1DSettings.value = settings;
                    }
                  })
                  .catch((err) => {
                    console.error('Failed to auto-request Ping1D settings:', err);
                  });
              }
            }
          }
        }

        if (parsedMessage.DeviceConfig?.Ping360Config) {
          this.ping360Settings.value = parsedMessage.DeviceConfig.Ping360Config;
        }

        this.parseResponseForSettings(parsedMessage);
      } catch (err) {
        this.error.value = `Failed to parse message: ${err.message}`;
      }
    };

    this.ws.onerror = () => {
      this.error.value = 'WebSocket error';
    };

    this.ws.onclose = (evt) => {
      this.isConnected.value = false;
      this.ws = null;

      if (evt.code !== 1000 && !this.reconnecting.value) {
        this.attemptReconnect();
      }
    };
  }

  disconnect(): void {
    if (this.reconnectTimeout !== null) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    this.reconnecting.value = false;
    this.reconnectAttempts.value = 0;

    if (this.ws) {
      this.ws.close(1000, 'Intentional disconnect');
    }
  }

  reconnect(): void {
    this.reconnectAttempts.value = 0;
    this.reconnecting.value = false;

    if (this.reconnectTimeout !== null) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.ws) {
      this.ws.close();
    }

    this.connect();
  }

  sendRequest(request: DeviceRequest): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      this.error.value = 'WebSocket not connected';
      return false;
    }
    this.ws.send(JSON.stringify(request));
    return true;
  }

  private parseResponseForSettings(response: SettingsResponse): void {
    if (response?.DeviceMessage?.PingMessage?.Ping1D) {
      const ping1D = response.DeviceMessage.PingMessage.Ping1D;

      if (!this.ping1DSettings.value) {
        this.ping1DSettings.value = {
          scan_start: 0,
          scan_length: 0,
          gain_setting: 0,
          speed_of_sound: 1500,
          mode_auto: 0,
        };
      }

      if (ping1D.ModeAuto) {
        this.ping1DSettings.value.mode_auto = ping1D.ModeAuto.mode_auto;
      }

      if (ping1D.Range) {
        this.ping1DSettings.value.scan_start = ping1D.Range.scan_start / 1000;
        this.ping1DSettings.value.scan_length = ping1D.Range.scan_length / 1000;
      }

      if (ping1D.GainSetting) {
        this.ping1DSettings.value.gain_setting = ping1D.GainSetting.gain_setting;
      }

      if (ping1D.SpeedOfSound) {
        this.ping1DSettings.value.speed_of_sound = Math.round(
          ping1D.SpeedOfSound.speed_of_sound / 1000
        );
      }
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts.value >= this.maxReconnectAttempts) {
      this.error.value = `Failed to reconnect after ${this.maxReconnectAttempts} attempts`;
      this.reconnecting.value = false;
      return;
    }

    this.reconnecting.value = true;
    this.reconnectAttempts.value++;

    const delay = 5000; // 5 seconds

    this.error.value = `Connection lost. Reconnecting in ${delay / 1000}s (Attempt ${this.reconnectAttempts.value}/${this.maxReconnectAttempts})`;

    this.reconnectTimeout = window.setTimeout(() => {
      this.connect();
    }, delay);
  }

  async getPing360Settings(forceUpdate = false): Promise<Ping360Settings | null> {
    if (this.ping360Settings.value && !forceUpdate) {
      return this.ping360Settings.value;
    }

    try {
      const response = await this.apiService.getPing360Settings(this.uuid);

      if (response?.DeviceConfig?.Ping360Config) {
        this.ping360Settings.value = response.DeviceConfig.Ping360Config;
        return this.ping360Settings.value;
      }
      return null;
    } catch (error) {
      console.error('Error fetching Ping360 settings:', error);
      return this.ping360Settings.value;
    }
  }

  /**
   * Adjust Ping360 settings to achieve a desired range
   * Supports absolute values (e.g. "30m") or percentage changes (e.g. "10%")
   *
   * @param rangeTarget - Target range as string (e.g. "30m", "+5m", "-10m", "+10%", "-25%")
   * @returns Promise<boolean> - True if settings were successfully updated
   */
  async adjustRange(rangeTarget: string): Promise<boolean> {
    // Make sure we have Ping360 settings
    if (!this.ping360Settings.value) {
      try {
        await this.getPing360Settings(true);
      } catch (error) {
        this.error.value = `Failed to get current settings: ${error.message}`;
        return false;
      }

      if (!this.ping360Settings.value) {
        this.error.value = 'Unable to get current Ping360 settings';
        return false;
      }
    }

    const currentSettings = this.ping360Settings.value;
    const currentRange = calculateRange(currentSettings);

    let newRange: number;

    if (rangeTarget.endsWith('%')) {
      const percentChange = Number.parseFloat(rangeTarget.replace('%', ''));
      if (Number.isNaN(percentChange)) {
        this.error.value = `Invalid percentage value: ${rangeTarget}`;
        return false;
      }

      if (rangeTarget.startsWith('+')) {
        newRange = currentRange * (1 + percentChange / 100);
      } else if (rangeTarget.startsWith('-')) {
        newRange = currentRange * (1 - Math.abs(percentChange) / 100);
      } else {
        newRange = currentRange * (percentChange / 100);
      }
    } else {
      const metersValue = Number.parseFloat(rangeTarget.replace('m', ''));
      if (Number.isNaN(metersValue)) {
        this.error.value = `Invalid range value: ${rangeTarget}`;
        return false;
      }

      if (rangeTarget.startsWith('+')) {
        newRange = currentRange + metersValue;
      } else if (rangeTarget.startsWith('-')) {
        newRange = currentRange - Math.abs(metersValue);
      } else {
        newRange = metersValue;
      }
    }

    newRange = Math.max(1, Math.min(newRange, 100));

    const numberOfSamples = currentSettings.number_of_samples || MAX_NUMBER_OF_POINTS;
    const speedOfSound = currentSettings.speed_of_sound || 1500;

    const samplePeriod = calculateSamplePeriod(newRange, numberOfSamples, speedOfSound);

    const clampedSamplePeriod = Math.max(
      MIN_SAMPLE_PERIOD,
      Math.min(MAX_SAMPLE_PERIOD, samplePeriod)
    );

    const transmitDuration = calculateTransmitDuration(newRange, speedOfSound, clampedSamplePeriod);

    const updatedSettings = {
      ...currentSettings,
      sample_period: clampedSamplePeriod,
      transmit_duration: transmitDuration,
    };

    try {
      const apiService = new ApiService(this.serverUrl);
      await apiService.setPing360Settings(this.uuid, updatedSettings);

      this.ping360Settings.value = updatedSettings;

      return true;
    } catch (error) {
      this.error.value = `Failed to update settings: ${error.message}`;
      return false;
    }
  }

  /**
   * Check angle settings and update polar mode
   */
  async checkViewSector(): Promise<string> {
    try {
      if (!this.ping360Settings.value) {
        try {
          await this.getPing360Settings(true);
        } catch (error) {
          console.error('Failed to get settings:', error);
          return 'full';
        }

        if (!this.ping360Settings.value) {
          return 'full';
        }
      }

      const settings = this.ping360Settings.value;

      if (settings) {
        const startAngleDegrees = settings.start_angle;
        const stopAngleDegrees = settings.stop_angle;

        const isStartInUpperRange = startAngleDegrees >= 100 && startAngleDegrees <= 300;
        const isStopInUpperRange = stopAngleDegrees >= 100 && stopAngleDegrees <= 300;

        if (isStartInUpperRange && isStopInUpperRange) {
          return 'upper-sector';
        }
        return 'full';
      }
    } catch (error) {
      console.error('Error checking view sector', error);
    }
    return 'full';
  }

  async getPing1DSettings(forceUpdate = false): Promise<Ping1DSettings | null> {
    if (this.ping1DSettings.value && !forceUpdate) {
      return this.ping1DSettings.value;
    }

    try {
      const settings: Partial<Ping1DSettings> = {};

      const modeAutoResponse = await this.sendPing1DCommand('ModeAuto');
      const modeAutoData = modeAutoResponse as ModeAutoResponse;
      if (modeAutoData?.DeviceMessage?.PingMessage?.Ping1D?.ModeAuto) {
        settings.mode_auto = modeAutoData.DeviceMessage.PingMessage.Ping1D.ModeAuto.mode_auto;
      }

      const rangeResponse = await this.sendPing1DCommand('Range');
      const rangeData = rangeResponse as RangeResponse;
      if (rangeData?.DeviceMessage?.PingMessage?.Ping1D?.Range) {
        const range = rangeData.DeviceMessage.PingMessage.Ping1D.Range;
        settings.scan_start = range.scan_start / 1000;
        settings.scan_length = range.scan_length / 1000;
      }

      const gainResponse = await this.sendPing1DCommand('GainSetting');
      const gainData = gainResponse as GainSettingResponse;
      if (gainData?.DeviceMessage?.PingMessage?.Ping1D?.GainSetting) {
        settings.gain_setting = gainData.DeviceMessage.PingMessage.Ping1D.GainSetting.gain_setting;
      }

      const speedResponse = await this.sendPing1DCommand('SpeedOfSound');
      const speedData = speedResponse as SpeedOfSoundResponse;
      if (speedData?.DeviceMessage?.PingMessage?.Ping1D?.SpeedOfSound) {
        settings.speed_of_sound = Math.round(
          speedData.DeviceMessage.PingMessage.Ping1D.SpeedOfSound.speed_of_sound / 1000
        );
      }

      this.ping1DSettings.value = settings as Ping1DSettings;
      return this.ping1DSettings.value;
    } catch (error) {
      console.error('Error fetching Ping1D settings:', error);
      return this.ping1DSettings.value;
    }
  }

  async sendPing1DCommand(
    command: string,
    payload: Record<string, unknown> | null = null
  ): Promise<unknown> {
    try {
      const response = await this.apiService.sendPing1DCommand(this.uuid, command, payload);

      this.parseResponseForSettings(response as SettingsResponse);

      return response;
    } catch (error) {
      console.error(`Error sending Ping1D command ${command}:`, error);
      return null;
    }
  }
}
