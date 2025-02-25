<template>
  <template v-if="isWidgetRoute">
    <v-app class="h-screen w-screen bg-transparent">
      <router-view />
    </v-app>
  </template>
  <template v-else>
    <v-app>
      <v-main class="bg-gradient-to-r from-cyan-500 to-blue-500">
        <MainView />
      </v-main>
    </v-app>
  </template>
</template>

<script setup>
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import MainView from './views/Main.vue';

const route = useRoute();
const isWidgetRoute = computed(() => route.path.startsWith('/addons/widget/'));
</script>