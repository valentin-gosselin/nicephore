<script lang="ts">
  // "Envoi" page: folders of this computer sent to Immich, engine progress
  // and the log of what was done.
  import { Button, Heading, IconButton, ProgressBar, Switch, Text } from '@immich/ui';
  import { mdiDeleteOutline, mdiFolderOutline, mdiFolderPlusOutline, mdiPause, mdiPlay, mdiRefresh } from '@mdi/js';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import {
    activerRegleEnvoi,
    ajouterRegleEnvoi,
    echecsEnvoi,
    etatEnvoi,
    journalEnvoi,
    lancerEnvoi,
    pauseEnvoi,
    reessayerEnvoi,
    reglesEnvoi,
    supprimerRegleEnvoi,
    type Cible,
    type EntreeJournal,
    type EtatEnvoi,
    type FichierEnEchec,
    type RegleEnvoi,
  } from '../../lib/api';
  import Dialogue from '../../lib/Dialogue.svelte';
  import DialogueRegle from '../../lib/DialogueRegle.svelte';
  import { notifier } from '../../lib/notifications.svelte';

  let regles = $state<RegleEnvoi[] | null>(null);
  let erreurBase = $state<string | null>(null);
  let etat = $state<EtatEnvoi | null>(null);
  let journal = $state<EntreeJournal[]>([]);
  let ajout = $state(false);
  let aRetirer = $state<RegleEnvoi | null>(null);
  let echecs = $state<{ regle: number; fichiers: FichierEnEchec[] } | null>(null);

  const nombre = (n: number) => n.toLocaleString('fr-FR');
  const mo = (octets: number) =>
    octets >= 1_073_741_824
      ? `${(octets / 1_073_741_824).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`
      : `${(octets / 1_048_576).toLocaleString('fr-FR', { maximumFractionDigits: 0 })} Mo`;
  const nomDossier = (d: string) => d.split(/[\\/]/).filter(Boolean).pop() ?? d;
  const pluriel = (n: number, mot: string) => `${nombre(n)} ${mot}${n > 1 ? 's' : ''}`;

  function libelleCible(c: Cible) {
    if (c.genre === 'album') return `Album « ${c.nom} »`;
    if (c.genre === 'albumParDossier') return 'Un album par dossier';
    return 'Photothèque, sans album';
  }

  const libellesJournal: Record<string, string> = {
    envoye: 'Envoyé',
    album_cree: 'Album créé',
    album_disparu: 'Album supprimé sur le serveur, pas recréé',
    dans_la_corbeille: 'Déjà sur le serveur, dans sa corbeille',
    ignore: 'Ignoré',
    abandon: 'Abandonné après plusieurs essais',
    regle_ajoutee: 'Dossier ajouté',
    regle_supprimee: 'Dossier retiré',
  };

  function detailJournal(e: EntreeJournal) {
    switch (e.action) {
      case 'album_cree':
        return `« ${e.detail ?? ''} »`;
      case 'regle_ajoutee':
      case 'regle_supprimee':
        return e.detail ?? '';
      default:
        return [e.chemin, e.detail && `(${e.detail})`].filter(Boolean).join(' ');
    }
  }

  const heure = (ms: number) =>
    new Date(ms).toLocaleString('fr-FR', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });

  const description = $derived.by(() => {
    if (!etat) return '';
    if (!etat.connecte) return 'Non connecté au serveur.';
    if (etat.enPause) return 'En pause.';
    const sur = etat.total ? ` (${nombre(etat.faits)} sur ${nombre(etat.total)})` : '';
    switch (etat.phase) {
      case 'analyse':
        return 'Lecture des dossiers…';
      case 'empreintes':
        return `Calcul des empreintes des nouveaux fichiers${sur}`;
      case 'verification':
        return `Comparaison avec le serveur${sur}`;
      case 'envoi':
        return `Envoi${sur}, ${mo(etat.octetsFaits)} sur ${mo(etat.octetsTotal)}`;
      case 'albums':
        return 'Rangement dans les albums…';
      default:
        return etat.dernierPassage ? `À jour, dernière vérification à ${heure(etat.dernierPassage)}.` : 'En attente.';
    }
  });
  const progression = $derived(
    !etat || etat.phase === 'inactif'
      ? null
      : etat.phase === 'envoi' && etat.octetsTotal
        ? etat.octetsFaits / etat.octetsTotal
        : etat.total
          ? etat.faits / etat.total
          : null,
  );

  async function rafraichir() {
    try {
      regles = await reglesEnvoi();
      erreurBase = null;
    } catch (e) {
      erreurBase = String(e);
      regles = [];
    }
    journal = await journalEnvoi(60).catch(() => []);
  }

  async function ajouter(dossier: string, cible: Cible) {
    try {
      await ajouterRegleEnvoi(dossier, cible);
      notifier(`« ${nomDossier(dossier)} » ajouté : l’analyse commence.`, 'succes');
      await rafraichir();
      return true;
    } catch (e) {
      notifier(String(e), 'erreur');
      return false;
    }
  }

  async function basculer(regle: RegleEnvoi, actif: boolean) {
    try {
      await activerRegleEnvoi(regle.id, actif);
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function retirer(regle: RegleEnvoi) {
    aRetirer = null;
    try {
      await supprimerRegleEnvoi(regle.id);
      notifier(`« ${nomDossier(regle.dossier)} » n’est plus envoyé.`, 'succes');
      await rafraichir();
    } catch (e) {
      notifier(String(e), 'erreur');
    }
  }

  async function voirEchecs(regle: RegleEnvoi) {
    echecs = { regle: regle.id, fichiers: await echecsEnvoi(regle.id).catch(() => []) };
  }

  async function reessayer(regle: number) {
    const n = await reessayerEnvoi(regle).catch(() => 0);
    echecs = null;
    notifier(`${pluriel(n, 'fichier')} remis dans la file.`, 'succes');
    await rafraichir();
  }

  onMount(() => {
    void rafraichir();
    etatEnvoi().then((e) => (etat = e));
    let precedente = '';
    const arret = listen<EtatEnvoi>('envoi', ({ payload }) => {
      etat = payload;
      // Counters and log follow the phases, not every byte.
      const cle = `${payload.phase}/${payload.regle}/${payload.phase === 'envoi' ? payload.faits : ''}`;
      if (cle !== precedente) {
        precedente = cle;
        void rafraichir();
      }
    });
    return () => void arret.then((f) => f());
  });
</script>

<svelte:head><title>Envoi · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-3xl space-y-8 p-8">
    <div class="flex items-start justify-between gap-4">
      <div class="space-y-1">
        <Heading size="large">Envoi vers Immich</Heading>
        <Text color="muted">
          Les photos et vidéos des dossiers choisis partent sur le serveur, au fur et à mesure qu’elles arrivent. Rien
          n’est jamais supprimé, ni ici ni sur le serveur.
        </Text>
      </div>
      <Button class="shrink-0 whitespace-nowrap" leadingIcon={mdiFolderPlusOutline} onclick={() => (ajout = true)} disabled={!!erreurBase}>
        Ajouter un dossier
      </Button>
    </div>

    {#if erreurBase}
      <p class="rounded-xl bg-red-500/10 p-4 text-sm text-red-500">{erreurBase}</p>
    {/if}

    {#if etat && regles?.length}
      <section class="etat space-y-3 rounded-2xl p-4" aria-label="État de l’envoi">
        <div class="flex items-center gap-3">
          <Text class="min-w-0 flex-1">{description}</Text>
          <IconButton
            icon={mdiRefresh}
            aria-label="Vérifier maintenant"
            variant="ghost"
            color="secondary"
            shape="round"
            disabled={etat.enPause || etat.phase !== 'inactif'}
            onclick={() => lancerEnvoi()}
          />
          <IconButton
            icon={etat.enPause ? mdiPlay : mdiPause}
            aria-label={etat.enPause ? 'Reprendre l’envoi' : 'Mettre l’envoi en pause'}
            variant="ghost"
            color="secondary"
            shape="round"
            onclick={() => pauseEnvoi(!etat?.enPause)}
          />
        </div>
        {#if progression != null}<ProgressBar progress={Math.min(1, progression)} />{/if}
        {#if etat.fichier && etat.phase !== 'inactif'}
          <Text size="small" color="muted" class="truncate">{etat.fichier}</Text>
        {/if}
        {#if etat.erreur}<p class="text-sm text-red-500">{etat.erreur}</p>{/if}
      </section>
    {/if}

    {#if regles && !regles.length && !erreurBase}
      <div class="vide space-y-2 rounded-2xl p-8 text-center">
        <Text>Aucun dossier n’est envoyé pour l’instant.</Text>
        <Text size="small" color="muted">
          Choisissez par exemple le dossier où arrivent les photos du téléphone ou de l’appareil photo.
        </Text>
      </div>
    {/if}

    <ul class="space-y-3">
      {#each regles ?? [] as regle (regle.id)}
        {@const erreur = regle.dossierAbsent ? 'Dossier introuvable (disque débranché ?) : rien n’est fait en attendant.' : etat?.erreursRegles[regle.id]}
        <li class="regle space-y-2 rounded-2xl p-4" class:inactive={!regle.actif}>
          <div class="flex items-center gap-3">
            <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" class="shrink-0 opacity-70"><path fill="currentColor" d={mdiFolderOutline} /></svg>
            <div class="min-w-0 flex-1">
              <div class="truncate font-semibold">{nomDossier(regle.dossier)}</div>
              <div class="truncate text-xs opacity-60" title={regle.dossier}>{regle.dossier}</div>
            </div>
            <Switch
              checked={regle.actif}
              aria-label={regle.actif ? 'Suspendre ce dossier' : 'Reprendre ce dossier'}
              onCheckedChange={(v: boolean) => basculer(regle, v)}
            />
            <IconButton
              icon={mdiDeleteOutline}
              aria-label={`Ne plus envoyer ${nomDossier(regle.dossier)}`}
              variant="ghost"
              color="secondary"
              shape="round"
              onclick={() => (aRetirer = regle)}
            />
          </div>
          <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
            <span class="opacity-70">{libelleCible(regle.cible)}</span>
            <span class="compteurs">
              {pluriel(regle.bilan.envoyes, 'envoyé')} · {nombre(regle.bilan.presents)} déjà sur le serveur{#if regle.bilan.enAttente}
                · {nombre(regle.bilan.enAttente)} en attente{/if}
            </span>
            {#if regle.bilan.echecs + regle.bilan.ignores}
              <button class="lien text-red-500" onclick={() => voirEchecs(regle)}>
                {pluriel(regle.bilan.echecs + regle.bilan.ignores, 'fichier')} non envoyé{regle.bilan.echecs + regle.bilan.ignores > 1 ? 's' : ''}
              </button>
            {/if}
          </div>
          {#if erreur}<p class="text-sm text-red-500">{erreur}</p>{/if}
        </li>
      {/each}
    </ul>

    {#if journal.length}
      <section class="space-y-2">
        <Heading size="small">Dernières actions</Heading>
        <ol class="journal text-sm">
          {#each journal as entree (entree.id)}
            <li class="flex gap-3 py-1">
              <span class="w-28 shrink-0 opacity-60">{heure(entree.horodatage)}</span>
              <span class="shrink-0">{libellesJournal[entree.action] ?? entree.action}</span>
              <span class="min-w-0 truncate opacity-70" title={detailJournal(entree)}>{detailJournal(entree)}</span>
            </li>
          {/each}
        </ol>
      </section>
    {/if}
  </div>
</div>

{#if ajout}
  <DialogueRegle onvalider={ajouter} onfermer={() => (ajout = false)} />
{/if}

{#if aRetirer}
  {@const regle = aRetirer}
  <Dialogue titre="Ne plus envoyer ce dossier ?" onfermer={() => (aRetirer = null)}>
    <div class="space-y-4">
      <Text>
        Nicéphore oubliera « {nomDossier(regle.dossier)} ». Les fichiers restent sur cet ordinateur et les photos déjà
        envoyées restent sur le serveur.
      </Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (aRetirer = null)}>Annuler</Button>
        <Button color="danger" onclick={() => retirer(regle)}>Ne plus envoyer</Button>
      </div>
    </div>
  </Dialogue>
{/if}

{#if echecs}
  {@const liste = echecs}
  <Dialogue titre="Fichiers non envoyés" onfermer={() => (echecs = null)} largeur={620}>
    <div class="space-y-3">
      <ul class="space-y-2 text-sm">
        {#each liste.fichiers as f (f.chemin)}
          <li>
            <div class="break-all font-medium">{f.chemin}</div>
            <div class="opacity-70">{f.erreur ?? 'raison inconnue'}{#if f.essais} · {pluriel(f.essais, 'essai')}{/if}</div>
          </li>
        {/each}
      </ul>
      <div class="flex justify-end">
        <Button onclick={() => reessayer(liste.regle)}>Réessayer</Button>
      </div>
    </div>
  </Dialogue>
{/if}

<style>
  .etat,
  .regle,
  .vide {
    background: color-mix(in oklab, currentColor 5%, transparent);
  }
  .regle.inactive {
    opacity: 0.6;
  }
  .lien {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .journal li + li {
    border-top: 1px solid color-mix(in oklab, currentColor 8%, transparent);
  }
</style>
