<template>
  <div class="w-full h-full aspect-square flex justify-center items-center">
    <canvas ref="canvas" class="w-full h-full"></canvas>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

interface SonarMeasurement {
  angle: number;
  data: Uint8Array;
}

const props = withDefaults(
  defineProps<{
    numLines: number;
    lineLength: number;
    measurement: SonarMeasurement | null;
    colorPalette: string;
    getColorFromPalette: (value: number, palette: string) => number[];
    startAngle: number;
    endAngle: number;
    yaw_angle: number;
  }>(),
  {
    measurement: null,
    startAngle: 0,
    endAngle: 360,
    yaw_angle: 0,
  }
);

const canvas = ref<HTMLCanvasElement | null>(null);

let gl: WebGLRenderingContext | null = null;
let shaderProgram: WebGLProgram | null = null;
let texture: WebGLTexture | null = null;

const textureData = ref(new Uint8Array(props.numLines * props.lineLength * 4));
const currentAngle = ref(0);
const previousYaw = ref(0);

const vsSource = `
	attribute vec4 aVertexPosition;
	attribute vec2 aTextureCoord;
	varying vec2 vTextureCoord;
	void main(void) {
	  gl_Position = aVertexPosition;
	  vTextureCoord = aTextureCoord;
	}
  `;

const fsSource = `
  precision highp float;
  varying vec2 vTextureCoord;
  uniform sampler2D uSampler;
  uniform float uStartAngle;
  uniform float uEndAngle;

  void main(void) {
    vec2 polar = vTextureCoord;
    float angle = atan(polar.y - 0.5, polar.x - 0.5) + 3.14159/2.0;
    float angleDegrees = degrees(angle);
    if (angleDegrees < 0.0) angleDegrees += 360.0;
    float radius = length(polar - 0.5) * 2.0;

    bool inSector = uStartAngle <= uEndAngle
      ? (angleDegrees >= uStartAngle && angleDegrees <= uEndAngle)
      : (angleDegrees >= uStartAngle || angleDegrees <= uEndAngle);

    if (radius > 1.0 || !inSector) {
      gl_FragColor = vec4(0.1, 0.1, 0.1, 0.0); // Transparent background
    } else {
      float texAngle = (angle + 3.14159) / (2.0 * 3.14159);
      if (texAngle > 1.0) {
        texAngle -= 1.0;
      }
      gl_FragColor = texture2D(uSampler, vec2(radius, texAngle));
    }
  }
`;

const initWebGL = () => {
  const canvasElement = canvas.value;
  if (!canvasElement) return;

  gl = canvasElement.getContext('webgl');
  if (!gl) {
    console.error('Unable to initialize WebGL.');
    return;
  }

  gl.clearColor(0.0, 0.0, 0.0, 0.0);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  shaderProgram = initShaderProgram(gl, vsSource, fsSource);
  setupBuffers();
  setupTexture();
  resizeCanvas();
};

const loadShader = (
  gl: WebGLRenderingContext,
  type: number,
  source: string
): WebGLShader | null => {
  const shader = gl.createShader(type);
  if (!shader) {
    console.error('Unable to create shader.');
    return null;
  }

  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error(`An error occurred compiling the shaders: ${gl.getShaderInfoLog(shader)}`);
    gl.deleteShader(shader);
    return null;
  }

  return shader;
};
const initShaderProgram = (
  gl: WebGLRenderingContext,
  vsSource: string,
  fsSource: string
): WebGLProgram | null => {
  const vertexShader = loadShader(gl, gl.VERTEX_SHADER, vsSource);
  const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, fsSource);

  if (!vertexShader || !fragmentShader) {
    console.error('Failed to load shaders.');
    return null;
  }

  const shaderProgram = gl.createProgram();
  if (!shaderProgram) {
    console.error('Unable to create shader program.');
    return null;
  }

  gl.attachShader(shaderProgram, vertexShader);
  gl.attachShader(shaderProgram, fragmentShader);
  gl.linkProgram(shaderProgram);

  if (!gl.getProgramParameter(shaderProgram, gl.LINK_STATUS)) {
    console.error(
      `Unable to initialize the shader program: ${gl.getProgramInfoLog(shaderProgram)}`
    );
    return null;
  }

  return shaderProgram;
};

const setupBuffers = () => {
  if (!gl || !shaderProgram) return;

  const positionBuffer = gl.createBuffer();
  if (!positionBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  const positions = [-1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0];
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.STATIC_DRAW);

  const textureCoordBuffer = gl.createBuffer();
  if (!textureCoordBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  const textureCoordinates = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(textureCoordinates), gl.STATIC_DRAW);

  const vertexPosition = gl.getAttribLocation(shaderProgram, 'aVertexPosition');
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.vertexAttribPointer(vertexPosition, 2, gl.FLOAT, false, 0, 0);
  gl.enableVertexAttribArray(vertexPosition);

  const textureCoord = gl.getAttribLocation(shaderProgram, 'aTextureCoord');
  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  gl.vertexAttribPointer(textureCoord, 2, gl.FLOAT, false, 0, 0);
  gl.enableVertexAttribArray(textureCoord);
};

const rotateTextureData = (lineOffset: number) => {
  const tempData = new Uint8Array(textureData.value);

  const bytesPerLine = props.lineLength * 4;

  for (let i = 0; i < props.numLines; i++) {
    let sourceLineIndex = i - lineOffset;

    if (sourceLineIndex < 0) {
      sourceLineIndex += props.numLines;
    } else if (sourceLineIndex >= props.numLines) {
      sourceLineIndex -= props.numLines;
    }

    const destStart = i * bytesPerLine;
    const sourceStart = sourceLineIndex * bytesPerLine;
    textureData.value.set(tempData.slice(sourceStart, sourceStart + bytesPerLine), destStart);
  }
};

const setupTexture = () => {
  if (!gl) return;

  texture = gl.createTexture();
  if (!texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    gl.RGBA,
    props.lineLength,
    props.numLines,
    0,
    gl.RGBA,
    gl.UNSIGNED_BYTE,
    textureData.value
  );
};

const updateTexture = () => {
  if (!gl || !texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texSubImage2D(
    gl.TEXTURE_2D,
    0,
    0,
    0,
    props.lineLength,
    props.numLines,
    gl.RGBA,
    gl.UNSIGNED_BYTE,
    textureData.value
  );
};

const render = () => {
  if (!gl || !shaderProgram) return;

  gl.clear(gl.COLOR_BUFFER_BIT);

  gl.useProgram(shaderProgram);

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(gl.getUniformLocation(shaderProgram, 'uSampler'), 0);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uStartAngle'), props.startAngle);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uEndAngle'), props.endAngle);

  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
};

const updateSonarData = (angle: number, newData: Uint8Array) => {
  const yawDiff = props.yaw_angle - previousYaw.value;
  if (yawDiff !== 0) {
    const linesPerDegree = props.numLines / 360;
    const lineOffset = Math.round(yawDiff * linesPerDegree);
    rotateTextureData(lineOffset);
    previousYaw.value = props.yaw_angle;
  }

  const lineIndex = angle % props.numLines;
  const start = lineIndex * props.lineLength * 4;
  const maxDataLength = Math.floor((textureData.value.length - start) / 4);

  let processedData: Uint8Array;
  if (start + newData.length * 4 > textureData.value.length) {
    console.warn(
      `Data exceeds texture bounds. Trimming data from ${newData.length} to ${maxDataLength} elements.`
    );
    processedData = newData.slice(0, maxDataLength);
  } else {
    processedData = newData;
  }

  for (let i = 0; i < processedData.length; i++) {
    const color = props.getColorFromPalette(processedData[i], props.colorPalette);
    const index = start + i * 4;
    if (index + 3 < textureData.value.length) {
      textureData.value.set(color, index);
    } else {
      console.warn(`Index out of bounds: ${index}. Skipping this pixel.`);
      break;
    }
  }

  currentAngle.value = angle;
  updateTexture();
  render();
};

const resizeCanvas = () => {
  if (canvas.value) {
    canvas.value.width = canvas.value.clientWidth;
    canvas.value.height = canvas.value.clientHeight;
    if (gl) {
      gl.viewport(0, 0, canvas.value.width, canvas.value.height);
      render();
    }
  }
};

onMounted(() => {
  nextTick(() => {
    initWebGL();
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeCanvas);
});

watch(
  () => props.yaw_angle,
  (newYaw) => {
    if (newYaw !== previousYaw.value) {
      const yawDiff = newYaw - previousYaw.value;
      const linesPerDegree = props.numLines / 360;
      const lineOffset = Math.round(yawDiff * linesPerDegree);
      rotateTextureData(lineOffset);
      previousYaw.value = newYaw;
      updateTexture();
      render();
    }
  }
);

watch(
  () => props.measurement,
  (newMeasurement) => {
    if (newMeasurement) {
      try {
        updateSonarData(newMeasurement.angle, newMeasurement.data);
      } catch (error) {
        console.error('Error updating sonar data:', error);
      }
    }
  },
  { deep: true }
);

watch([() => props.startAngle, () => props.endAngle], () => {
  render();
});
</script>
