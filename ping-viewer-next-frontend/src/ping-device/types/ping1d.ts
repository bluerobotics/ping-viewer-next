export interface Ping1DData {
  distance: number;
  confidence: number;
  transmit_duration: number;
  ping_number: number;
  scan_start: number;
  scan_length: number;
  gain_setting: number;
  speed_of_sound: number;
  mode_auto: number;
}

export interface Ping1DSettings {
  scan_start: number;
  scan_length: number;
  gain_setting: number;
  speed_of_sound: number;
  mode_auto: number;
}
