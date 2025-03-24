import { type Ref, ref } from 'vue';
import type { PingDeviceAPI } from '../types/common';

type PolarModeType = 'full' | 'upper-sector' | 'lower-sector';

/**
 * Service to monitor Ping360 settings and adjust polar mode accordingly
 */
export class PolarModeService {
  private polarMode: Ref<PolarModeType> = ref('full');
  private deviceApi: PingDeviceAPI;
  private intervalId: number | null = null;

  /**
   * Initialize the service with a device API
   * @param deviceApi The Ping360 device API
   */
  constructor(deviceApi: PingDeviceAPI) {
    this.deviceApi = deviceApi;
    this.startMonitoring();
  }

  getPolarMode(): Ref<PolarModeType> {
    return this.polarMode;
  }

  setPolarMode(mode: PolarModeType): void {
    this.polarMode.value = mode;
  }

  private startMonitoring(): void {
    if (!this.deviceApi) return;

    this.checkAngleSettings();

    this.intervalId = window.setInterval(() => {
      this.checkAngleSettings();
    }, 2000);
  }

  async checkAngleSettings(): Promise<void> {
    try {
      const settings = await this.deviceApi.ping360.getSettings();

      if (settings) {
        const startAngleDegrees = this.deviceApi.ping360.gradiansToDegrees(settings.start_angle);
        const stopAngleDegrees = this.deviceApi.ping360.gradiansToDegrees(settings.stop_angle);

        const isStartInUpperRange = startAngleDegrees >= 100 && startAngleDegrees <= 300;
        const isStopInUpperRange = stopAngleDegrees >= 100 && stopAngleDegrees <= 300;

        if (isStartInUpperRange && isStopInUpperRange) {
          this.polarMode.value = 'upper-sector';
        } else {
          this.polarMode.value = 'full';
        }
      }
    } catch (error) {
      console.error('Error checking angle settings:', error);
    }
  }

  destroy(): void {
    if (this.intervalId !== null) {
      window.clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }
}
