// Connection state shared by the shell, the pages and the settings.
import { changerCompte, compteVerrouille, etatConnexion, seDeconnecter, type Connexion } from './api';

export const session = $state({
  connexion: null as Connexion | null,
  /** Portable mode: server of a sealed connection waiting for its passphrase. */
  verrouille: null as string | null,
  verification: true,
  /** Login screen shown to add another account. */
  ajout: false,
  erreur: null as string | null,
});

export async function verifierSession() {
  session.verification = true;
  session.erreur = null;
  try {
    session.connexion = await etatConnexion();
    session.verrouille = session.connexion ? null : await compteVerrouille();
  } catch (e) {
    session.erreur = String(e);
  } finally {
    session.verification = false;
  }
}

/** Reloads the whole interface: nothing of the previous account stays. */
function repartir() {
  window.location.assign('/');
}

/** Logs out of this account; another remembered one takes over, if any. */
export async function deconnecter() {
  await seDeconnecter();
  session.connexion = null;
  session.verrouille = null;
  repartir();
}

export async function basculerCompte(cle: string) {
  await changerCompte(cle);
  repartir();
}

/** After logging in from the "add an account" screen. */
export function compteAjoute() {
  session.ajout = false;
  repartir();
}
