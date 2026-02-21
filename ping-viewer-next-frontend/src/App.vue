<template>
  <template v-if="isWidgetRoute">
    <v-app class="h-screen w-screen bg-transparent">
      <router-view />
    </v-app>
  </template>
  <template v-else>
    <v-app>
      <v-main class="bg-[radial-gradient(ellipse_at_50%_40%,#005C84,#00223A)]">
        <MainView />
      </v-main>
    </v-app>
  </template>
</template>

<script setup>
import { computed, watchEffect } from 'vue';
import { useRoute } from 'vue-router';
import { useTheme } from 'vuetify';
import MainView from './views/Main.vue';
import '@/styles/main.css';

const route = useRoute();
const isWidgetRoute = computed(() => route.path.startsWith('/addons/widget/'));
const theme = useTheme();

watchEffect(() => {
  if (isWidgetRoute.value) {
    theme.global.name.value = 'light';
  }
});
</script>
