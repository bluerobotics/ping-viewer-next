import type { Ping360Settings } from '../types/ping360';
import {
  MAX_NUMBER_OF_POINTS,
  MAX_TRANSMIT_DURATION,
  MIN_SAMPLE_PERIOD,
  MIN_TRANSMIT_DURATION,
  SAMPLE_PERIOD_TICK_DURATION,
} from './constants';

export const calculateRange = (settings: Partial<Ping360Settings>): number => {
  const samplePeriod = (settings.sample_period || MIN_SAMPLE_PERIOD) * SAMPLE_PERIOD_TICK_DURATION;
  const numberOfSamples = settings.number_of_samples || MAX_NUMBER_OF_POINTS;
  return (
    Math.round(((samplePeriod * numberOfSamples * (settings.speed_of_sound || 1500)) / 2) * 10) / 10
  );
};

export const calculateSamplePeriod = (
  desiredRange: number,
  numberOfSamples: number,
  speedOfSound: number
): number => {
  return Math.ceil(
    (2 * desiredRange) / (numberOfSamples * speedOfSound * SAMPLE_PERIOD_TICK_DURATION)
  );
};

export const degreesToGradians = (degrees: number): number => {
  if (degrees === 360) {
    return 399;
  }
  return Math.round((degrees * 400) / 360);
};

export const gradiansToDegrees = (gradians: number): number => {
  if (gradians === 399) {
    return 360;
  }
  return Math.round((gradians * 360) / 400);
};

export const calculateTransmitDurationMax = (samplePeriod: number): number => {
  return Math.min(
    MAX_TRANSMIT_DURATION,
    Math.floor(samplePeriod * SAMPLE_PERIOD_TICK_DURATION * 64e6)
  );
};

export const calculateTransmitDuration = (
  range: number,
  speedOfSound: number,
  samplePeriod: number
): number => {
  let autoDuration = Math.round((8000 * range) / speedOfSound);

  autoDuration = Math.round(
    Math.max(Math.ceil(2.5 * samplePeriod * SAMPLE_PERIOD_TICK_DURATION * 1e6), autoDuration)
  );

  return Math.round(
    Math.max(
      MIN_TRANSMIT_DURATION,
      Math.min(calculateTransmitDurationMax(samplePeriod), autoDuration)
    )
  );
};
