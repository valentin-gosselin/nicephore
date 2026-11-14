// Safety net of a window opened for a photo of this computer: the viewer
// shows it once sized to the photo; if that has not happened after a few
// seconds (a slow file), the window shows anyway. Disarmed as soon as the
// viewer shows it, so a window closed in the meantime stays closed.
let minuteur: ReturnType<typeof setTimeout> | null = null;

export function armerFilet(montrer: () => void, ms = 5000) {
  desarmerFilet();
  minuteur = setTimeout(() => {
    minuteur = null;
    montrer();
  }, ms);
}

export function desarmerFilet() {
  if (minuteur) clearTimeout(minuteur);
  minuteur = null;
}
