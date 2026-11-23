<script lang="ts">
  // Every public link of the user, like « Liens partagés » in Immich web:
  // what each one shares, its address and options; copy, change, delete.
  import { Heading, LoadingSpinner, Text } from '@immich/ui';
  import { onMount } from 'svelte';
  import { liensPartages, supprimerLien, type LienPartage } from '../../lib/api';
  import Dialogue from '../../lib/Dialogue.svelte';
  import FormulaireLien from '../../lib/FormulaireLien.svelte';
  import { t } from '../../lib/i18n.svelte';
  import LigneLien from '../../lib/LigneLien.svelte';
  import { notifier } from '../../lib/notifications.svelte';

  let liens = $state<LienPartage[] | null>(null);
  let erreur = $state<string | null>(null);
  let edite = $state<LienPartage | null>(null);

  async function charger() {
    try {
      liens = await liensPartages();
    } catch (e) {
      erreur = String(e);
    }
  }

  onMount(() => void charger());

  async function supprimer(lien: LienPartage) {
    try {
      await supprimerLien(lien.id);
      liens = (liens ?? []).filter((l) => l.id !== lien.id);
      notifier(t('Lien supprimé'), 'succes');
    } catch (e) {
      notifier(t('Suppression impossible : {raison}', { raison: String(e) }), 'erreur');
    }
  }
</script>

<svelte:head><title>{t('Liens partagés')} · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-4xl space-y-6 p-6">
    <div>
      <Heading size="small">{t('Liens partagés')}</Heading>
      <Text size="small" color="muted">{t('Partager les photos et vidéos via un lien')}</Text>
    </div>
    {#if erreur}
      <Text color="danger">{erreur}</Text>
    {:else if !liens}
      <LoadingSpinner />
    {:else if !liens.length}
      <Text color="muted">{t('Vous n’avez aucun lien partagé. Le bouton « Partager » d’un album ou d’une sélection en crée un.')}</Text>
    {:else}
      <div class="liens space-y-1">
        {#each liens as l (l.id)}
          <LigneLien lien={l} onmodifier={() => (edite = l)} onsupprimer={() => void supprimer(l)} />
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if edite}
  <Dialogue titre={t('Modifier le lien')} onfermer={() => (edite = null)} largeur={540}>
    <FormulaireLien
      lien={edite}
      onfini={async () => {
        edite = null;
        await charger();
        notifier(t('Lien enregistré'), 'succes');
      }}
      onannuler={() => (edite = null)}
    />
  </Dialogue>
{/if}
