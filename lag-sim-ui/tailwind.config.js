export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts}"],
  theme: {
    extend: {
      colors: {
        neon: {
          pink: "#ff3ccf",
          cyan: "#26fff2",
          lime: "#adff2f",
          red: "#ff2d2d",
          blue: "#2d6bff",
          purple: "#a855f7"
        }
      },
      fontFamily: {
        glitch: ["'Space Mono'", "monospace"],
        title: ["'Orbitron'", "sans-serif"]
      },
      boxShadow: {
        glow: "0 0 16px rgba(255, 60, 207, 0.6), 0 0 32px rgba(38, 255, 242, 0.4)"
      }
    }
  },
  plugins: []
}