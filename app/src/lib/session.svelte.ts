// Connection state shared by the shell, the pages and the settings.
import { etatConnexion, seDeconnecter, type Connexion } from './api';

export const session = $state({
  connexion: null as Connexion | null,
  verification: true,
  erreur: null as string | null,
});

export async function verifierSession() {
  session.verification = true;
  session.erreur = null;
  try {
    session.connexion = await etatConnexion();
  } catch (e) {
    session.erreur = String(e);
  } finally {
    session.verification = false;
  }
}

export async function deconnecter() {
  await seDeconnecter();
  session.connexion = null;
}
