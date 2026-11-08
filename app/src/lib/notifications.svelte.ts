// Small toasts at the bottom of the window, optionally with an action
// (e.g. "Undo" after moving photos to the trash).
export type Notification = {
  id: number;
  texte: string;
  type: 'info' | 'succes' | 'erreur';
  action?: { libelle: string; executer: () => void };
};

export const notifications = $state<Notification[]>([]);
let suivant = 1;

export function notifier(
  texte: string,
  type: Notification['type'] = 'info',
  action?: Notification['action'],
  duree = type === 'erreur' ? 9000 : 5000,
): number {
  const id = suivant++;
  notifications.push({ id, texte, type, action });
  if (duree > 0) setTimeout(() => retirer(id), duree);
  return id;
}

export function remplacer(id: number, texte: string, type: Notification['type'] = 'info', duree = 5000) {
  const n = notifications.find((x) => x.id === id);
  if (!n) return notifier(texte, type, undefined, duree);
  n.texte = texte;
  n.type = type;
  if (duree > 0) setTimeout(() => retirer(id), duree);
  return id;
}

export function retirer(id: number) {
  const index = notifications.findIndex((n) => n.id === id);
  if (index >= 0) notifications.splice(index, 1);
}
