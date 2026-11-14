// Zoom and pan of a photo in a viewer: the wheel zooms around the pointer,
// a double click goes between "fit" and 100 %, dragging pans, and the photo
// never leaves the area it fills.

const borne = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

export class Zoom {
  /** 1: the photo fits the area. */
  echelle = $state(1);
  x = $state(0);
  y = $state(0);
  /** Scale where one pixel of the photo is one pixel of the screen. */
  reel = $state(1);
  /** Fitted size of the photo as shown (rotation included) and of the area. */
  private ajuste = { l: 0, h: 0 };
  private zone = { l: 0, h: 0 };
  private glisse: { x: number; y: number } | null = null;

  get zoome() {
    return this.echelle > 1.001;
  }

  /** Zoom in percent of the photo's real size. */
  get pourcentage() {
    return Math.round((this.echelle / this.reel) * 100);
  }

  private get maximum() {
    return Math.max(4, this.reel * 4);
  }

  dimensionner(ajusteL: number, ajusteH: number, zoneL: number, zoneH: number, reel: number) {
    this.ajuste = { l: ajusteL, h: ajusteH };
    this.zone = { l: zoneL, h: zoneH };
    this.reel = reel > 0 ? reel : 1;
    this.borner();
  }

  reinitialiser() {
    this.echelle = 1;
    this.x = 0;
    this.y = 0;
    this.glisse = null;
  }

  /** Zooms to `echelle` keeping the point (px, py) of the area (from its centre) in place. */
  zoomerVers(echelle: number, px = 0, py = 0) {
    const nouvelle = borne(echelle, 1, this.maximum);
    const k = nouvelle / this.echelle;
    this.x = px - (px - this.x) * k;
    this.y = py - (py - this.y) * k;
    this.echelle = nouvelle;
    this.borner();
  }

  /** One step in or out (keyboard, buttons). */
  pas(sens: 1 | -1) {
    this.zoomerVers(this.echelle * (sens > 0 ? 1.25 : 1 / 1.25));
  }

  molette(e: WheelEvent) {
    e.preventDefault();
    const boite = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // Lines (mouse wheels on some systems) or pixels (touchpads).
    const delta = e.deltaMode === 1 ? e.deltaY * 33 : e.deltaY;
    const facteur = Math.exp(-borne(delta, -120, 120) * 0.0018);
    this.zoomerVers(this.echelle * facteur, e.clientX - boite.left - boite.width / 2, e.clientY - boite.top - boite.height / 2);
  }

  /** Fit, or 100 % (twice the fit for photos smaller than the area). */
  doubleClic(e: MouseEvent) {
    if (this.zoome) return this.reinitialiser();
    const boite = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const cible = this.reel > 1.2 ? this.reel : 2;
    this.zoomerVers(cible, e.clientX - boite.left - boite.width / 2, e.clientY - boite.top - boite.height / 2);
  }

  appui(e: PointerEvent) {
    if (!this.zoome || e.button !== 0) return;
    this.glisse = { x: e.clientX - this.x, y: e.clientY - this.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  mouvement(e: PointerEvent) {
    if (!this.glisse) return;
    this.x = e.clientX - this.glisse.x;
    this.y = e.clientY - this.glisse.y;
    this.borner();
  }

  relache() {
    this.glisse = null;
  }

  get glisseEnCours() {
    return this.glisse !== null;
  }

  /** Keeps the edges of the zoomed photo beyond the area's edges. */
  private borner() {
    const maxX = Math.max(0, (this.ajuste.l * this.echelle - this.zone.l) / 2);
    const maxY = Math.max(0, (this.ajuste.h * this.echelle - this.zone.h) / 2);
    this.x = borne(this.x, -maxX, maxX);
    this.y = borne(this.y, -maxY, maxY);
  }
}
