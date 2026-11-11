// Tour of the showcase server for the README screenshots (scripts/vitrine.sh
// captures): maximized window, then the main pages, each captured once its
// images are loaded. Reads only; local showcase server only.
import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import { t } from './i18n.svelte';

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch(() => {});
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function quand(condition: () => boolean, essais = 150) {
  for (let i = 0; i < essais && !condition(); i++) await attendre(100);
  return condition();
}

/** Every image on screen loaded (thumbnails, covers, viewer). */
async function imagesChargees() {
  await quand(() => {
    const visibles = [...document.images].filter((img) => {
      const r = img.getBoundingClientRect();
      return r.width > 0 && r.bottom > 0 && r.top < window.innerHeight;
    });
    // Timeline tiles get their <img> once the thumbnail is known.
    const tuiles = [...document.querySelectorAll('.image-tuile')].filter((t) => t.getBoundingClientRect().top < window.innerHeight);
    return (
      visibles.length > 0 &&
      visibles.every((img) => img.complete && img.naturalWidth > 0) &&
      tuiles.every((t) => (t.querySelector('img')?.naturalWidth ?? 0) > 0)
    );
  }, 300);
  await attendre(800);
}

async function aller(chemin: string) {
  await goto(chemin);
  await attendre(1500);
  await imagesChargees();
}

const element = <T extends Element>(selecteur: string, filtre: (e: T) => boolean = () => true) =>
  quand(() => [...document.querySelectorAll<T>(selecteur)].some(filtre)).then(() => [...document.querySelectorAll<T>(selecteur)].find(filtre));

export async function lancerScenarioVitrine() {
  try {
    // Through the title bar button, like a user (the window API is not allowed).
    (await element<HTMLButtonElement>(`[aria-label="${t('Agrandir la fenêtre')}"]`))?.click();
    await attendre(1500);
    await journal('vitrine');

    await aller('/');
    await capture('v1-photos');

    // The Immich search panel, from the header.
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }));
    await quand(() => !!document.querySelector('#panneau-recherche'));
    await attendre(800);
    await capture('v2-recherche');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    (document.activeElement as HTMLElement | null)?.blur();

    await aller('/albums');
    await capture('v3-albums');

    (await element<HTMLAnchorElement>('a.album', (a) => (a.textContent ?? '').includes('Venice')))?.click();
    await attendre(1500);
    await imagesChargees();
    await capture('v4-album');

    // A photo with its info panel.
    (await element<HTMLElement>('.tuile .image-tuile'))?.click();
    await attendre(1500);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'i' }));
    await attendre(1500);
    await imagesChargees();
    await capture('v5-visionneuse');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await attendre(800);

    await aller('/carte');
    await attendre(5000);
    await capture('v6-carte');

    await aller('/explorer');
    await capture('v7-explorer');

    await aller('/synchro');
    (await element<HTMLButtonElement>('button', (b) => (b.textContent ?? '').includes(t('Ajouter un dossier'))))?.click();
    await element('input[name="sens"]');
    await attendre(800);
    await capture('v8-synchro');

    await journal('vitrine terminée');
  } catch (e) {
    await journal(`vitrine interrompue : ${e}`);
  }
  await invoke('quitter');
}
