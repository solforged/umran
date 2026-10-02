import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  root: "web",
  // Relative asset paths, so the build works at any path, such as GitHub
  // Pages' /umran/.
  base: "./",
  plugins: [
    react(),
    VitePWA({
      registerType: "prompt",
      injectRegister: false,
      includeAssets: ["icon.svg", "apple-touch-icon.png"],
      manifest: {
        id: "./",
        // A short installed name (Umran.app on macOS); the subtitle stays on
        // the web page's title.
        name: "Umran",
        short_name: "Umran",
        description: "A seeded language-change simulator: communities, contact, sound change, and borrowing.",
        start_url: "./",
        scope: "./",
        display: "standalone",
        background_color: "#ece0c2",
        theme_color: "#ece0c2",
        icons: [
          { src: "icon-192.png", sizes: "192x192", type: "image/png", purpose: "any" },
          { src: "icon-512.png", sizes: "512x512", type: "image/png", purpose: "any" },
          { src: "icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
        ],
      },
      workbox: {
        // The engine must be available even when a world first opens offline.
        globPatterns: ["**/*.{js,css,html,wasm,png,svg}"],
        maximumFileSizeToCacheInBytes: 4 * 1024 * 1024,
        navigateFallback: "index.html",
        cleanupOutdatedCaches: true,
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/fonts\.googleapis\.com\//,
            handler: "StaleWhileRevalidate",
            options: {
              cacheName: "umran-font-styles",
              cacheableResponse: { statuses: [0, 200] },
              expiration: { maxEntries: 8, maxAgeSeconds: 365 * 24 * 60 * 60 },
            },
          },
          {
            urlPattern: /^https:\/\/fonts\.gstatic\.com\//,
            handler: "CacheFirst",
            options: {
              cacheName: "umran-font-files",
              cacheableResponse: { statuses: [0, 200] },
              expiration: { maxEntries: 64, maxAgeSeconds: 365 * 24 * 60 * 60 },
            },
          },
        ],
      },
    }),
  ],
  build: { target: "es2023", outDir: "../dist", emptyOutDir: true },
  server: { host: "127.0.0.1", port: 5173, strictPort: true },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
});
