<template>
	<div class="waterfall-container w-full h-full">
		<canvas ref="waterfallCanvas" class="w-full h-full"></canvas>
	</div>
</template>

<script>
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

      const oldTextureData = textureData ? new Uint8Array(textureData) : null;
      const oldWidth = textureData ? Math.floor(textureData.length / (newHeight * 4)) : 0;

      textureData = new Uint8Array(newWidth * newHeight * 4);

      if (oldTextureData) {
        const copyWidth = Math.min(oldWidth, newWidth);
        for (let y = 0; y < newHeight; y++) {
          for (let x = 0; x < copyWidth; x++) {
            const newIndex = (y * newWidth + x) * 4;
            const oldIndex = (y * oldWidth + x) * 4;
            textureData[newIndex] = oldTextureData[oldIndex];
            textureData[newIndex + 1] = oldTextureData[oldIndex + 1];
            textureData[newIndex + 2] = oldTextureData[oldIndex + 2];
            textureData[newIndex + 3] = oldTextureData[oldIndex + 3];
          }
        }
      }

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

    function createYCoordMapping(newHeight, dataLength, scaleRatio) {
      if (!props.antialiasing) {
        // Simple mapping without interpolation
        const mapping = new Array(newHeight);
        for (let y = 0; y < newHeight; y++) {
          const normalizedY = y / newHeight;
          const scaledY = normalizedY * scaleRatio * dataLength;
          const index = Math.floor(scaledY);
          mapping[y] = [index, index];
        }
        return { mapping, weights: new Array(newHeight).fill(0) };
      }

      // Enhanced antialiased mapping with improved interpolation
      const mapping = new Array(newHeight);
      const weights = new Array(newHeight);

      for (let y = 0; y < newHeight; y++) {
        const normalizedY = y / newHeight;
        const scaledY = normalizedY * scaleRatio * dataLength;

        const baseIndex = Math.floor(scaledY);
        const nextIndex = Math.min(baseIndex + 1, dataLength - 1);

        const fraction = scaledY - baseIndex;
        const smoothFraction = fraction * fraction * (3 - 2 * fraction);

        mapping[y] = [baseIndex, nextIndex];
        weights[y] = smoothFraction;
      }

      return { mapping, weights };
    }

    function getInterpolatedValue(data, index1, index2, fraction) {
      if (!props.antialiasing) {
        return data[index1];
      }

      const value1 = data[index1];
      const value2 = data[index2];

      const t = fraction;
      const t2 = t * t;
      const t3 = t2 * t;

      return value1 * (1 - 3 * t2 + 2 * t3) + value2 * (3 * t2 - 2 * t3);
    }

    function updateTexture(redrawAll = false) {
      if (!gl || !textureData) return;

      const newWidth = effectiveWidth.value;
      const newHeight = props.height;

      if (redrawAll) {
        textureData.fill(0);

        for (const [columnIndex, measurement] of measurementHistory.value.entries()) {
          if (columnIndex >= newWidth) return;

          const x = newWidth - 1 - columnIndex;
          const dataLength = measurement.data.length;
          const scaleRatio =
            (virtualMaxDepth.value - props.minDepth) /
            (measurement.maxDepth - measurement.minDepth);

          if (scaleRatio !== lastScaleRatio) {
            yCoordCache = createYCoordMapping(newHeight, dataLength, scaleRatio);
            lastScaleRatio = scaleRatio;
          }

          for (let y = 0; y < newHeight; y++) {
            const [index1, index2] = yCoordCache.mapping[y];
            const fraction = yCoordCache.weights[y];

            if (index1 < dataLength) {
              const interpolatedValue = getInterpolatedValue(
                measurement.data,
                index1,
                index2,
                fraction
              );

              const color = props.getColorFromPalette(interpolatedValue, props.colorPalette);
              const index = (y * newWidth + x) * 4;

              textureData[index] = color[0];
              textureData[index + 1] = color[1];
              textureData[index + 2] = color[2];
              textureData[index + 3] = color[3] !== undefined ? color[3] : 255;
            }
          }
        }
      } else {
        // Just shift existing data
        for (let y = 0; y < newHeight; y++) {
          const rowOffset = y * newWidth * 4;
          textureData.copyWithin(rowOffset, rowOffset + 4, rowOffset + newWidth * 4);
        }

        // Clear last column
        for (let y = 0; y < newHeight; y++) {
          const index = (y * newWidth + newWidth - 1) * 4;
          textureData.fill(0, index, index + 4);
        }

        while (pendingUpdates.value.length > 0) {
          const measurement = pendingUpdates.value.shift();
          const dataLength = measurement.data.length;
          const scaleRatio =
            (virtualMaxDepth.value - props.minDepth) /
            (measurement.maxDepth - measurement.minDepth);

          if (scaleRatio !== lastScaleRatio) {
            yCoordCache = createYCoordMapping(newHeight, dataLength, scaleRatio);
            lastScaleRatio = scaleRatio;
          }

          for (let y = 0; y < newHeight; y++) {
            const [index1, index2] = yCoordCache.mapping[y];
            const fraction = yCoordCache.weights[y];

            if (index1 < dataLength) {
              const interpolatedValue = getInterpolatedValue(
                measurement.data,
                index1,
                index2,
                fraction
              );

              const color = props.getColorFromPalette(interpolatedValue, props.colorPalette);
              const index = (y * newWidth + newWidth - 1) * 4;

              textureData[index] = color[0];
              textureData[index + 1] = color[1];
              textureData[index + 2] = color[2];
              textureData[index + 3] = color[3] !== undefined ? color[3] : 255;
            }
          }
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
          const maxHistoricalDepth = Math.max(...measurementHistory.value.map((m) => m.maxDepth));
          virtualMaxDepth.value = Math.max(props.maxDepth, maxHistoricalDepth);
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

    function resizeCanvas() {
      if (!waterfallCanvas.value) return;

      const rect = waterfallCanvas.value.getBoundingClientRect();
      waterfallCanvas.value.width = rect.width;
      waterfallCanvas.value.height = rect.height;

      if (gl) {
        updateTextureSize();
        render();
      }
    }

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
    watch(() => props.colorPalette, updateWaterfall);
    watch(
      () => effectiveWidth.value,
      () => {
        updateTextureSize();
        updateWaterfall();
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