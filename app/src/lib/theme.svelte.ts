// Light/dark theme. "System" asks Tauri for the desktop theme (read from the
// freedesktop portal on Linux, the registry on Windows) instead of relying on
// the webview's prefers-color-scheme, which WebKitGTK does not always
// forward. The effective theme is handed to @immich/ui as an explicit choice.
import { ThemePreference, themeManager } from '@immich/ui';
import { getCurrentWindow } from '@tauri-apps/api/window';

export type ChoixTheme = 'systeme' | 'clair' | 'sombre';
const CLE = 'nicephore-theme';

function lire(): ChoixTheme {
  try {
    const valeur = localStorage.getItem(CLE);
    if (valeur === 'clair' || valeur === 'sombre') return valeur;
  } catch {
    // Storage unavailable: follow the system.
  }
  return 'systeme';
}

export const theme = $state({
  choix: lire(),
  systeme: (matchMedia('(prefers-color-scheme: dark)').matches ? 'sombre' : 'clair') as 'clair' | 'sombre',
});

function appliquer() {
  const effectif = theme.choix === 'systeme' ? theme.systeme : theme.choix;
  themeManager.setPreference(effectif === 'sombre' ? ThemePreference.Dark : ThemePreference.Light);
}

export async function initialiserTheme() {
  appliquer();
  try {
    const fenetre = getCurrentWindow();
    const actuel = await fenetre.theme();
    if (actuel) theme.systeme = actuel === 'dark' ? 'sombre' : 'clair';
    await fenetre.onThemeChanged(({ payload }) => {
      theme.systeme = payload === 'dark' ? 'sombre' : 'clair';
      appliquer();
    });
  } catch {
    // Outside Tauri or no portal: keep the webview's answer.
  }
  appliquer();
}

export function choisirTheme(choix: ChoixTheme) {
  theme.choix = choix;
  try {
    localStorage.setItem(CLE, choix);
  } catch {
    // Not remembered, still applied.
  }
  appliquer();
}
