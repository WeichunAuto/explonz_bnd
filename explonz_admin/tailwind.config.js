/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{html,rs,ts,tsx}", // do not add empty space between file types
  ],
  theme: {
    extend: {
      fontFamily: {
        dongle: ["Dongle", "ui-sans-serif", "system-ui"],
      },
    },
  },
  plugins: [require("@tailwindcss/line-clamp")],
};
