// In-app portable mode scenario, two launches on the same `donnees` folder
// (scripts/banc.sh --portable): the first logs in through the login screen
// and seals the connection with a passphrase, the second unlocks it.
// Disposable local server only.
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const PHRASE = 'phrase du banc portable';

let reussites = 0;
let echecs = 0;

async function verifier(nom: string, condition: boolean, detail = '') {
  if (condition) reussites++;
  else echecs++;
  await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}${detail ? ` (${detail})` : ''}`);
}

async function element<T extends Element>(selecteur: string, filtre?: (e: T) => boolean, essais = 100): Promise<T | null> {
  for (let i = 0; i < essais; i++) {
    const trouves = [...document.querySelectorAll<T>(selecteur)].filter((e) => !filtre || filtre(e));
    if (trouves.length) return trouves[0];
    await attendre(100);
  }
  return null;
}

/** Field of an @immich/ui `Field` by its label. */
function champ(libelle: string) {
  const label = [...document.querySelectorAll('label')].find((l) => (l.textContent ?? '').includes(libelle));
  const id = label?.getAttribute('for');
  return (id ? document.getElementById(id) : label?.parentElement?.querySelector('input')) as HTMLInputElement | null;
}

function saisir(champ: HTMLInputElement | null, texte: string) {
  if (!champ) return;
  champ.value = texte;
  champ.dispatchEvent(new Event('input', { bubbles: true }));
}

const bouton = (texte: string) => element<HTMLButtonElement>('button', (b) => b.textContent?.trim() === texte);
const texte = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';

async function connecte() {
  return !!(await element('[aria-label="Navigation principale"]', undefined, 150));
}

export async function lancerScenarioPortable(etape: string) {
  try {
    const ids = await invoke<{ url: string; email: string; motDePasse: string } | null>('identifiants_banc');
    if (!ids) {
      await journal('scénario portable refusé : pas de serveur local jetable');
      return;
    }
    await journal(`scénario portable, ${etape}`);
    if (etape === 'portable-1') {
      await element('input');
      saisir(champ('Adresse du serveur'), ids.url);
      (await bouton('Continuer'))?.click();
      await element('input[type="email"]');
      saisir(champ('Adresse e-mail'), ids.email);
      saisir(champ('Mot de passe'), ids.motDePasse);
      await attendre(300);
      await verifier('champ de phrase de passe proposé en portable', !!champ('Phrase de passe') && texte().includes('chiffré par une phrase'));
      saisir(champ('Phrase de passe'), PHRASE);
      saisir(champ('La même, pour vérifier'), PHRASE);
      await attendre(300);
      await capture('p1-connexion');
      (await bouton('Se connecter'))?.click();
      await verifier('connecté par l’écran de connexion', await connecte());
      await goto('/reglages');
      await attendre(2000);
      await verifier('section Mode portable', texte().includes('Mode portable') && texte().includes('donnees'));
      await verifier('connexion chiffrée annoncée', texte().includes('chiffrée par votre phrase de passe'));
      await verifier('pas de démarrage avec la session', !texte().includes('Lancer Nicéphore à l’ouverture de session'));
      await capture('p2-reglages');
    } else {
      await verifier('écran de déverrouillage', !!(await bouton('Déverrouiller')) && texte().includes(ids.url));
      await capture('p3-deverrouillage');
      saisir(champ('Phrase de passe'), 'pas la bonne phrase');
      await attendre(200);
      (await bouton('Déverrouiller'))?.click();
      await element('[role="alert"], .text-danger', undefined, 80);
      await attendre(500);
      await verifier('mauvaise phrase refusée', texte().includes('phrase de passe incorrecte'), texte().slice(0, 200));
      saisir(champ('Phrase de passe'), PHRASE);
      await attendre(200);
      (await bouton('Déverrouiller'))?.click();
      await verifier('déverrouillé avec la bonne phrase', await connecte());
      await capture('p4-deverrouille');
    }
    await journal(`scénario terminé : ${reussites} réussites, ${echecs} échecs`);
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
  }
  await invoke('quitter');
}
