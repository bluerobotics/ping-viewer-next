<template>
  <div class="flex">
    <div
      :class="[
        'bg-gray-800 text-white flex flex-col items-center pt-5 transition-all duration-300 ease-in-out overflow-hidden h-full',
        isCollapsed ? 'w-20' : 'w-64',
      ]"
    >
      <ul class="w-full">
        <li
          v-for="(component, name) in components"
          :key="name"
          @click="selectedComponent = name"
          class="py-4 px-5 text-left cursor-pointer text-lg hover:bg-gray-700"
        >
          {{ name }}
        </li>
      </ul>
      <button
        @click="toggleSidebar"
        class="mt-auto mb-5 bg-gray-700 text-white py-2 px-4 cursor-pointer w-full text-base hover:bg-teal-500"
      >
        Toggle Sidebar
      </button>
    </div>

    <div
      class="flex-grow p-0 bg-black justify-center items-center overflow-hidden box-border"
    >
      <component :is="components[selectedComponent]" class="w-full h-full" />
    </div>
  </div>
</template>

<script setup>
import { ref } from "vue";
import WebsocketClient from "../widgets/WebsocketClient.vue";
import Ping360Widget from "../widgets/sonar360/Ping360Loader.vue";


const selectedComponent = ref("WebsocketClient");
const isCollapsed = ref(false);

const components = {
	WebsocketClient,
  Ping360Widget
};

const toggleSidebar = () => {
	isCollapsed.value = !isCollapsed.value;
};
</script>
