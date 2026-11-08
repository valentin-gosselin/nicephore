// Flat list of assets (search results, a place...) shown with the same grid,
// viewer and selection as the timeline: one untitled section, filled page by
// page as the user scrolls towards its end.
import type { PageResultats } from './api';
import { CLE_LISTE, Chronologie, depuisResume, type Position } from './modele-chronologie.svelte';

export class ChronologieListe extends Chronologie {
  /** Next page to load, or null when everything is there. */
  private page: number | null = 1;
  private enChargement: Promise<void> | null = null;
  private vus = new Set<string>();
  /** More results exist beyond the loaded ones. */
  plusDeResultats = $state(false);

  constructor(private chargerPage: (page: number) => Promise<PageResultats>) {
    super();
    this.parJour = false;
    this.afficherFrise = false;
  }

  override async demarrer() {
    this.initialiser([{ timeBucket: CLE_LISTE, count: 0 }]);
    this.sections[0].etat = 'charge';
    await this.suite();
  }

  /** Loads the next page (once at a time). */
  private suite(): Promise<void> {
    if (this.page == null) return Promise.resolve();
    if (this.enChargement) return this.enChargement;
    const section = this.sections[0];
    const page = this.page;
    this.enChargement = this.chargerPage(page)
      .then((resultat) => {
        for (const brut of resultat.assets) {
          // Pages may overlap when the library changes meanwhile.
          if (this.vus.has(brut.id)) continue;
          this.vus.add(brut.id);
          section.assets.push(depuisResume(brut));
        }
        section.nombre = section.assets.length;
        this.nombreTotal = section.assets.length;
        this.page = resultat.pageSuivante;
        this.plusDeResultats = resultat.pageSuivante != null;
        this.disposerSection(section);
        this.recalculer();
        this.surChangement?.();
      })
      .finally(() => (this.enChargement = null));
    return this.enChargement;
  }

  /** Loads more when the end of the list comes within a screen or so. */
  override demander(_debut: number, fin: number) {
    const section = this.sections[0];
    if (section && fin > section.haut + section.hauteur - 800) void this.suite();
  }

  override chargerSection(): Promise<void> {
    return this.suite();
  }

  override async voisin(position: Position, sens: 1 | -1): Promise<Position | null> {
    const section = this.sections[0];
    if (sens === 1 && section && position.index + 1 >= section.assets.length) await this.suite();
    return super.voisin(position, sens);
  }

  /** A flat list has no month index: run the search again. */
  override actualiserMois() {
    return this.rafraichir();
  }

  /** After an action (trash, archive...): run the search again. */
  override async rafraichir() {
    const section = this.sections[0];
    section.assets = [];
    this.vus.clear();
    this.page = 1;
    await this.suite();
  }
}
