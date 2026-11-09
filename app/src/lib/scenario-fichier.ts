// In-app scenario of the default viewer (scripts/banc.sh --fichier): a
// photo opened from the command line like the file manager does, its
// neighbour, the cloud badge, sending it now, and a second launch that goes
// to the running window. Disposable local server only.
import { invoke } from '@tauri-apps/api/core';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const texte = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';

let reussites = 0;
let echecs = 0;
async function verifier(nom: string, condition: boolean, detail = '') {
  if (condition) reussites++;
  else echecs++;
  await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}${detail ? ` (${detail})` : ''}`);
}

async function quand(condition: () => boolean, essais = 100) {
  for (let i = 0; i < essais && !condition(); i++) await attendre(100);
  return condition();
}

const bouton = (libelle: string) =>
  [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === libelle || b.getAttribute('aria-label') === libelle);

export async function lancerScenarioFichier() {
  try {
    await journal('scénario visionneuse locale');
    const affichee = await quand(() => location.pathname === '/fichier' && (document.querySelector<HTMLImageElement>('img.image')?.naturalWidth ?? 0) > 0);
    await verifier('photo ouverte depuis la ligne de commande et affichée', affichee, location.search.slice(0, 120));
    await verifier('position dans le dossier', texte().includes('2 / 3'));
    await verifier('nuage barré : pas sur Immich', await quand(() => texte().includes('Pas sur Immich')));
    await capture('f1-pas-sur-immich');
    bouton('Envoyer maintenant')?.click();
    await verifier('envoyée, puis nuage coché', await quand(() => texte().includes('Sur Immich') && !texte().includes('Pas sur Immich'), 150));
    await capture('f2-envoyee');
    bouton('Précédente')?.click();
    await verifier('voisine du dossier', await quand(() => location.search.includes('a-sur-serveur')));
    await verifier('voisine déjà sur Immich', await quand(() => texte().includes('Sur Immich') && !texte().includes('Pas sur Immich'), 150));
    const nouvelle = decodeURIComponent(new URLSearchParams(location.search).get('chemin') ?? '').replace('a-sur-serveur', 'c-autre');
    await invoke('lancer_seconde_instance', { chemin: nouvelle });
    await verifier('seconde ouverture dans la même fenêtre', await quand(() => location.search.includes('c-autre'), 150), location.search.slice(0, 120));
    await capture('f3-seconde-ouverture');
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  await invoke('quitter');
}
