// In-app multi-account scenario (scripts/banc.sh --comptes), portable mode
// on the disposable server: add a second account, switch back and forth
// (each sealed with its own passphrase), log out of one. Every switch
// reloads the interface, so the step is kept in sessionStorage.
import { invoke } from '@tauri-apps/api/core';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const PHRASES = ['phrase du premier compte', 'phrase du second compte'];
const CLE = 'nicephore-scenario-comptes';

type Ids = { url: string; email: string; motDePasse: string; email2: string; motDePasse2: string };

async function element<T extends Element>(selecteur: string, filtre?: (e: T) => boolean, essais = 120): Promise<T | null> {
  for (let i = 0; i < essais; i++) {
    const trouves = [...document.querySelectorAll<T>(selecteur)].filter((e) => !filtre || filtre(e));
    if (trouves.length) return trouves[0];
    await attendre(100);
  }
  return null;
}

function champ(libelle: string) {
  const label = [...document.querySelectorAll('label')].find((l) => (l.textContent ?? '').includes(libelle));
  const id = label?.getAttribute('for');
  return (id ? document.getElementById(id) : label?.parentElement?.querySelector('input')) as HTMLInputElement | null;
}

function saisir(c: HTMLInputElement | null, texte: string) {
  if (!c) return;
  c.value = texte;
  c.dispatchEvent(new Event('input', { bubbles: true }));
}

const bouton = (texte: string) => element<HTMLButtonElement>('button', (b) => b.textContent?.trim() === texte);
const compteActif = async () => (await element<HTMLButtonElement>('[aria-label^="Compte de"]', undefined, 150))?.getAttribute('aria-label') ?? '';

async function verifier(nom: string, condition: boolean, detail = '') {
  await journal(`${condition ? 'ok' : 'ÉCHEC'} : ${nom}${detail ? ` (${detail})` : ''}`);
}

async function connecter(url: string, email: string, motDePasse: string, phrase: string) {
  await element('input');
  if (champ('Adresse du serveur')) {
    saisir(champ('Adresse du serveur'), url);
    (await bouton('Continuer'))?.click();
  }
  await element('input[type="email"]');
  saisir(champ('Adresse e-mail'), email);
  saisir(champ('Mot de passe'), motDePasse);
  saisir(champ('Phrase de passe'), phrase);
  saisir(champ('La même, pour vérifier'), phrase);
  await attendre(300);
  (await bouton('Se connecter'))?.click();
}

async function deverrouiller(phrase: string) {
  await bouton('Déverrouiller');
  saisir(champ('Phrase de passe'), phrase);
  await attendre(200);
  (await bouton('Déverrouiller'))?.click();
}

async function ouvrirMenu() {
  (await element<HTMLButtonElement>('[aria-label^="Compte de"]'))?.click();
  await attendre(800);
}

export async function lancerScenarioComptes() {
  const ids = await invoke<Ids | null>('identifiants_banc');
  if (!ids) {
    await journal('scénario comptes refusé : pas de serveur local jetable');
    return invoke('quitter');
  }
  const etape = Number(sessionStorage.getItem(CLE) ?? '0');
  const suite = (n: number) => sessionStorage.setItem(CLE, String(n));
  await journal(`scénario comptes, étape ${etape}`);
  try {
    if (etape === 0) {
      await connecter(ids.url, ids.email, ids.motDePasse, PHRASES[0]);
      await verifier('premier compte connecté', (await compteActif()).includes('Test Nicéphore'));
      await ouvrirMenu();
      suite(1);
      (await bouton('Ajouter un compte'))?.click();
      await attendre(500);
      const ecran = document.body.textContent ?? '';
      await verifier('écran « Ajouter un compte »', ecran.includes('Ajouter un compte') && ecran.includes('revenir au compte actuel'));
      await capture('c1-ajout');
      await connecter(ids.url, ids.email2, ids.motDePasse2, PHRASES[1]);
      // The interface reloads on success.
      await attendre(15_000);
      await verifier('rechargement après l’ajout', false);
    } else if (etape === 1) {
      await verifier('second compte actif', (await compteActif()).includes('Second compte'));
      await element('.entete');
      await attendre(1500);
      const entete = document.querySelector('.entete')?.textContent?.replace(/\s+/g, ' ') ?? '';
      await verifier('ses propres photos (3)', entete.includes('3 éléments'), entete.slice(0, 60));
      const liste = await invoke<{ nom: string; actif: boolean; scelle: boolean }[]>('comptes');
      await verifier('deux comptes mémorisés, scellés', liste.length === 2 && liste.every((c) => c.scelle));
      await ouvrirMenu();
      await capture('c2-menu-deux-comptes');
      const autre = await element<HTMLButtonElement>('[role="menu"] button', (b) => b.textContent?.includes('Test Nicéphore') ?? false);
      await verifier('l’autre compte proposé dans le menu', !!autre);
      suite(2);
      autre?.click();
      await attendre(15_000);
      await verifier('rechargement après le changement', false);
    } else if (etape === 2) {
      await verifier('phrase demandée pour le compte scellé', !!(await bouton('Déverrouiller')));
      await deverrouiller(PHRASES[0]);
      await verifier('revenu au premier compte', (await compteActif()).includes('Test Nicéphore'));
      await ouvrirMenu();
      suite(3);
      (await bouton('Se déconnecter'))?.click();
      await attendre(15_000);
      await verifier('rechargement après la déconnexion', false);
    } else if (etape === 3) {
      await verifier('le compte restant prend la suite (phrase demandée)', !!(await bouton('Déverrouiller')));
      await deverrouiller(PHRASES[1]);
      await verifier('second compte actif', (await compteActif()).includes('Second compte'));
      const liste = await invoke<unknown[]>('comptes');
      await verifier('un seul compte mémorisé', liste.length === 1);
      await capture('c3-fin');
      suite(4);
      await journal('scénario terminé');
      await invoke('quitter');
    }
  } catch (e) {
    await journal(`scénario interrompu : ${e}`);
    await invoke('quitter');
  }
}
