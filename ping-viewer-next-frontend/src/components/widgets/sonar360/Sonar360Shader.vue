<template>
  <div class="w-full h-full aspect-square flex justify-center items-center">
    <canvas ref="canvas" class="w-full h-full"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue';

interface SonarMeasurement {
  angle: number;
  data: Uint8Array;
}

const props = defineProps<{
  numLines: number;
  lineLength: number;
  measurement: SonarMeasurement | null;
}>();

const canvas = ref<HTMLCanvasElement | null>(null);

let gl: WebGLRenderingContext | null = null;
let shaderProgram: WebGLProgram | null = null;
let texture: WebGLTexture | null = null;

const intensityData = new Uint8Array(props.numLines * props.lineLength);
let currentAngle = 0;

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

void main(void) {
  vec2 polar = vTextureCoord;
  float angle = atan(polar.y - 0.5, polar.x - 0.5) + 3.14159/2.0;
  float radius = length(polar - 0.5) * 2.0;

  if (radius > 1.0) {
    gl_FragColor = vec4(0.1, 0.1, 0.1, 0.0); // Transparent background
  } else {
    float texAngle = (angle + 3.14159) / (2.0 * 3.14159);
    if (texAngle > 1.0) {
      texAngle -= 1.0;
    }
    float intensity = texture2D(uSampler, vec2(radius, texAngle)).r;
    vec4 baseColor = vec4(intensity, intensity, intensity, 1.0);

    gl_FragColor = baseColor;
  }
}
`;

const initWebGL = () => {
  // ... [This function remains unchanged]
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

const loadShader = (gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null => {
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

const initShaderProgram = (gl: WebGLRenderingContext, vsSource: string, fsSource: string): WebGLProgram | null => {
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
    console.error(`Unable to initialize the shader program: ${gl.getProgramInfoLog(shaderProgram)}`);
    return null;
  }

  return shaderProgram;
};

const setupBuffers = () => {
  if (!gl || !shaderProgram) return;

  const positionBuffer = gl.createBuffer();
  if (!positionBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  const positions = [
    -1.0,  1.0,
     1.0,  1.0,
    -1.0, -1.0,
     1.0, -1.0,
  ];
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(positions), gl.STATIC_DRAW);

  const textureCoordBuffer = gl.createBuffer();
  if (!textureCoordBuffer) return;

  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  const textureCoordinates = [
    0.0,  0.0,
    1.0,  0.0,
    0.0,  1.0,
    1.0,  1.0,
  ];
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

const setupTexture = () => {
  if (!gl) return;

  texture = gl.createTexture();
  if (!texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
};

const updateTexture = () => {
  if (!gl || !texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, props.lineLength, props.numLines, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, intensityData);
};

const render = () => {
  if (!gl || !shaderProgram) return;

  gl.clear(gl.COLOR_BUFFER_BIT);

  gl.useProgram(shaderProgram);

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(gl.getUniformLocation(shaderProgram, 'uSampler'), 0);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uAngle'), currentAngle);

  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
};

const updateSonarData = (angle: number, newData: Uint8Array) => {
  const lineIndex = angle;
  const start = lineIndex * props.lineLength;
  intensityData.set(newData, start);
  currentAngle = angle;
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

watch(() => props.measurement, (newMeasurement) => {
  if (newMeasurement) {
    console.log('Received new sonar measurement at angle:', newMeasurement.angle);
    updateSonarData(newMeasurement.angle, newMeasurement.data);
  }
});
</script>