<template>
  <div class="w-full h-full aspect-square flex justify-center items-center">
    <canvas ref="canvas" class="w-full h-full"></canvas>
  </div>
</template>

<script setup lang="ts">
import { onKeyStroke } from '@vueuse/core';
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

interface SonarMeasurement {
  angle: number;
  data: Uint8Array;
  // Vehicle heading in degrees when this line was captured. Rows hold world
  // bearings so the plot stays anchored to the world while the vehicle turns.
  yaw?: number;
}

const props = withDefaults(
  defineProps<{
    numLines: number;
    lineLength?: number;
    measurement: SonarMeasurement | null;
    measurements?: SonarMeasurement[] | null;
    colorPalette: string;
    getColorFromPalette: (value: number, palette: string) => number[];
    startAngle: number;
    endAngle: number;
    yaw_angle: number;
    rotation?: number;
    // Maximum data radius in normalized 0..1 units. Should match
    // `Sonar360Mask`'s `maxRadius / 50` so the rendered data ends exactly at
    // the outermost depth arc instead of overshooting it by half a stroke.
    maxRadius?: number;
    isFrozen?: boolean;
  }>(),
  {
    lineLength: 1200,
    measurement: null,
    measurements: null,
    startAngle: 0,
    endAngle: 360,
    yaw_angle: 0,
    rotation: 0,
    maxRadius: 0.99,
    isFrozen: false,
  }
);

const canvas = ref<HTMLCanvasElement | null>(null);

// Rows per device gradian. One row per gradian left no slack: a bearing shifted
// by the vehicle heading rounded onto a row a neighbouring line already owned
// and skipped the rows in between, tearing the plot into arcs.
const ROWS_PER_GRADIAN = 4;
// A turning vehicle advances the world bearing by more than a gradian between
// lines. Painting the arc since the previous line closes that gap. A larger jump
// is a seek rather than a sweep, so the line only paints its own width.
const MAX_SPAN_GRADIANS = 4;

const totalRows = () => props.numLines * ROWS_PER_GRADIAN;

let gl: WebGLRenderingContext | null = null;
let renderFrame = 0;
let shaderProgram: WebGLProgram | null = null;
let texture: WebGLTexture | null = null;

let currentLineLength = props.lineLength;
let textureData = new Uint8Array(totalRows() * props.lineLength * 4);
const linePixels = new Uint8Array(props.lineLength * 4);
let spanPixels = new Uint8Array(props.lineLength * 4 * ROWS_PER_GRADIAN);
let lastWorldRow: number | null = null;
const currentAngle = ref(0);

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
  uniform float uMaxRadius;
  uniform float uYawAngle;
  uniform float uRotation;

  void main(void) {
    vec2 polar = vTextureCoord;
    float angle = atan(polar.y - 0.5, polar.x - 0.5) + 3.14159/2.0;
    // Screen bearing, measured clockwise from the top of the view. The view is
    // forward-up, so the grid is not rotated and this is the sonar's own frame.
    float angleDegrees = mod(degrees(angle), 360.0);
    float radius = length(polar - 0.5) * 2.0;

    bool inSector = uStartAngle <= uEndAngle
      ? (angleDegrees >= uStartAngle && angleDegrees <= uEndAngle)
      : (angleDegrees >= uStartAngle || angleDegrees <= uEndAngle);

    // Rows hold world bearings, so the current heading is added back to find the
    // row under this pixel. Turning the vehicle then swings the whole picture
    // while the grid stays on its forward. Partial sectors are fitted to the view
    // and remain in the sonar frame.
    float headingOffset = (uStartAngle == 0.0 && uEndAngle == 360.0) ? uYawAngle : 0.0;

    if (radius > uMaxRadius || !inSector) {
      gl_FragColor = vec4(0.1, 0.1, 0.1, 0.0); // Transparent background
    } else {
      float texAngle = fract((angleDegrees + headingOffset + uRotation) / 360.0 + 0.5);
      gl_FragColor = texture2D(uSampler, vec2(radius / uMaxRadius, texAngle));
    }
  }
`;

const initWebGL = () => {
  if (!canvas.value) return;

  gl = canvas.value.getContext('webgl', {
    alpha: true,
    premultipliedAlpha: false,
    preserveDrawingBuffer: true,
  });

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

  const positions = new Float32Array([-1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0]);
  const textureCoords = new Float32Array([0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

  const positionBuffer = gl.createBuffer();
  const textureCoordBuffer = gl.createBuffer();

  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);

  const vertexPosition = gl.getAttribLocation(shaderProgram, 'aVertexPosition');
  gl.enableVertexAttribArray(vertexPosition);
  gl.vertexAttribPointer(vertexPosition, 2, gl.FLOAT, false, 0, 0);

  gl.bindBuffer(gl.ARRAY_BUFFER, textureCoordBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, textureCoords, gl.STATIC_DRAW);

  const textureCoord = gl.getAttribLocation(shaderProgram, 'aTextureCoord');
  gl.enableVertexAttribArray(textureCoord);
  gl.vertexAttribPointer(textureCoord, 2, gl.FLOAT, false, 0, 0);
};

const resizeTextureBuffers = (newLineLength: number) => {
  if (newLineLength === currentLineLength) return;

  currentLineLength = newLineLength;
  textureData = new Uint8Array(totalRows() * newLineLength * 4);
  lastWorldRow = null;

  // Don't rebuild when the frame is frozen
  if (gl && texture && !props.isFrozen) {
    uploadFullTexture();
    render();
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
    currentLineLength,
    totalRows(),
    0,
    gl.RGBA,
    gl.UNSIGNED_BYTE,
    textureData
  );
};

const uploadFullTexture = () => {
  if (!gl || !texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    gl.RGBA,
    currentLineLength,
    totalRows(),
    0,
    gl.RGBA,
    gl.UNSIGNED_BYTE,
    textureData
  );
};

const render = () => {
  if (!gl || !shaderProgram || !texture) return;

  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.useProgram(shaderProgram);

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(gl.getUniformLocation(shaderProgram, 'uSampler'), 0);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uStartAngle'), props.startAngle);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uEndAngle'), props.endAngle);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uMaxRadius'), props.maxRadius);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uYawAngle'), props.yaw_angle);
  gl.uniform1f(gl.getUniformLocation(shaderProgram, 'uRotation'), props.rotation);

  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
};

// Several samples can land in one display frame. Upload all of them, then draw once.
const scheduleRender = () => {
  if (renderFrame) return;
  renderFrame = requestAnimationFrame(() => {
    renderFrame = 0;
    render();
  });
};

const wrapRow = (row: number) => {
  const rows = totalRows();
  return ((row % rows) + rows) % rows;
};

// Copies the line into every row it covers, then uploads the contiguous runs.
// Filling the whole arc is what keeps a turning sweep free of gaps.
const paintRows = (rowsToPaint: number[]) => {
  const stride = currentLineLength * 4;
  const sorted = [...new Set(rowsToPaint)].sort((a, b) => a - b);

  const runs: Array<[number, number]> = [];
  let start = sorted[0];
  let previous = sorted[0];
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i] === previous + 1) {
      previous = sorted[i];
      continue;
    }
    runs.push([start, previous]);
    start = sorted[i];
    previous = sorted[i];
  }
  runs.push([start, previous]);

  for (const [runStart, runEnd] of runs) {
    const height = runEnd - runStart + 1;
    const needed = height * stride;
    if (spanPixels.length < needed) spanPixels = new Uint8Array(needed);
    for (let r = 0; r < height; r++) {
      spanPixels.set(linePixels, r * stride);
    }
    textureData.set(spanPixels.subarray(0, needed), runStart * stride);
  }

  if (props.isFrozen || !gl || !texture) return;

  gl.bindTexture(gl.TEXTURE_2D, texture);
  for (const [runStart, runEnd] of runs) {
    const height = runEnd - runStart + 1;
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      runStart,
      currentLineLength,
      height,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      spanPixels.subarray(0, height * stride)
    );
  }
};

const writeLine = (angle: number, newData: Uint8Array, yaw = 0) => {
  if (newData.length !== currentLineLength) {
    resizeTextureBuffers(newData.length);
  }

  for (let i = 0; i < newData.length; i++) {
    const color = props.getColorFromPalette(newData[i], props.colorPalette);
    const index = i * 4;

    linePixels[index] = color[0]; // R
    linePixels[index + 1] = color[1]; // G
    linePixels[index + 2] = color[2]; // B
    linePixels[index + 3] = color[3]; // A
  }

  const isFullCircle = props.startAngle === 0 && props.endAngle === 360;
  // Gradians, in the world frame. The device reports the angle in the sonar's
  // frame, so the heading at capture time turns it into a world bearing.
  const worldGradian = isFullCircle ? angle + (yaw * props.numLines) / 360 : angle;
  const endRow = wrapRow(Math.round(worldGradian * ROWS_PER_GRADIAN));

  const ownHalf = Math.floor(ROWS_PER_GRADIAN / 2);
  const ownRows: number[] = [];
  for (let r = -ownHalf; r < ROWS_PER_GRADIAN - ownHalf; r++) {
    ownRows.push(wrapRow(endRow + r));
  }

  const rowsToPaint = [...ownRows];
  if (isFullCircle && lastWorldRow !== null) {
    const forward = wrapRow(endRow - lastWorldRow);
    const back = wrapRow(lastWorldRow - endRow);
    const span = Math.min(forward, back);
    if (span > 0 && span <= MAX_SPAN_GRADIANS * ROWS_PER_GRADIAN) {
      // Bridge the arc swept since the previous line so no row is left empty.
      const step = forward <= back ? 1 : -1;
      for (let r = 1; r <= span; r++) {
        rowsToPaint.push(wrapRow(lastWorldRow + step * r));
      }
    }
  }
  lastWorldRow = endRow;

  currentAngle.value = angle;
  paintRows(rowsToPaint);
};

const resizeCanvas = () => {
  if (!canvas.value) return;

  const dpr = window.devicePixelRatio || 1;
  // clientWidth is the layout box. getBoundingClientRect would report the larger
  // axis-aligned box of the head-down flip and stretch the buffer.
  const width = canvas.value.clientWidth;
  const height = canvas.value.clientHeight;
  if (width === 0 || height === 0) return;
  canvas.value.width = width * dpr;
  canvas.value.height = height * dpr;

  if (gl) {
    gl.viewport(0, 0, canvas.value.width, canvas.value.height);
    render();
  }
};

const clearShaderContent = () => {
  textureData.fill(0);
  lastWorldRow = null;

  if (gl && texture) {
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      currentLineLength,
      totalRows(),
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      textureData
    );
    render();
  }
};

onKeyStroke(['r', 'R'], () => {
  clearShaderContent();
});

onMounted(() => {
  nextTick(() => {
    initWebGL();
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeCanvas);
  if (renderFrame) cancelAnimationFrame(renderFrame);
});

watch(
  () => props.isFrozen,
  (frozen) => {
    if (frozen) return;
    uploadFullTexture();
    render();
  }
);

watch(
  () => props.measurements,
  (lines) => {
    if (!lines?.length) return;
    for (const line of lines) {
      if (line?.data) writeLine(line.angle, line.data, line.yaw ?? props.yaw_angle);
    }
    scheduleRender();
  }
);

watch(
  () => props.measurement,
  (line) => {
    if (props.measurements?.length) return;
    if (line?.data) {
      writeLine(line.angle, line.data, line.yaw ?? props.yaw_angle);
      scheduleRender();
    }
  }
);

watch(
  [
    () => props.startAngle,
    () => props.endAngle,
    () => props.maxRadius,
    () => props.yaw_angle,
    () => props.rotation,
  ],
  (next, previous) => {
    // A changed sector swaps the row frame between sonar and world bearings, so
    // the previous world row would bridge a meaningless arc.
    if (next[0] !== previous?.[0] || next[1] !== previous?.[1]) {
      lastWorldRow = null;
    }
    scheduleRender();
  }
);
</script>
