import type { BaseResponse, DeviceMessage } from './common';
import type { Ping360Settings } from './ping360';

export interface ConfigResponse extends BaseResponse {
  type: 'config';
  updated: boolean;
}

export interface PingDeviceResponse {
  DeviceMessage: DeviceMessage;
}

export type SettingsResponse = {
  DeviceMessage?: {
    PingMessage?: {
      Ping1D?: {
        ModeAuto?: { mode_auto: number };
        Range?: { scan_start: number; scan_length: number };
        GainSetting?: { gain_setting: number };
        SpeedOfSound?: { speed_of_sound: number };
      };
    };
  };
  DeviceConfig?: {
    Ping360Config?: Ping360Settings;
  };
};

export interface ModeAutoResponse {
  DeviceMessage: {
    PingMessage: {
      Ping1D?: {
        ModeAuto: {
          mode_auto: number;
        };
      };
    };
  };
}

export interface RangeResponse {
  DeviceMessage: {
    PingMessage: {
      Ping1D?: {
        Range: {
          scan_start: number;
          scan_length: number;
        };
      };
    };
  };
}

export interface GainSettingResponse {
  DeviceMessage: {
    PingMessage: {
      Ping1D?: {
        GainSetting: {
          gain_setting: number;
        };
      };
    };
  };
}

export interface SpeedOfSoundResponse {
  DeviceMessage: {
    PingMessage: {
      Ping1D?: {
        SpeedOfSound: {
          speed_of_sound: number;
        };
      };
    };
  };
}

export interface Ping360ConfigResponse {
  DeviceConfig?: {
    Ping360Config?: Ping360Settings;
  };
}

export type PingCommandResponse =
  | ModeAutoResponse
  | RangeResponse
  | GainSettingResponse
  | SpeedOfSoundResponse;

export type DeviceResponse = ConfigResponse | PingDeviceResponse;
