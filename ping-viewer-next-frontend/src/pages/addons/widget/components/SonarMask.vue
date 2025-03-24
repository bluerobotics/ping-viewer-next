<template>
  <div class="sonar-mask" :style="{ width: width + 'px', height: height + 'px' }">

    <div class="mask-overlay" :class="{ 'edit-mode': editMode }">

      <div v-if="editMode && usePolarCoordinates" class="center-marker"></div>

      <div v-if="editMode && usePolarCoordinates" class="polar-grid">
        <div v-for="(circle, i) in polarCircles" :key="`circle-${i}`" class="polar-circle" :style="{
          width: (circle.radius * 2) + 'px',
          height: (circle.radius * 2) + 'px',
          left: `calc(50% - ${circle.radius}px)`,
          top: `calc(50% - ${circle.radius}px)`
        }">
          <div class="polar-circle-label" v-if="i % 2 === 0">
            {{ Math.round(((i + 1) / polarGridSettings.circleCount) * 100) / 100 }}
          </div>
        </div>

        <div v-for="(angle, i) in polarAngles" :key="`angle-${i}`" class="polar-angle-line" :style="{
          transform: `translate(-50%, -50%) rotate(${angle}deg)`,
          left: '50%',
          top: '50%',
          width: Math.max(width, height) + 'px'
        }">
          <div class="polar-angle-label" v-if="polarGridSettings.circleCount > 2 && i % 2 === 0" :style="{
            transform: `translate(-50%, -50%) rotate(${-angle}deg)`,
            left: '95%'
          }">
            {{ angle }}°
          </div>
        </div>
      </div>

      <div v-if="editMode && showGrid && !usePolarCoordinates" class="grid-overlay">
        <div v-for="row in gridRows" :key="`row-${row}`" class="grid-row" :style="{
          top: (gridSettings.offsetY + (row * gridCellHeight)) + 'px',
          width: width + 'px',
          height: '1px'
        }">
        </div>
        <div v-for="col in gridCols" :key="`col-${col}`" class="grid-column" :style="{
          left: (gridSettings.offsetX + (col * gridCellWidth)) + 'px',
          height: height + 'px',
          width: '1px'
        }">
        </div>
      </div>

      <div v-for="(button, index) in activeButtons" :key="button.id" class="mask-button" :class="{
        'being-dragged': draggedButtonIndex === index,
        'being-resized': resizingButtonIndex === index
      }" :style="getButtonStyle(button)" @mousedown="editMode && startDrag($event, index)"
        @click="!editMode && !preventClick && handleButtonClick(button)">
        <v-icon v-if="button.icon">{{ button.icon }}</v-icon>
        <span v-else>{{ button.label }}</span>

        <div v-if="editMode" class="resize-handle" @mousedown.stop="startResize($event, index)">
          <v-icon small>mdi-resize-bottom-right</v-icon>
        </div>
      </div>
    </div>

    <div v-if="editMode" class="mask-controls" :style="menuPositionStyle">
      <v-card class="control-panel" @mousedown="startMenuDrag">
        <v-card-title class="text-h6 draggable-header">
          <v-icon small class="mr-2">mdi-drag</v-icon>
          Edit Mask
          <v-spacer></v-spacer>
        </v-card-title>

        <v-divider></v-divider>

        <v-card-text>



          <h3 class="text-subtitle-1 mb-2">Available Buttons</h3>
          <v-chip-group>
            <v-chip v-for="button in availableButtons" :key="button.id"
              :color="isButtonActive(button.id) ? 'primary' : ''" @click="toggleButton(button)" class="ma-1">
              <v-icon left v-if="button.icon">{{ button.icon }}</v-icon>
              {{ button.label }}
            </v-chip>
          </v-chip-group>

          <v-divider class="my-3"></v-divider>

          <div v-if="selectedButton">
            <h3 class="text-subtitle-1 mb-2">Button Properties</h3>
            <v-text-field v-model="selectedButton.label" label="Label" outlined dense></v-text-field>
            <v-text-field v-model="selectedButton.icon" label="Icon (mdi-*)" outlined dense></v-text-field>

            <v-radio-group v-model="selectedButton.coordinate.type" label="Coordinate System" class="mt-3">
              <v-radio label="Standard" value="standard" @change="convertToStandard(selectedButton)"></v-radio>
              <v-radio label="Polar" value="polar" @change="convertToPolar(selectedButton)"></v-radio>
            </v-radio-group>

            <div v-if="selectedButton.coordinate.type === 'standard'" class="d-flex">
              <v-text-field v-model.number="selectedButton.coordinate.x" label="X Position (0-1)" type="number" min="0"
                max="1" step="0.01" outlined dense class="mr-2"></v-text-field>
              <v-text-field v-model.number="selectedButton.coordinate.y" label="Y Position (0-1)" type="number" min="0"
                max="1" step="0.01" outlined dense></v-text-field>
            </div>

            <div v-if="selectedButton.coordinate.type === 'polar'" class="d-flex">
              <v-text-field v-model.number="selectedButton.coordinate.r" label="Radius (0-1)" type="number" min="0"
                max="1" step="0.01" outlined dense class="mr-2"></v-text-field>
              <v-text-field v-model.number="selectedButton.coordinate.angle" label="Angle (°)" type="number" min="0"
                max="360" step="1" outlined dense></v-text-field>
            </div>

            <v-color-picker v-model="selectedButton.color" hide-inputs hide-canvas class="ma-2"></v-color-picker>
          </div>

          <v-divider class="my-3"></v-divider>

          <div v-if="selectedButton" class="mb-4">
            <h3 class="text-subtitle-1 mb-2">Grid Settings</h3>
            <div v-if="!usePolarCoordinates" class="d-flex align-center mb-2">
              <v-text-field v-model.number="gridSettings.size" label="Grid Size" type="number" min="5" max="100"
                outlined dense class="mr-2" @change="updateGrid"></v-text-field>
            </div>

            <div v-if="usePolarCoordinates" class="mt-3">
              <div class="text-subtitle-2">Polar Grid Settings</div>
              <div class="d-flex">
                <v-text-field v-model.number="polarGridSettings.circleCount" label="Circle Count" type="number" min="1"
                  max="10" outlined dense class="mr-2" @change="updatePolarGrid"></v-text-field>
                <v-text-field v-model.number="polarGridSettings.angleStep" label="Angle Step (°)" type="number" min="5"
                  max="90" outlined dense @change="updatePolarGrid"></v-text-field>
              </div>
            </div>

            <v-switch v-model="snapToGrid" label="Snap to Grid" dense></v-switch>
          </div>
        </v-card-text>

        <v-card-actions>
          <v-btn color="primary" text @click="exportToFile">
            <v-icon left>mdi-export</v-icon>
            Export
          </v-btn>
          <v-btn color="primary" text @click="$refs.fileInput.click()">
            <v-icon left>mdi-import</v-icon>
            Import
          </v-btn>
          <input ref="fileInput" type="file" accept=".json" style="display: none" @change="importFromFile" />
          <v-spacer></v-spacer>
          <v-btn color="primary" @click="saveAndExitEditMode">
            Save
          </v-btn>
          <v-btn color="error" text @click="resetToDefaults">
            Defaults
          </v-btn>
        </v-card-actions>
      </v-card>
    </div>

    <v-btn v-if="!editMode && allowEditMode" fab small color="primary" class="edit-button" @click="enterEditMode">
      <v-icon>mdi-pencil</v-icon>
    </v-btn>
  </div>
</template>

<script>
import { computed, defineComponent, onMounted, ref, watch } from 'vue';

export default defineComponent({
  name: 'SonarMask',

  props: {
    width: {
      type: Number,
      required: true,
    },
    height: {
      type: Number,
      required: true,
    },
    type: {
      type: String,
      required: true,
      validator: (value) => ['ping1d', 'ping360'].includes(value.toLowerCase()),
    },
    allowEditMode: {
      type: Boolean,
      required: false,
      default: false,
    },
    polar_mode: {
      type: String,
      required: true,
      default: 'full',
      validator: (value) => ['full', 'upper-sector', 'lower-sector'].includes(value.toLowerCase()),
    },
  },

  emits: ['button-click'],

  setup(props, { emit }) {
    const editMode = ref(false);
    const draggedButtonIndex = ref(null);
    const resizingButtonIndex = ref(null);
    const selectedButton = ref(null);

    const usePolarCoordinates = computed(() => {
      return selectedButton.value && selectedButton.value.coordinate.type === 'polar';
    });

    const preventClick = ref(false);

    const menuPosition = ref({ x: 50, y: 50 });
    const menuDragging = ref(false);
    const menuPositionStyle = computed(() => ({
      top: `${menuPosition.value.y}%`,
      left: `${menuPosition.value.x}%`,
      transform: 'translate(-50%, -50%)',
    }));

    const showGrid = computed(() => {
      return editMode.value && selectedButton.value;
    });
    const snapToGrid = ref(true);
    const squareGrid = ref(true);
    const gridSettings = ref({
      size: 20,
      offsetX: 0,
      offsetY: 0,
    });

    const polarGridSettings = ref({
      circleCount: 10,
      angleStep: 10,
    });

    const polarCircles = computed(() => {
      const squareSize = Math.min(props.width, props.height);
      const maxRadius = squareSize / 2;

      return Array.from({ length: polarGridSettings.value.circleCount }, (_, i) => {
        const fraction = (i + 1) / polarGridSettings.value.circleCount;
        return {
          radius: maxRadius * fraction,
          distance: fraction,
        };
      });
    });

    const polarAngles = computed(() => {
      const step = polarGridSettings.value.angleStep;
      const count = Math.floor(360 / step);
      return Array.from({ length: count }, (_, i) => i * step);
    });

    const updatePolarGrid = () => {
      polarGridSettings.value.circleCount = Math.max(
        1,
        Math.min(10, polarGridSettings.value.circleCount)
      );
      polarGridSettings.value.angleStep = Math.max(
        5,
        Math.min(90, polarGridSettings.value.angleStep)
      );

      polarCircles.value;
      polarAngles.value;
    };

    const fileInput = ref(null);

    const gridCellWidth = computed(() => gridSettings.value.size);
    const gridCellHeight = computed(() =>
      squareGrid.value
        ? gridSettings.value.size
        : gridSettings.value.size * (props.height / props.width)
    );

    const gridCols = computed(() => {
      const numCols = Math.floor((props.width - gridSettings.value.offsetX) / gridCellWidth.value);
      return Array.from({ length: numCols }, (_, i) => i);
    });

    const gridRows = computed(() => {
      const numRows = Math.floor(
        (props.height - gridSettings.value.offsetY) / gridCellHeight.value
      );
      return Array.from({ length: numRows }, (_, i) => i);
    });

    const updateGrid = () => {
      gridSettings.value.size = Math.max(5, Math.min(100, gridSettings.value.size));
      gridSettings.value.offsetX = Math.max(0, Math.min(50, gridSettings.value.offsetX));
      gridSettings.value.offsetY = Math.max(0, Math.min(50, gridSettings.value.offsetY));

      gridCellWidth.value;
      gridCellHeight.value;
      gridRows.value;
      gridCols.value;
      grid.value;
      gridCols.value;
    };

    const standardToPolar = (x, y) => {
      const centerX = 0.5;
      const centerY = 0.5;
      const relX = x - centerX;
      const relY = centerY - y;
      const aspectRatio = props.width / props.height;

      let normalizedX = relX;
      let normalizedY = relY;

      if (aspectRatio > 1) {
        normalizedX = relX / aspectRatio;
      } else if (aspectRatio < 1) {
        normalizedY = relY * aspectRatio;
      }

      const r = Math.min(1, Math.sqrt(normalizedX * normalizedX + normalizedY * normalizedY) * 2);

      let angle = Math.atan2(normalizedY, normalizedX) * (180 / Math.PI);
      if (angle < 0) angle += 360;

      return {
        type: 'polar',
        r,
        angle,
      };
    };

    const polarToStandard = (r, angle) => {
      const angleRad = angle * (Math.PI / 180);

      let x = 0.5 + (r / 2) * Math.cos(angleRad);
      let y = 0.5 - (r / 2) * Math.sin(angleRad);

      const aspectRatio = props.width / props.height;
      if (aspectRatio > 1) {
        x = 0.5 + (r / 2) * Math.cos(angleRad) * aspectRatio;
      } else if (aspectRatio < 1) {
        y = 0.5 - ((r / 2) * Math.sin(angleRad)) / aspectRatio;
      }

      return {
        type: 'standard',
        x: Math.max(0, Math.min(1, x)),
        y: Math.max(0, Math.min(1, y)),
      };
    };

    const convertToStandard = (button) => {
      if (button.coordinate.type === 'polar') {
        const standard = polarToStandard(button.coordinate.r, button.coordinate.angle);
        button.coordinate = standard;
      }
    };

    const convertToPolar = (button) => {
      if (button.coordinate.type === 'standard') {
        const polar = standardToPolar(button.coordinate.x, button.coordinate.y);
        button.coordinate = polar;
      }
    };

    const snapPositionToGrid = (coordinate) => {
      if (!snapToGrid.value) return coordinate;

      if (coordinate.type === 'polar') {
        const angleStep = polarGridSettings.value.angleStep;
        const snappedAngle = Math.round(coordinate.angle / angleStep) * angleStep;

        const circleCount = polarGridSettings.value.circleCount;
        const radiusStep = 1 / circleCount;
        const snappedRadius = Math.round(coordinate.r / radiusStep) * radiusStep;

        return {
          type: 'polar',
          r: snappedRadius,
          angle: snappedAngle,
        };
      }
      const cellSizeX = gridCellWidth.value / props.width;
      const cellSizeY = gridCellHeight.value / props.height;

      const snappedX = Math.round(coordinate.x / cellSizeX) * cellSizeX;
      const snappedY = Math.round(coordinate.y / cellSizeY) * cellSizeY;

      return {
        type: 'standard',
        x: Math.max(0, Math.min(1, snappedX)),
        y: Math.max(0, Math.min(1, snappedY)),
      };
    };

    const coordinateToPixels = (coordinate) => {
      const centerX = props.width / 2;
      let centerY;
      let maxRadius;

      if (coordinate.type === 'polar') {
        const squareSize = Math.min(props.width, props.height);

        if (props.polar_mode === 'upper-sector') {
          centerY = props.height;
          maxRadius = Math.min(squareSize, props.width / 2);
        } else if (props.polar_mode === 'lower-sector') {
          centerY = 0;
          maxRadius = Math.min(squareSize, props.width / 2);
        } else {
          centerY = props.height / 2;
          maxRadius = squareSize / 2;
        }

        const angleRad = coordinate.angle * (Math.PI / 180);
        const radius = coordinate.r * maxRadius;

        return {
          x: centerX + radius * Math.cos(angleRad),
          y: centerY - radius * Math.sin(angleRad),
        };
      }

      return {
        x: coordinate.x * props.width,
        y: coordinate.y * props.height,
      };
    };

    const getButtonStyle = (button) => {
      const btnWidth = button.size?.width || 60;
      const btnHeight = button.size?.height || 60;

      const pixelPos = coordinateToPixels(button.coordinate);

      return {
        left: `calc(${pixelPos.x}px - ${btnWidth / 2}px)`,
        top: `calc(${pixelPos.y}px - ${btnHeight / 2}px)`,
        width: `${btnWidth}px`,
        height: `${btnHeight}px`,
        backgroundColor: button.color || '#1976D2',
      };
    };

    const ensureButtonCoordinateFormat = (button) => {
      if (!button.coordinate) {
        if (button.position) {
          button.coordinate = {
            type: 'standard',
            x: button.position.x,
            y: button.position.y,
          };
        } else if (button.polarPosition) {
          button.coordinate = {
            type: 'polar',
            r: button.polarPosition.distance,
            angle: button.polarPosition.angle,
          };
        } else {
          button.coordinate = {
            type: 'standard',
            x: 0.5,
            y: 0.5,
          };
        }
      }

      if (!button.size) {
        button.size = { width: 60, height: 60 };
      }

      return button;
    };

    const ping360DefaultButtons = [
      {
        id: 'range-sequence-up',
        label: '⬆︎',
        icon: 'mdi-plus',
        action: 'sequence_range',
        value: 'up',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 115,
        },
        size: { width: 30, height: 30 },
        color: '#E91E63',
      },
      {
        id: 'range-sequence-down',
        label: '⬇︎',
        icon: 'mdi-minus',
        action: 'sequence_range',
        value: 'down',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 130,
        },
        size: { width: 30, height: 30 },
        color: '#E91E63',
      },
      {
        id: 'sector-30',
        label: '30°',
        action: 'set_sector',
        value: 30,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 20,
        },
        size: { width: 30, height: 30 },
        color: '#FF9800',
      },
      {
        id: 'sector-60',
        label: '60°',
        action: 'set_sector',
        value: 60,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 35,
        },
        size: { width: 30, height: 30 },
        color: '#FF9800',
      },
      {
        id: 'sector-180',
        label: '180°',
        action: 'set_sector',
        value: 180,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 50,
        },
        size: { width: 30, height: 30 },
        color: '#FF9800',
      },
      {
        id: 'sector-360',
        label: '360°',
        action: 'set_sector',
        value: 360,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 65,
        },
        size: { width: 30, height: 30 },
        color: '#FF9800',
      },
      {
        id: 'gain-increase',
        label: 'G+',
        action: 'increase_gain',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 145,
        },
        size: { width: 30, height: 30 },
        color: '#9C27B0',
      },
      {
        id: 'gain-decrease',
        label: 'G-',
        action: 'decrease_gain',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 160,
        },
        size: { width: 30, height: 30 },
        color: '#9C27B0',
      },
    ];

    const ping1dDefaultButtons = [
      {
        id: 'range-increase-1',
        label: '+1',
        icon: 'mdi-plus',
        action: 'increase_range',
        value: '+1',
        coordinate: {
          type: 'standard',
          x: 0.05,
          y: 0.35,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'range-decrease-1',
        label: '-1',
        icon: 'mdi-minus',
        action: 'decrease_range',
        value: '-1',
        coordinate: {
          type: 'standard',
          x: 0.05,
          y: 0.5,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'auto-gain',
        label: 'Auto',
        icon: 'mdi-auto-fix',
        action: 'toggle_auto_gain',
        coordinate: {
          type: 'standard',
          x: 0.05,
          y: 0.65,
        },
        size: { width: 40, height: 40 },
        color: '#F44336',
      },
    ];

    const additionalButtons = [
      {
        id: 'range-increase-10',
        label: '+10%',
        icon: 'mdi-plus',
        action: 'increase_range',
        value: '+10%',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 130,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'range-decrease-10',
        label: '-10%',
        icon: 'mdi-minus',
        action: 'decrease_range',
        value: '-10%',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 150,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'range-increase-5m',
        label: '+5m',
        icon: 'mdi-arrow-expand',
        action: 'increase_range',
        value: '+5m',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 110,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'range-decrease-5m',
        label: '-5m',
        icon: 'mdi-arrow-collapse',
        action: 'decrease_range',
        value: '-5m',
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 170,
        },
        size: { width: 40, height: 40 },
        color: '#1976D2',
      },
      {
        id: 'set-range-10m',
        label: '10m',
        action: 'set_range',
        value: 10,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 90,
        },
        size: { width: 40, height: 40 },
        color: '#4CAF50',
      },
      {
        id: 'set-range-20m',
        label: '20m',
        action: 'set_range',
        value: 20,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 70,
        },
        size: { width: 40, height: 40 },
        color: '#4CAF50',
      },
      {
        id: 'set-range-30m',
        label: '30m',
        action: 'set_range',
        value: 30,
        coordinate: {
          type: 'polar',
          r: 1,
          angle: 50,
        },
        size: { width: 40, height: 40 },
        color: '#4CAF50',
      },
      {
        id: 'gain-increase',
        label: 'G+',
        icon: 'mdi-signal-variant',
        action: 'increase_gain',
        coordinate: {
          type: 'standard',
          x: 0.03,
          y: 0.6,
        },
        size: { width: 40, height: 40 },
        color: '#9C27B0',
      },
      {
        id: 'gain-decrease',
        label: 'G-',
        icon: 'mdi-signal-variant',
        action: 'decrease_gain',
        coordinate: {
          type: 'standard',
          x: 0.03,
          y: 0.7,
        },
        size: { width: 40, height: 40 },
        color: '#9C27B0',
      },
      {
        id: 'set-range-50m',
        label: '50m',
        action: 'set_range',
        value: 50,
        coordinate: {
          type: 'standard',
          x: 0.12,
          y: 0.25,
        },
        size: { width: 40, height: 40 },
        color: '#4CAF50',
      },
      {
        id: 'sector-90',
        label: '90°',
        action: 'set_sector',
        value: 90,
        coordinate: {
          type: 'polar',
          r: 0.8,
          angle: 20,
        },
        size: { width: 40, height: 40 },
        color: '#FF9800',
      },
    ];

    const defaultButtons = computed(() => {
      return props.type.toLowerCase() === 'ping360' ? ping360DefaultButtons : ping1dDefaultButtons;
    });

    const availableButtons = computed(() => {
      const allButtons = [...defaultButtons.value, ...additionalButtons];
      const uniqueButtons = [];
      const seenIds = new Set();

      for (const button of allButtons) {
        if (!seenIds.has(button.id)) {
          seenIds.add(button.id);
          uniqueButtons.push(ensureButtonCoordinateFormat({ ...button }));
        }
      }

      return uniqueButtons;
    });

    const storageKey = computed(() => `sonar-mask-${props.type.toLowerCase()}`);

    const loadButtonsFromStorage = () => {
      if (props.allowEditMode) {
        try {
          const savedButtons = localStorage.getItem(storageKey.value);
          let buttons;

          if (savedButtons) {
            buttons = JSON.parse(savedButtons);
            buttons = buttons.map((btn) => ensureButtonCoordinateFormat(btn));
          } else {
            buttons = [...defaultButtons.value];
          }

          return buttons;
        } catch (e) {
          console.error('Error loading buttons from storage:', e);
          return defaultButtons.value;
        }
      }

      return defaultButtons.value;
    };

    const activeButtons = ref(loadButtonsFromStorage());

    const saveButtonsToStorage = () => {
      try {
        localStorage.setItem(storageKey.value, JSON.stringify(activeButtons.value));
      } catch (e) {
        console.error('Error saving buttons to storage:', e);
      }
    };

    const enterEditMode = () => {
      editMode.value = true;
    };

    const saveAndExitEditMode = () => {
      saveButtonsToStorage();
      editMode.value = false;
      selectedButton.value = null;
    };

    const resetToDefaults = () => {
      activeButtons.value = [...defaultButtons.value];
      saveButtonsToStorage();
    };

    const isButtonActive = (buttonId) => {
      return activeButtons.value.some((button) => button.id === buttonId);
    };

    const toggleButton = (button) => {
      if (isButtonActive(button.id)) {
        activeButtons.value = activeButtons.value.filter((b) => b.id !== button.id);
        if (selectedButton.value && selectedButton.value.id === button.id) {
          selectedButton.value = null;
        }
      } else {
        const newButton = ensureButtonCoordinateFormat({ ...button });

        const existingPositions = activeButtons.value.map((b) => coordinateToPixels(b.coordinate));

        if (newButton.coordinate.type === 'standard') {
          let newPos = { ...newButton.coordinate };
          const relativeSpacingX = 60 / props.width;
          const relativeSpacingY = 60 / props.height;

          let attempts = 0;
          const pixelPos = coordinateToPixels(newPos);

          while (
            existingPositions.some(
              (p) => Math.abs(p.x - pixelPos.x) < 60 && Math.abs(p.y - pixelPos.y) < 60
            ) &&
            attempts < 10
          ) {
            newPos.y += relativeSpacingY * 1.2;
            pixelPos.y = newPos.y * props.height;
            attempts++;
          }

          if (attempts >= 10) {
            newPos.x += relativeSpacingX * 2;
            newPos.y = 0.05;
          }

          if (snapToGrid.value) {
            newPos = snapPositionToGrid(newPos);
          }

          newButton.coordinate = newPos;
        }

        activeButtons.value.push(newButton);
        selectedButton.value = newButton;
      }
    };

    const handleButtonClick = (button) => {
      emit('button-click', {
        action: button.action,
        value: button.value,
        id: button.id,
      });
    };

    const startDrag = (event, index) => {
      event.preventDefault();
      draggedButtonIndex.value = index;
      selectedButton.value = activeButtons.value[index];

      const handleMouseMove = (e) => {
        if (draggedButtonIndex.value !== null) {
          const rect = event.target.closest('.sonar-mask').getBoundingClientRect();

          const button = activeButtons.value[draggedButtonIndex.value];

          if (button.coordinate.type === 'standard') {
            let newX = (e.clientX - rect.left) / rect.width;
            let newY = (e.clientY - rect.top) / rect.height;

            newX = Math.max(0, Math.min(1, newX));
            newY = Math.max(0, Math.min(1, newY));

            button.coordinate.x = newX;
            button.coordinate.y = newY;

            if (snapToGrid.value) {
              button.coordinate = snapPositionToGrid(button.coordinate);
            }
          } else {
            const centerX = rect.width / 2;
            const centerY = rect.height / 2;

            const relX = e.clientX - rect.left - centerX;
            const relY = -(e.clientY - rect.top - centerY);

            const squareSize = Math.min(rect.width, rect.height);
            const maxRadius = squareSize / 2;
            const r = Math.min(1, Math.sqrt(relX * relX + relY * relY) / maxRadius);

            let angle = Math.atan2(relY, relX) * (180 / Math.PI);
            if (angle < 0) angle += 360;

            button.coordinate.r = r;
            button.coordinate.angle = angle;

            if (snapToGrid.value) {
              button.coordinate = snapPositionToGrid(button.coordinate);
            }
          }
        }
      };

      const handleMouseUp = () => {
        draggedButtonIndex.value = null;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    };

    const startResize = (event, index) => {
      event.preventDefault();
      resizingButtonIndex.value = index;
      selectedButton.value = activeButtons.value[index];

      const startX = event.clientX;
      const startY = event.clientY;
      const startWidth = activeButtons.value[index].size?.width || 60;
      const startHeight = activeButtons.value[index].size?.height || 60;

      const handleMouseMove = (e) => {
        if (resizingButtonIndex.value !== null) {
          let newWidth = Math.max(40, startWidth + (e.clientX - startX));
          let newHeight = Math.max(40, startHeight + (e.clientY - startY));

          if (snapToGrid.value) {
            const stepSize = usePolarCoordinates.value ? 10 : gridCellWidth.value;
            newWidth = Math.max(40, Math.round(newWidth / stepSize) * stepSize);
            newHeight = Math.max(40, Math.round(newHeight / stepSize) * stepSize);

            if (e.ctrlKey) {
              const maxDimension = Math.max(newWidth, newHeight);
              newWidth = maxDimension;
              newHeight = maxDimension;
            }
          }

          activeButtons.value[resizingButtonIndex.value].size = {
            width: newWidth,
            height: newHeight,
          };
        }
      };

      const handleMouseUp = () => {
        resizingButtonIndex.value = null;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    };

    watch(selectedButton, (newButton) => {
      if (newButton && newButton.coordinate.type === 'polar') {
        updatePolarGrid();
      }
    });

    watch(
      () => props.polar_mode,
      () => {
        if (editMode.value) {
          if (usePolarCoordinates.value) {
            updatePolarGrid();
          }
        }

        if (activeButtons.value.length > 0) {
          activeButtons.value = [...activeButtons.value];
        }
      }
    );

    const exportToFile = () => {
      try {
        const exportButtons = activeButtons.value.map((button) => {
          return {
            id: button.id,
            label: button.label,
            icon: button.icon,
            action: button.action,
            value: button.value,
            coordinate: button.coordinate,
            size: button.size,
            color: button.color,
          };
        });

        const configData = {
          type: props.type,
          gridSettings: gridSettings.value,
          squareGrid: squareGrid.value,
          polarGridSettings: polarGridSettings.value,
          usePolarCoordinates: usePolarCoordinates.value,
          buttons: exportButtons,
        };

        const jsonData = JSON.stringify(configData, null, 2);
        const blob = new Blob([jsonData], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = `sonar-mask-${props.type.toLowerCase()}-config.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      } catch (e) {
        console.error('Error exporting configuration:', e);
      }
    };

    const importFromFile = (event) => {
      const file = event.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const config = JSON.parse(e.target.result);

          if (config.type && config.buttons && Array.isArray(config.buttons)) {
            if (config.usePolarCoordinates !== undefined) {
              usePolarCoordinates.value = Boolean(config.usePolarCoordinates);
            }

            if (config.gridSettings) {
              gridSettings.value = {
                size: Math.max(5, Math.min(100, config.gridSettings.size || 20)),
                offsetX: Math.max(0, Math.min(50, config.gridSettings.offsetX || 0)),
                offsetY: Math.max(0, Math.min(50, config.gridSettings.offsetY || 0)),
              };

              if (config.squareGrid !== undefined) {
                squareGrid.value = Boolean(config.squareGrid);
              }
            }

            if (config.polarGridSettings) {
              polarGridSettings.value = {
                circleCount: Math.max(1, Math.min(10, config.polarGridSettings.circleCount || 5)),
                angleStep: Math.max(5, Math.min(90, config.polarGridSettings.angleStep || 30)),
              };
            }

            activeButtons.value = config.buttons.map((button) => {
              const formattedButton = ensureButtonCoordinateFormat(button);

              if (formattedButton.coordinate.type === 'standard') {
                formattedButton.coordinate.x = Math.max(
                  0,
                  Math.min(1, formattedButton.coordinate.x)
                );
                formattedButton.coordinate.y = Math.max(
                  0,
                  Math.min(1, formattedButton.coordinate.y)
                );
              } else if (formattedButton.coordinate.type === 'polar') {
                formattedButton.coordinate.r = Math.max(
                  0,
                  Math.min(1, formattedButton.coordinate.r)
                );
                formattedButton.coordinate.angle = formattedButton.coordinate.angle % 360;
                if (formattedButton.coordinate.angle < 0) {
                  formattedButton.coordinate.angle += 360;
                }
              }

              return formattedButton;
            });

            saveButtonsToStorage();
          }
        } catch (e) {
          console.error('Error importing configuration:', e);
        }

        event.target.value = '';
      };

      reader.readAsText(file);
    };

    const startMenuDrag = (event) => {
      event.preventDefault();
      menuDragging.value = true;

      const startX = event.clientX;
      const startY = event.clientY;
      const startMenuX = menuPosition.value.x;
      const startMenuY = menuPosition.value.y;

      const handleMouseMove = (e) => {
        if (menuDragging.value) {
          const dx = ((e.clientX - startX) / window.innerWidth) * 100;
          const dy = ((e.clientY - startY) / window.innerHeight) * 100;

          menuPosition.value = {
            x: Math.min(Math.max(startMenuX + dx, 10), 90),
            y: Math.min(Math.max(startMenuY + dy, 10), 90),
          };
        }
      };

      const handleMouseUp = () => {
        menuDragging.value = false;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    };

    onMounted(() => {
      activeButtons.value = activeButtons.value.map((btn) => ensureButtonCoordinateFormat(btn));
    });

    return {
      editMode,
      activeButtons,
      availableButtons,
      selectedButton,
      draggedButtonIndex,
      resizingButtonIndex,
      showGrid,
      snapToGrid,
      squareGrid,
      gridSettings,
      gridRows,
      gridCols,
      gridCellWidth,
      gridCellHeight,
      fileInput,
      preventClick,
      menuPositionStyle,
      usePolarCoordinates,
      polarGridSettings,
      polarCircles,
      polarAngles,

      enterEditMode,
      saveAndExitEditMode,
      resetToDefaults,
      handleButtonClick,
      isButtonActive,
      toggleButton,
      startDrag,
      startResize,
      updateGrid,
      updatePolarGrid,
      exportToFile,
      importFromFile,
      startMenuDrag,
      getButtonStyle,
      convertToStandard,
      convertToPolar,
    };
  },
});
</script>

<style scoped>
.sonar-mask {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  overflow: visible;
  z-index: 20;
}

.mask-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: all;
  z-index: 10;
}

.mask-overlay.edit-mode {
  background-color: rgba(0, 0, 0, 0.2);
  border: 2px dashed rgba(255, 255, 255, 0.5);
  pointer-events: all;
}

.center-marker {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 10px;
  height: 10px;
  background-color: rgba(255, 255, 255, 0.7);
  border-radius: 50%;
  transform: translate(-50%, -50%);
  pointer-events: none;
  z-index: 5;
}

.center-marker::before,
.center-marker::after {
  content: '';
  position: absolute;
  background-color: rgba(255, 255, 255, 0.7);
}

.center-marker::before {
  top: -5px;
  left: 50%;
  width: 1px;
  height: 20px;
  transform: translateX(-50%);
}

.center-marker::after {
  top: 50%;
  left: -5px;
  width: 20px;
  height: 1px;
  transform: translateY(-50%);
}

/* Polar grid styles */
.polar-grid {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 5;
}

.polar-circle {
  position: absolute;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 50%;
  pointer-events: none;
}

.polar-circle-label {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  background-color: rgba(0, 0, 0, 0.5);
  color: rgba(255, 255, 255, 0.8);
  font-size: 10px;
  padding: 1px 4px;
  border-radius: 2px;
  white-space: nowrap;
}

.polar-angle-line {
  position: absolute;
  height: 1px;
  background-color: rgba(255, 255, 255, 0.3);
  pointer-events: none;
  transform-origin: center;
}

.polar-angle-label {
  position: absolute;
  top: 50%;
  background-color: rgba(0, 0, 0, 0.5);
  color: rgba(255, 255, 255, 0.8);
  font-size: 10px;
  padding: 1px 4px;
  border-radius: 2px;
  white-space: nowrap;
}

.grid-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 5;
}

.grid-row {
  position: absolute;
  height: 1px;
  background-color: rgba(255, 255, 255, 0.3);
  pointer-events: none;
}

.grid-column {
  position: absolute;
  width: 1px;
  background-color: rgba(255, 255, 255, 0.3);
  pointer-events: none;
}

.mask-button {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: white;
  cursor: pointer;
  user-select: none;
  box-shadow: 0 3px 5px rgba(0, 0, 0, 0.3);
  transition: transform 0.2s, box-shadow 0.2s;
  font-size: 14px;
  font-weight: bold;
  z-index: 100;
  pointer-events: auto;
  opacity: 0.9;
  transform-origin: center center;
}

.mask-button:hover {
  transform: scale(1.05);
  box-shadow: 0 5px 8px rgba(0, 0, 0, 0.4);
}

.mask-button.being-dragged {
  transform: scale(1.1);
  box-shadow: 0 8px 15px rgba(0, 0, 0, 0.5);
  opacity: 0.8;
  z-index: 101;
}

.mask-button.being-resized {
  opacity: 0.8;
  z-index: 101;
}

.resize-handle {
  position: absolute;
  bottom: -10px;
  right: -10px;
  cursor: nwse-resize;
  color: rgba(255, 255, 255, 0.8);
  background-color: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.edit-button {
  position: absolute;
  bottom: 20px;
  right: 20px;
  z-index: 30;
}

.mask-controls {
  position: absolute;
  z-index: 40;
  width: 90%;
  max-width: 400px;
  transition: none;
}

.control-panel {
  max-height: 80vh;
  overflow-y: auto;
}

.draggable-header {
  cursor: move;
  user-select: none;
  background-color: rgba(0, 0, 0, 0.05);
}
</style>