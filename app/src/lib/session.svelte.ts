// Connection state shared by the shell, the pages and the settings.
import { compteVerrouille, etatConnexion, seDeconnecter, type Connexion } from './api';

export const session = $state({
  connexion: null as Connexion | null,
  /** Portable mode: server of a sealed connection waiting for its passphrase. */
  verrouille: null as string | null,
  verification: true,
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

export async function deconnecter() {
  await seDeconnecter();
  session.connexion = null;
  session.verrouille = null;
}
