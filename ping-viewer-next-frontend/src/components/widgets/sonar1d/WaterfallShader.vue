<template>
	<div class="waterfall-container w-full h-full">
		<canvas ref="waterfallCanvas" class="w-full h-full"></canvas>
	</div>
</template>

<script>
import { onKeyStroke, useDebounceFn } from '@vueuse/core';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';

export default {
  name: 'WaterfallShader',
  props: {
    width: { type: Number, default: 500 },
    height: { type: Number, default: 400 },
    columnCount: { type: Number, default: 200 },
    sensorData: { type: Array, default: () => [] },
    maxDepth: { type: Number, required: true },
    minDepth: { type: Number, required: true },
    colorPalette: { type: String, default: 'ocean' },
    getColorFromPalette: { type: Function, required: true },
    antialiasing: { type: Boolean, default: true },
    antialiasingInterpolationSteps: { type: Number, default: 10 },
  },
  emits: ['update:columnCount'],
  setup(props) {
    const waterfallCanvas = ref(null);
    let gl;
    let shaderProgram;
    let vertexBuffer;
    let textureCoordBuffer;
    let texture;
    let textureData;
    let textureWidth = 0;
    let textureHeight = 0;
    let colorLUT = null;
    let colorLUT32 = null;
    let colorLUTPalette = null;

    const measurementHistory = ref([]);
    const virtualMaxDepth = ref(props.maxDepth);

    const effectiveWidth = computed(() => Math.min(props.width, props.columnCount));

    const pendingUpdates = ref([]);
    let yCoordCache = null;
    let lastScaleRatio = 0;

    const vertexShaderSource = `
		attribute vec2 a_position;
		attribute vec2 a_texCoord;
		varying vec2 v_texCoord;
		void main() {
		  gl_Position = vec4(a_position, 0.0, 1.0);
		  v_texCoord = a_texCoord;
		}
	  `;

    const fragmentShaderSource = `
		precision mediump float;
		uniform sampler2D u_image;
		uniform float u_virtualMaxDepth;
		uniform float u_minDepth;
		varying vec2 v_texCoord;

		void main() {
			gl_FragColor = texture2D(u_image, v_texCoord);
		}
		`;

    function initWebGL() {
      gl = waterfallCanvas.value.getContext('webgl', { alpha: true });
      if (!gl) {
        console.error('WebGL not supported');
        return;
      }

      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

      const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
      const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource);

      if (!vertexShader || !fragmentShader) {
        console.error('Failed to create shaders');
        return;
      }

      shaderProgram = createProgram(gl, vertexShader, fragmentShader);
      if (!shaderProgram) {
        console.error('Failed to create shader program');
        return;
      }

      const vertices = new Float32Array([-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
      vertexBuffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

      const textureCoords = new Float32Array([0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]);
      textureCoordBuffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, textureCoords, gl.STATIC_DRAW);

      texture = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

      updateTextureSize();
    }

    function updateTextureSize() {
      if (!gl || !texture) return;

      const newWidth = effectiveWidth.value;
      const newHeight = props.height;

      if (newWidth === textureWidth && newHeight === textureHeight) return;

      const oldData = textureData;
      const oldWidth = textureWidth;
      const oldHeight = textureHeight;

      textureData = new Uint8Array(newWidth * newHeight * 4);

      if (oldData && oldWidth > 0 && oldHeight > 0) {
        const copyBytes = Math.min(oldWidth, newWidth) * 4;
        const copyRows = Math.min(oldHeight, newHeight);
        for (let y = 0; y < copyRows; y++) {
          const oldRowStart = y * oldWidth * 4;
          textureData.set(oldData.subarray(oldRowStart, oldRowStart + copyBytes), y * newWidth * 4);
        }
      }

      textureWidth = newWidth;
      textureHeight = newHeight;

      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        newWidth,
        newHeight,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        textureData
      );
    }

    function buildColorLUT(palette) {
      if (colorLUTPalette === palette && colorLUT) return;
      const getColor = props.getColorFromPalette;
      colorLUT = new Uint8Array(256 * 4);
      for (let i = 0; i < 256; i++) {
        const color = getColor(i, palette);
        const off = i * 4;
        colorLUT[off] = color[0];
        colorLUT[off + 1] = color[1];
        colorLUT[off + 2] = color[2];
        colorLUT[off + 3] = color[3] !== undefined ? color[3] : 255;
      }
      colorLUT32 = new Uint32Array(colorLUT.buffer);
      colorLUTPalette = palette;
    }

    function createYCoordMapping(newHeight, dataLength, scaleRatio) {
      const indices1 = new Int32Array(newHeight);
      const indices2 = new Int32Array(newHeight);
      const weights = new Float32Array(newHeight);
      const aa = props.antialiasing;

      for (let y = 0; y < newHeight; y++) {
        const scaledY = (y / newHeight) * scaleRatio * dataLength;
        const baseIndex = Math.floor(scaledY);
        indices1[y] = baseIndex;
        if (aa) {
          indices2[y] = Math.min(baseIndex + 1, dataLength - 1);
          const f = scaledY - baseIndex;
          weights[y] = f * f * (3 - 2 * f);
        } else {
          indices2[y] = baseIndex;
        }
      }

      return { indices1, indices2, weights };
    }

    function fillColumn(td32, x, newWidth, newHeight, data, dataLength) {
      const { indices1, indices2, weights: w } = yCoordCache;
      const lut32 = colorLUT32;
      const aa = props.antialiasing;

      for (let y = 0; y < newHeight; y++) {
        const i1 = indices1[y];
        if (i1 >= dataLength) continue;

        let value;
        if (aa) {
          const v1 = data[i1];
          const v2 = data[indices2[y]];
          const t = w[y];
          const t2 = t * t;
          const t3 = t2 * t;
          value = v1 * (1 - 3 * t2 + 2 * t3) + v2 * (3 * t2 - 2 * t3);
        } else {
          value = data[i1];
        }

        const lutIdx = value <= 0 ? 0 : value >= 255 ? 255 : (value + 0.5) | 0;
        td32[y * newWidth + x] = lut32[lutIdx];
      }
    }

    function updateTexture(redrawAll = false) {
      if (!gl || !textureData) return;

      const newWidth = effectiveWidth.value;
      const newHeight = props.height;
      const minDepth = props.minDepth;
      const vMaxDepth = virtualMaxDepth.value;

      buildColorLUT(props.colorPalette);
      const td32 = new Uint32Array(textureData.buffer);

      if (redrawAll) {
        textureData.fill(0);

        const history = measurementHistory.value;
        for (let col = 0; col < history.length; col++) {
          if (col >= newWidth) break;

          const measurement = history[col];
          const x = newWidth - 1 - col;
          const data = measurement.data;
          const dataLength = data.length;
          const scaleRatio = (vMaxDepth - minDepth) / (measurement.maxDepth - measurement.minDepth);

          if (scaleRatio !== lastScaleRatio) {
            yCoordCache = createYCoordMapping(newHeight, dataLength, scaleRatio);
            lastScaleRatio = scaleRatio;
          }

          fillColumn(td32, x, newWidth, newHeight, data, dataLength);
        }
      } else {
        for (let y = 0; y < newHeight; y++) {
          const rowOffset = y * newWidth * 4;
          textureData.copyWithin(rowOffset, rowOffset + 4, rowOffset + newWidth * 4);
        }

        const lastCol = newWidth - 1;
        for (let y = 0; y < newHeight; y++) {
          td32[y * newWidth + lastCol] = 0;
        }

        while (pendingUpdates.value.length > 0) {
          const measurement = pendingUpdates.value.shift();
          const data = measurement.data;
          const dataLength = data.length;
          const scaleRatio = (vMaxDepth - minDepth) / (measurement.maxDepth - measurement.minDepth);

          if (scaleRatio !== lastScaleRatio) {
            yCoordCache = createYCoordMapping(newHeight, dataLength, scaleRatio);
            lastScaleRatio = scaleRatio;
          }

          fillColumn(td32, lastCol, newWidth, newHeight, data, dataLength);
        }
      }

      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texSubImage2D(
        gl.TEXTURE_2D,
        0,
        0,
        0,
        newWidth,
        newHeight,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        textureData
      );

      render();
    }

    function updateWaterfall() {
      if (!gl || !textureData) return;

      const newData = props.sensorData;

      if (newData.length > 0) {
        const oldVirtualMaxDepth = virtualMaxDepth.value;

        const measurement = {
          data: [...newData],
          maxDepth: props.maxDepth,
          minDepth: props.minDepth,
          timestamp: Date.now(),
        };

        if (props.maxDepth > virtualMaxDepth.value) {
          virtualMaxDepth.value = props.maxDepth;

          // Drop data to keep only 25% of maximum columns when rescaling
          // This way observed a decent refresh rate, in sync with incoming data
          const keepColumns = Math.floor(props.columnCount / 4);
          measurementHistory.value = measurementHistory.value.slice(0, keepColumns);
        }

        measurementHistory.value.unshift(measurement);
        pendingUpdates.value.push(measurement);

        while (measurementHistory.value.length > props.columnCount) {
          measurementHistory.value.pop();
        }

        if (measurementHistory.value.length > 0) {
          let maxHistoricalDepth = props.maxDepth;
          for (const m of measurementHistory.value) {
            if (m.maxDepth > maxHistoricalDepth) maxHistoricalDepth = m.maxDepth;
          }
          virtualMaxDepth.value = maxHistoricalDepth;
        }

        // If virtualMaxDepth changed, redraw everything
        const redrawAll = oldVirtualMaxDepth !== virtualMaxDepth.value;
        updateTexture(redrawAll);
      }
    }

    function render() {
      if (!gl) return;

      gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);

      gl.useProgram(shaderProgram);

      const virtualMaxDepthLocation = gl.getUniformLocation(shaderProgram, 'u_virtualMaxDepth');
      gl.uniform1f(virtualMaxDepthLocation, virtualMaxDepth.value);

      const minDepthLocation = gl.getUniformLocation(shaderProgram, 'u_minDepth');
      gl.uniform1f(minDepthLocation, props.minDepth);

      const positionLocation = gl.getAttribLocation(shaderProgram, 'a_position');
      gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
      gl.enableVertexAttribArray(positionLocation);
      gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);

      const texCoordLocation = gl.getAttribLocation(shaderProgram, 'a_texCoord');
      gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
      gl.enableVertexAttribArray(texCoordLocation);
      gl.vertexAttribPointer(texCoordLocation, 2, gl.FLOAT, false, 0, 0);

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, texture);
      const samplerLocation = gl.getUniformLocation(shaderProgram, 'u_image');
      gl.uniform1i(samplerLocation, 0);

      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }

    function createShader(gl, type, source) {
      const shader = gl.createShader(type);
      gl.shaderSource(shader, source);
      gl.compileShader(shader);

      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error('Shader compilation error:', gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
      }
      return shader;
    }

    function createProgram(gl, vertexShader, fragmentShader) {
      const program = gl.createProgram();
      gl.attachShader(program, vertexShader);
      gl.attachShader(program, fragmentShader);
      gl.linkProgram(program);

      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
        console.error('Program linking error:', gl.getProgramInfoLog(program));
        return null;
      }
      return program;
    }

    const debouncedRebuildTexture = useDebounceFn(() => {
      if (!gl) return;
      updateTextureSize();
      yCoordCache = null;
      lastScaleRatio = 0;
      updateTexture(true);
    }, 100);

    function resizeCanvas() {
      if (!waterfallCanvas.value) return;

      const rect = waterfallCanvas.value.getBoundingClientRect();
      waterfallCanvas.value.width = rect.width;
      waterfallCanvas.value.height = rect.height;

      if (gl) {
        render();
        debouncedRebuildTexture();
      }
    }

    function clearShaderContent() {
      measurementHistory.value = [];
      pendingUpdates.value = [];

      virtualMaxDepth.value = props.maxDepth;

      if (textureData) {
        textureData.fill(0);
      }
    }

    onKeyStroke(['r', 'R'], () => {
      clearShaderContent();
    });

    onMounted(() => {
      resizeCanvas();
      initWebGL();
      window.addEventListener('resize', resizeCanvas);
    });

    onUnmounted(() => {
      if (gl) {
        gl.deleteProgram(shaderProgram);
        gl.deleteBuffer(vertexBuffer);
        gl.deleteBuffer(textureCoordBuffer);
        gl.deleteTexture(texture);
      }
      window.removeEventListener('resize', resizeCanvas);
    });

    watch(() => props.sensorData, updateWaterfall, { deep: true });
    watch(
      () => props.colorPalette,
      () => {
        colorLUTPalette = null;
        updateTexture(true);
      }
    );
    watch(
      () => effectiveWidth.value,
      () => {
        debouncedRebuildTexture();
      }
    );

    return {
      waterfallCanvas,
      virtualMaxDepth,
      measurementHistory,
    };
  },
};
</script>