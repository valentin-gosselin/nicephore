// Development benchmark: scripted scrolling through the real timeline, with
// frame timing, "time to a full screen" and screenshots. Enabled only when
// the Rust side runs with NICEPHORE_BANC_DOSSIER set.
import { goto } from '$app/navigation';
import { invoke } from '@tauri-apps/api/core';

export type ControleBanc = {
  conteneur: HTMLElement;
  hauteurTotale: () => number;
  allerA: (y: number) => void;
  ouvrirPremiere: () => boolean;
  ouvrirPremiereVideo: () => boolean;
  basculerInfos: () => void;
  fermer: () => void;
  nombreSelectionnes: () => number;
  etatMois: () => string;
};

const journal = (message: string) => invoke('journal', { message });
const capture = (nom: string) => invoke('capture', { nom }).catch((e) => journal(`capture ${nom} : ${e}`));
const attendre = (ms: number) => new Promise((r) => setTimeout(r, ms));
const image = () => new Promise<number>((r) => requestAnimationFrame(r));

/** Visible tiles, and how many already show their real thumbnail. */
function remplissage(conteneur: HTMLElement) {
  const vue = conteneur.getBoundingClientRect();
  let total = 0;
  let pretes = 0;
  let demandees = 0;
  for (const tuile of conteneur.querySelectorAll<HTMLElement>('.tuile')) {
    const b = tuile.getBoundingClientRect();
    if (b.bottom < vue.top || b.top > vue.bottom) continue;
    total++;
    const img = tuile.querySelector('img');
    if (img) demandees++;
    if (img && img.complete && img.naturalWidth > 0 && !img.classList.contains('opacity-0')) pretes++;
  }
  return { total, pretes, demandees };
}

/** Milliseconds until every visible tile shows its thumbnail (max 15 s). */
async function tempsRemplissage(conteneur: HTMLElement): Promise<number> {
  const debut = performance.now();
  while (performance.now() - debut < 15_000) {
    const { total, pretes } = remplissage(conteneur);
    if (total > 0 && pretes === total) return Math.round(performance.now() - debut);
    await attendre(50);
  }
  return -1;
}

/** Scrolls at `ecransParSeconde` for `duree` ms and measures frame times. */
async function defiler(c: ControleBanc, ecransParSeconde: number, duree: number) {
  const intervalles: number[] = [];
  const echantillons: number[] = [];
  const demandes: number[] = [];
  const hauteurVue = c.conteneur.clientHeight;
  const depart = c.conteneur.scrollTop;
  let position = depart;
  let precedent = await image();
  const debut = precedent;
  let prochainEchantillon = debut + 250;
  while (precedent - debut < duree) {
    const t = await image();
    const dt = t - precedent;
    intervalles.push(dt);
    // Track the position ourselves: scrollTop may round fractional steps.
    position += (ecransParSeconde * hauteurVue * dt) / 1000;
    c.conteneur.scrollTop = position;
    if (t >= prochainEchantillon) {
      const { total, pretes, demandees } = remplissage(c.conteneur);

      if (total) {
        echantillons.push(pretes / total);
        demandes.push(demandees / total);
      }
      prochainEchantillon += 250;
    }
    precedent = t;
  }
  intervalles.sort((a, b) => a - b);
  const moyenne = intervalles.reduce((s, v) => s + v, 0) / intervalles.length;
  const p95 = intervalles[Math.floor(intervalles.length * 0.95)];
  const saccades = intervalles.filter((v) => v > 50).length;
  const rempli = echantillons.length
    ? Math.round((100 * echantillons.reduce((s, v) => s + v, 0)) / echantillons.length)
    : 0;
  return {
    parcouru: Math.round((c.conteneur.scrollTop - depart) / hauteurVue),
    ips: Math.round(1000 / moyenne),
    p95: Math.round(p95 * 10) / 10,
    max: Math.round(intervalles[intervalles.length - 1]),
    saccades,
    rempli,
    demande: demandes.length ? Math.round((100 * demandes.reduce((s, v) => s + v, 0)) / demandes.length) : 0,
  };
}

export async function lancerBanc(c: ControleBanc, debutChargement: number) {
  try {
    const el = c.conteneur;
    const avant = el.scrollTop;
    el.scrollTop = 500;
    await image();
    await journal(
      `diagnostic : scrollHeight ${el.scrollHeight}, clientHeight ${el.clientHeight}, ` +
        `scrollTop ${avant} puis ${el.scrollTop} après 500, overflowY ${getComputedStyle(el).overflowY}`,
    );
    const enfants = [...el.children].map(
      (e) => `${e.tagName}.${(e as HTMLElement).className} style="${e.getAttribute('style')}" h=${(e as HTMLElement).offsetHeight}`,
    );
    await journal(`enfants : ${enfants.join(' | ')} ; total calculé ${c.hauteurTotale()}`);
    el.scrollTop = avant;
    await image();
    const premier = await tempsRemplissage(c.conteneur);
    await journal(`  ${await invoke('stats_images')}`);
    await journal(`premier écran rempli en ${premier} ms (${Math.round(performance.now() - debutChargement)} ms depuis l'ouverture de la chronologie)`);
    await attendre(300);
    await capture('1-haut');

    const normal = await defiler(c, 1.5, 8000);
    await journal(`défilement 1,5 écran/s : ${JSON.stringify(normal)}`);
    await journal(`  ${await invoke('stats_images')}`);
    await journal(`  écran rempli après arrêt en ${await tempsRemplissage(c.conteneur)} ms`);

    const avantRapide = c.etatMois();
    const rapide = await defiler(c, 8, 3000);
    await journal(`défilement rapide 8 écrans/s : ${JSON.stringify(rapide)}`);
    await journal(`  mois avant ${avantRapide}, après ${c.etatMois()}`);
    await journal(`  écran rempli après arrêt en ${await tempsRemplissage(c.conteneur)} ms`);

    for (const [nom, part] of [['2-milieu', 0.5], ['3-anciennes', 0.9]] as const) {
      c.allerA(c.hauteurTotale() * part);
      await journal(`saut à ${part * 100} % : écran rempli en ${await tempsRemplissage(c.conteneur)} ms`);
      await attendre(300);
      await capture(nom);
    }

    c.allerA(0);
    await tempsRemplissage(c.conteneur);
    if (c.ouvrirPremiere()) {
      await attendre(1500);
      await capture('4-visionneuse');
      c.basculerInfos();
      await attendre(1200);
      await capture('5-infos');
      c.fermer();
    }

    await attendre(500);
    if (c.ouvrirPremiereVideo()) {
      await attendre(4000);
      const video = document.querySelector('video');
      await journal(
        video
          ? `vidéo : prête ${video.readyState}/4, lue jusqu'à ${video.currentTime.toFixed(1)} s sur ${video.duration.toFixed(1)} s, erreur ${video.error?.message ?? 'aucune'}`
          : 'vidéo : aucun lecteur affiché',
      );
      await capture('6-video');
      c.fermer();
    } else {
      await journal('vidéo : aucune vidéo visible pour le test');
    }
    // Multi-selection: three clicks on the check marks, then a shift+click range.
    await attendre(800);
    const coches = [...c.conteneur.querySelectorAll<HTMLButtonElement>('.tuile .coche')].filter((b) => {
      const r = b.getBoundingClientRect();
      return r.top > c.conteneur.getBoundingClientRect().top && r.bottom < innerHeight;
    });
    for (const coche of coches.slice(0, 3)) coche.click();
    const derniere = coches[Math.min(9, coches.length - 1)];
    derniere?.dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true }));
    await attendre(400);
    await journal(`sélection : ${c.nombreSelectionnes()} éléments (3 clics puis Maj+clic sur la ${Math.min(10, coches.length)}e coche visible)`);
    await capture('7-selection');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));

    // Every view of the sidebar, read only.
    for (const [chemin, nom] of [
      ['/favoris', '8-favoris'],
      ['/archive', '9-archive'],
      ['/corbeille', '10-corbeille'],
      ['/recherche?texte=concert', '12-recherche-concert'],
      ['/recherche?type=video&du=2025-01-01', '13-recherche-videos'],
      ['/explorer', '14-explorer'],
      ['/reglages', '11-reglages'],
    ]) {
      const debutVue = performance.now();
      await goto(chemin);
      await attendre(chemin.startsWith('/explorer') ? 4000 : 2500);
      await journal(`vue ${chemin} : ${document.querySelector('main')?.textContent?.replace(/\s+/g, ' ').trim().slice(0, 90)}`);
      await capture(nom);
      if (chemin === '/explorer') {
        const personne = document.querySelector<HTMLAnchorElement>('a.personne');
        await journal(`explorer : ${document.querySelectorAll('a.personne').length} personnes, ${document.querySelectorAll('a.lieu').length} lieux`);
        if (personne) {
          personne.click();
          await attendre(2500);
          await journal(`page personne : ${document.querySelector('main')?.textContent?.replace(/\s+/g, ' ').trim().slice(0, 90)}`);
          await capture('15-personne');
          // Back to Explorer, then forward to the person again.
          document.querySelector<HTMLButtonElement>('[aria-label^="Retour"]')?.click();
          await attendre(1500);
          const apresRetour = location.pathname;
          document.querySelector<HTMLButtonElement>('[aria-label^="Avancer"]')?.click();
          await attendre(1500);
          await journal(`navigation : retour vers ${apresRetour}, avance vers ${location.pathname}`);
        }
      }
      void debutVue;
    }
    await journal('banc terminé');
  } catch (e) {
    await journal(`banc interrompu : ${e}`);
  }
  await invoke('quitter');
}
