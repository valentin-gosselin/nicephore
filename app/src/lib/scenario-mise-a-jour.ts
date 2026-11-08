// In-app update scenario (scripts/banc.sh --mise-a-jour): an AppImage
// finds a newer signed version on a local test server, installs it and
// restarts; the restarted copy reports its version.
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const texte = () => document.querySelector('main')?.textContent?.replace(/\s+/g, ' ') ?? '';

async function attendreTexte(morceau: string, essais = 150) {
  for (let i = 0; i < essais; i++) {
    if (texte().includes(morceau)) return true;
    await attendre(100);
  }
  return false;
}

const bouton = (libelle: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === libelle);

export async function lancerScenarioMiseAJour() {
  const version = await getVersion();
  await journal(`scénario de mise à jour, version ${version}`);
  if (version !== '0.0.1') {
    await journal(`${version === '0.0.2' ? 'ok' : 'ÉCHEC'} : redémarré dans la version ${version}`);
    await invoke('quitter');
    return;
  }
  let ok = 0;
  let ko = 0;
  const verifier = async (nom: string, condition: boolean) => {
    if (condition) ok++;
    else ko++;
    await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}`);
  };
  await goto('/reglages');
  await verifier('section Mises à jour', await attendreTexte('Mises à jour'));
  bouton('Rechercher')?.click();
  await verifier('version 0.0.2 trouvée', await attendreTexte('Nicéphore 0.0.2 est disponible'));
  await verifier('notes de version affichées', texte().includes('Version de test du banc'));
  await capture('m1-disponible');
  const installer = bouton('Installer et redémarrer');
  await verifier('installable (AppImage)', !!installer);
  await journal(`avant installation : ${ok} réussites, ${ko} échecs`);
  installer?.click();
  // Nicéphore restarts on success; otherwise the error shows here (a bad
  // signature must be refused).
  for (let i = 0; i < 600; i++) {
    const erreur = document.querySelector('[aria-label="Mises à jour"] .text-danger, [aria-label="Mises à jour"] [class*="danger"]');
    if (erreur?.textContent?.trim()) {
      await journal(`installation refusée : ${erreur.textContent.trim()}`);
      await capture('m2-refus');
      await invoke('quitter');
      return;
    }
    await attendre(100);
  }
  await journal(`ÉCHEC : ni redémarrage ni erreur (${texte().slice(0, 200)})`);
  await invoke('quitter');
}
