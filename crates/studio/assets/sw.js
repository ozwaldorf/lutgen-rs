var cacheName = "lutgen-studio-pwa";
var filesToCache = [
  "./",
  "./index.html",
  "./lutgen_studio.js",
  "./lutgen_studio_bg.wasm",
  "./worker.js",
  "./worker_bg.wasm",
];

/* Start the service worker and cache all of the app's content */
self.addEventListener("install", function (e) {
  e.waitUntil(
    caches.open(cacheName).then(function (cache) {
      return cache.addAll(filesToCache);
    }),
  );
});

/* Fetch content, otherwise when offline serve cached content */
self.addEventListener("fetch", function (e) {
  e.respondWith(
    fetch(e.request).catch((_) => {
      caches.match(e.request).then(function (response) {
        if (response == undefined) {
          throw new Error("Offline and app not cached");
        } else {
          return response;
        }
      });
    }),
  );
});
