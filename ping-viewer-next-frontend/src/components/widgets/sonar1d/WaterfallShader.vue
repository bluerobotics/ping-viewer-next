<template>
  <canvas ref="canvasRef" class="w-full h-full block" />
</template>

<script setup lang="ts">
import { onKeyStroke } from '@vueuse/core';
import { onMounted, onBeforeUnmount, ref, watch } from 'vue';

const props = withDefaults(
  defineProps<{
    width: number;
    height: number;
    maxDepth: number;
    minDepth: number;
    columnCount?: number;
    sensorData: number[];
    colorPalette: string;
    getColorFromPalette: (value: number, palette: string) => number[];
  }>(),
  { columnCount: 200 }
);

defineEmits<{ 'update:columnCount': [value: number] }>();

const canvasRef = ref<HTMLCanvasElement | null>(null);

// Vertex shader: renders a fullscreen quad. Positions are in clip-space [-1,1],
// mapped to UVs [0,1] for texture sampling.
const VERT_SRC = `#version 300 es
in vec2 a_position;
out vec2 v_uv;

void main() {
  v_uv = a_position * 0.5 + 0.5;
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

// Fragment shader: samples sonar history from a circular-buffer data texture
// (R8 – single channel intensity) and maps the scalar through a 1-D RGBA
// palette texture for final color output.
//
// Coordinate conventions:
//   Horizontal: screen left = oldest ping, right = newest ping.
//   Vertical:   screen top = shallow (sensorData index 0),
//               screen bottom = deep (sensorData last index).
const FRAG_SRC = `#version 300 es
precision highp float;

in vec2 v_uv;
out vec4 fragColor;

uniform sampler2D u_data;
uniform sampler2D u_palette;
uniform sampler2D u_depth;    // per-column maxDepth (R32F, columnCount × 1)

uniform int   u_cols;         // total column slots (history length)
uniform int   u_startCol;     // texture column that holds the oldest valid data
uniform int   u_validCols;    // how many columns currently contain data
uniform float u_minDepth;     // display minimum depth (metres)
uniform float u_maxDepth;     // display maximum depth (virtualMaxDepth, metres)

void main() {
  int col = min(int(floor(v_uv.x * float(u_cols))), u_cols - 1);

  int empty = u_cols - u_validCols;
  if (col < empty) {
    fragColor = vec4(0.0, 0.0, 0.0, 0.0);
    return;
  }

  int dataIdx = col - empty;
  int texCol  = (u_startCol + dataIdx) % u_cols;
  float tx = (float(texCol) + 0.5) / float(u_cols);

  // ty: 0.0 = top/shallow, 1.0 = bottom/deep
  float ty = 1.0 - v_uv.y;

  // Per-column depth scaling: each column's data covers [minDepth, colMaxDepth],
  // but the display spans [minDepth, virtualMaxDepth]. Pixels below the column's
  // own range are transparent.
  float colMaxDepth  = texture(u_depth, vec2(tx, 0.5)).r;
  float displayRange = u_maxDepth - u_minDepth;
  float colRange     = colMaxDepth - u_minDepth;
  float ratio        = displayRange > 0.0 ? colRange / displayRange : 1.0;

  if (ty > ratio) {
    fragColor = vec4(0.0, 0.0, 0.0, 0.0);
    return;
  }

  // Rescale ty into the data texture's full [0,1] range for this column
  float dataTy = ratio > 0.0 ? ty / ratio : 0.0;

  float intensity = texture(u_data, vec2(tx, dataTy)).r;
  fragColor = texture(u_palette, vec2(intensity, 0.5));
}
`;

let gl: WebGL2RenderingContext | null = null;
let program: WebGLProgram | null = null;
let vao: WebGLVertexArrayObject | null = null;
let vbo: WebGLBuffer | null = null;
let dataTexture: WebGLTexture | null = null;
let paletteTexture: WebGLTexture | null = null;
let depthTexture: WebGLTexture | null = null;

let locCols: WebGLUniformLocation | null = null;
let locStartCol: WebGLUniformLocation | null = null;
let locValidCols: WebGLUniformLocation | null = null;
let locData: WebGLUniformLocation | null = null;
let locPalette: WebGLUniformLocation | null = null;
let locDepth: WebGLUniformLocation | null = null;
let locMinDepth: WebGLUniformLocation | null = null;
let locMaxDepth: WebGLUniformLocation | null = null;

let writeIndex = 0;
let columnsWritten = 0;
let currentBinCount = 0;
let needsRender = false;
let rafId = 0;
let resizeObserver: ResizeObserver | null = null;
let virtualMaxDepth = 0;

// Reusable buffers – avoid per-frame allocation
let columnBuffer: Uint8Array | null = null;
let depthValues: Float32Array | null = null;
const depthUploadBuf = new Float32Array(1);

function compileShader(type: number, src: string): WebGLShader {
  const shader = gl!.createShader(type)!;
  gl!.shaderSource(shader, src);
  gl!.compileShader(shader);
  if (!gl!.getShaderParameter(shader, gl!.COMPILE_STATUS)) {
    const log = gl!.getShaderInfoLog(shader);
    gl!.deleteShader(shader);
    throw new Error(`Shader compile error: ${log}`);
  }
  return shader;
}

function buildProgram(vSrc: string, fSrc: string): WebGLProgram {
  const vs = compileShader(gl!.VERTEX_SHADER, vSrc);
  const fs = compileShader(gl!.FRAGMENT_SHADER, fSrc);
  const prog = gl!.createProgram()!;
  gl!.attachShader(prog, vs);
  gl!.attachShader(prog, fs);
  gl!.linkProgram(prog);
  if (!gl!.getProgramParameter(prog, gl!.LINK_STATUS)) {
    const log = gl!.getProgramInfoLog(prog);
    gl!.deleteProgram(prog);
    throw new Error(`Program link error: ${log}`);
  }
  gl!.deleteShader(vs);
  gl!.deleteShader(fs);
  return prog;
}

function createDataTexture(width: number, height: number) {
  if (dataTexture) gl!.deleteTexture(dataTexture);

  dataTexture = gl!.createTexture();
  gl!.activeTexture(gl!.TEXTURE0);
  gl!.bindTexture(gl!.TEXTURE_2D, dataTexture);

  // R8: one byte per texel, normalized to 0.0–1.0 in the shader
  gl!.texImage2D(gl!.TEXTURE_2D, 0, gl!.R8, width, height, 0, gl!.RED, gl!.UNSIGNED_BYTE, null);

  // LINEAR on the depth axis gives smooth gradients between bins.
  // Because we sample at exact texel centres on X, LINEAR won't bleed
  // across columns in the circular buffer.
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MIN_FILTER, gl!.LINEAR);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MAG_FILTER, gl!.LINEAR);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_S, gl!.CLAMP_TO_EDGE);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_T, gl!.CLAMP_TO_EDGE);

  // Per-column depth metadata (R32F, width × 1).
  // Stores each column's maxDepth so the shader can scale the vertical
  // extent proportionally to the display range.
  if (depthTexture) gl!.deleteTexture(depthTexture);
  depthTexture = gl!.createTexture();
  gl!.activeTexture(gl!.TEXTURE2);
  gl!.bindTexture(gl!.TEXTURE_2D, depthTexture);
  gl!.texImage2D(gl!.TEXTURE_2D, 0, gl!.R32F, width, 1, 0, gl!.RED, gl!.FLOAT, null);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MIN_FILTER, gl!.NEAREST);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MAG_FILTER, gl!.NEAREST);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_S, gl!.CLAMP_TO_EDGE);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_T, gl!.CLAMP_TO_EDGE);

  writeIndex = 0;
  columnsWritten = 0;
  currentBinCount = height;
  virtualMaxDepth = 0;
  columnBuffer = new Uint8Array(height);
  depthValues = new Float32Array(width);
}

function buildPaletteTexture() {
  const SIZE = 256;
  const rgba = new Uint8Array(SIZE * 4);

  for (let i = 0; i < SIZE; i++) {
    const c = props.getColorFromPalette(i, props.colorPalette);
    const off = i * 4;
    rgba[off] = c[0];
    rgba[off + 1] = c[1];
    rgba[off + 2] = c[2];
    rgba[off + 3] = c[3] ?? 255;
  }

  if (!paletteTexture) paletteTexture = gl!.createTexture();
  gl!.activeTexture(gl!.TEXTURE1);
  gl!.bindTexture(gl!.TEXTURE_2D, paletteTexture);
  gl!.texImage2D(gl!.TEXTURE_2D, 0, gl!.RGBA8, SIZE, 1, 0, gl!.RGBA, gl!.UNSIGNED_BYTE, rgba);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MIN_FILTER, gl!.LINEAR);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_MAG_FILTER, gl!.LINEAR);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_S, gl!.CLAMP_TO_EDGE);
  gl!.texParameteri(gl!.TEXTURE_2D, gl!.TEXTURE_WRAP_T, gl!.CLAMP_TO_EDGE);
}

function pushColumn(data: number[]) {
  if (!gl || !dataTexture || !depthTexture) return;
  const bins = data.length;
  if (bins === 0) return;

  // Recreate the data texture when the number of depth bins changes
  if (bins !== currentBinCount) {
    createDataTexture(props.columnCount, bins);
  }

  // Clamp intensity values into 0–255 and pack into reusable buffer
  const col = columnBuffer!;
  for (let i = 0; i < bins; i++) {
    col[i] = data[i] < 0 ? 0 : data[i] > 255 ? 255 : (data[i] + 0.5) | 0;
  }

  // Upload intensity column
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, dataTexture);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, writeIndex, 0, 1, bins, gl.RED, gl.UNSIGNED_BYTE, col);

  // Record this column's maxDepth in the depth metadata texture
  depthValues![writeIndex] = props.maxDepth;
  depthUploadBuf[0] = props.maxDepth;
  gl.activeTexture(gl.TEXTURE2);
  gl.bindTexture(gl.TEXTURE_2D, depthTexture);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, writeIndex, 0, 1, 1, gl.RED, gl.FLOAT, depthUploadBuf);

  writeIndex = (writeIndex + 1) % props.columnCount;
  if (columnsWritten < props.columnCount) columnsWritten++;

  // Recompute virtualMaxDepth from all valid columns
  let maxD = props.maxDepth;
  const valid = Math.min(columnsWritten, props.columnCount);
  for (let i = 0; i < valid; i++) {
    if (depthValues![i] > maxD) maxD = depthValues![i];
  }
  virtualMaxDepth = maxD;

  scheduleRender();
}

function render() {
  rafId = 0;
  if (!gl || !program || !needsRender) return;
  needsRender = false;

  const canvas = canvasRef.value;
  if (!canvas || canvas.width === 0 || canvas.height === 0) return;

  gl.viewport(0, 0, canvas.width, canvas.height);
  gl.clearColor(0, 0, 0, 0);
  gl.clear(gl.COLOR_BUFFER_BIT);

  if (columnsWritten === 0) return;

  gl.useProgram(program);

  // Bind data texture to unit 0
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, dataTexture);
  gl.uniform1i(locData, 0);

  // Bind palette texture to unit 1
  gl.activeTexture(gl.TEXTURE1);
  gl.bindTexture(gl.TEXTURE_2D, paletteTexture);
  gl.uniform1i(locPalette, 1);

  // Bind depth metadata texture to unit 2
  gl.activeTexture(gl.TEXTURE2);
  gl.bindTexture(gl.TEXTURE_2D, depthTexture);
  gl.uniform1i(locDepth, 2);

  // Circular buffer uniforms
  const validCols = Math.min(columnsWritten, props.columnCount);
  const startCol =
    (((writeIndex - validCols) % props.columnCount) + props.columnCount) % props.columnCount;

  gl.uniform1i(locCols, props.columnCount);
  gl.uniform1i(locStartCol, startCol);
  gl.uniform1i(locValidCols, validCols);
  gl.uniform1f(locMinDepth, props.minDepth);
  gl.uniform1f(locMaxDepth, virtualMaxDepth);

  gl.bindVertexArray(vao);
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  gl.bindVertexArray(null);
}

function scheduleRender() {
  needsRender = true;
  if (!rafId) {
    rafId = requestAnimationFrame(render);
  }
}

function clearWaterfall() {
  if (!gl || !dataTexture || !depthTexture) return;

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, dataTexture);
  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    gl.R8,
    depthValues!.length,
    currentBinCount,
    0,
    gl.RED,
    gl.UNSIGNED_BYTE,
    null
  );

  gl.activeTexture(gl.TEXTURE2);
  gl.bindTexture(gl.TEXTURE_2D, depthTexture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, depthValues!.length, 1, 0, gl.RED, gl.FLOAT, null);

  writeIndex = 0;
  columnsWritten = 0;
  virtualMaxDepth = 0;
  depthValues!.fill(0);

  scheduleRender();
}

onKeyStroke(['r', 'R'], clearWaterfall);

function syncCanvasSize() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const dpr = window.devicePixelRatio || 1;
  const displayW = Math.round(canvas.clientWidth * dpr);
  const displayH = Math.round(canvas.clientHeight * dpr);

  if (canvas.width !== displayW || canvas.height !== displayH) {
    canvas.width = displayW;
    canvas.height = displayH;
    scheduleRender();
  }
}

onMounted(() => {
  const canvas = canvasRef.value!;
  gl = canvas.getContext('webgl2', { antialias: false, alpha: true, premultipliedAlpha: false });
  if (!gl) {
    console.error('WaterfallShader: WebGL 2 not available');
    return;
  }

  // R8 textures have 1 byte per texel; the default UNPACK_ALIGNMENT of 4
  // would misalign every row when uploading single-pixel-wide columns.
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);

  // Shader program
  program = buildProgram(VERT_SRC, FRAG_SRC);
  locCols = gl.getUniformLocation(program, 'u_cols');
  locStartCol = gl.getUniformLocation(program, 'u_startCol');
  locValidCols = gl.getUniformLocation(program, 'u_validCols');
  locData = gl.getUniformLocation(program, 'u_data');
  locPalette = gl.getUniformLocation(program, 'u_palette');
  locDepth = gl.getUniformLocation(program, 'u_depth');
  locMinDepth = gl.getUniformLocation(program, 'u_minDepth');
  locMaxDepth = gl.getUniformLocation(program, 'u_maxDepth');

  // Fullscreen quad (triangle strip: BL → BR → TL → TR)
  const positions = new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]);
  vao = gl.createVertexArray();
  gl.bindVertexArray(vao);
  vbo = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
  gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
  const aPos = gl.getAttribLocation(program, 'a_position');
  gl.enableVertexAttribArray(aPos);
  gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
  gl.bindVertexArray(null);

  // Allocate textures
  const initialBins = props.sensorData.length || 200;
  createDataTexture(props.columnCount, initialBins);
  buildPaletteTexture();

  // Match canvas backing-store resolution to CSS layout size
  syncCanvasSize();

  // Observe container resize
  resizeObserver = new ResizeObserver(syncCanvasSize);
  resizeObserver.observe(canvas);

  // Ingest initial data if available
  if (props.sensorData && props.sensorData.length > 0) {
    pushColumn(props.sensorData);
  }
});

onBeforeUnmount(() => {
  if (rafId) cancelAnimationFrame(rafId);
  resizeObserver?.disconnect();

  if (gl) {
    if (dataTexture) gl.deleteTexture(dataTexture);
    if (paletteTexture) gl.deleteTexture(paletteTexture);
    if (depthTexture) gl.deleteTexture(depthTexture);
    if (vbo) gl.deleteBuffer(vbo);
    if (vao) gl.deleteVertexArray(vao);
    if (program) gl.deleteProgram(program);
    gl.getExtension('WEBGL_lose_context')?.loseContext();
  }

  gl = null;
  program = null;
  vao = null;
  vbo = null;
  dataTexture = null;
  paletteTexture = null;
  depthTexture = null;
  columnBuffer = null;
  depthValues = null;
});

watch(
  () => props.sensorData,
  (data) => {
    if (data && data.length > 0) pushColumn(data);
  }
);

watch([() => props.colorPalette, () => props.getColorFromPalette], () => {
  if (gl) {
    buildPaletteTexture();
    scheduleRender();
  }
});

watch(
  () => props.columnCount,
  (next, prev) => {
    if (gl && next !== prev) {
      createDataTexture(next, currentBinCount || 200);
      scheduleRender();
    }
  }
);

watch([() => props.width, () => props.height], syncCanvasSize);
</script>
