// Shared scroll state: during a fast scroll (fling, scrubber drag) no
// thumbnail is requested from the server, so the network is not flooded with
// images that will never be seen. Once every thumbnail is on disk
// (`vignettesLocales`, set when preloading is complete), they are cheap to
// read and show up even while flinging.
export const defilement = $state({ rapide: false, vignettesLocales: false });
