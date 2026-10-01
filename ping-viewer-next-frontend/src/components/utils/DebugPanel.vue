<template>
  <div class="debug-overlay">
    <div class="debug-line dim">
      {{ serverUrl || '—' }} · {{ websocketStatus }}
    </div>

    <template v-for="dev in devices" :key="deviceKey(dev)">
      <div class="debug-separator" />
      <div class="debug-line label">
        {{ dev.device_type || '?' }}
        <span class="dim"> · {{ dev.status }}</span>
      </div>
      <div class="debug-line dim">{{ deviceKey(dev) }}</div>
      <div v-if="dev.source" class="debug-line dim">{{ formatSource(dev.source) }}</div>

      <!-- Base data from properties -->
      <template v-if="deviceProperties[deviceKey(dev)]">
        <div class="debug-separator-thin" />
        <div class="debug-line label">Base data</div>
        <div v-for="(val, key) in deviceProperties[deviceKey(dev)]" :key="key" class="debug-line">
          <span class="dim">{{ key }}:</span> {{ val }}
        </div>
      </template>

      <!-- Live streaming data -->
      <template v-if="liveData[deviceKey(dev)] && Object.keys(liveData[deviceKey(dev)]).length > 0">
        <div class="debug-separator-thin" />
        <div class="debug-line label">{{ dev.device_type }} data</div>
        <div v-for="(val, key) in liveData[deviceKey(dev)]" :key="key" class="debug-line">
          <span class="dim">{{ key }}:</span> {{ val }}
        </div>
      </template>

      <!-- Extra info (temperature, voltage, etc.) -->
      <template v-if="extraInfo[deviceKey(dev)] && Object.keys(extraInfo[deviceKey(dev)]).length > 0">
        <div v-for="(val, key) in extraInfo[deviceKey(dev)]" :key="key" class="debug-line">
          <span class="dim">{{ key }}:</span> {{ val }}
        </div>
      </template>
    </template>

    <div v-if="devices.length === 0" class="debug-line dim">
      No devices found
    </div>
  </div>
</template>

<script setup>
import { deviceKey, deviceWebSocketUrl, slotPayload } from '@/ping-device/utils/device-slot';
import { onMounted, onUnmounted, reactive, ref, watch } from 'vue';

const props = defineProps({
  activeDevice: {
    type: Object,
    default: null,
  },
  deviceData: {
    type: Object,
    default: () => ({}),
  },
  serverUrl: {
    type: String,
    default: '',
  },
  websocketStatus: {
    type: String,
    default: 'Disconnected',
  },
});

const devices = ref([]);
const liveData = reactive({});
const extraInfo = reactive({});
const deviceProperties = reactive({});
const sockets = new Map();
let pollInterval = null;
let extraInfoInterval = null;

const fetchDevices = async () => {
  if (!props.serverUrl) return;
  try {
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ command: 'List', module: 'DeviceManager' }),
    });
    if (!response.ok) return;
    const data = await response.json();
    const list = data.DeviceInfo || [];
    devices.value = list;

    for (const dev of list) {
      extractProperties(dev);
      if (dev.status === 'ContinuousMode') {
        connectDeviceWs(dev);
      }
    }

    for (const [id] of sockets) {
      if (!list.find((d) => deviceKey(d) === id)) {
        disconnectDeviceWs(id);
      }
    }
  } catch {
    // silently ignore
  }
};

const extractProperties = (dev) => {
  const props_data = {};
  const p = dev.properties;
  if (!p) {
    deviceProperties[deviceKey(dev)] = props_data;
    return;
  }

  const info =
    p?.Ping1D?.common?.device_information ||
    p?.Ping360?.common?.device_information ||
    p?.Common?.device_information;

  if (info) {
    const major = info.firmware_version_major ?? '';
    const minor = info.firmware_version_minor ?? '';
    const patch = info.firmware_version_patch ?? '';
    if (major !== '' || minor !== '' || patch !== '') {
      props_data.FW = `${major}.${minor}.${patch}`;
    }
    if (info.device_type !== undefined) props_data['Device type'] = info.device_type;
    if (info.device_revision !== undefined) props_data['Device Revision'] = info.device_revision;
  }

  const proto =
    p?.Ping1D?.common?.protocol_version ||
    p?.Ping360?.common?.protocol_version ||
    p?.Common?.protocol_version;

  if (proto) {
    props_data.Protocol = `${proto.version_major}.${proto.version_minor}.${proto.version_patch}`;
  }

  deviceProperties[deviceKey(dev)] = props_data;
};

const connectDeviceWs = (dev) => {
  if (sockets.has(deviceKey(dev))) return;
  if (!props.serverUrl) return;

  try {
    const wsUrl = deviceWebSocketUrl(props.serverUrl, dev);
    const ws = new WebSocket(wsUrl);

    ws.onmessage = (event) => {
      try {
        const parsed = JSON.parse(event.data);
        const ping = parsed?.DeviceMessage?.PingMessage;
        if (!ping) return;

        if (ping.Ping1D?.Profile) {
          const d = ping.Ping1D.Profile;
          liveData[deviceKey(dev)] = {
            'Distance (mm)': d.distance,
            'Auto (bool)': d.mode_auto === 1 ? 'true' : 'false',
            'Scan Start (mm)': d.scan_start,
            'Scan Length (mm)': d.scan_length,
            'Ping (#)': d.ping_number,
            'Transmit duration (μs)': d.transmit_duration,
            'Gain (setting)': d.gain_setting,
            'Confidence (%)': d.confidence,
            'Speed of sound (mm/s)': d.speed_of_sound,
          };
        } else if (ping.Ping1D?.AutoDeviceData) {
          const d = ping.Ping1D.AutoDeviceData;
          liveData[deviceKey(dev)] = {
            'Distance (mm)': d.distance,
            'Auto (bool)': d.mode_auto === 1 ? 'true' : 'false',
            'Scan Start (mm)': d.scan_start,
            'Scan Length (mm)': d.scan_length,
            'Ping (#)': d.ping_number,
            'Transmit duration (μs)': d.transmit_duration,
            'Gain (setting)': d.gain_setting,
            'Confidence (%)': d.confidence,
            'Speed of sound (mm/s)': d.speed_of_sound,
          };
        } else if (ping.Ping360?.AutoDeviceData) {
          const d = ping.Ping360.AutoDeviceData;
          liveData[deviceKey(dev)] = {
            Angle: d.angle,
            Mode: d.mode,
            'Gain (setting)': d.gain_setting,
            'Start angle': d.start_angle,
            'Stop angle': d.stop_angle,
            'Num steps': d.num_steps,
            Samples: d.number_of_samples,
            'Sample period': d.sample_period,
            'Transmit duration': d.transmit_duration,
            'Transmit frequency': d.transmit_frequency,
            'Speed of sound (mm/s)': d.speed_of_sound,
            'Data length': d.data_length,
          };
        }
      } catch {
        // ignore parse errors
      }
    };

    ws.onerror = () => {};
    ws.onclose = () => {
      sockets.delete(deviceKey(dev));
    };

    sockets.set(deviceKey(dev), ws);
  } catch {
    // ignore connection errors
  }
};

const disconnectDeviceWs = (id) => {
  const ws = sockets.get(id);
  if (ws) {
    ws.close();
    sockets.delete(id);
  }
};

const disconnectAllWs = () => {
  for (const [id] of sockets) {
    disconnectDeviceWs(id);
  }
};

const sendDeviceCommand = async (device, command) => {
  if (!props.serverUrl) return null;
  try {
    const requestKey = device.device_type === 'Ping360' ? 'Ping360' : 'Ping1D';
    const response = await fetch(`${props.serverUrl}/device_manager/request`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify({
        command: 'Ping',
        module: 'DeviceManager',
        payload: {
          ...slotPayload(device),
          device_request: { [requestKey]: command },
        },
      }),
    });
    if (!response.ok) return null;
    return await response.json();
  } catch {
    return null;
  }
};

const fetchExtraInfo = async () => {
  for (const dev of devices.value) {
    if (dev.status !== 'ContinuousMode') continue;

    const info = extraInfo[deviceKey(dev)] || {};

    if (dev.device_type === 'Ping1D' || dev.device_type === 'Common') {
      const [tempResp, pcbResp, voltResp, intervalResp] = await Promise.all([
        sendDeviceCommand(dev, 'ProcessorTemperature'),
        sendDeviceCommand(dev, 'PcbTemperature'),
        sendDeviceCommand(dev, 'Voltage5'),
        sendDeviceCommand(dev, 'PingInterval'),
      ]);

      const p1d = (r) => r?.DeviceMessage?.PingMessage?.Ping1D;

      const procTemp = p1d(tempResp)?.ProcessorTemperature;
      if (procTemp)
        info['Processor temperature (C)'] = (procTemp.processor_temperature / 100).toFixed(1);

      const pcbTemp = p1d(pcbResp)?.PcbTemperature;
      if (pcbTemp) info['PCB temperature (C)'] = (pcbTemp.pcb_temperature / 100).toFixed(1);

      const volt = p1d(voltResp)?.Voltage5;
      if (volt) info['Board voltage (V)'] = (volt.voltage_5 / 1000).toFixed(2);

      const interval = p1d(intervalResp)?.PingInterval;
      if (interval) info['Ping interval (ms)'] = interval.ping_interval;
    }

    extraInfo[deviceKey(dev)] = { ...info };
  }
};

const startPolling = () => {
  stopPolling();
  fetchDevices();
  pollInterval = setInterval(fetchDevices, 3000);
  extraInfoInterval = setInterval(fetchExtraInfo, 5000);
  setTimeout(fetchExtraInfo, 1500);
};

const stopPolling = () => {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
  if (extraInfoInterval) {
    clearInterval(extraInfoInterval);
    extraInfoInterval = null;
  }
};

watch(
  () => props.serverUrl,
  (url) => {
    if (url) startPolling();
    else {
      stopPolling();
      disconnectAllWs();
    }
  }
);

onMounted(() => {
  if (props.serverUrl) startPolling();
});

onUnmounted(() => {
  stopPolling();
  disconnectAllWs();
});

const formatSource = (source) => {
  if (!source) return '';
  if (typeof source === 'string') return source;
  if (source.Serial) return `Serial: ${source.Serial.port} @ ${source.Serial.baudrate}`;
  if (source.Udp) return `UDP: ${source.Udp.host}:${source.Udp.port}`;
  if (source.UdpStream) return `UDP: ${source.UdpStream.ip}:${source.UdpStream.port}`;
  if (source.SerialStream)
    return `Serial: ${source.SerialStream.path} @ ${source.SerialStream.baudrate}`;
  return JSON.stringify(source);
};
</script>

<style scoped>
.debug-overlay {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 1100;
  pointer-events: none;
  font-family: 'Roboto Mono', monospace;
  font-size: 0.72rem;
  line-height: 1.5;
  color: rgba(255, 255, 255, 1);
  text-align: center;
  max-width: 80vw;
  max-height: 80vh;
  overflow: hidden;
  padding: 0.75rem 1.25rem;
  border-radius: 0.5rem;
  background-color: rgba(0, 0, 0, 0.10);
  border: 1px solid rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(25px);
  -webkit-backdrop-filter: blur(16px);
  box-shadow: 0px 8px 8px 0px #00000033, 0px 8px 12px 6px #00000016;
}

.debug-line {
  white-space: nowrap;
}

.debug-line.dim,
.dim {
  opacity: 0.6;
}

.debug-line.label {
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-top: 0.15rem;
  opacity: 0.8;
}

.debug-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.2);
  margin: 0.4rem auto;
  width: 60%;
}

.debug-separator-thin {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 0.2rem auto;
  width: 40%;
}
</style>
