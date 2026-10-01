export type DeviceSelection = 'Common' | 'Ping1D' | 'Ping360' | 'Auto';

export interface DeviceSlotWrapper {
  device_type: DeviceSelection;
  slot: number;
}

export type SourceSelection =
  | {
      UdpStream: {
        ip: string;
        port: number;
        mac_address: string | null;
      };
    }
  | { SerialStream: { path: string; baudrate: number } }
  | { FakeStream: Record<string, never> };

export interface CreateRequest {
  source: SourceSelection;
  device_selection: DeviceSelection;
}

export interface Ping360Config {
  mode: number;
  gain_setting: number;
  transmit_duration: number;
  sample_period: number;
  transmit_frequency: number;
  number_of_samples: number;
  start_angle: number;
  stop_angle: number;
  num_steps: number;
  delay: number;
}

export type ModifyDeviceCommand =
  | { SetIp: string }
  | { SetPing360Config: Ping360Config }
  | 'GetPing360Config';

export interface ModifyDevice extends DeviceSlotWrapper {
  modify: ModifyDeviceCommand;
}

export type Ping1DRequest =
  | 'DeviceID'
  | 'ModeAuto'
  | 'Distance'
  | 'Profile'
  | 'SpeedOfSound'
  | 'Voltage5'
  | 'DeviceId'
  | 'FirmwareVersion'
  | 'Range'
  | 'TransmitDuration'
  | 'PingInterval'
  | 'ProcessorTemperature'
  | 'PcbTemperature'
  | 'GeneralInfo'
  | 'GainSetting'
  | 'PingEnable'
  | 'DistanceSimple'
  | 'GotoBootloader'
  | { SetDeviceId: { device_id: number } }
  | { SetModeAuto: { mode_auto: number } }
  | { SetPingInterval: { ping_interval: number } }
  | { SetPingEnable: { ping_enabled: number } }
  | { SetSpeedOfSound: { speed_of_sound: number } }
  | { SetRange: { scan_start: number; scan_length: number } }
  | { SetGainSetting: { gain_setting: number } }
  | { ContinuousStart: { id: number } }
  | { ContinuousStop: { id: number } };

export type Ping360Request =
  | 'MotorOff'
  | 'DeviceData'
  | 'AutoDeviceData'
  | { SetDeviceId: { id: number; reserved: number } }
  | {
      Transducer: {
        mode: number;
        gain_setting: number;
        angle: number;
        transmit_duration: number;
        sample_period: number;
        transmit_frequency: number;
        number_of_samples: number;
        transmit: number;
        reserved: number;
      };
    }
  | { Reset: { bootloader: number; reserved: number } }
  | {
      AutoTransmit: {
        mode: number;
        gain_setting: number;
        transmit_duration: number;
        sample_period: number;
        transmit_frequency: number;
        number_of_samples: number;
        start_angle: number;
        stop_angle: number;
        num_steps: number;
        delay: number;
      };
    };

export type PingCommonRequest =
  | 'DeviceInformation'
  | 'ProtocolVersion'
  | { SetDeviceId: { device_id: number } };

export type PingRequest =
  | { Ping1D: Ping1DRequest }
  | { Ping360: Ping360Request }
  | { Common: PingCommonRequest }
  | 'GetSubscriber'
  | 'Upgrade'
  | 'Stop';

export interface PingPayload extends DeviceSlotWrapper {
  device_request: PingRequest;
}

export type DeviceManagerRequest =
  | { module: 'DeviceManager'; command: 'AutoCreate' }
  | { module: 'DeviceManager'; command: 'Create'; payload: CreateRequest }
  | { module: 'DeviceManager'; command: 'Delete'; payload: DeviceSlotWrapper }
  | { module: 'DeviceManager'; command: 'List' }
  | { module: 'DeviceManager'; command: 'Info'; payload: DeviceSlotWrapper }
  | { module: 'DeviceManager'; command: 'Search' }
  | { module: 'DeviceManager'; command: 'Ping'; payload: PingPayload }
  | { module: 'DeviceManager'; command: 'GetDeviceHandler'; payload: DeviceSlotWrapper }
  | { module: 'DeviceManager'; command: 'ModifyDevice'; payload: ModifyDevice }
  | { module: 'DeviceManager'; command: 'EnableContinuousMode'; payload: DeviceSlotWrapper }
  | { module: 'DeviceManager'; command: 'DisableContinuousMode'; payload: DeviceSlotWrapper };
