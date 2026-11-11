// Pulls the community translations of Immich (immich-app/immich, i18n/*.json,
// translated on Weblate) for every Nicéphore text that Immich also has, and
// writes one `src/lib/traductions/<langue>.json` per language. Run before a
// release (`npm run traductions`), so each version ships the latest work of
// the Immich translators. No dependency, Node 18+ (global fetch).
//
// Matching, from `src/lib/traductions/en.json` (French text -> English):
// - a Nicéphore French text equal to an Immich French text takes that key;
// - otherwise a Nicéphore English text equal to an Immich English text;
// - `immich-cles.json` forces a key, or `null` to refuse a wrong match.
// Texts with values ({nom}...) are taken only when Immich uses the same
// names; plurals (`a|b` keys, ICU in Immich) are left out. Anything missing
// falls back to English in the app.
//
// Usage: node scripts/traductions-immich.mjs [git ref, default main]
import { readFile, writeFile, readdir, unlink } from 'node:fs/promises';

const ref = process.argv[2] ?? 'main';
const racine = new URL('../src/lib/traductions/', import.meta.url);
const brut = (chemin) => `https://raw.githubusercontent.com/immich-app/immich/${ref}/${chemin}`;

async function json(url) {
  const reponse = await fetch(url);
  if (!reponse.ok) throw new Error(`${url} : ${reponse.status}`);
  return reponse.json();
}

// The texts of the @immich/ui components, with the Immich keys of the same name.
const CLES_IMMICH_UI = [
  'cancel',
  'close',
  'confirm',
  'expand',
  'collapse',
  'save',
  'open_menu',
  'show_password',
  'hide_password',
];

const normaliser = (s) => s.replace(/[’‘]/g, "'").replace(/[\s\u00a0\u202f]+/g, ' ').trim().replace(/[.!]$/, '').toLowerCase();
// Names of the values of a text: `{name}`, and `{count, plural, ...}` blocks.
const valeurs = (s) => [...new Set([...s.matchAll(/\{(\w+)(?:\}|,\s*plural,|,\s*number\})/g)].map((m) => m[1]))].sort().join(',');
const icu = (s) => /\{\w+,\s*(plural|select)/.test(s);
const pluriel = (s) => /\{\w+,\s*plural,/.test(s) && !/\{\w+,\s*select,/.test(s);
// `{count, number}`: one form for every count, also fine for a plural key.
const nombre = (s) => /\{\w+,\s*number\}/.test(s) && !/\{\w+,\s*select,/.test(s);

const nosTextes = JSON.parse(await readFile(new URL('en.json', racine), 'utf8'));
// French text -> Immich key, `{ cle, valeurs: { ours: immich } }` when the
// values have other names, or null to refuse an automatic match.
const forcees = JSON.parse(await readFile(new URL('immich-cles.json', racine), 'utf8').catch(() => '{}'));

// The list of Immich languages, from the repository tree.
const arbre = await json(`https://api.github.com/repos/immich-app/immich/git/trees/${ref}?recursive=1`);
const codes = arbre.tree
  .map((e) => e.path.match(/^i18n\/([\w-]+)\.json$/)?.[1])
  .filter((c) => c && c !== 'en' && c !== 'fr');

const immichFr = await json(brut('i18n/fr.json'));
const immichEn = await json(brut('i18n/en.json'));
const parFrancais = new Map();
const parAnglais = new Map();
for (const [cle, texte] of Object.entries(immichFr)) if (typeof texte === 'string' && !icu(texte)) parFrancais.set(normaliser(texte), cle);
for (const [cle, texte] of Object.entries(immichEn)) if (typeof texte === 'string' && !icu(texte)) parAnglais.set(normaliser(texte), cle);

/** Gives an Immich text the value names of Nicéphore. */
function renommer(texte, noms) {
  let sortie = texte;
  for (const [nous, immich] of Object.entries(noms)) {
    sortie = sortie.replace(new RegExp(`\\{${immich}(\\}|,\\s*plural,|,\\s*number\\})`, 'g'), `{${nous}$1`);
  }
  return sortie;
}

/** The Immich text usable for a Nicéphore text, or undefined. */
function adapter(francais, texte, noms) {
  if (typeof texte !== 'string' || !texte.trim()) return undefined;
  const forme = renommer(texte, noms);
  if (francais.includes('|')) return (pluriel(forme) || nombre(forme)) && valeurs(forme) === valeurs(francais) ? forme : undefined;
  // A plural block is fine too: `t` evaluates it with the values given.
  return (!icu(forme) || pluriel(forme)) && valeurs(forme) === valeurs(francais) ? forme : undefined;
}

// French text of Nicéphore -> { cle, noms }.
const correspondances = {};
for (const [francais, anglais] of Object.entries(nosTextes)) {
  let lien;
  if (francais in forcees) {
    const f = forcees[francais];
    lien = f === null ? undefined : typeof f === 'string' ? { cle: f, noms: {} } : { cle: f.cle, noms: f.valeurs ?? {} };
  } else if (!francais.includes('|')) {
    const cle = parFrancais.get(normaliser(francais)) ?? parAnglais.get(normaliser(anglais));
    if (cle) lien = { cle, noms: {} };
  }
  if (lien && adapter(francais, immichEn[lien.cle], lien.noms)) correspondances[francais] = lien;
  else if (lien && francais in forcees) console.warn(`refusé (valeurs ou format) : ${francais} -> ${lien.cle}`);
}

// Old generated files go, in case Immich dropped a language.
for (const fichier of await readdir(racine)) {
  if (/^[\w-]+\.json$/.test(fichier) && !['en.json', 'immich-cles.json', 'langues.json'].includes(fichier)) {
    await unlink(new URL(fichier, racine));
  }
}

const langues = [];
for (const code of codes) {
  const textes = await json(brut(`i18n/${code}.json`)).catch(() => null);
  if (!textes) continue;
  const sortie = {};
  for (const [francais, { cle, noms }] of Object.entries(correspondances)) {
    const texte = adapter(francais, textes[cle], noms);
    if (texte) sortie[francais] = texte;
  }
  const immichUi = {};
  for (const cle of CLES_IMMICH_UI) if (typeof textes[cle] === 'string' && textes[cle].trim()) immichUi[cle] = textes[cle];
  if (Object.keys(sortie).length === 0) continue;
  const fichier = code.replace(/_/g, '-');
  await writeFile(new URL(`${fichier}.json`, racine), JSON.stringify({ ...sortie, _immichUi: immichUi }, null, 2) + '\n');
  langues.push({ code: fichier, textes: Object.keys(sortie).length });
}

langues.sort((a, b) => b.textes - a.textes || a.code.localeCompare(b.code));
const total = Object.keys(nosTextes).length;
await writeFile(
  new URL('langues.json', racine),
  JSON.stringify({ source: `immich-app/immich@${ref}`, total, communs: Object.keys(correspondances).length, langues }, null, 2) + '\n',
);
console.log(`${Object.keys(correspondances).length} textes communs avec Immich sur ${total}, ${langues.length} langues écrites.`);
