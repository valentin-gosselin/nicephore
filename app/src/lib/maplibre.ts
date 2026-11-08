// MapLibre set up once for every map of the app. MapLibre only guesses its
// worker's address on http(s) pages; Tauri serves the UI from
// tauri://localhost, so the bundled worker file is named explicitly.
import { setWorkerUrl } from 'maplibre-gl';
import urlWorker from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';
import 'maplibre-gl/dist/maplibre-gl.css';

setWorkerUrl(urlWorker);

export * from 'maplibre-gl';
