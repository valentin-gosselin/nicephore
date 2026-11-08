// Blurry placeholders decoded from Immich thumbhashes, shown while the real
// thumbnail loads. Decoding is cheap but cached anyway (fast scroll back).
import { thumbHashToDataURL } from 'thumbhash';

const cache = new Map<string, string>();
const LIMITE = 4000;

export function apercuFlou(thumbhash: string | null): string | null {
  if (!thumbhash) return null;
  const connu = cache.get(thumbhash);
  if (connu) return connu;
  try {
    const octets = Uint8Array.from(atob(thumbhash), (c) => c.charCodeAt(0));
    const url = thumbHashToDataURL(octets);
    if (cache.size >= LIMITE) cache.delete(cache.keys().next().value!);
    cache.set(thumbhash, url);
    return url;
  } catch {
    return null;
  }
}

/** Assets whose real thumbnail was already displayed once in this session. */
export const dejaAffichees = new Set<string>();
