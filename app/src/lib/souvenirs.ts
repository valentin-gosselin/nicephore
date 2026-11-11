// Wording of memories, shared by the strip and the memory page.
import type { Souvenir } from './api';
import { t } from './i18n.svelte';

export function titreSouvenir(s: Souvenir): string {
  if (s.genre === 'birthday' && s.personne) return t('Anniversaire de {personne}', { personne: s.personne });
  const ecart = new Date().getFullYear() - s.annee;
  return ecart <= 0 ? t('Cette année') : ecart === 1 ? t('Il y a un an') : t('Il y a {n} ans', { n: ecart });
}

/** Local date as YYYY-MM-DD (the server picks the memories of that day). */
export function aujourdhui(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
