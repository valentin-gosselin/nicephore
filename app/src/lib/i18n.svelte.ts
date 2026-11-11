// Interface languages. The French text is the key: components write
// `t('Rechercher par type')`, and `traductions/<langue>.json` maps each
// French text to its translation. A text missing in a language falls back
// to English, then to French.
//
// English is written for Nicéphore. The other languages come from the
// community translations of Immich, for the texts both apps share:
// `npm run traductions` (scripts/traductions-immich.mjs) refreshes them,
// and runs before each release.
//
// Values go in braces: `t('{nom} + {n} personnes', { nom, n })`. Plurals
// use `tp(n, '{n} photo', '{n} photos')`: the key is the singular and the
// plural joined by "|", and the translation has the same two forms.
//
// `t` reads `langue.code` and the loaded texts, both $state: a call made in
// a template or a $derived follows a change of language. A text computed
// once at module level would not: keep those as functions or $derived.
import { setLocale, setTranslations } from '@immich/ui';
import en from './traductions/en.json';
import resume from './traductions/langues.json';

/** A language code: `fr`, `en`, `de`, `pt-BR`, `zh-Hans`... */
export type Langue = string;
export type ChoixLangue = Langue | 'systeme';

// Generated languages, loaded on demand.
const fichiers = import.meta.glob<{ default: Record<string, unknown> }>('./traductions/*.json');
const fichierDe = (code: string) => fichiers[`./traductions/${code}.json`];
// Machine translations of the texts Immich does not have (`traductions/auto`,
// never touched by the Immich script). The community translation wins.
const fichiersAuto = import.meta.glob<{ default: Record<string, unknown> }>('./traductions/auto/*.json');
const fichierAutoDe = (code: string) => fichiersAuto[`./traductions/auto/${code}.json`];

const nomDe = (code: string) => {
  try {
    const nom = new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code;
    return nom.charAt(0).toLocaleUpperCase(code) + nom.slice(1);
  } catch {
    return code;
  }
};

/** The languages offered, with the share of the interface they translate. */
export const LANGUES: { code: Langue; nom: string; part: number }[] = [
  { code: 'fr', nom: 'Français', part: 1 },
  { code: 'en', nom: 'English', part: 1 },
  ...resume.langues
    .filter((l) => fichierDe(l.code))
    .map((l) => ({ code: l.code, nom: nomDe(l.code), part: l.textes / resume.total }))
    .sort((a, b) => a.nom.localeCompare(b.nom)),
];

const dictionnaires = $state<Record<string, Record<string, string>>>({ fr: {}, en });

const CLE = 'nicephore-langue';

function lireChoix(): ChoixLangue {
  try {
    const v = localStorage.getItem(CLE);
    if (v === 'systeme' || LANGUES.some((l) => l.code === v)) return v as ChoixLangue;
  } catch {
    // Default below.
  }
  return 'systeme';
}

/** The offered language closest to `choix` ("de-AT" gives "de"). */
function deduire(choix: ChoixLangue): Langue {
  const demande = choix !== 'systeme' ? choix : typeof navigator !== 'undefined' ? navigator.language : 'fr';
  const codes = LANGUES.map((l) => l.code);
  const exact = codes.find((c) => c.toLowerCase() === demande.toLowerCase());
  if (exact) return exact;
  const base = demande.split(/[-_]/)[0].toLowerCase();
  return codes.find((c) => c.toLowerCase() === base) ?? codes.find((c) => c.split('-')[0] === base) ?? 'en';
}

const choixInitial = lireChoix();
export const langue = $state<{ choix: ChoixLangue; code: Langue }>({ choix: choixInitial, code: deduire(choixInitial) });

/** BCP 47 locale for dates and numbers (`toLocaleString`, `Intl`). */
export const locale = () => (langue.code === 'fr' ? 'fr-FR' : langue.code === 'en' ? 'en-US' : langue.code);

/** The translation of `cle` in the current language, English failing that. */
function traduction(cle: string): string | undefined {
  if (langue.code === 'fr') return undefined;
  return dictionnaires[langue.code]?.[cle] ?? (en as Record<string, string>)[cle];
}

/**
 * Fills `{name}` values and evaluates ICU plural blocks as Immich writes
 * them (`{count, plural, one {# photo} other {# photos}}`, with `=0` and the
 * plural categories of the language).
 */
function remplir(texte: string, valeurs?: Record<string, string | number>, nombres: Record<string, number> = {}) {
  let sortie = '';
  let i = 0;
  while (i < texte.length) {
    const debut = texte.indexOf('{', i);
    if (debut < 0) break;
    const entete = /^\{(\w+),\s*plural,\s*/.exec(texte.slice(debut));
    if (!entete) {
      sortie += texte.slice(i, debut + 1);
      i = debut + 1;
      continue;
    }
    // The whole block, braces balanced.
    let profondeur = 0;
    let fin = debut;
    for (; fin < texte.length; fin++) {
      if (texte[fin] === '{') profondeur++;
      else if (texte[fin] === '}' && --profondeur === 0) break;
    }
    const nom = entete[1];
    const n = nombres[nom] ?? (Number(String(valeurs?.[nom] ?? 0).replace(/[\s\u202f,]/g, '')) || 0);
    const options = new Map<string, string>();
    const corps = texte.slice(debut + entete[0].length, fin);
    for (let j = 0; j < corps.length; ) {
      const selecteur = /^\s*(=\d+|\w+)\s*\{/.exec(corps.slice(j));
      if (!selecteur) break;
      let k = j + selecteur[0].length;
      let p = 1;
      const ouvre = k;
      for (; k < corps.length && p > 0; k++) {
        if (corps[k] === '{') p++;
        else if (corps[k] === '}') p--;
      }
      options.set(selecteur[1], corps.slice(ouvre, k - 1));
      j = k;
    }
    const forme = options.get(`=${n}`) ?? options.get(new Intl.PluralRules(locale()).select(n)) ?? options.get('other') ?? '';
    sortie += texte.slice(i, debut) + forme.replace(/#/g, n.toLocaleString(locale()));
    i = fin + 1;
  }
  sortie += texte.slice(i);
  // `{count, number}`: the number in the language's format.
  sortie = sortie.replace(/\{(\w+),\s*number\}/g, (_, nom) =>
    (nombres[nom] ?? (Number(String(valeurs?.[nom] ?? 0).replace(/[\s\u202f,]/g, '')) || 0)).toLocaleString(locale()),
  );
  if (!valeurs) return sortie;
  return sortie.replace(/\{(\w+)\}/g, (tout, nom) => (nom in valeurs ? String(valeurs[nom]) : tout));
}

/** The text in the interface language. */
export function t(texte: string, valeurs?: Record<string, string | number>): string {
  return remplir(traduction(texte) ?? texte, valeurs);
}

/** Singular or plural form for `n` (also passed as `{n}`). */
export function tp(n: number, singulier: string, pluriel: string, valeurs?: Record<string, string | number>): string {
  const traduit = traduction(`${singulier}|${pluriel}`);
  const tout = { n: n.toLocaleString(locale()), ...valeurs };
  // From Immich: an ICU plural block, any number of forms.
  if (traduit && /\{\w+,\s*(plural,|number\})/.test(traduit)) return remplir(traduit, tout, { n });
  const [un, plusieurs] = traduit ? traduit.split('|') : [singulier, pluriel];
  // French puts 0 and 1 in the singular, English only 1.
  const forme = new Intl.PluralRules(locale()).select(n) === 'one' ? un : plusieurs;
  return remplir(forme, tout);
}

// Texts of the @immich/ui components (calendars, menus, toasts).
const textesImmichUi: Record<'fr' | 'en', Record<string, string>> = {
  fr: {
    cancel: 'Annuler',
    close: 'Fermer',
    confirm: 'Confirmer',
    expand: 'Déplier',
    collapse: 'Replier',
    search_placeholder: 'Rechercher…',
    search_no_results: 'Aucun résultat',
    show_password: 'Afficher le mot de passe',
    hide_password: 'Masquer le mot de passe',
    open_menu: 'Ouvrir le menu',
    navigate_next: 'Suivant',
    navigate_previous: 'Précédent',
    open_calendar: 'Ouvrir le calendrier',
    toast_success_title: 'Réussi',
    toast_info_title: 'Information',
    toast_warning_title: 'Attention',
    toast_danger_title: 'Erreur',
    save: 'Enregistrer',
  },
  en: {
    cancel: 'Cancel',
    close: 'Close',
    confirm: 'Confirm',
    expand: 'Expand',
    collapse: 'Collapse',
    search_placeholder: 'Search…',
    search_no_results: 'No results',
    show_password: 'Show password',
    hide_password: 'Hide password',
    open_menu: 'Open menu',
    navigate_next: 'Next',
    navigate_previous: 'Previous',
    open_calendar: 'Open calendar',
    toast_success_title: 'Success',
    toast_info_title: 'Info',
    toast_warning_title: 'Warning',
    toast_danger_title: 'Error',
    save: 'Save',
  },
};

function appliquer() {
  setLocale(locale());
  const autres = (dictionnaires[langue.code] as Record<string, unknown> | undefined)?._immichUi as Record<string, string> | undefined;
  setTranslations(langue.code === 'fr' ? textesImmichUi.fr : { ...textesImmichUi.en, ...autres });
  if (typeof document !== 'undefined') document.documentElement.lang = langue.code;
}
/** Loads the texts of a generated language once, then applies it. */
async function charger(code: Langue) {
  const fichier = fichierDe(code);
  const auto = fichierAutoDe(code);
  if ((fichier || auto) && !dictionnaires[code]) {
    try {
      const [immich, machine] = await Promise.all([fichier?.(), auto?.()]);
      dictionnaires[code] = { ...(machine?.default ?? {}), ...(immich?.default ?? {}) } as Record<string, string>;
    } catch {
      // English stands in.
    }
  }
  if (langue.code === code) appliquer();
}

appliquer();
void charger(langue.code);

/** Changes the language, remembered for the next launches. */
export function choisirLangue(choix: ChoixLangue) {
  langue.choix = choix;
  langue.code = deduire(choix);
  try {
    localStorage.setItem(CLE, choix);
  } catch {
    // Only a convenience.
  }
  appliquer();
  void charger(langue.code);
}
