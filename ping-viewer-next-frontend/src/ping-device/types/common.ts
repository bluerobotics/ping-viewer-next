import type { Ref } from 'vue';
import type { Ping1DData, Ping1DSettings } from './ping1d';
import type { Ping360Data, Ping360Settings } from './ping360';
import type { DeviceResponse } from './responses';

export type DeviceType = 'ping360' | 'ping1d' | 'unknown';

export interface BaseRequest {
  type: string;
  uuid: string;
}

export interface BaseResponse {
  type: string;
  status: 'success' | 'error';
  uuid: string;
}

export interface DeviceAgentState {
  ws: WebSocket | null;
  isConnected: Ref<boolean>;
  messages: Ref<DeviceResponse[]>;
  latestPing360Data: Ref<Ping360Data | null>;
  latestPing1DData: Ref<Ping1DData | null>;

  ping360Settings: Ref<Ping360Settings | null>;
  ping1DSettings: Ref<Ping1DSettings | null>;

  polarMode: Ref<string>;

  deviceType: Ref<DeviceType>;
  error: Ref<string | null>;
  reconnectAttempts: Ref<number>;
  reconnecting: Ref<boolean>;
  reconnectTimeout: number | null;
  maxReconnectAttempts: number;
  reconnectDelay: number;

  getPing360Settings: (forceUpdate?: boolean) => Promise<Ping360Settings | null>;
  getPing1DSettings: (forceUpdate?: boolean) => Promise<Ping1DSettings | null>;
  sendPing1DCommand: (
    command: string,
    payload?: Record<string, unknown> | null
  ) => Promise<unknown>;
  checkViewSector: () => Promise<string>;
}

export interface DeviceMessage {
  PingMessage: {
    Ping360?: {
      AutoDeviceData: Ping360Data;
    };
    Ping1D?: {
      AutoDeviceData: Ping1DData;
    };
  };
  device_id: string;
}

export interface PingDeviceAPI {
  isConnected: Ref<boolean>;
  error: Ref<string | null>;
  deviceType: Ref<DeviceType>;
  reconnecting: Ref<boolean>;
  reconnectAttempts: Ref<number>;

  isPing360: Ref<boolean>;
  isPing1D: Ref<boolean>;
  isRecording: Ref<boolean>;

  data: {
    messages: Ref<DeviceResponse[]>;
    ping360: Ref<Ping360Data | null>;
    ping1D: Ref<Ping1DData | null>;
    ping360Settings: Ref<Ping360Settings | null>;
    ping1DSettings: Ref<Ping1DSettings | null>;
    polarMode: Ref<string>;
  };

  constants: {
    SAMPLE_PERIOD_TICK_DURATION: number;
    MIN_SAMPLE_PERIOD: number;
    MAX_SAMPLE_PERIOD: number;
    MIN_NUMBER_OF_POINTS: number;
    MAX_NUMBER_OF_POINTS: number;
    MIN_TRANSMIT_DURATION: number;
    MAX_TRANSMIT_DURATION: number;
  };

  common: {
    connect: () => void;
    disconnect: () => void;
    reconnect: () => void;
    sendConfigRequest: (key: string, value: number | string | boolean) => boolean;
    toggleRecording: () => Promise<boolean>;
    startRecording: () => Promise<boolean>;
    stopRecording: () => Promise<boolean>;
    getRecordingStatus: () => Promise<RecordingStatus | null>;
  };

  ping360: {
    startScan: (parameters?: Partial<Ping360Settings>) => Promise<boolean>;
    stopScan: () => Promise<boolean>;
    setRange: (rangeTarget: string) => Promise<boolean>;

    getSettings: (forceUpdate?: boolean) => Promise<Ping360Settings | null>;
    setSettings: (settings: Partial<Ping360Settings>) => Promise<boolean>;

    calculateRange: (settings: Partial<Ping360Settings>) => number;
    calculateSamplePeriod: (
      desiredRange: number,
      numberOfSamples: number,
      speedOfSound: number
    ) => number;
    calculateTransmitDuration: (
      range: number,
      speedOfSound: number,
      samplePeriod: number
    ) => number;
    calculateTransmitDurationMax: (samplePeriod: number) => number;
    degreesToGradians: (degrees: number) => number;
    gradiansToDegrees: (gradians: number) => number;
  };

  ping1D: {
    getDistance: () => boolean;
    getProfile: () => boolean;

    getSettings: (forceUpdate?: boolean) => Promise<Ping1DSettings | null>;
    setAutoMode: (autoMode: boolean) => Promise<boolean>;
    setRange: (scanStart: number, scanLength: number) => Promise<boolean>;
    setGainSetting: (gainSetting: number) => Promise<boolean>;
    setSpeedOfSound: (speedOfSound: number) => Promise<boolean>;
  };
}

export interface RecordingStatus {
  device_id: string;
  is_active: boolean;
  file_path?: string;
  start_time?: string;
  device_type?: string;
}
