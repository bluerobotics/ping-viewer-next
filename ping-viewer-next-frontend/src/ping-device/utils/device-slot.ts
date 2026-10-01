export interface DeviceSlot {
  device_type: string;
  slot: number;
}

export function deviceKey(device: { device_type: string; slot: number | string }): string {
  return `${device.device_type}:${device.slot}`;
}

export function slotPayload(device: { device_type: string; slot: number | string }): DeviceSlot {
  return {
    device_type: device.device_type,
    slot: Number(device.slot),
  };
}

export function sameSlot(
  a: { device_type?: string; slot?: number | string } | null | undefined,
  b: { device_type?: string; slot?: number | string } | null | undefined
): boolean {
  if (a?.device_type == null || a.slot == null || b?.device_type == null || b.slot == null) {
    return false;
  }
  return a.device_type === b.device_type && Number(a.slot) === Number(b.slot);
}

export function deviceWebSocketUrl(
  serverUrl: string,
  device: { device_type: string; slot: number | string }
): string {
  const url = serverUrl.includes('://') ? new URL(serverUrl) : new URL(`http://${serverUrl}`);
  const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${url.host}/ws?device_type=${encodeURIComponent(device.device_type)}&slot=${device.slot}`;
}

export function recordingsActionUrl(
  serverUrl: string,
  device: { device_type: string; slot: number | string },
  action: string
): string {
  return `${serverUrl}/v1/recordings_manager/${encodeURIComponent(device.device_type)}/${device.slot}/${action}`;
}

export function widgetUrl(
  origin: string,
  serverUrl: string,
  device: { device_type: string; slot: number | string }
): string | null {
  const widgetType = device.device_type?.toLowerCase();
  if (widgetType !== 'ping1d' && widgetType !== 'ping360') return null;
  return `${origin}/addons/widget/${widgetType}/?server=${serverUrl}&slot=${device.slot}`;
}
