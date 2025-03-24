import type {
  GainSettingResponse,
  ModeAutoResponse,
  Ping360ConfigResponse,
  RangeResponse,
  SpeedOfSoundResponse,
} from '../types/responses';

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

  async enableContinuousMode(uuid: string, parameters?: Record<string, unknown>): Promise<unknown> {
    return this.sendHttpRequest('EnableContinuousMode', 'device_manager', {
      uuid,
      ...parameters,
    });
  }

  async disableContinuousMode(uuid: string): Promise<unknown> {
    return this.sendHttpRequest('DisableContinuousMode', 'device_manager', {
      uuid,
    });
  }

  async getPing360Settings(uuid: string): Promise<Ping360ConfigResponse> {
    return this.sendHttpRequest('ModifyDevice', 'device_manager', {
      uuid,
      modify: 'GetPing360Config',
    }) as Promise<Ping360ConfigResponse>;
  }

  async setPing360Settings(uuid: string, settings: Record<string, unknown>): Promise<unknown> {
    return this.sendHttpRequest('ModifyDevice', 'device_manager', {
      uuid,
      modify: {
        SetPing360Config: {
          mode: 1,
          ...settings,
        },
      },
    });
  }

  async sendPing1DCommand(
    uuid: string,
    command: string,
    payload: Record<string, unknown> | null = null
  ): Promise<unknown> {
    return this.sendHttpRequest('Ping', 'device_manager', {
      device_request: {
        Ping1D: payload ? { [command]: payload } : command,
      },
      uuid,
    });
  }

  async getPing1DModeAuto(uuid: string): Promise<ModeAutoResponse> {
    return this.sendPing1DCommand(uuid, 'ModeAuto') as Promise<ModeAutoResponse>;
  }

  async getPing1DRange(uuid: string): Promise<RangeResponse> {
    return this.sendPing1DCommand(uuid, 'Range') as Promise<RangeResponse>;
  }

  async getPing1DGainSetting(uuid: string): Promise<GainSettingResponse> {
    return this.sendPing1DCommand(uuid, 'GainSetting') as Promise<GainSettingResponse>;
  }

  async getPing1DSpeedOfSound(uuid: string): Promise<SpeedOfSoundResponse> {
    return this.sendPing1DCommand(uuid, 'SpeedOfSound') as Promise<SpeedOfSoundResponse>;
  }
}
