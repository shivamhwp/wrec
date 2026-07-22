// @ts-check
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, fontProviders } from "astro/config";

// https://astro.build/config
export default defineConfig({
  site: "https://wrec.app",
  // compressHTML strips newline-only whitespace between text and inline
  // elements, gluing words to links ("Seearchitecture").
  compressHTML: false,
  vite: {
    plugins: [tailwindcss()],
  },
  fonts: [
    {
      provider: fontProviders.local(),
      name: "Geist",
      cssVariable: "--font-geist",
      options: {
        variants: [
          {
            src: ["./src/assets/fonts/geist/Geist[wght].ttf"],
            weight: "100 900",
            style: "normal",
          },
        ],
      },
    },
    {
      provider: fontProviders.local(),
      name: "Geist Mono",
      cssVariable: "--font-geist-mono",
      options: {
        variants: [
          {
            src: ["./src/assets/fonts/geist/GeistMono[wght].ttf"],
            weight: "100 900",
            style: "normal",
          },
        ],
      },
    },
    {
      provider: fontProviders.local(),
      name: "Departure Mono",
      cssVariable: "--font-departure",
      options: {
        variants: [
          {
            src: ["./src/assets/fonts/departure/DepartureMono-Regular.otf"],
            weight: "400",
            style: "normal",
          },
        ],
      },
    },
  ],
});
