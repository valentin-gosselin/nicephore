<script lang="ts">
  // "Envoi" page: folders of this computer sent to Immich, engine progress
  // and the log of what was done.
  import { Button, Heading, IconButton, ProgressBar, Switch, Text } from '@immich/ui';
  import { mdiDeleteOutline, mdiFolderArrowUpOutline, mdiFolderDownloadOutline, mdiFolderPlusOutline, mdiPause, mdiPlay, mdiRefresh } from '@mdi/js';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import {
    activerRegleEnvoi,
    ajouterReception,
    ajouterRegleEnvoi,
    recevoirDeNouveau,
    echecsEnvoi,
    etatEnvoi,
    journalEnvoi,
    lancerEnvoi,
    pauseEnvoi,
    reessayerEnvoi,
    reglesEnvoi,
    supprimerRegleEnvoi,
    type Cible,
    type Selection,
    type EntreeJournal,
    type EtatEnvoi,
    type FichierEnEchec,
    type RegleEnvoi,
  } from '../../lib/api';
  import Dialogue from '../../lib/Dialogue.svelte';
  import DialogueRegle, { type Demande } from '../../lib/DialogueRegle.svelte';
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

  function libelleSelection(s: Selection | null) {
    if (!s) return '';
    switch (s.genre) {
      case 'album':
        return `l’album « ${s.nom} »`;
      case 'personne':
        return `les photos de ${s.nom}`;
      case 'favoris':
        return 'les favoris';
      case 'periode':
        return s.du && s.au ? `du ${dateCourte(s.du)} au ${dateCourte(s.au)}` : s.du ? `depuis le ${dateCourte(s.du)}` : `jusqu’au ${dateCourte(s.au ?? '')}`;
      default:
        return 'toute la photothèque';
    }
  }
  const dateCourte = (iso: string) => new Date(`${iso}T12:00:00`).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short', year: 'numeric' });

  async function recevoirEncore(regle: RegleEnvoi) {
    const n = await recevoirDeNouveau(regle.id).catch(() => 0);
    notifier(`${pluriel(n, 'fichier')} à recevoir de nouveau.`, 'succes');
    await rafraichir();
  }

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
    recu: 'Reçu',
    retire_ici: 'Effacé ici, pas retéléchargé',
    retire_du_serveur: 'Retiré sur le serveur, gardé ici',
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
    if (etat.attente) return `En attente : ${etat.attente}. L’envoi reprendra tout seul.`;
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
      case 'reception':
        return `Réception${sur}, ${mo(etat.octetsFaits)} sur ${mo(etat.octetsTotal)}`;
      default:
        return etat.dernierPassage ? `À jour, dernière vérification à ${heure(etat.dernierPassage)}.` : 'En attente.';
    }
  });
  const progression = $derived(
    !etat || etat.phase === 'inactif'
      ? null
      : (etat.phase === 'envoi' || etat.phase === 'reception') && etat.octetsTotal
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

  async function ajouter(demande: Demande) {
    try {
      if (demande.sens === 'montante') await ajouterRegleEnvoi(demande.dossier, demande.cible);
      else await ajouterReception(demande.dossier, demande.selection, demande.rangement);
      notifier(`« ${nomDossier(demande.dossier)} » ajouté : la synchro commence.`, 'succes');
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
      notifier(`« ${nomDossier(regle.dossier)} » n’est plus synchronisé.`, 'succes');
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
      // Counters and log follow the phases and the end of every pass (a
      // quick pass may only show as idle to idle), not every byte.
      const cle = `${payload.phase}/${payload.regle}/${payload.phase === 'envoi' ? payload.faits : ''}/${payload.dernierPassage}`;
      if (cle !== precedente) {
        precedente = cle;
        void rafraichir();
      }
    });
    return () => void arret.then((f) => f());
  });
</script>

<svelte:head><title>Synchro · Nicéphore</title></svelte:head>

<div class="h-full overflow-y-auto">
  <div class="mx-auto max-w-3xl space-y-8 p-8">
    <div class="flex items-start justify-between gap-4">
      <div class="space-y-1">
        <Heading size="large">Synchronisation</Heading>
        <Text color="muted">
          Des dossiers de cet ordinateur envoyés vers Immich, et des photos d’Immich reçues dans des dossiers, au fur et à
          mesure. Rien n’est jamais supprimé, ni ici ni sur le serveur.
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
            <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" class="shrink-0 opacity-70">
              <path fill="currentColor" d={regle.sens === 'descendante' ? mdiFolderDownloadOutline : mdiFolderArrowUpOutline} />
            </svg>
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
              aria-label={`${regle.sens === 'descendante' ? 'Ne plus recevoir dans' : 'Ne plus envoyer'} ${nomDossier(regle.dossier)}`}
              variant="ghost"
              color="secondary"
              shape="round"
              onclick={() => (aRetirer = regle)}
            />
          </div>
          {#if regle.sens === 'descendante'}
            <div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
              <span class="opacity-70">
                Reçoit {libelleSelection(regle.selection)}{regle.rangement === 'anneeMois' ? ', par année et mois' : ''}
              </span>
              <span class="compteurs">
                {pluriel(regle.bilan.recus, 'reçu')}{#if regle.bilan.aRecevoir} · {nombre(regle.bilan.aRecevoir)} à recevoir{/if}{#if regle.bilan.retiresServeur}
                  · {nombre(regle.bilan.retiresServeur)} retiré{regle.bilan.retiresServeur > 1 ? 's' : ''} sur le serveur, gardé{regle.bilan.retiresServeur > 1 ? 's' : ''} ici{/if}
              </span>
              {#if regle.bilan.retiresIci + regle.bilan.echecsReception}
                <button class="lien" onclick={() => recevoirEncore(regle)}>
                  {pluriel(regle.bilan.retiresIci + regle.bilan.echecsReception, 'fichier')} effacé{regle.bilan.retiresIci + regle.bilan.echecsReception > 1 ? 's' : ''} ici
                  ou en échec : recevoir de nouveau
                </button>
              {/if}
            </div>
          {:else}
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
          {/if}
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
  <Dialogue titre={regle.sens === 'descendante' ? 'Ne plus recevoir dans ce dossier ?' : 'Ne plus envoyer ce dossier ?'} onfermer={() => (aRetirer = null)}>
    <div class="space-y-4">
      <Text>
        Nicéphore oubliera « {nomDossier(regle.dossier)} ». Les fichiers restent sur cet ordinateur et les photos déjà
        envoyées restent sur le serveur.
      </Text>
      <div class="flex justify-end gap-2">
        <Button variant="ghost" color="secondary" onclick={() => (aRetirer = null)}>Annuler</Button>
        <Button color="danger" onclick={() => retirer(regle)}>{regle.sens === 'descendante' ? 'Ne plus recevoir' : 'Ne plus envoyer'}</Button>
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
