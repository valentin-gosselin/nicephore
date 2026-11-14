// Right-click menus with the @immich/ui context menu, opened under the
// pointer (it anchors on an element: a point-sized one is placed there).
import { menuManager, type MenuItems } from '@immich/ui';

let ancre: HTMLDivElement | null = null;
let ouvert = false;
let fermeLe = 0;

export function ouvrirMenu(e: MouseEvent, items: MenuItems) {
  e.preventDefault();
  e.stopPropagation();
  ancre ??= document.body.appendChild(document.createElement('div'));
  Object.assign(ancre.style, { position: 'fixed', left: `${e.clientX}px`, top: `${e.clientY}px`, width: '0', height: '0', pointerEvents: 'none' });
  ouvert = true;
  void menuManager.show({ target: ancre, items, position: 'top-left', size: 'medium' }).finally(() => {
    ouvert = false;
    fermeLe = performance.now();
  });
}

/**
 * A menu is open, or was just closed by the key being handled: viewers
 * leave that key alone (Escape closes the menu, not the viewer).
 */
export const menuActif = () => ouvert || performance.now() - fermeLe < 400;
