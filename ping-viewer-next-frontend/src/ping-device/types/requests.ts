import type { BaseRequest } from './common';

export interface TestRequest extends BaseRequest {
  type: 'test';
  data: string;
}

export interface ConfigRequest extends BaseRequest {
  type: 'config';
  key: string;
  value: number | string | boolean;
}

export interface Ping360Request extends BaseRequest {
  type: 'ping360';
  command: 'start_scan' | 'stop_scan' | 'set_settings' | 'get_settings';
  parameters?: Record<string, unknown>;
}

export interface Ping1DRequest extends BaseRequest {
  type: 'ping1d';
  command: 'get_distance' | 'get_profile' | 'set_settings' | 'get_settings';
  parameters?: Record<string, unknown>;
}

export type DeviceRequest = TestRequest | ConfigRequest | Ping360Request | Ping1DRequest;
