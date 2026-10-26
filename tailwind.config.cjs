/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        deck: {
          bg: '#1a1a1a',
          highlight: '#2d2d2d',
          accent: '#ff5722',
        }
      }
    },
  },
  plugins: [],
}
