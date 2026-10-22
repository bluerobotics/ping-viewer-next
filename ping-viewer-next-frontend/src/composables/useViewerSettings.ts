import { ref } from 'vue';

export interface DisplayProfile {
  a_scan: boolean;
  color_palette: string;
  debug_mode: boolean;
  background_mode: string;
  background_color: string;
}

export type UnitSystem = 'metric' | 'imperial';

export interface ViewerSettings {
  revision: number;
  head_down: boolean;
  unit_system: UnitSystem;
  mount_offset: number;
  display: DisplayProfile;
}

// Latest settings accepted from the service.
export const viewerSettings = ref<ViewerSettings | null>(null);

let serverUrl = '';
let socket: WebSocket | null = null;

function accept(next: ViewerSettings) {
  if (next.revision > (viewerSettings.value?.revision ?? -1)) {
    viewerSettings.value = next;
  }
}

// The service answers with the settings it stored, so the accepted revision is
// applied here instead of waiting for the websocket broadcast. A stale
// `expected_revision` is answered with 409 and the current settings, which
// replace the local ones the same way.
export async function updateViewerSettings(update: object): Promise<ViewerSettings> {
  const response = await fetch(`${serverUrl}/v1/settings`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(update),
  });
  if (!response.ok && response.status !== 409) {
    throw new Error(`HTTP ${response.status}`);
  }
  const settings = await response.json();
  accept(settings);
  return settings;
}

function readLegacy(key: string) {
  try {
    return JSON.parse(localStorage.getItem(key) ?? 'null');
  } catch {
    return null;
  }
}

// Revision 0 means nobody has saved to the service yet,
// so the settings in this browser are imported
async function migrateLegacySettings(current: ViewerSettings) {
  const display = readLegacy('display-settings');
  const headDown = readLegacy('sonar360.headDown');
  const storedUnits = localStorage.getItem('unitSystem');
  const unitSystem = storedUnits === 'metric' || storedUnits === 'imperial' ? storedUnits : null;
  let settings = current;
  if (current.revision === 0 && (display || unitSystem || typeof headDown === 'boolean')) {
    const profile = current.display;
    settings = await updateViewerSettings({
      expected_revision: 0,
      head_down: typeof headDown === 'boolean' ? headDown : undefined,
      unit_system: unitSystem ?? undefined,
      display: display
        ? {
            a_scan: display.aScan ?? profile.a_scan,
            color_palette: display.colorPalette ?? profile.color_palette,
            debug_mode: display.debugMode ?? profile.debug_mode,
            background_mode: display.backgroundMode ?? profile.background_mode,
            background_color: display.backgroundColor ?? profile.background_color,
          }
        : undefined,
    });
  }
  localStorage.removeItem('display-settings');
  localStorage.removeItem('sonar360.headDown');
  localStorage.removeItem('unitSystem');
  return settings;
}

async function load() {
  try {
    const response = await fetch(`${serverUrl}/v1/settings`);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    accept(await migrateLegacySettings(await response.json()));
  } catch (error) {
    console.error('Failed to load viewer settings:', error);
  }
}

function connect() {
  const { protocol, host } = new URL(serverUrl);
  const filter = encodeURIComponent('^\\{"ViewerSettings"');
  const current = new WebSocket(
    `${protocol === 'https:' ? 'wss' : 'ws'}://${host}/ws?filter=${filter}`
  );
  socket = current;

  current.onopen = () => {
    // The service may have restarted with older revisions, so start over
    viewerSettings.value = null;
    load();
  };
  current.onmessage = (event) => accept(JSON.parse(event.data).ViewerSettings);
  current.onclose = () => {
    setTimeout(() => {
      if (socket === current) connect();
    }, 5000);
  };
}

// Keeps `viewerSettings` in step with the service
export function syncViewerSettings(url: string) {
  if (url === serverUrl) return;
  serverUrl = url;
  socket?.close();
  connect();
}
