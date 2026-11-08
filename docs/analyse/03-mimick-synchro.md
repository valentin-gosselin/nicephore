# Analyse du moteur de synchro de Mimick (v9.13.0, GPL-3.0-or-later)

Source : /home/goss/.cache/immich-ref/mimick (lecture seule, rien construit ni exécuté). Dernier commit lu : 1d10c25. Périmètre : tout sauf src/library/** (j'ai tout de même lu src/library/album_sync.rs, album_sync/diff.rs et local_source.rs, car c'est là que vit réellement la synchro bidirectionnelle et la mirror des suppressions ; ils sont dans l'arborescence library mais ce sont du moteur pur).

Tous les chemins sont relatifs à la racine du dépôt.

---

## 0. Vue d'ensemble

Mimick est un démon mono-processus : GTK4/libadwaita + runtime Tokio (`#[tokio::main]`, src/main.rs:96). Le "moteur" est un ensemble de modules plats dans src/, sans crate séparée :

- `monitor.rs` (watcher notify + stabilisation + hash SHA-1)
- `startup_scan.rs` + `startup_scan/{enumeration,reconcile}.rs` (rattrapage au démarrage, rayon)
- `queue_manager.rs` + `queue_manager/{progress,upload}.rs` (file d'envoi, workers, retries)
- `sync_index.rs` (index persistant des fichiers synchronisés)
- `remote_sync.rs` (réconciliateur périodique 5 min + suppression locale vers distant)
- `library/album_sync.rs` + `album_sync/diff.rs` (diff album vs dossier, téléchargements, suppressions)
- `api_client/*` (client Immich reqwest, bascule LAN/WAN)
- `config.rs`, `config/{schema,folders,keyring}.rs`, `profile.rs`, `state_manager.rs`, `diagnostics.rs`, `runtime_env.rs`, `app_context.rs`

Le moteur n'est PAS isolé : il est câblé à GTK/glib/gettext par endroits (voir section 3), mais le couplage est mince et localisé.

---

## 1. Architecture du moteur de synchro

### 1.1 Modèle de données de l'index de synchro (src/sync_index.rs)

- Stockage : UN SEUL fichier JSON `~/.local/share/mimick/synced_index.json` (dossier data, déplacé depuis le cache pour survivre aux "clear cache", migration rename puis copy : sync_index.rs:300-340). Format : `{"files": { "<chemin absolu string>": SyncedFileRecord }}` (sync_index.rs:48-52).
- Clé : chemin absolu local (String lossy). Pas d'identifiant d'asset serveur stocké.
- `SyncedFileRecord` (sync_index.rs:23-35) : `size: u64`, `modified_ms: u64`, `checksum: String` (SHA-1 hex 40 car.), `album_name: Option<String>`, `album_id: Option<String>`.
- Le "sharding" est uniquement en mémoire : 16 `RwLock<Shard>` indexés par hash du chemin (`DefaultHasher`, sync_index.rs:107-111), chaque shard a `entries` + un index inverse `checksum_to_path`. L'écriture sérialise la fusion de tous les shards dans un seul JSON (flush : sync_index.rs:255-270). Donc ce n'est pas une base ; c'est un HashMap sérialisé en entier à chaque flush (coût O(N) sur de grosses photothèques).
- Écriture : atomique (tmp + rename, src/util.rs:17-40), flag `dirty` par shard remis à false seulement après succès. Flush toutes les 10 s (main.rs:373-382, spawn_blocking) et quand la file devient idle (progress.rs:430-434) et à l'arrêt.
- Décision (`sync_decision`, sync_index.rs:127-150) : pas d'entrée ou (size, mtime_ms) différents => `NeedsUpload` ; entrée identique mais album_name/album_id différents => `NeedsReassociate` (pas de ré-envoi, juste ajout à l'album) ; sinon `UpToDate`. Le hash n'est donc recalculé que si la taille ou la mtime a changé (`fresh_checksum`, sync_index.rs:177-188).
- Index inverse par checksum : `local_path_for_checksum`, `paths_for_checksum` (balayage de tous les shards, O(N)).
- Autres états persistés :
  - `config.json` (~/.config/mimick), écrit atomiquement.
  - `status.json` (cache, snapshot AppState, écrit à l'arrêt propre seulement ; state_manager.rs:289-330).
  - `retries.json` (cache, écrit à l'arrêt seulement ; queue_manager.rs:397-407).
  - Clé API : trousseau (oo7) ou fichier fallback base64 0600.
  - Caches (miniatures, exif, raw_decode, video) avec plafond disque global, purge au démarrage (cache_manager.rs).

### 1.2 Détection de changements

- Temps réel : crate `notify` 8.2 (`recommended_watcher` = inotify sur Linux) en récursif sur chaque dossier surveillé (monitor.rs:352-388). Sous Flatpak : `PollWatcher` à 2 s (monitor.rs:391-405), car les dossiers portail ne donnent pas d'inotify fiable.
- Le watcher tourne dans un thread OS dédié avec boucle `recv_timeout(500ms)` et canal de commandes pour remplacer les dossiers à chaud (`MonitorHandle::replace_watch_paths`, monitor.rs:336-349 : unwatch tout puis rewatch).
- Filtres : extension média uniquement (media_kinds.rs), ignore `.tmp/.part/.crdownload/~` (monitor.rs:479-492), règles par dossier (`FolderRules::matches` : cachés, taille max, extensions autorisées, config/folders.rs:55-90), `DownloadOnly` ignore les événements d'upload.
- Debounce par chemin : 2 s (monitor.rs:312-320), table nettoyée au-delà de 1000 entrées. Un set `active_tasks` empêche les doublons en cours de hash.
- Stabilisation : `wait_for_file_completion` (monitor.rs:413-446) : taille inchangée 3 fois de suite (poll 1 s), time-out d'inactivité 300 s, retourne false si le fichier disparaît. Tâche Tokio async, pas de thread par fichier.
- Hash : pool de `num_cpus/2` (clamp 1..4) workers Tokio ; chaque hash en `spawn_blocking`, lecture par blocs de 64 Ko (monitor.rs:88, 205-227, 449-462).
- Rattrapage : au démarrage (si background_sync activé) et à "Sync Now", `queue_unsynced_files` (startup_scan.rs:49-114) :
  1. énumération parallèle rayon des dossiers (`par_iter` sur les entrées de config, parcours récursif manuel par pile, enumeration.rs:19-93) ;
  2. 3 modes de rattrapage par dossier : `Full`, `RecentOnly` (mtime < 7 jours), `NewFilesOnly` (created >= dernier sync réussi) (enumeration.rs:177-209) ;
  3. décision via l'index (UpToDate/Reassociate/Upload), hash seulement si nécessaire, 16 en parallèle (`for_each_concurrent(16)`, startup_scan.rs:137) ;
  4. pré-vol serveur groupé : `bulk-upload-check` par lots de 500 (asset_lookup.rs:14-60) ; ce qui existe déjà est réassocié à l'album sans envoi (enumeration.rs:328-393) ;
  5. mirror des suppressions + réconciliation album vers dossier + `prune_missing` de l'index.
- Distant vers local : `remote_sync::run_album_reconciler` (remote_sync.rs:16-35) toutes les 5 min (polling pur, pas d'événements serveur), saute si background désactivé, file en pause ou serveur injoignable. Par dossier : liste TOUS les assets de l'album (pages de 1000 via `/api/search/metadata` avec `albumIds`), énumère et hashe le dossier local, calcule le diff.
- Il n'y a PAS de scan périodique local (le watcher + le rattrapage au démarrage + "Sync Now" suffisent). Les événements manqués pendant l'exécution (overflow inotify) ne sont rattrapés qu'au prochain démarrage ou Sync Now.

### 1.3 Déduplication

- Hash local : SHA-1 (crate `sha1`) car c'est le checksum natif d'Immich. BLAKE3 est une dépendance mais n'est utilisée que pour des clés de cache (clé de cache raw_decode, test inactif "prepared for Phase 1.5" : monitor.rs:480-495). Le commentaire d'en-tête de monitor.rs:5-6 dit "BLAKE3" mais le code fait du SHA-1 : doc périmée.
- Côté serveur : `POST /api/assets/bulk-upload-check` avec `{assets:[{id:checksum, checksum}]}` (asset_lookup.rs:62-111). En upload unitaire, UN appel bulk-upload-check PAR fichier juste avant l'envoi (queue_manager/upload.rs:44), la version "batch de 500" n'est utilisée que dans le rattrapage de démarrage.
- Filet de sécurité : réponse 409 sur `POST /api/assets` => id extrait du JSON, sinon sentinelle `"DUPLICATE"` puis nouveau lookup (api_client/upload.rs:238-256, queue_manager/upload.rs:165-189).
- Depuis la v3 (CHANGELOG.md:265) l'envoi ne transmet plus `deviceAssetId`/`deviceId` ; le multipart ne contient que `assetData`, `fileCreatedAt`, `fileModifiedAt`, `isFavorite`, et `sidecarData` optionnel (upload.rs:94-100). Pas de header `x-immich-checksum`.
- Correctif timeline : après chaque envoi réussi, 3 PUT `/api/assets` avec `timeZone` à +2 s, +8 s, +20 s (upload.rs:183-210) pour contrer la réécriture des métadonnées par le job serveur. Astuce utile, mais fire-and-forget (non persistée, perdue si on quitte dans les 20 s).
- Dates : `fileCreatedAt` = min(birth time, mtime) (upload_helpers.rs:83-101), fuseau IANA lu dans $TZ, /etc/localtime ou /etc/timezone (Linux uniquement, upload_helpers.rs:112-136).

### 1.4 Envoi : file, workers, retries

- `QueueManager` (queue_manager.rs:104-126) : canal `tokio::mpsc` borné à 64, `pending_paths: HashSet<String>` pour refuser les doublons (queue_manager.rs:205-211), 10 workers Tokio lancés en permanence (`MAX_WORKERS=10`, progress.rs:112), un `AtomicUsize worker_limit` (config 1..10, défaut 3) : les workers d'indice >= limite attendent sur un `Notify` (progress.rs:195-208). Un seul `Receiver` partagé sous `tokio::sync::Mutex` (pattern work-stealing simple).
- Chaque tâche (`FileTask`, queue_manager.rs:77-102, sérialisable serde avec défauts) : path, watch_path, checksum SHA-1 précalculé, album_id/name, `reassociate_only`, `skip_album`, `sidecar_path`.
- Pause : `wait_until_allowed` (queue_manager.rs:468-511) sonde toutes les 3 s : pause manuelle, réseau mesuré (AtomicBool alimenté par gio::NetworkMonitor), batterie (/sys/class/power_supply), heures calmes (fenêtre locale avec wrap, `is_quiet_hour`, queue_manager.rs:454-465). Une tâche déjà dépilée attend ; pas de désabonnement du canal.
- Échec : la tâche va dans `retry_list` en mémoire (progress.rs:337-350). Elle est rejouée UNIQUEMENT au prochain upload réussi (`requeue_retries_after_success`, progress.rs:85-109) ou sur "Retry" manuel. Il n'y a ni backoff exponentiel (le roadmap le liste encore non fait, roadmap.md "Exponential backoff"), ni plafond de tentatives, ni délai. Le commentaire d'en-tête de queue_manager.rs:4 ("exponential back-off") est faux.
- Persistance des retries : écrite seulement à l'arrêt propre (flush_retries) ; chargée puis fichier vidé au démarrage, ré-injectée après 5 s (progress.rs:133-177). Un crash perd la liste de retries, mais l'index (seule source de vérité) rattrape tout au prochain scan : design auto-réparateur.
- Arrêt propre : `shutdown(deadline)` refuse les nouveaux tâches, attend les uploads actifs, puis annule via `CancellationToken` (queue_manager.rs:364-395). Les envois annulés tombent en échec donc en retry persisté.
- Compteurs/progression : `Arc<parking_lot::Mutex<AppState>>` unique mis à jour directement par les workers (verrou global, UI le lit via un timer de 500 ms).
- Notifications : 1 notif de lot à la fin de la file, 1 notif "connexion perdue" après 3 échecs consécutifs (progress.rs:385-403).

### 1.5 Concurrence

- Tokio multi-thread pour tout l'I/O, `spawn_blocking` pour les hash et les parcours disque, un thread OS pour le watcher notify, thread GTK pour l'UI. `rayon` uniquement dans l'énumération du rattrapage (par dossier, pas par fichier).
- Verrous : `parking_lot::Mutex/RwLock` partout (non réentrants : un commentaire de local_source.rs:158-164 documente un risque de deadlock évité), `tokio::sync::Mutex` pour les `Receiver` partagés et le cache d'albums.
- Anti-course : `ReconcileLocks::try_acquire(album_id)` (app_context.rs:48-82) saute une réconciliation si une autre tourne déjà sur le même album ; `album_create_locks` par nom d'album pour éviter la création en double (albums.rs:266-273) ; `connection_check_lock` pour coalescer les sondes LAN/WAN (api_client/mod.rs:215-222).
- Anti-boucle : `RecentSelfPaths` (TTL 60 s, "consume" à la première occurrence) marque les fichiers que Mimick vient d'écrire (téléchargements) ou de supprimer, pour que le watcher ne les remonte pas (app_context.rs:21-46 ; usage file_events.rs:36 et album_sync.rs:112).

### 1.6 Synchro bidirectionnelle (dossier <=> album)

Mode par dossier `FolderSyncMethod` : `UploadOnly` (défaut), `DownloadOnly`, `Full` (config/folders.rs:101-110). Un dossier est lié à UN album (par id ou nom, ou "nom du dossier") ; un dossier "bibliothèque seule" (`uploads_to_library` : album_id et album_name tous deux None) ne fait jamais de download ni de suppression.

Algorithme (`diff_album_vs_folder`, album_sync/diff.rs:32-103) :
1. Liste complète des assets distants de l'album (pages de 1000), ceux sans checksum sont comptés `remote_unhashed`.
2. Énumération locale + checksum (index si taille/mtime inchangées, sinon SHA-1 séquentiel `spawn_blocking`) (diff.rs:274-306).
3. Comparaison PAR CHECKSUM (ensemble), jamais par nom :
   - distant absent localement => `to_download` (sauf s'il correspond à un "orphelin" renommé, voir plus bas) ;
   - local absent du distant et jamais synchronisé vers cet album => `to_upload` ;
   - local absent du distant mais déjà synchronisé (l'index le dit) = "supprimé côté serveur" => `to_delete_local` si la règle est active (désactivée en dur, voir plus bas), sinon ignoré (ou RE-ENVOYÉ si l'utilisateur a lancé un Sync Now manuel : diff.rs:254-260, le serveur "perd" alors contre le local).
4. Détection de renommage local : un enregistrement d'index orphelin (fichier disparu) dont le checksum réapparaît sous un autre chemin => migration de l'enregistrement (`migrate_renamed_record`, diff.rs:235-244, 308-338), pas de re-téléchargement ni de suppression.
5. `UploadOnly` vide `to_download` ; `DownloadOnly` vide `to_upload` (diff.rs:90-94).

Conflits : il n'y a pas de notion de conflit de contenu. Identité = checksum ; un fichier modifié localement a un nouveau checksum donc il est vu comme "nouveau fichier" (upload d'une nouvelle version, l'ancienne reste sur le serveur). Les collisions de nom au téléchargement sont résolues par suffixe `name (1).ext` jusqu'à 999 (album_sync.rs:260-286) ; `safe_filename` coupe la traversée de répertoire (sanitize.rs:22-38).

Téléchargement (`download_album_asset`, album_sync.rs:101-130) : GET `/api/assets/{id}/original`, écriture directe au chemin final (stream_to_file, library.rs:25-44) : pas de fichier temporaire, pas de rename, pas de vérification de checksum ni de taille après coup. Séquentiel (une boucle for), sans parallélisme. Puis `record_synced` dans l'index.

### 1.7 Miroir des suppressions et garde-fous

Deux flags par dossier (config/folders.rs:22-27), tous deux `false` par défaut :
- `delete_folder_to_album` : local supprimé => asset distant mis à la corbeille Immich. (Attention : la doc du champ ligne 22 dit "Delete local file when remote asset is removed", l'inverse du comportement réel ; nommage et doc confus.)
- `delete_album_to_folder` : distant supprimé => fichier local à la corbeille. DÉSACTIVÉ EN DUR : `ALBUM_TO_FOLDER_TRASH_AVAILABLE = false` (album_sync.rs:23-25), `execute_local_deletions` ne fait que logguer (album_sync.rs:198-211) car le trash portail Flatpak échoue sur les chemins FUSE. Donc en pratique Mimick ne supprime jamais de fichier local. Le code d'origine est laissé en commentaire.

Chemins de suppression distante (3 déclencheurs distincts) :
1. Temps réel : événement `Remove` notify => `MonitorEvent::Deleted` => `handle_local_deletion` (file_events.rs:147-154) => `build_local_deletion_request` retrouve l'asset par checksum dans l'album (parcours paginé de l'album ENTIER pour CHAQUE suppression, remote_sync.rs:252-273) puis `trash_remote_after_local_delete`. Aucune garde de masse.
2. Au démarrage : `trash_remote_assets_for_missing_local_files` (startup_scan.rs:259-311) : tout enregistrement d'index sous le dossier dont le chemin n'est pas dans `seen_paths` => asset distant mis à la corbeille. Aucune garde de masse (voir "Avoid" #1).
3. Périodique/Sync Now : diff album/dossier => `to_delete_remote` via `compute_remote_deletions` (album_sync.rs:377-401) : enregistrement orphelin (fichier absent du disque ET de l'énumération) appartenant à cet album, avec asset distant de même checksum.

Garde-fous réellement présents :
- Corbeille Immich (`DELETE /api/assets` avec `force:false`, library.rs:17-22 et ~388-425), jamais de suppression définitive automatique.
- Si l'asset est membre de plus d'un album (`GET /api/albums?assetId=`) : simple retrait de cet album au lieu de corbeille (remote_sync.rs:184-196, album_sync.rs:166-179). Mais si le comptage échoue (None), le repli est la corbeille (album_sync.rs:176-178, remote_sync.rs:190-195) : repli agressif.
- Confirmation en deux passes pour les suppressions de masse : si plus de 5 éléments à supprimer dans un lot, chaque élément doit être observé manquant 2 fois (`PendingDeletions`, app_context.rs:84-109 ; album_sync.rs:415-439). Compteurs en mémoire seulement (perdus au redémarrage), clé = album + checksum ; les éléments qui "reviennent" sont effacés (`clear_pending_deletions`). Ne protège PAS les chemins 1 et 2 ci-dessus, ni les lots de 5 ou moins.
- Pas de suppression si des assets distants n'ont pas de checksum (`remote_unhashed > 0`, diff.rs:247-253).
- Pas de réconciliation si le dossier racine n'est pas un répertoire (reconcile.rs:35-37), ni si l'album n'existe pas/pas de réponse (diff échoue => rien). Une pagination qui échoue en cours de route renvoie Err => aucune suppression (fail-closed correct).
- Un dossier lié à "bibliothèque seule" force `restrict_to_library_uploads` (UploadOnly + aucun delete, config/folders.rs:39-43).
- Verrou par album contre deux réconciliations simultanées.
- Les enregistrements d'index sont conservés si l'appel serveur échoue (réessai au prochain tick).

### 1.8 Bascule LAN/WAN (src/api_client/mod.rs)

- Deux URL (interne, externe) avec interrupteur d'activation chacune. `check_connection` teste dans l'ordre interne puis externe par `GET /api/server/ping` (timeout 2 s) et exige `{"res":"pong"}` (mod.rs:305-350). L'URL active est mémorisée ; elle est invalidée (None) sur toute erreur réseau d'envoi/téléchargement et sur 502-504, ce qui force un nouveau test au prochain appel (upload.rs:152-157, 377-382). Coalescence des sondes : 1 s (mod.rs:215-222). Étiquette de route LAN/WAN/Custom pour l'UI. Pas de test de santé périodique en arrière-plan en dehors de l'appel naturel du réconciliateur (5 min) et des envois.
- Timeout global reqwest 300 s, timeouts spécifiques 10/15 s par endpoint, pool `pool_max_idle_per_host = 24`.
- Défaut : un seul `ImmichApiClient` (un compte, une clé) partagé via `AppContext`.

### 1.9 Divers

- Profils : variable d'environnement `MIMICK_PROFILE` isole config/data/cache/clé trousseau/app-id (profile.rs). Pas un "mode portable" (dossiers XDG de `dirs` toujours).
- Diagnostics : bundle texte/JSON redacté (chemins réduits au nom de fichier, pas d'URL ni de clé, diagnostics.rs:1-140).
- Tests : 627 attributs de test au total, mais concentrés sur le parsing API (93) et la config (30). Le coeur sync est peu couvert : sync_index 8, queue_manager 8, monitor 5, startup_scan 3, remote_sync 0, album_sync/diff 0. Aucun test contre un serveur Immich mocké (wiki/Testing.md, section "Coverage Gaps", le reconnaît). CI : fmt + clippy -D warnings + cargo test sur archlinux (ci.yml), Flatpak/AppImage/AUR.

---

## 2. Inventaire des endpoints Immich utilisés

Version minimale : README.md:32 "Supports Immich v3.0.0+". Breaking change annoncée au CHANGELOG.md:264 ("Minimum supported Immich version is now v3.0+, v1.x no longer supported"). Adaptations v3 : plus de `deviceAssetId`/`deviceId` à l'envoi, propriétaire d'album via `albumUsers`, width/height entiers (CHANGELOG.md:265-267). `memories.rs:34` demande `for` en date simple (v3.1+) puis en timestamp (v3.0) : seul endroit avec un double format. Renommer un tag exige plus récent que 3.0.0 (CHANGELOG.md:174). Je n'ai trouvé AUCUNE vérification de version serveur dans le moteur (pas de gate sur `/api/server/version`) ; la compatibilité est supposée.

Endpoints utilisés PAR LE MOTEUR DE SYNCHRO (coeur) :

| Usage | Méthode et chemin | Fichier |
|---|---|---|
| Santé / bascule LAN-WAN | GET /api/server/ping (attend {"res":"pong"}) | api_client/mod.rs:318 |
| Envoi | POST /api/assets (multipart : assetData, fileCreatedAt, fileModifiedAt, isFavorite, sidecarData) ; 200/201 ok, 409 doublon, 413, 401/403, 502-504 | upload.rs:354 |
| Pré-vol dédup | POST /api/assets/bulk-upload-check (lots de 500 au scan, 1 par fichier à l'envoi) | albums/asset_lookup.rs:20,64 |
| Correctif fuseau | PUT /api/assets {ids, timeZone} (x3 retardé) | upload_helpers.rs:26 |
| Liste albums / cache nom vers id | GET /api/albums ; POST /api/albums (création) | albums.rs:74,188 |
| Ajout/retrait membres | PUT /api/albums/{id}/assets ; DELETE /api/albums/{id}/assets | albums.rs:321,421 |
| Albums d'un asset | GET /api/albums?assetId={id} | albums.rs:380 |
| Assets d'un album (diff) | POST /api/search/metadata {albumIds, page, size:1000} | library.rs:166 |
| Téléchargement | GET /api/assets/{id}/original | library.rs:94 |
| Suppression (corbeille) | DELETE /api/assets {ids, force:false} | library.rs:388-425 |
| Utilisateur courant | GET /api/users/me | library.rs:345 |

Endpoints de la bibliothèque UI (hors moteur, pour mémoire) : /api/assets/{id}, /thumbnail?size=, /video/playback, /edits, /ocr, /api/assets/statistics, /api/search/{metadata,smart,explore,cities,places,suggestions,statistics,random}, /api/timeline/{bucket,buckets}, /api/people*, /api/faces*, /api/tags*, /api/stacks*, /api/memories*, /api/partners*, /api/shared-links*, /api/activities*, /api/duplicates*, /api/map/markers, /api/view/folder*, /api/trash/{restore,restore/assets,empty}, /api/download/{info,archive}, /api/server/{config,features,about,statistics,version-history}, /api/users/me/preferences, /api/albums/{id}/users et /user/{id}. Externe : https://photon.komoot.io/api/ (recherche de lieux) et tuiles VersaTiles/OSM.

Auth : en-tête `x-api-key` partout (clé API Immich), pas d'OAuth ni de session. Permissions requises documentées dans le message d'erreur d'authentification : asset upload+update, album read/create/albumAsset.create (upload.rs:401).

Non utilisés mais pertinents pour votre cible (à ajouter chez vous) : `POST /api/sync/delta-sync`, `/api/sync/stream` (sync incrémentale serveur), `GET /api/assets/{id}/metadata`, header `x-immich-checksum` à l'upload, `PUT /api/assets/{id}/original` (remplacement), checksum base64 vs hex. Mimick liste TOUT l'album à chaque tick de 5 min, ce qui ne passe pas à l'échelle.

---

## 3. Couplage GTK/glib et extractibilité

Verdict : extractible en crate "core" sans GTK à ~85 pour cent, avec un travail de découplage modeste. Environ 25 occurrences de couplage dans les fichiers du moteur, concentrées sur peu de points :

Couplages à couper (fichier : nature) :
- `gettextrs::gettext` / `i18n::fill` dans le moteur lui-même : queue_manager.rs (raisons de pause, 6 appels), progress.rs, api_client/{mod,upload,errors,albums,tags,search,timeline}.rs (textes d'erreur ApiIssue), sanitize.rs, startup_scan/enumeration.rs. Les messages utilisateur sont fabriqués DANS le moteur. Remède : le moteur renvoie des codes d'erreur typés (enum) et c'est l'UI (Svelte) qui traduit.
- `runtime_env.rs` : `gtk::gio::NetworkMonitor` pour "réseau mesuré" (metered), état dans un AtomicBool alimenté depuis le thread GTK ; batterie via /sys/class/power_supply (Linux seul).
- `notifications.rs` : `gio::Notification` + `glib::idle_add_once`.
- `app_context.rs` : `AppContext` contient `ThumbnailCache`, `LibraryState`, `tokio::sync::watch<GridStyle>` (UI) ; il faut le scinder en `EngineContext` (config, state, api_client, queue_manager, monitor, sync_index, live_watch_paths, RecentSelfPaths, ReconcileLocks, PendingDeletions) et un contexte UI.
- `startup_scan/reconcile.rs` et `remote_sync.rs` importent `crate::library::album_sync` / `library::local_source` : le vrai moteur de sync bidirectionnelle est rangé sous library/ ; il faut le remonter dans le crate core.
- `main.rs` / `main/*.rs` : bootstrap GTK, flags tray via `glib::MainContext::spawn_local`, `adw::Application` pour l'instance unique (D-Bus). 
- `autostart.rs` : ashpd Background + `gtk::Window` en paramètre.
- `tray_icon.rs` : ksni + `ashpd::is_sandboxed()`.
- `config/keyring.rs` : oo7 (+ `tokio::task::block_in_place` + `block_on`, qui panique en runtime current_thread).
- `album_sync.rs` : ashpd TrashProxy (code mort sous flag).
- `ImmichApiClient` : zéro dépendance GTK en dehors de gettext. Très bon candidat à l'extraction tel quel.
- `sync_index.rs`, `monitor.rs`, `queue_manager` (hors gettext), `startup_scan` (hors imports library), `config/folders.rs`, `profile.rs`, `util.rs`, `state_manager.rs` (serde pur), `sidecar.rs`, `media_kinds.rs` : portables en l'état.

Le canal d'intégration est propre : `QueueManager`/`Monitor` communiquent par `mpsc`/`Arc<Mutex<AppState>>` et l'UI interroge l'état toutes les 500 ms. Pour Tauri il suffit de remplacer ce polling par des événements (`app.emit`) alimentés depuis un `watch`/broadcast.

Dépendances Linux-only ou non portables et remplaçants :

| Dépendance / code | Problème | Remplaçant cross-plateforme |
|---|---|---|
| `ashpd` (portails : Background, Trash, Location, FileChooser) | Linux/Flatpak seulement | Tauri plugin autostart (`tauri-plugin-autostart`), crate `trash` pour la corbeille (Windows Corbeille, Linux FreeDesktop trash), dialogue via `tauri-plugin-dialog` |
| `oo7` (Secret Service / portail) | Linux seulement | crate `keyring` (Windows Credential Manager, macOS, Secret Service) ; ou `tauri-plugin-stronghold`. À éviter : fallback base64 en clair (keyring.rs:277-345 ; c'est de l'obfuscation, pas du chiffrement) |
| `ksni` (tray StatusNotifierItem) | Linux seulement | `tauri::tray` (Tauri 2 natif, Windows/Linux/macOS) |
| `gettext-rs` + `gettext-system` | libintl système, absent sous Windows | côté Svelte : `svelte-i18n`/`paraglide` ; côté Rust : codes d'erreur typés sans texte |
| `gtk4/libadwaita/gdk4/libshumate/gstreamer` | UI Linux | Remplacés par Tauri + Svelte (carte : MapLibre GL JS ; vidéo : balise `<video>`) |
| `gio::NetworkMonitor` (réseau mesuré) | GLib | Windows : API `NetworkInformation`/`INetworkCostManager` (crate `windows`) ; Linux : D-Bus NetworkManager `Metered` (crate `zbus`) ; à abstraire derrière un trait |
| `/sys/class/power_supply` (batterie) | Linux seulement | crate `battery` ou `starship-battery` ; Windows `GetSystemPowerStatus` |
| `$TZ`, `/etc/localtime`, `/etc/timezone` (fuseau IANA) | Linux seulement | crate `iana-time-zone` ou `localzone` |
| `/.flatpak-info` (détection bac à sable), `PollWatcher` conditionnel | Flatpak | non pertinent Windows ; garder un switch de polling configurable |
| `std::os::unix::PermissionsExt/OpenOptionsExt` (0600) | déjà sous `#[cfg(unix)]` | OK ; sous Windows ACL par défaut du profil |
| `dirs` (XDG) | portable | OK ; ajouter un mode portable (dossier de l'exe) |
| `libc::malloc_trim`, glibc arenas | glibc | non nécessaire |
| `file_timestamps` via `metadata.created()` | birth time absent sur certains FS Linux | sous Windows OK ; garder le min(created, modified) |
| `notify` 8.2 | portable (ReadDirectoryChangesW sous Windows) | OK, mais sous Windows prévoir : renommages = `Modify(Name)` (pas de Remove), fichiers verrouillés par le copieur, chemins >260 car., sensibilité casse, réseau SMB sans événements (PollWatcher) |
| `native-tls` (reqwest) | OpenSSL sous Linux, SChannel sous Windows | acceptable ; `rustls` simplifie le packaging |
| `sha1 = 0.11`, `blake3`, `tokio`, `rayon`, `reqwest`, `serde`, `chrono`, `parking_lot`, `tokio-util`, `futures-util`, `flexi_logger`, `url`, `base64`, `tempfile` | portables | OK |

Portables Windows tels quels : tout `api_client` hors gettext, `sync_index`, `FileTask`/queue, `FolderRules`, `startup_scan` (parcours manuel), `diagnostics`, `sidecar`.

"Open With" / gestionnaire de fichiers : uniquement via le .desktop (setup/dev.nicx.mimick.desktop) : `Exec=mimick %F` avec une longue liste de `MimeType=` (images, RAW, vidéos) et une action "Upload with Mimick" `Exec=mimick --upload %F`. Les chemins arrivent par ligne de commande dans l'instance primaire (GTK `HANDLES_COMMAND_LINE`, instance unique via D-Bus, main/ui_lifecycle.rs:138-190) et alimentent la queue avec `skip_album=true`. Équivalent Windows : entrées de registre `HKCU\Software\Classes\*\shell\...` (menu contextuel) ou `tauri-plugin-single-instance` pour relayer les arguments vers l'instance en cours + `tauri-plugin-deep-link`.

Extraction possible ? Oui : un crate `mimick-core` contenant api_client (sans gettext), sync_index, monitor, queue, startup_scan, album_sync/diff, config schema. Prérequis : (a) trait `Platform` (metered, battery, timezone, trash, secrets, notifier), (b) erreurs typées au lieu de texte traduit, (c) `AppContext` scindé, (d) remplacer le modèle `Arc<Mutex<AppState>>` + polling par un flux d'événements, (e) tests d'intégration contre un Immich mocké. Licence GPL-3.0-or-later : tout code réutilisé fait hériter la licence ; une proposition d'extraction à l'auteur est donc la voie propre, sinon il faut se contenter de réimplémenter les idées (non protégées).

---

## 4. Listes "Keep" et "Avoid"

### 4.1 KEEP (bonnes idées / astuces de robustesse)

1. Index de synchro comme seule source de vérité + empreinte (taille, mtime_ms) : hash recalculé seulement si l'empreinte change (sync_index.rs:127-188). La file et les retries peuvent être perdus sans conséquence (le scan recalcule).
2. `NeedsReassociate` distinct de `NeedsUpload` : changement d'album = appel d'album seulement, sans renvoi (sync_index.rs:21-26, file_events.rs:130).
3. Pré-vol `bulk-upload-check` par lots de 500 avant d'enfiler : les fichiers déjà présents n'entrent jamais dans le pipeline d'envoi (enumeration.rs:328-393).
4. Gestion explicite du 409 (id dans le JSON, repli "DUPLICATE" puis re-lookup) (upload.rs:238-256, queue_manager/upload.rs:165-189).
5. Streaming de l'upload (`FramedRead` + `Part::stream_with_length`), mémoire constante (upload.rs:78-92).
6. Stabilisation d'écriture : 3 mesures de taille identiques + timeout d'inactivité 300 s + ignore `.part/.tmp/.crdownload/~` + debounce 2 s (monitor.rs:20-24, 312-320, 413-446).
7. Hash en blocs de 64 Ko sur `spawn_blocking`, pool borné à num_cpus/2 plafonné à 4 (monitor.rs:88).
8. Fallback `PollWatcher` quand inotify n'est pas fiable (dossiers réseau/portail).
9. Anti-boucle `RecentSelfPaths` (TTL 60 s) pour ignorer ses propres écritures/suppressions (app_context.rs:21-46).
10. Verrou par album (`ReconcileLocks` avec garde RAII) contre les réconciliations concurrentes ; verrou par nom pour la création d'album ; coalescence des sondes de connexion.
11. Suppression à la corbeille Immich uniquement, retrait d'album plutôt que corbeille si l'asset est dans plusieurs albums (remote_sync.rs:184-196).
12. Confirmation en deux observations pour les suppressions de masse (>5) et réinitialisation quand l'élément réapparaît (app_context.rs:84-109, album_sync.rs:415-460). Bonne idée, à généraliser (voir Avoid).
13. Refus de décider une suppression quand des assets distants n'ont pas de checksum (diff.rs:247).
14. Détection de renommage local par checksum d'orphelin (diff.rs:235-338) : évite télécharger + supprimer.
15. Écritures atomiques tmp + rename pour config, index, état, retries (util.rs:17-40) ; flag `dirty` remis à zéro uniquement après succès (sync_index.rs:255-270).
16. Bascule LAN d'abord puis WAN avec `ping` validant la réponse `pong` (pas juste un 200) et invalidation de l'URL active sur erreur (mod.rs:305-350).
17. Concurrence réglable à chaud (10 workers dormants + `AtomicUsize` + `Notify`) (progress.rs:179-208).
18. Politique d'environnement (réseau mesuré, batterie, heures calmes avec fenêtre à cheval sur minuit) appliquée au moment de dépiler (queue_manager.rs:454-511).
19. Arrêt gracieux avec délai d'attente puis annulation par `CancellationToken`, les annulés passent en retry persisté.
20. Mode de rattrapage par dossier (Full / RecentOnly / NewFilesOnly) pour borner le premier scan.
21. Sidecar XMP : détection `photo.jpg.xmp` puis `photo.xmp`, insensible à la casse, envoyé en `sidecarData` (sidecar.rs).
22. Correctif de fuseau retardé après upload (2/8/20 s) pour contrer la réécriture du job de métadonnées.
23. Profils isolés (`MIMICK_PROFILE`) avec validation stricte du nom (profile.rs) : modèle pour multi-compte/portable.
24. Diagnostics redactés (chemins réduits au nom de fichier) pour le support (diagnostics.rs).
25. `safe_filename` contre la traversée de répertoire sur les noms venant du serveur (sanitize.rs:22-38).
26. Modèle de config tolérant : `WatchPathEntry` untagged (String ou objet), `#[serde(default)]` partout, défauts sûrs (UploadOnly, deletes à false) (config/folders.rs).
27. Séparation `skip_album` (téléversement "bibliothèque" sans album) et verrouillage des règles dangereuses dans ce mode.

### 4.2 AVOID (faiblesses, bugs, risques de perte de données)

Risques de perte de données (par ordre de gravité, à vérifier avant de s'en inspirer ; je n'ai rien exécuté, ce sont des lectures de code) :

1. Suppression distante de masse sans garde au démarrage (startup_scan.rs:259-311, 338-359). Quand `delete_folder_to_album` est actif, tout enregistrement d'index sous le dossier absent de `seen_paths` fait mettre l'asset distant à la corbeille, sans la confirmation à deux passes. `seen_paths` n'est alimenté que pour les fichiers retenus par le mode de rattrapage (enumeration.rs:159-170) : avec `RecentOnly` ou `NewFilesOnly`, les anciens fichiers ne sont pas dans `seen_paths` donc seraient traités comme "supprimés". De plus : dossier racine absent (disque externe/montage non monté, enumeration.rs:71-74 retourne vide) ou sous-dossier illisible (`read_dir` en erreur, enumeration.rs:124-131) => aucun chemin vu => corbeille de tout. Seule la corbeille Immich (récupérable) limite les dégâts. Ce défaut est conditionné à l'option (défaut false) mais c'est le plus dangereux du code.
2. Suppression en temps réel sans garde de masse ni délai (file_events.rs:147-154, monitor.rs:281-291) : chaque événement `Remove` (suppression par erreur, déplacement hors du dossier, outil qui réécrit en supprimant puis recréant) met l'asset à la corbeille. Pas de période de grâce, pas de seuil. `Remove` de notify peut aussi venir d'un démontage.
3. Repli agressif : si le comptage d'albums échoue (`None`), on met en corbeille plutôt que de s'abstenir (album_sync.rs:176-178, remote_sync.rs:190-195). Une panne réseau transitoire pendant la vérification transforme un simple retrait d'album en suppression de l'asset.
4. Les détections "orphelin" utilisent `Path::exists()` qui renvoie false sur erreur de permission aussi (album_sync.rs:366, 411) : fichier illisible = traité comme supprimé.
5. Téléchargement non atomique et non vérifié (library.rs:25-44) : écrit directement au chemin final, sans `.part` + rename, sans comparaison de taille/checksum. Une coupure laisse un fichier tronqué au bon nom (le watcher/rattrapage peut ensuite le téléverser comme "nouveau" fichier corrompu, car son checksum ne correspond à rien côté serveur, et le même asset sera retéléchargé en `nom (1).ext`).
6. Condition de course empreinte/hash dans l'index : `record_synced` relit (size, mtime) au moment de la FIN de l'envoi, mais le checksum de la tâche date du hash initial (sync_index.rs:156-158 appelé depuis progress.rs:69-72). Si le fichier est modifié entre le hash et la fin de l'envoi, l'index enregistre la nouvelle empreinte avec l'ancien checksum : la modification n'est plus jamais détectée. L'empreinte devrait être capturée avant le hash et enregistrée telle quelle.
7. Index illisible => réinitialisé silencieusement : `load_entries` renvoie une table vide sur erreur de parse (sync_index.rs:370-387) et le prochain flush écrase le fichier corrompu, sans sauvegarde `.bak`. Pour `config.json`, un échec de parse garde les défauts puis un `save()` peut écraser la config utilisateur (config.rs:53-71).
8. Syncs "Sync Now" manuelle qui RE-ENVOIE un fichier déjà synchronisé puis supprimé côté serveur (diff.rs:254-260) : le serveur perd toujours ; pas de notion de tombstone/état de suppression distant ni d'ancêtre commun (pas de modèle à trois états). Pas de vrai moteur de conflits : un fichier modifié des deux côtés devient deux fichiers.
9. Confirmation à deux passes en mémoire seulement, déclenchée par 2 appels consécutifs (rien n'empêche deux "Sync Now" rapprochés de franchir le seuil) (app_context.rs:84-109) ; seuil fixe de 5, non configurable, par lot et pas en pourcentage de l'album.
10. Nom d'album comme clé fonctionnelle : le dossier est lié par nom ("Default (Folder Name)") et `get_or_create_album` par nom ; Immich autorise des albums homonymes, d'où collisions possibles (albums.rs:251-296). Un renommage d'album côté serveur casse le lien par nom.
11. Clé API : fallback fichier encodé en base64 (pas de chiffrement), préféré au trousseau s'il existe (keyring.rs:20-27, 277-345). `log_config` journalise les URL en clair (main.rs:273-280, pas la clé).
12. Réutilisation d'un asset déjà en corbeille serveur : `bulk-upload-check` renvoie l'id même si l'asset est dans la corbeille ; le code l'ajoute à l'album et marque "synchronisé" (queue_manager/upload.rs:44-48) sans vérifier `isTrashed` (aucune occurrence de `isTrashed` dans le moteur). L'utilisateur croit sa photo sauvegardée alors qu'elle est dans la corbeille Immich.

Faiblesses de conception / robustesse :

13. Pas de backoff, pas de plafond de tentatives : un fichier définitivement en échec (413, droits) boucle à chaque succès suivant ; échecs d'authentification (401/403) comptés comme échecs de fichier (upload.rs:147-150). Pas de distinction erreur transitoire / définitive.
14. Retries et état écrits seulement à l'arrêt propre (queue_manager.rs:397-407) ; file de 64 en mémoire. Récupérable via l'index, mais l'UI "échecs" est perdue au crash.
15. `AppState` = un unique `Mutex` global touché à chaque octet de progression (callbacks de progression, queue_manager.rs:50-73) : contention avec 10 workers.
16. `ShardedSyncIndex` : sharding purement cosmétique (un seul fichier JSON, sérialisé en entier ; recherche par checksum O(N) sur tous les shards ; deux tables clones). Prévoir SQLite/redb/sled avec index sur checksum et écritures incrémentielles pour 100k+ fichiers.
17. Liste intégrale de l'album tous les 5 min par dossier (+ hash séquentiel du dossier entier, diff.rs:290-304) : coût et latence proportionnels à la taille de la photothèque ; pas d'utilisation de `/api/sync/*` (delta). La recherche de l'asset à supprimer parcourt l'album entier à chaque suppression (remote_sync.rs:252).
18. Téléchargements strictement séquentiels (album_sync.rs:91-97), sans reprise (`Range`), sans limite de bande passante.
19. `unique_destination` a une fenêtre de course (existe puis crée) et un plafond de 999 (album_sync.rs:260-286) ; clé d'anti-boucle consommée à la première occurrence seulement : les événements notify multiples d'un même téléchargement peuvent repasser (atténué par `record_synced` qui rend la décision UpToDate).
20. `Monitor` : `replace_watches` désabonne tout puis réabonne (fenêtre aveugle) ; Remove sur répertoire entier non géré (le filtre `is_supported_media_path` ne voit que les fichiers) ; débordements inotify non rattrapés avant le prochain démarrage ; noms de fichiers non UTF-8 aplatis par `to_string_lossy` (clé d'index ambiguë).
21. Doc/commentaires périmés ou contradictoires : monitor.rs:5 "BLAKE3", queue_manager.rs:4 "exponential back-off", FolderRules `delete_folder_to_album` doc inversée (folders.rs:22), `AppContext` "replaces..." ; `ALBUM_TO_FOLDER_TRASH_AVAILABLE=false` laisse une option UI inopérante.
22. Texte utilisateur traduit généré dans le moteur (gettext) : empêche la réutilisation et le test déterministe.
23. Couverture de tests faible sur le coeur critique : remote_sync et album_sync/diff à zéro test, startup_scan trois tests, aucun test d'intégration sur serveur mocké (wiki/Testing.md "Coverage Gaps"), alors que ce sont les chemins qui suppriment.
24. Les chemins `block_in_place` + `block_on` dans keyring.rs sont fragiles hors runtime multi-thread.
25. `reqwest` timeout global 300 s : un envoi de gros fichier lent est coupé (vidéos de plusieurs Go sur lien lent) ; les 10 s sur albums/ajout peuvent suffire pour de grosses listes.
26. Pas de gestion des fichiers verrouillés (Windows) ni des hard links/symlinks (suivi de boucles de liens non vu dans les parcours manuels) ; aucun support de la limite de chemin 260 caractères.

### 4.3 Recommandations pour votre moteur (Tauri 2 + Svelte, Windows + Linux)

- Reprendre : index (taille, mtime) comme vérité, `NeedsReassociate`, bulk-upload-check par lots, gestion 409, streaming, stabilisation de fichier, anti-boucle, verrous RAII par album, corbeille uniquement, confirmation multi-passes, détection de renommage par checksum, écritures atomiques, profils isolés, diagnostics redactés.
- Corriger d'emblée : état d'index à trois voies (local / serveur / dernier état synchronisé) avec tombstones pour de vrais conflits ; seuil de suppression en pourcentage ET absolu, persistant, avec confirmation UI, et test "racine montée/lisible" avant toute suppression ; téléchargement `.part` + vérification + rename ; empreinte capturée avant hash ; index en base embarquée ; backoff exponentiel avec classes d'erreurs ; état de file persistant (journal) ; sync incrémentale via `/api/sync/*` plutôt que listes complètes.
- Pour "fichiers virtuels / placeholders" : Mimick n'a rien d'équivalent (aucun placeholder, aucun mode hors ligne, un seul compte). C'est du terrain vierge : Windows Cloud Files API (cfapi) / ProjFS, et FUSE ou XDG sur Linux ; à concevoir à part.
