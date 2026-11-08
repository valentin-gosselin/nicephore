// Timeline model: the month index comes first (counts only), months are
// loaded on demand when they approach the viewport, and their height is
// estimated until then so the scroll range is right from the start.

import { detailMois, type DetailMois, type Mois } from './api';
import { disposer } from './disposition';

export type Asset = {
  id: string;
  ratio: number;
  image: boolean;
  favori: boolean;
  thumbhash: string | null;
  duree: string | null;
  /** Local capture date, as seen by the photographer. */
  date: Date;
  jour: string;
};

type LigneJour = { type: 'jour'; y: number; libelle: string; cle: string };
type LignePhotos = {
  type: 'photos';
  y: number;
  hauteur: number;
  cases: { asset: number; x: number; largeur: number }[];
};
type Ligne = LigneJour | LignePhotos;

type EtatSection = 'vide' | 'chargement' | 'charge' | 'erreur';

export class Section {
  readonly cle: string;
  readonly nombre: number;
  readonly libelle: string;
  readonly annee: number;
  etat: EtatSection = 'vide';
  assets: Asset[] = [];
  lignes: Ligne[] = [];
  /** Height of the section including its title. */
  hauteur = 0;
  /** Absolute top position in the timeline. */
  haut = 0;

  constructor(mois: Mois) {
    this.cle = mois.timeBucket;
    this.nombre = mois.count;
    const [annee, numero] = mois.timeBucket.split('-').map(Number);
    this.annee = annee;
    const libelle = new Date(Date.UTC(annee, numero - 1, 1)).toLocaleDateString('fr-FR', {
      month: 'long',
      year: 'numeric',
      timeZone: 'UTC',
    });
    this.libelle = libelle.charAt(0).toUpperCase() + libelle.slice(1);
  }
}

export type TuileVisible = {
  asset: Asset;
  section: number;
  index: number;
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
};

export type Position = { section: number; index: number };

export const HAUTEUR_TITRE_MOIS = 56;
export const HAUTEUR_TITRE_JOUR = 36;
export const ECART = 4;
const PARALLELE = 4;

function formaterDuree(valeur: number | string | null): string | null {
  if (valeur == null) return null;
  let secondes: number;
  if (typeof valeur === 'number') secondes = valeur / 1000;
  else {
    // Older servers send "H:MM:SS.mmm".
    const [h, m, s] = valeur.split(':').map(Number);
    secondes = (h || 0) * 3600 + (m || 0) * 60 + (s || 0);
  }
  if (!Number.isFinite(secondes) || secondes <= 0) return null;
  const total = Math.round(secondes);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}

function versAssets(detail: DetailMois): Asset[] {
  return detail.id.map((id, i) => {
    const decalage = detail.localOffsetHours?.[i] ?? 0;
    // Shift the UTC instant by the capture offset, then read UTC fields.
    const date = new Date(Date.parse(detail.fileCreatedAt[i]) + decalage * 3_600_000);
    return {
      id,
      ratio: detail.ratio[i] || 1,
      image: detail.isImage[i],
      favori: detail.isFavorite?.[i] ?? false,
      thumbhash: detail.thumbhash?.[i] ?? null,
      duree: detail.isImage[i] ? null : formaterDuree(detail.duration?.[i] ?? null),
      date,
      jour: date.toISOString().slice(0, 10),
    };
  });
}

const formatJour = new Intl.DateTimeFormat('fr-FR', {
  weekday: 'short',
  day: 'numeric',
  month: 'short',
  year: 'numeric',
  timeZone: 'UTC',
});

function libelleJour(jour: string): string {
  const aujourdhui = new Date();
  const local = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  if (jour === local(aujourdhui)) return "Aujourd'hui";
  const hier = new Date(aujourdhui.getTime() - 86_400_000);
  if (jour === local(hier)) return 'Hier';
  const texte = formatJour.format(new Date(`${jour}T00:00:00Z`));
  return texte.charAt(0).toUpperCase() + texte.slice(1);
}

export class Chronologie {
  sections: Section[] = [];
  /** Bumped on every layout change; the view reads it to refresh. */
  revision = $state(0);
  hauteurTotale = $state(0);
  nombreTotal = $state(0);

  largeur = 0;
  hauteurCible = 220;
  private enCours = 0;
  private attente: number[] = [];
  private promesses = new Map<number, Promise<void>>();
  /** Called after a month changed height, so the view can keep its anchor. */
  surChangement: (() => void) | null = null;

  initialiser(mois: Mois[]) {
    this.sections = mois.map((m) => new Section(m));
    this.nombreTotal = mois.reduce((n, m) => n + m.count, 0);
    // Every month needs an (estimated) height right away: anchors and the
    // scrubber rely on it.
    for (const section of this.sections) this.disposerSection(section);
    this.recalculer();
  }

  /** Width or zoom change: lay out every loaded month again. */
  redisposer(largeur: number, hauteurCible: number) {
    this.largeur = largeur;
    this.hauteurCible = hauteurCible;
    for (const section of this.sections) this.disposerSection(section);
    this.recalculer();
  }

  private disposerSection(section: Section) {
    // Before the first measure, lay out on a sane width rather than nothing.
    const largeur = Math.max(this.largeur, 320);
    if (section.etat !== 'charge') {
      // Estimate: average ratio 1.5, plus a few day titles.
      const rangees = Math.ceil((section.nombre * 1.5 * this.hauteurCible) / largeur);
      const jours = Math.min(section.nombre, 6);
      section.hauteur =
        HAUTEUR_TITRE_MOIS + rangees * (this.hauteurCible + ECART) + jours * HAUTEUR_TITRE_JOUR;
      return;
    }
    const lignes: Ligne[] = [];
    let y = HAUTEUR_TITRE_MOIS;
    let debut = 0;
    const assets = section.assets;
    while (debut < assets.length) {
      const jour = assets[debut].jour;
      let fin = debut;
      while (fin < assets.length && assets[fin].jour === jour) fin++;
      // A day can appear twice in a month when time zones reorder assets,
      // so the key uses the position, not the date.
      lignes.push({ type: 'jour', y, libelle: libelleJour(jour), cle: `${section.cle}/${lignes.length}` });
      y += HAUTEUR_TITRE_JOUR;
      const ratios = assets.slice(debut, fin).map((a) => a.ratio);
      const { rangees, hauteur } = disposer(ratios, largeur, this.hauteurCible, ECART);
      for (const rangee of rangees) {
        lignes.push({
          type: 'photos',
          y: y + rangee.y,
          hauteur: rangee.hauteur,
          cases: rangee.cases.map((c) => ({ asset: debut + c.index, x: c.x, largeur: c.largeur })),
        });
      }
      y += hauteur + ECART * 4;
      debut = fin;
    }
    section.lignes = lignes;
    section.hauteur = y;
  }

  private recalculer() {
    let haut = 0;
    for (const section of this.sections) {
      section.haut = haut;
      haut += section.hauteur;
    }
    this.hauteurTotale = haut;
    this.revision++;
  }

  /** Index of the month containing the absolute position `y`. */
  sectionA(y: number): number {
    // First month whose bottom edge is below `y`.
    let bas = 0;
    let hautIdx = this.sections.length - 1;
    while (bas < hautIdx) {
      const milieu = (bas + hautIdx) >> 1;
      const section = this.sections[milieu];
      if (section.haut + section.hauteur <= y) bas = milieu + 1;
      else hautIdx = milieu;
    }
    return Math.max(0, bas);
  }

  /** Scroll anchor: month under the top edge and the offset inside it. */
  ancre(defilement: number): { section: number; decalage: number } {
    const section = this.sectionA(defilement);
    return { section, decalage: defilement - (this.sections[section]?.haut ?? 0) };
  }

  positionAncre(ancre: { section: number; decalage: number }): number {
    const section = this.sections[ancre.section];
    if (!section) return 0;
    return section.haut + Math.min(ancre.decalage, Math.max(0, section.hauteur - 1));
  }

  /** Queues the months overlapping [debut, fin], nearest to `centre` first. */
  demander(debut: number, fin: number, centre: number) {
    if (!this.sections.length) return;
    const premiere = this.sectionA(debut);
    const derniere = this.sectionA(fin);
    const voulues: number[] = [];
    for (let i = premiere; i <= derniere; i++) {
      if (this.sections[i].etat === 'vide') voulues.push(i);
    }
    const distance = (i: number) =>
      Math.abs(this.sections[i].haut + this.sections[i].hauteur / 2 - centre);
    // Drop stale requests: only the current window matters.
    this.attente = voulues.sort((a, b) => distance(a) - distance(b));
    this.pomper();
  }

  private pomper() {
    while (this.enCours < PARALLELE && this.attente.length) {
      const index = this.attente.shift()!;
      if (this.sections[index].etat !== 'vide') continue;
      void this.chargerSection(index);
    }
  }

  chargerSection(index: number): Promise<void> {
    const existante = this.promesses.get(index);
    if (existante) return existante;
    const section = this.sections[index];
    if (!section || section.etat === 'charge') return Promise.resolve();
    section.etat = 'chargement';
    this.enCours++;
    const promesse = detailMois(section.cle)
      .then((detail) => {
        section.assets = versAssets(detail);
        section.etat = 'charge';
      })
      .catch((erreur) => {
        console.error(`mois ${section.cle}`, erreur);
        section.etat = 'erreur';
      })
      .finally(() => {
        this.enCours--;
        this.promesses.delete(index);
        this.disposerSection(section);
        this.recalculer();
        this.surChangement?.();
        this.pomper();
      });
    this.promesses.set(index, promesse);
    return promesse;
  }

  /** Visible titles and tiles between two absolute positions. */
  visibles(debut: number, fin: number) {
    const titres: { cle: string; y: number; libelle: string; mois: boolean }[] = [];
    const tuiles: TuileVisible[] = [];
    const attentes: { cle: string; y: number; hauteur: number }[] = [];
    if (!this.sections.length) return { titres, tuiles, attentes };

    for (let s = this.sectionA(debut); s < this.sections.length; s++) {
      const section = this.sections[s];
      if (section.haut > fin) break;
      titres.push({ cle: section.cle, y: section.haut, libelle: section.libelle, mois: true });
      if (section.etat !== 'charge') {
        attentes.push({
          cle: section.cle,
          y: section.haut + HAUTEUR_TITRE_MOIS,
          hauteur: section.hauteur - HAUTEUR_TITRE_MOIS,
        });
        continue;
      }
      for (const ligne of section.lignes) {
        const y = section.haut + ligne.y;
        if (ligne.type === 'jour') {
          if (y + HAUTEUR_TITRE_JOUR >= debut && y <= fin) {
            titres.push({ cle: ligne.cle, y, libelle: ligne.libelle, mois: false });
          }
          continue;
        }
        if (y + ligne.hauteur < debut || y > fin) continue;
        for (const c of ligne.cases) {
          tuiles.push({
            asset: section.assets[c.asset],
            section: s,
            index: c.asset,
            x: c.x,
            y,
            largeur: c.largeur,
            hauteur: ligne.hauteur,
          });
        }
      }
    }
    return { titres, tuiles, attentes };
  }

  asset(position: Position): Asset | null {
    return this.sections[position.section]?.assets[position.index] ?? null;
  }

  /** Next or previous asset, loading the neighbouring month when needed. */
  async voisin(position: Position, sens: 1 | -1): Promise<Position | null> {
    const section = this.sections[position.section];
    const index = position.index + sens;
    if (section && index >= 0 && index < section.assets.length) {
      return { section: position.section, index };
    }
    for (let s = position.section + sens; s >= 0 && s < this.sections.length; s += sens) {
      await this.chargerSection(s);
      const voisine = this.sections[s];
      if (voisine.etat === 'charge' && voisine.assets.length) {
        return { section: s, index: sens === 1 ? 0 : voisine.assets.length - 1 };
      }
    }
    return null;
  }

  /** Position of an asset in the timeline, to scroll to it after the viewer. */
  hautAsset(position: Position): number | null {
    const section = this.sections[position.section];
    if (!section || section.etat !== 'charge') return null;
    for (const ligne of section.lignes) {
      if (ligne.type === 'photos' && ligne.cases.some((c) => c.asset === position.index)) {
        return section.haut + ligne.y;
      }
    }
    return null;
  }

  /** Year marks for the scrubber: first month of every year. */
  annees(): { annee: number; haut: number }[] {
    const marques: { annee: number; haut: number }[] = [];
    let precedente = NaN;
    for (const section of this.sections) {
      if (section.annee !== precedente) {
        marques.push({ annee: section.annee, haut: section.haut });
        precedente = section.annee;
      }
    }
    return marques;
  }
}
