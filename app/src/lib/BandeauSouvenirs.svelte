<script lang="ts">
  // "On this day" strip above the timeline, like the Immich apps.
  import { onMount } from 'svelte';
  import { souvenirs as chargerSouvenirs, urlImage, type Souvenir } from './api';
  import { apercuFlou } from './miniatures';
  import { aujourdhui, titreSouvenir } from './souvenirs';

  let liste = $state<Souvenir[]>([]);

  onMount(() => {
    chargerSouvenirs(aujourdhui())
      .then((s) => (liste = s.filter((x) => x.assets.length > 0)))
      .catch(() => (liste = []));
  });
</script>

{#if liste.length}
  <nav class="bandeau flex shrink-0 gap-3 overflow-x-auto px-5 py-3" aria-label="Souvenirs du jour">
    {#each liste as s (s.id)}
      {@const couverture = s.assets[0]}
      <a
        href={`/souvenir?id=${s.id}`}
        class="souvenir"
        style:background-image={apercuFlou(couverture.thumbhash) ? `url(${apercuFlou(couverture.thumbhash)})` : undefined}
      >
        <img src={urlImage('apercu', couverture.id, couverture.thumbhash)} alt="" loading="lazy" />
        <span class="legende">
          <span class="font-semibold">{titreSouvenir(s)}</span>
          <span class="text-xs opacity-80">{s.assets.length} élément{s.assets.length > 1 ? 's' : ''}</span>
        </span>
      </a>
    {/each}
  </nav>
{/if}

<style>
  .souvenir {
    position: relative;
    flex: 0 0 auto;
    width: 168px;
    height: 112px;
    overflow: hidden;
    border-radius: 12px;
    background-size: cover;
  }
  .souvenir img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 160ms;
  }
  .souvenir:hover img {
    transform: scale(1.05);
  }
  .legende {
    position: absolute;
    inset: auto 0 0 0;
    display: flex;
    flex-direction: column;
    padding: 18px 10px 7px;
    font-size: 13px;
    color: white;
    background: linear-gradient(to top, rgb(0 0 0 / 0.7), transparent);
  }
</style>
