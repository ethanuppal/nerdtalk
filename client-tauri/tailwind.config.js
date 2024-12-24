/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{html,js,ts,jsx,tsx}", // Adjust based on your framework
  ],
  theme: {
    extend: {
      fontSize: {
        xxs: "0.625rem"  
      },
      colors: {
        "primary-color": "var(--primary-color)",
        "background-color": "var(--background-color)",
        "primary-text": "var(--primary-text)",
        "secondary-text": "var(--secondary-text)"
      },
    },
  },
  plugins: [],
};
