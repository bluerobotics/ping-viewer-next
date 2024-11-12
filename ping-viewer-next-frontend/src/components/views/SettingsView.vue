<template>
    <div class="bg-gray-900 text-white h-full flex flex-col p-6">
      <div class="max-w-4xl mx-auto w-full">
        <h1 class="text-2xl font-bold mb-6">Settings</h1>

        <v-tabs v-model="activeTab" class="mb-4">
          <v-tab value="mavlink">
            <v-icon start>mdi-compass</v-icon>
            MAVLink
          </v-tab>
        </v-tabs>

        <v-window v-model="activeTab">
          <v-window-item value="mavlink">
            <MavlinkSettings />
          </v-window-item>

        </v-window>
      </div>
    </div>
  </template>

  <script setup>
import { ref } from 'vue';
import MavlinkSettings from '../utils/MavlinkSettings.vue';

const props = defineProps({
  serverUrl: {
    type: String,
    required: true,
  },
});

const activeTab = ref('mavlink');
</script>

  <style scoped>
  :deep(.v-tabs) {
    background-color: transparent !important;
  }

  :deep(.v-tab) {
    color: #ffffff80;
  }

  :deep(.v-tab--selected) {
    color: #fff;
  }

  :deep(.v-window) {
    background-color: transparent !important;
  }
  </style>