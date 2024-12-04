<template>
    <v-container class="py-8">
        <v-row>
            <v-col cols="12" md="8" class="mx-auto">
                <div class="d-flex align-center mb-6">
                    <v-btn variant="text" :to="{ path: '/' }" class="mr-4">
                        <v-icon start>mdi-arrow-left</v-icon>
                        Back to main Application View
                    </v-btn>
                    <h1 class="text-h3">Available Widgets</h1>
                </div>

                <v-card v-for="widget in widgets" :key="widget.type" class="mb-6">
                    <v-card-title class="text-h5 d-flex align-center">
                        <v-icon :icon="widget.icon" class="mr-2" />
                        {{ widget.name }}
                    </v-card-title>

                    <v-card-text>
                        <p class="mb-4">{{ widget.description }}</p>

                        <v-expansion-panels>
                            <v-expansion-panel>
                                <v-expansion-panel-title>Integration Parameters</v-expansion-panel-title>
                                <v-expansion-panel-text>
                                    <v-list>
                                        <v-list-item v-for="param in widget.parameters" :key="param.name">
                                            <v-list-item-title>
                                                <code>{{ param.name }}</code>
                                                <v-chip size="small" :color="param.required ? 'error' : 'info'"
                                                    class="ml-2">
                                                    {{ param.required ? 'Required' : 'Optional' }}
                                                </v-chip>
                                            </v-list-item-title>
                                            <v-list-item-subtitle>{{ param.description }}</v-list-item-subtitle>
                                        </v-list-item>
                                    </v-list>
                                </v-expansion-panel-text>
                            </v-expansion-panel>

                            <v-expansion-panel>
                                <v-expansion-panel-title>Usage Example</v-expansion-panel-title>
                                <v-expansion-panel-text>
                                    <v-alert type="info" variant="tonal" class="mb-4">
                                        Replace <code>your-server</code> and <code>device-id</code> with your actual
                                        values.
                                    </v-alert>
                                    <pre class="bg-grey-darken-4 pa-4 rounded"><code>{{ widget.example }}</code></pre>
                                </v-expansion-panel-text>
                            </v-expansion-panel>
                        </v-expansion-panels>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup>
const commonParameters = [
  {
    name: 'server',
    description: 'URL of the PingViewer server (e.g., http://localhost:8080)',
    required: false,
  },
  {
    name: 'uuid',
    description: 'Device ID of the sensor',
    required: true,
  },
];

const widgetDefinitions = {
  ping1d: {
    type: 'ping1d',
    name: 'Ping1D Widget',
    icon: 'mdi-altimeter',
    description: 'Visualize Ping1D sonar data with depth information and waterfall display.',
    parameters: [...commonParameters],
    example: `<iframe
    src="/addons/widget/ping1d?server=http://your-server:8080&device=device-id"
    width="800"
    height="600"
    frameborder="0"
  ></iframe>`,
  },

  ping360: {
    type: 'ping360',
    name: 'Ping360 Widget',
    icon: 'mdi-radar',
    description: 'Display Ping360 scanning sonar data with real-time visualization.',
    parameters: [...commonParameters],
    example: `<iframe
    src="/addons/widget/ping360?server=http://your-server:8080&device=device-id"
    width="800"
    height="600"
    frameborder="0"
  ></iframe>`,
  },
};

const widgets = Object.values(widgetDefinitions);
</script>
