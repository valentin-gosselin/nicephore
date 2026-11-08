// Shared scroll state: during a fast scroll (fling, scrubber drag) no
// thumbnail is requested, so the network is not flooded with images that
// will never be seen.
export const defilement = $state({ rapide: false });
