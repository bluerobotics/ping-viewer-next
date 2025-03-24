export interface Ping360Data {
  angle: number;
  data: number[];
  data_length: number;
  gain_setting: number;
  mode: number;
  num_steps: number;
  number_of_samples: number;
  sample_period: number;
  start_angle: number;
  stop_angle: number;
  transmit_duration: number;
  transmit_frequency: number;
  speed_of_sound: number;
}

export interface Ping360Settings {
  gain_setting: number;
  transmit_duration: number;
  sample_period: number;
  transmit_frequency: number;
  number_of_samples: number;
  speed_of_sound: number;
  start_angle: number;
  stop_angle: number;
  mode: number;
  num_steps: number;
  delay: number;
}
