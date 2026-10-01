<template>
  <WebsocketAnalysisView :serverUrl="serverUrl" />
</template>

<script setup>
import WebsocketAnalysisView from '@/components/views/WebsocketAnalysisView.vue';
import { computed } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();

const serverUrl = computed(() => {
  const q = route.query?.server;
  if (typeof q === 'string' && q.length > 0) return q;
  if (Array.isArray(q) && q.length > 0) return q[0];
  return window.location.origin;
});
</script>
