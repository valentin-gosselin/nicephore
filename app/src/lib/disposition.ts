// Justified row layout, as in the Immich webapp: every row fills the width
// exactly, and its height stays as close as possible to the target height.

export type Case = { index: number; x: number; largeur: number };
export type Rangee = { y: number; hauteur: number; cases: Case[] };

/** Extreme panoramas and very tall images are clamped to keep rows usable. */
const borner = (ratio: number) => Math.min(Math.max(ratio || 1, 0.3), 3.5);

export function disposer(
  ratios: number[],
  largeur: number,
  hauteurCible: number,
  ecart: number,
): { rangees: Rangee[]; hauteur: number } {
  const rangees: Rangee[] = [];
  let y = 0;
  let debut = 0;
  let somme = 0;

  const fermer = (fin: number, hauteur: number) => {
    const cases: Case[] = [];
    let x = 0;
    for (let i = debut; i < fin; i++) {
      const l = borner(ratios[i]) * hauteur;
      cases.push({ index: i, x, largeur: l });
      x += l + ecart;
    }
    // Absorb rounding so the row ends exactly on the right edge.
    if (cases.length && hauteur !== hauteurCible) {
      const derniere = cases[cases.length - 1];
      derniere.largeur = largeur - derniere.x;
    }
    rangees.push({ y, hauteur, cases });
    y += hauteur + ecart;
    debut = fin;
    somme = 0;
  };

  for (let i = 0; i < ratios.length; i++) {
    somme += borner(ratios[i]);
    const n = i - debut + 1;
    const largeurACible = somme * hauteurCible + (n - 1) * ecart;
    if (largeurACible < largeur) continue;

    // The row overflows: keep the last image or push it to the next row,
    // whichever gives a height closer to the target.
    const avec = (largeur - (n - 1) * ecart) / somme;
    const sommeSans = somme - borner(ratios[i]);
    const sans = n > 1 ? (largeur - (n - 2) * ecart) / sommeSans : Infinity;
    if (n > 1 && Math.abs(sans - hauteurCible) < Math.abs(avec - hauteurCible) && sans < hauteurCible * 1.5) {
      fermer(i, sans);
      somme = borner(ratios[i]);
    } else {
      fermer(i + 1, avec);
    }
  }
  // The last row keeps the target height instead of being stretched.
  if (debut < ratios.length) fermer(ratios.length, hauteurCible);

  return { rangees, hauteur: Math.max(0, y - ecart) };
}
