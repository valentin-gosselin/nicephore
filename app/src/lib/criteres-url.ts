// Search criteria <-> URL query string, so a search is a plain link that
// back/forward navigation and the sidebar can reach.
import type { CriteresRecherche, ModeRecherche } from './api';

const MODES: ModeRecherche[] = ['contenu', 'fichierNom', 'description', 'texte'];

export function lireCriteres(params: Pick<URLSearchParams, "get" | "getAll">): CriteresRecherche {
  const mode = params.get('mode') as ModeRecherche | null;
  const type = params.get('type');
  return {
    texte: params.get('texte') ?? '',
    mode: mode && MODES.includes(mode) ? mode : 'contenu',
    typeMedia: type === 'image' || type === 'video' ? type : undefined,
    personnes: params.getAll('personne'),
    ville: params.get('ville') ?? undefined,
    pays: params.get('pays') ?? undefined,
    du: params.get('du') ?? undefined,
    au: params.get('au') ?? undefined,
    favoris: params.get('favoris') === '1',
    archive: params.get('archive') === '1',
  };
}

export function urlRecherche(c: CriteresRecherche, noms: Record<string, string> = {}): string {
  const p = new URLSearchParams();
  if (c.texte?.trim()) p.set('texte', c.texte.trim());
  if (c.mode && c.mode !== 'contenu') p.set('mode', c.mode);
  if (c.typeMedia) p.set('type', c.typeMedia);
  for (const id of c.personnes ?? []) {
    p.append('personne', id);
    if (noms[id]) p.append('nom', `${id}:${noms[id]}`);
  }
  if (c.ville) p.set('ville', c.ville);
  if (c.pays) p.set('pays', c.pays);
  if (c.du) p.set('du', c.du);
  if (c.au) p.set('au', c.au);
  if (c.favoris) p.set('favoris', '1');
  if (c.archive) p.set('archive', '1');
  return `/recherche?${p}`;
}

/** Display names of the people in the URL (`nom=<id>:<name>`). */
export function nomsPersonnes(params: Pick<URLSearchParams, "getAll">): Record<string, string> {
  const noms: Record<string, string> = {};
  for (const valeur of params.getAll('nom')) {
    const i = valeur.indexOf(':');
    if (i > 0) noms[valeur.slice(0, i)] = valeur.slice(i + 1);
  }
  return noms;
}

export function estVide(c: CriteresRecherche) {
  return !c.texte?.trim() && !c.typeMedia && !c.personnes?.length && !c.ville && !c.pays && !c.du && !c.au && !c.favoris;
}
