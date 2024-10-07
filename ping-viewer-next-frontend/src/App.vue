<template>
  <div class="flex h-screen bg-gray-100">
    <div
      :class="[
        'bg-gray-800 text-white transition-all duration-300 ease-in-out overflow-y-auto',
        drawer ? (mini) : 'w-0'
      ]"
    >
      <div class="py-4">
        <div v-for="(item, index) in menuItems" :key="index" @click="selectView(item.value)"
             class="flex items-center px-4 py-2 cursor-pointer hover:bg-gray-700"
             :class="{ 'bg-gray-700': selectedView === item.value }">
          <v-icon class="mr-4">{{ item.icon }}</v-icon>
          <span v-show="!mini">{{ item.text }}</span>
        </div>
      </div>
    </div>

    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="bg-white shadow z-10">
        <div class="flex items-center px-4 py-3">
          <v-app-bar-nav-icon @click="drawer = !drawer" class="mr-4"></v-app-bar-nav-icon>
          <h1 class="text-xl font-semibold">Ping Viewer Next</h1>
        </div>
      </div>

      <div class="flex-1 overflow-auto p-4">
        <component :is="views[selectedView]" class="h-full" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from "vue";
import DevicesView from "./components/views/DevicesView.vue";
import SettingsView from "./components/views/SettingsView.vue";
import StatisticsView from "./components/views/StatisticsView.vue";
import TryComponentView from "./components/views/TryComponentView.vue";

const drawer = ref(true);
const mini = ref(false);
const selectedView = ref("Settings");

const views = {
	Settings: SettingsView,
	Devices: DevicesView,
	TryOutComponents: TryComponentView,
	Statistics: StatisticsView,
};

const menuItems = [
	{ text: "Settings", icon: "mdi-cog", value: "Settings" },
	{ text: "Devices", icon: "mdi-devices", value: "Devices" },
	{ text: "Try Out Components", icon: "mdi-puzzle", value: "TryOutComponents" },
	{ text: "Statistics", icon: "mdi-chart-line", value: "Statistics" },
];

const selectView = (view) => {
	selectedView.value = view;
};
</script>
