<script lang="ts">
  /**
   * Text field with a filtered list of choices, like the Combobox of the
   * Immich web search filters (country, camera make...). Typing filters the
   * list; picking a value or clearing the field calls `onchoix`. As in
   * Immich, the list is `fixed` and placed from the field, so the scrolling
   * panel around it cannot crop it; it opens upwards when there is no room
   * below, and closes when the field scrolls out of view.
   */
  import { Icon } from '@immich/ui';
  import { mdiChevronDown, mdiClose } from '@mdi/js';
  import { fly } from 'svelte/transition';
  import { t } from './i18n.svelte';

  let {
    libelle,
    valeur,
    choix,
    indication,
    onchoix,
  }: {
    libelle: string;
    valeur: string | undefined;
    choix: string[];
    indication: string;
    onchoix: (valeur: string | undefined) => void;
  } = $props();

  const id = $props.id();
  let texte = $state('');
  let ouvert = $state(false);
  let surligne = $state(0);
  let champ = $state<HTMLInputElement>();

  // Position of the list (same rules and margins as Immich's Combobox).
  const marge = 15;
  const seuilBas = 225;
  let cadre = $state<DOMRect | null>(null);
  const versLeHaut = $derived(!!cadre && window.innerHeight - cadre.bottom <= seuilBas && cadre.top > window.innerHeight - cadre.bottom);
  const position = $derived.by(() => {
    if (!cadre) return null;
    const hauteur = (n: number) => `min(${Math.max(n, 0)}px,18rem)`;
    return versLeHaut
      ? { top: undefined, bottom: `${window.innerHeight - cadre.top}px`, hauteur: hauteur(cadre.top - marge) }
      : { top: `${cadre.bottom}px`, bottom: undefined, hauteur: hauteur(window.innerHeight - cadre.bottom - marge) };
  });
  function placer() {
    if (!ouvert || !champ) return;
    const r = champ.getBoundingClientRect();
    // Closed once the field is no longer fully visible in its panel.
    const conteneur = champ.closest('#panneau-recherche')?.getBoundingClientRect();
    if (conteneur && (r.top < conteneur.top || r.bottom > conteneur.bottom)) {
      ouvert = false;
      return;
    }
    cadre = r;
  }
  $effect(() => {
    if (!ouvert) return;
    placer();
    const suivre = () => placer();
    window.addEventListener('scroll', suivre, true);
    window.addEventListener('resize', suivre);
    return () => {
      window.removeEventListener('scroll', suivre, true);
      window.removeEventListener('resize', suivre);
    };
  });

  // The field shows the chosen value until the user types.
  $effect(() => {
    texte = valeur ?? '';
  });

  const filtres = $derived.by(() => {
    const q = texte.trim().toLowerCase();
    return !q || q === (valeur ?? '').toLowerCase() ? choix : choix.filter((c) => c.toLowerCase().includes(q));
  });

  function choisir(c: string | undefined) {
    ouvert = false;
    texte = c ?? '';
    if (c !== valeur) onchoix(c);
  }

  function surTouche(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      ouvert = true;
      const n = filtres.length;
      if (n) surligne = (surligne + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
    } else if (e.key === 'Enter' && ouvert) {
      e.preventDefault();
      e.stopPropagation();
      if (filtres[surligne]) choisir(filtres[surligne]);
    } else if (e.key === 'Escape' && ouvert) {
      e.stopPropagation();
      ouvert = false;
      texte = valeur ?? '';
    }
  }
</script>

<div class="champ-choix relative w-full" data-libelle={libelle}>
  <label for={id} class="mb-1 block text-xs font-light text-neutral-500">{libelle}</label>
  <div class="relative">
    <input
      {id}
      bind:this={champ}
      bind:value={texte}
      class="w-full rounded-lg bg-gray-100 px-4 py-2.5 pe-16 text-sm outline-none ring-1 ring-gray-200 transition-all focus:ring-primary dark:bg-gray-800 dark:ring-neutral-900"
      class:rounded-b-none={ouvert && !versLeHaut}
      class:rounded-t-none={ouvert && versLeHaut}
      placeholder={indication}
      autocomplete="off"
      role="combobox"
      aria-expanded={ouvert}
      aria-controls={`${id}-liste`}
      onfocus={() => {
        ouvert = true;
        surligne = 0;
      }}
      oninput={() => {
        ouvert = true;
        surligne = 0;
      }}
      onblur={() => {
        // Let a click on an option land first.
        setTimeout(() => {
          ouvert = false;
          const saisie = texte.trim();
          if (!saisie && valeur) choisir(undefined);
          else if (saisie !== (valeur ?? '')) texte = valeur ?? '';
        }, 150);
      }}
      onkeydown={surTouche}
    />
    <span class="absolute inset-y-0 end-2 flex items-center gap-1">
      {#if valeur}
        <button type="button" class="bouton rounded-full p-1" aria-label={t('Effacer {libelle}', { libelle })} onclick={() => choisir(undefined)}>
          <Icon icon={mdiClose} size="16" />
        </button>
      {/if}
      <button type="button" class="bouton rounded-full p-1" tabindex="-1" aria-label={t('Ouvrir {libelle}', { libelle })} onclick={() => champ?.focus()}>
        <Icon icon={mdiChevronDown} size="18" />
      </button>
    </span>
  </div>
  {#if ouvert}
    <ul
      id={`${id}-liste`}
      role="listbox"
      in:fly={{ duration: 250 }}
      class="fixed z-[70] overflow-y-auto border border-gray-300 bg-white text-start text-sm dark:border-gray-900 dark:bg-gray-800"
      class:rounded-b-xl={!versLeHaut}
      class:rounded-t-xl={versLeHaut}
      class:shadow-sm={!versLeHaut}
      style:top={position?.top}
      style:bottom={position?.bottom}
      style:left={cadre ? `${cadre.left}px` : undefined}
      style:width={cadre ? `${cadre.width}px` : undefined}
      style:max-height={position?.hauteur}
    >
      {#each filtres as c, i (c)}
        <li
          role="option"
          aria-selected={c === valeur}
          class="cursor-pointer px-4 py-2 transition-all hover:bg-gray-200 aria-selected:bg-gray-200 dark:hover:bg-gray-700 aria-selected:dark:bg-gray-700"
          class:surligne={i === surligne}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => choisir(c)}
          onkeydown={() => {}}
          onmouseenter={() => (surligne = i)}
        >
          {c}
        </li>
      {:else}
        <li class="px-4 py-2 opacity-60">{t('Aucun résultat')}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .bouton:hover {
    background: color-mix(in oklab, currentColor 12%, transparent);
  }
  .surligne {
    background: color-mix(in oklab, currentColor 12%, transparent);
  }
</style>
