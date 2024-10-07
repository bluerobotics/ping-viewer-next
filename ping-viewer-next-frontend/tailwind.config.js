const vuetifyBreakpoints = {
	xs: "0px",
	sm: "600px",
	md: "960px",
	lg: "1264px",
	xl: "1904px",
};
/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ["./public/**/*.html", "./src/**/*.{js,jsx,ts,tsx,vue}"],
	theme: {
		screens: vuetifyBreakpoints,
		extend: {},
	},
	corePlugins: {
		preflight: false,
	},
	plugins: [],
};
