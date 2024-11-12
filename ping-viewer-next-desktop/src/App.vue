<script>
import { ref, onMounted } from "vue";

const loading = ref(true);

const checkServerReady = () => {
  setTimeout(() => {
    loading.value = false;
  }, 2000);
};

onMounted(() => {
  checkServerReady();
});
</script>

<template>
  <div v-if="loading" class="preloader">
    <h1>Ping Viewer Next</h1>
    <p>Creating a local server...</p>
  </div>

  <div v-else class="container">
    <h1>Ping Viewer Next</h1>
    <p>Server is ready!</p>
  </div>
</template>

<style scoped>
.preloader, .container {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 100vw;
  height: 100vh;
  margin: 0;
  padding: 0;
  background-color: #f0f0f0;
}

h1 {
  font-size: 3rem;
  margin-bottom: 1rem;
}

p {
  font-size: 1.5rem;
  color: #666;
}

html, body {
  margin: 0;
  padding: 0;
  overflow: hidden;
}
</style>
