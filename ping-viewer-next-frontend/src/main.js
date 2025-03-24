/**
 * main.js
 *
 * Bootstraps Vuetify and other plugins then mounts the App`
 */

// Tailwind should be loaded before
import './styles/tailwind.css';

// Plugins
import { registerPlugins } from '@/plugins';

import { createPinia } from 'pinia';
// Components
import App from './App.vue';

// Composables
import { createApp } from 'vue';

const app = createApp(App);
const pinia = createPinia();

registerPlugins(app);

app.use(pinia);
app.mount('#app');
