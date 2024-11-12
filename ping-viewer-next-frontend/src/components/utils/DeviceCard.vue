<template>
  <v-card :elevation="4" class="device-card" :class="{
    'border-primary': selected,
    'cursor-pointer': clickable
  }" @click="handleClick" @dblclick="handleDoubleClick">
    <v-card-item>
      <template v-slot:prepend>
        <v-icon :icon="device.device_type === 'Ping360' ? 'mdi-radar' : 'mdi-altimeter'" size="large" class="mr-4" />
      </template>
      <v-card-title>{{ device.device_type }}</v-card-title>
      <v-card-subtitle>ID: {{ device.id }}</v-card-subtitle>
    </v-card-item>

    <v-card-text>
      <div v-if="device.source.SerialStream" class="mt-2">
        <div class="text-sm">
          <strong>Path:</strong> {{ device.source.SerialStream.path }}
        </div>
        <div class="text-sm">
          <strong>Baudrate:</strong> {{ device.source.SerialStream.baudrate }}
        </div>
      </div>
      <div v-if="device.source.UdpStream" class="mt-2">
        <div class="text-sm">
          <strong>IP:</strong> {{ device.source.UdpStream.ip }}
        </div>
        <div class="text-sm">
          <strong>Port:</strong> {{ device.source.UdpStream.port }}
        </div>
      </div>

      <div class="mt-2">
        <div class="text-sm">
          <strong>Status:</strong>
          <v-chip :color="getStatusColor(device.status)" size="small" class="ml-2">
            {{ device.status }}
          </v-chip>
        </div>
      </div>

      <div v-if="showClickHint && selected" class="text-sm text-blue-400 mt-2">
        Double-click to open
      </div>
    </v-card-text>

    <v-card-actions v-if="showActions">
      <v-btn :color="selected ? 'error' : 'primary'" variant="tonal" block @click.stop="$emit('toggle')">
        {{ selected ? 'Remove' : 'Add' }} Device
      </v-btn>
    </v-card-actions>
  </v-card>
</template>

<script setup>
const props = defineProps({
  device: {
    type: Object,
    required: true,
  },
  selected: {
    type: Boolean,
    default: false,
  },
  clickable: {
    type: Boolean,
    default: true,
  },
  showClickHint: {
    type: Boolean,
    default: false,
  },
  showActions: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(['click', 'dblclick', 'toggle']);

const getStatusColor = (status) => {
  const statusColors = {
    Running: 'success',
    Stopped: 'error',
    ContinuousMode: 'info',
    Error: 'error',
  };
  return statusColors[status] || 'warning';
};

const handleClick = (event) => {
  if (props.clickable) {
    emit('click', event);
  }
};

const handleDoubleClick = (event) => {
  if (props.clickable) {
    emit('dblclick', event);
  }
};
</script>

<style scoped>
.device-card {
  transition: all 0.3s ease;
}

.device-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
}

.border-primary {
  border: 2px solid rgb(var(--v-theme-primary));
}

.cursor-pointer {
  cursor: pointer;
}
</style>