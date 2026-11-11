// Capture dates and their time zones. Immich keeps the date with an offset
// plus an IANA zone ("Europe/Paris") or a fixed offset ("UTC+1").
import { locale } from './i18n.svelte';

const decalageFixe = /^UTC([+-])(\d{1,2})(?::?(\d{2}))?$/;

/** Offset of `fuseau` at that local date, as "+02:00". */
export function decalage(fuseau: string, local: string): string {
  const fixe = fuseau.match(decalageFixe);
  if (fixe) return `${fixe[1]}${fixe[2].padStart(2, '0')}:${fixe[3] ?? '00'}`;
  try {
    const instant = new Date(`${local}:00Z`);
    const nom = new Intl.DateTimeFormat('en-US', { timeZone: fuseau, timeZoneName: 'longOffset' })
      .formatToParts(instant)
      .find((p) => p.type === 'timeZoneName')?.value;
    const m = nom?.match(/GMT([+-]\d{2}:\d{2})/);
    return m ? m[1] : '+00:00';
  } catch {
    return '+00:00';
  }
}

/** "YYYY-MM-DDTHH:mm" in that zone → ISO with offset for Immich. */
export function versIso(local: string, fuseau: string): string {
  return `${local}:00.000${decalage(fuseau, local)}`;
}

/** An instant shown in a zone, as "YYYY-MM-DDTHH:mm" for a datetime field. */
export function versLocal(iso: string, fuseau: string | null | undefined): string {
  const instant = new Date(iso);
  const fixe = fuseau?.match(decalageFixe);
  if (fixe) {
    const minutes = (Number(fixe[2]) * 60 + Number(fixe[3] ?? 0)) * (fixe[1] === '-' ? -1 : 1);
    return new Date(instant.getTime() + minutes * 60_000).toISOString().slice(0, 16);
  }
  try {
    const p = Object.fromEntries(
      new Intl.DateTimeFormat('en-CA', {
        timeZone: fuseau ?? undefined,
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hourCycle: 'h23',
      })
        .formatToParts(instant)
        .map((x) => [x.type, x.value]),
    );
    return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`;
  } catch {
    return instant.toISOString().slice(0, 16);
  }
}

/** Time zone of this computer. */
export const fuseauLocal = () => Intl.DateTimeFormat().resolvedOptions().timeZone;

/** Capture date for display, in its own zone. */
export function formaterDate(iso: string, fuseau: string | null | undefined): string {
  const options: Intl.DateTimeFormatOptions = { dateStyle: 'full', timeStyle: 'short' };
  const fixe = fuseau?.match(decalageFixe);
  if (fixe) {
    const minutes = (Number(fixe[2]) * 60 + Number(fixe[3] ?? 0)) * (fixe[1] === '-' ? -1 : 1);
    return new Date(new Date(iso).getTime() + minutes * 60_000).toLocaleString(locale(), { ...options, timeZone: 'UTC' });
  }
  try {
    return new Date(iso).toLocaleString(locale(), { ...options, timeZone: fuseau ?? undefined });
  } catch {
    return new Date(iso).toLocaleString(locale(), options);
  }
}
