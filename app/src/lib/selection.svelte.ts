// Multi-selection of a timeline: selected assets by id (with the asset, so
// actions know e.g. whether they are all favorites), plus the anchor of the
// last click for shift+click ranges.
import { SvelteMap } from 'svelte/reactivity';
import type { Asset, Position } from './modele-chronologie.svelte';

export class Selection {
  elements = new SvelteMap<string, Asset>();
  ancre: Position | null = null;

  get taille() {
    return this.elements.size;
  }

  get active() {
    return this.elements.size > 0;
  }

  contient(id: string) {
    return this.elements.has(id);
  }

  basculer(asset: Asset, position: Position) {
    if (this.elements.has(asset.id)) this.elements.delete(asset.id);
    else this.elements.set(asset.id, asset);
    this.ancre = position;
  }

  ajouter(assets: Asset[]) {
    for (const a of assets) this.elements.set(a.id, a);
  }

  retirerTous(assets: Asset[]) {
    for (const a of assets) this.elements.delete(a.id);
  }

  vider() {
    this.elements.clear();
    this.ancre = null;
  }

  ids() {
    return [...this.elements.keys()];
  }

  tousFavoris() {
    return [...this.elements.values()].every((a) => a.favori);
  }
}
