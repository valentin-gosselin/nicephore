// Back and forward inside the app, which has no browser toolbar. The
// position in the history is tracked from SvelteKit navigations, so the
// buttons know when there is nothing left to go back to.
export const historique = $state({ position: 0, maximum: 0 });

let remplacement = false;

/** The next navigation replaces the current entry (e.g. a filter change). */
export function remplacerProchaineNavigation() {
  remplacement = true;
}

/** To call from `afterNavigate`. */
export function noterNavigation(type: string, delta: number | null | undefined) {
  if (type === 'enter') return;
  if (type === 'popstate') {
    historique.position = Math.max(0, historique.position + (delta ?? 0));
  } else if (remplacement) {
    remplacement = false;
  } else {
    historique.position += 1;
    historique.maximum = historique.position;
  }
}

/** Closes the photo viewer if open; returns true when it did. */
function fermerVisionneuse() {
  if (!document.querySelector('.visionneuse')) return false;
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  return true;
}

export function retour() {
  if (fermerVisionneuse()) return;
  if (historique.position > 0) history.back();
}

export function avance() {
  if (historique.position < historique.maximum) history.forward();
}
