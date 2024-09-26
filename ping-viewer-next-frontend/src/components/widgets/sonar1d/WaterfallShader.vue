<template>
	<div ref="waterfallContainer" class="waterfall-container w-full h-full">
	  <canvas ref="waterfallCanvas" class="w-full h-full"></canvas>
	</div>
  </template>

  <script>
import { computed, onMounted, onUnmounted, ref, watch } from "vue";

export default {
	name: "WaterfallShader",
	props: {
		width: { type: Number, default: 500 },
		height: { type: Number, default: 400 },
		columnCount: { type: Number, default: 200 },
		sensorData: { type: Array, default: () => [] },
		colorPalette: { type: String, default: "ocean" },
	},
	emits: ["update:columnCount"],
	setup(props, { emit }) {
		const waterfallContainer = ref(null);
		const waterfallCanvas = ref(null);
		let gl;
		let shaderProgram;
		let vertexBuffer;
		let texture;
		let textureData;

		const effectiveWidth = computed(() =>
			Math.min(props.width, props.columnCount),
		);

		const vertexShaderSource = `
        attribute vec2 a_position;
        attribute vec2 a_texCoord;
        varying vec2 v_texCoord;
        void main() {
          gl_Position = vec4(a_position, 0, 1);
          v_texCoord = a_texCoord;
        }
      `;

		const fragmentShaderSource = `
        precision mediump float;
        uniform sampler2D u_image;
        varying vec2 v_texCoord;
        void main() {
          gl_FragColor = texture2D(u_image, v_texCoord);
        }
      `;

		function initWebGL() {
			gl = waterfallCanvas.value.getContext("webgl", { alpha: true });
			if (!gl) {
				console.error("WebGL not supported");
				return;
			}

			gl.enable(gl.BLEND);
			gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

			const vertexShader = createShader(
				gl,
				gl.VERTEX_SHADER,
				vertexShaderSource,
			);
			const fragmentShader = createShader(
				gl,
				gl.FRAGMENT_SHADER,
				fragmentShaderSource,
			);
			shaderProgram = createProgram(gl, vertexShader, fragmentShader);

			vertexBuffer = gl.createBuffer();
			gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
			gl.bufferData(
				gl.ARRAY_BUFFER,
				new Float32Array([-1, -1, 0, 1, 1, -1, 1, 1, -1, 1, 0, 0, 1, 1, 1, 0]),
				gl.STATIC_DRAW,
			);

			texture = gl.createTexture();
			gl.bindTexture(gl.TEXTURE_2D, texture);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
			gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

			textureData = new Uint8Array(effectiveWidth.value * props.height * 4);
			gl.texImage2D(
				gl.TEXTURE_2D,
				0,
				gl.RGBA,
				effectiveWidth.value,
				props.height,
				0,
				gl.RGBA,
				gl.UNSIGNED_BYTE,
				textureData,
			);
		}

		function createShader(gl, type, source) {
			const shader = gl.createShader(type);
			gl.shaderSource(shader, source);
			gl.compileShader(shader);
			if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
				console.error("Shader compile error:", gl.getShaderInfoLog(shader));
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
				console.error("Program link error:", gl.getProgramInfoLog(program));
				return null;
			}
			return program;
		}
		function updateWaterfall() {
			if (!gl || !textureData) return;

			console.log(
				"Updating waterfall with",
				props.sensorData.length,
				"sensor data values",
			);

			for (let y = 0; y < props.height; y++) {
				for (let x = 0; x < effectiveWidth.value - 1; x++) {
					const sourceIndex = (y * effectiveWidth.value + x + 1) * 4;
					const targetIndex = (y * effectiveWidth.value + x) * 4;
					textureData[targetIndex] = textureData[sourceIndex];
					textureData[targetIndex + 1] = textureData[sourceIndex + 1];
					textureData[targetIndex + 2] = textureData[sourceIndex + 2];
					textureData[targetIndex + 3] = textureData[sourceIndex + 3];
				}
			}

			const newData = props.sensorData;
			const dataLength = newData.length;
			for (let y = 0; y < props.height; y++) {
				const dataIndex = Math.floor((y / props.height) * dataLength);
				const value = newData[dataIndex];
				const color = getColorFromPalette(value);
				const index = (y * effectiveWidth.value + effectiveWidth.value - 1) * 4;
				textureData[index] = color[0];
				textureData[index + 1] = color[1];
				textureData[index + 2] = color[2];
				textureData[index + 3] = color[3];
			}

			gl.bindTexture(gl.TEXTURE_2D, texture);
			gl.texImage2D(
				gl.TEXTURE_2D,
				0,
				gl.RGBA,
				effectiveWidth.value,
				props.height,
				0,
				gl.RGBA,
				gl.UNSIGNED_BYTE,
				textureData,
			);

			gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
			gl.clearColor(0, 0, 0, 0);
			gl.clear(gl.COLOR_BUFFER_BIT);

			gl.useProgram(shaderProgram);

			const positionLocation = gl.getAttribLocation(
				shaderProgram,
				"a_position",
			);
			const texCoordLocation = gl.getAttribLocation(
				shaderProgram,
				"a_texCoord",
			);

			gl.enableVertexAttribArray(positionLocation);
			gl.enableVertexAttribArray(texCoordLocation);

			gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
			gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 16, 0);
			gl.vertexAttribPointer(texCoordLocation, 2, gl.FLOAT, false, 16, 8);

			gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
		}
		function getColorFromPalette(value) {
			const intensity = value / 255;
			switch (props.colorPalette) {
				case "heatmap":
					return heatmapColorMap(intensity);
				case "grayscale":
					return grayscaleColorMap(intensity);
				case "ocean":
					return oceanColorMap(intensity);
				case "transparent":
					return transparentColorMap(intensity);
				case "gradient":
					return gradientColorMap(intensity);
				default:
					return gradientColorMap(intensity);
			}
		}

		function transparentColorMap(intensity) {
			return [255, 255, 255, Math.floor(intensity * 255)];
		}

		function heatmapColorMap(intensity) {
			const r = Math.floor(Math.min(1, intensity * 2) * 255);
			const g = Math.floor(Math.min(1, intensity * 4 - 1) * 255);
			const b = Math.floor(Math.max(0, intensity * 4 - 3) * 255);
			return [r, g, b, 255];
		}

		function grayscaleColorMap(intensity) {
			const value = Math.floor(intensity * 255);
			return [value, value, value, 255];
		}

		function oceanColorMap(intensity) {
			const r = Math.floor(Math.max(0, intensity - 0.5) * 2 * 255);
			const g = Math.floor(intensity * 255);
			const b = Math.floor(Math.min(1, intensity * 2) * 255);
			return [r, g, b, 255];
		}

		const gradients = {
			"Thermal blue": [
				{ pos: 0, color: [5, 34, 95] },
				{ pos: 0.25, color: [106, 168, 79] },
				{ pos: 0.5, color: [255, 255, 0] },
				{ pos: 0.75, color: [127, 96, 0] },
				{ pos: 1, color: [92, 15, 8] },
			],
			"Thermal black": [
				{ pos: 0, color: [0, 0, 0] },
				{ pos: 0.25, color: [106, 168, 79] },
				{ pos: 0.5, color: [255, 255, 0] },
				{ pos: 0.75, color: [127, 96, 0] },
				{ pos: 1, color: [92, 15, 8] },
			],
			"Thermal white": [
				{ pos: 0, color: [255, 255, 255] },
				{ pos: 0.25, color: [106, 168, 79] },
				{ pos: 0.5, color: [255, 255, 0] },
				{ pos: 0.75, color: [127, 96, 0] },
				{ pos: 1, color: [92, 15, 8] },
			],
			"Monochrome black": [
				{ pos: 0, color: [0, 0, 0] },
				{ pos: 1, color: [255, 255, 255] },
			],
			"Monochrome white": [
				{ pos: 0, color: [255, 255, 255] },
				{ pos: 1, color: [0, 0, 0] },
			],
			"Monochrome sepia": [
				{ pos: 0, color: [48, 33, 19] },
				{ pos: 1, color: [232, 201, 67] },
			],
		};

		function gradientColorMap(intensity) {
			const gradient =
				gradients[props.colorPalette] || gradients["Thermal blue"];

			for (let i = 1; i < gradient.length; i++) {
				if (intensity <= gradient[i].pos) {
					const t =
						(intensity - gradient[i - 1].pos) /
						(gradient[i].pos - gradient[i - 1].pos);
					const c1 = gradient[i - 1].color;
					const c2 = gradient[i].color;
					return [
						Math.round(c1[0] + t * (c2[0] - c1[0])),
						Math.round(c1[1] + t * (c2[1] - c1[1])),
						Math.round(c1[2] + t * (c2[2] - c1[2])),
						255,
					];
				}
			}
			return gradient[gradient.length - 1].color.concat(255);
		}

		function resizeCanvas() {
			if (!waterfallContainer.value || !waterfallCanvas.value) return;

			const { width, height } =
				waterfallContainer.value.getBoundingClientRect();
			waterfallCanvas.value.width = width;
			waterfallCanvas.value.height = height;

			console.log(`Canvas resized to ${width}x${height}`);

			if (gl) {
				gl.viewport(0, 0, width, height);
				textureData = new Uint8Array(effectiveWidth.value * height * 4);
				gl.bindTexture(gl.TEXTURE_2D, texture);
				gl.texImage2D(
					gl.TEXTURE_2D,
					0,
					gl.RGBA,
					effectiveWidth.value,
					height,
					0,
					gl.RGBA,
					gl.UNSIGNED_BYTE,
					textureData,
				);
			}

			updateWaterfall();
		}

		onMounted(() => {
			resizeCanvas();
			initWebGL();
			window.addEventListener("resize", resizeCanvas);
		});

		onUnmounted(() => {
			if (gl) {
				gl.deleteProgram(shaderProgram);
				gl.deleteBuffer(vertexBuffer);
				gl.deleteTexture(texture);
			}
			window.removeEventListener("resize", resizeCanvas);
		});

		watch(
			() => props.sensorData,
			(newData) => {
				console.log(
					"WaterfallShader received new sensor data:",
					newData.length,
					"values",
				);
				updateWaterfall();
			},
			{ deep: true },
		);

		watch(
			() => props.colorPalette,
			() => {
				console.log("Color palette changed, updating waterfall");
				updateWaterfall();
			},
		);
		return {
			waterfallContainer,
			waterfallCanvas,
		};
	},
};
</script>