import type {
  GainSettingResponse,
  ModeAutoResponse,
  Ping360ConfigResponse,
  RangeResponse,
  SpeedOfSoundResponse,
} from '../types/responses';
import type { DeviceSlot } from '../utils/device-slot';
import { slotPayload } from '../utils/device-slot';

/**
 * Service for making HTTP API calls to the device server
 */
export class ApiService {
  private serverUrl: string;

  constructor(serverUrl: string) {
    this.serverUrl = serverUrl;
  }

  setServerUrl(url: string): void {
    this.serverUrl = url;
  }

  async sendHttpRequest(
    moduleCommand: string,
    module: string,
    payload: Record<string, unknown>
  ): Promise<unknown> {
    try {
      const response = await fetch(`http://${this.serverUrl}/${module}/request`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({
          command: moduleCommand,
          module: module,
          payload,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      console.error('HTTP request error:', error);
      throw error;
    }
  }

  async enableContinuousMode(
    device: DeviceSlot,
    parameters?: Record<string, unknown>
  ): Promise<unknown> {
    return this.sendHttpRequest('EnableContinuousMode', 'device_manager', {
      ...slotPayload(device),
      ...parameters,
    });
  }

  async disableContinuousMode(device: DeviceSlot): Promise<unknown> {
    return this.sendHttpRequest('DisableContinuousMode', 'device_manager', slotPayload(device));
  }

  async getPing360Settings(device: DeviceSlot): Promise<Ping360ConfigResponse> {
    return this.sendHttpRequest('ModifyDevice', 'device_manager', {
      ...slotPayload(device),
      modify: 'GetPing360Config',
    }) as Promise<Ping360ConfigResponse>;
  }

  async setPing360Settings(
    device: DeviceSlot,
    settings: Record<string, unknown>
  ): Promise<unknown> {
    return this.sendHttpRequest('ModifyDevice', 'device_manager', {
      ...slotPayload(device),
      modify: {
        SetPing360Config: {
          mode: 1,
          ...settings,
        },
      },
    });
  }

  async sendPing1DCommand(
    device: DeviceSlot,
    command: string,
    payload: Record<string, unknown> | null = null
  ): Promise<unknown> {
    return this.sendHttpRequest('Ping', 'device_manager', {
      ...slotPayload(device),
      device_request: {
        Ping1D: payload ? { [command]: payload } : command,
      },
    });
  }

  async getPing1DModeAuto(device: DeviceSlot): Promise<ModeAutoResponse> {
    return this.sendPing1DCommand(device, 'ModeAuto') as Promise<ModeAutoResponse>;
  }

  async getPing1DRange(device: DeviceSlot): Promise<RangeResponse> {
    return this.sendPing1DCommand(device, 'Range') as Promise<RangeResponse>;
  }

  async getPing1DGainSetting(device: DeviceSlot): Promise<GainSettingResponse> {
    return this.sendPing1DCommand(device, 'GainSetting') as Promise<GainSettingResponse>;
  }

  async getPing1DSpeedOfSound(device: DeviceSlot): Promise<SpeedOfSoundResponse> {
    return this.sendPing1DCommand(device, 'SpeedOfSound') as Promise<SpeedOfSoundResponse>;
  }
}
