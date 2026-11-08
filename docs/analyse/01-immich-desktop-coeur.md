# Analyse du coeur de immich-desktop (client Qt/C++ non officiel, MIT)

Source : /home/goss/.cache/immich-ref/immich-desktop (HEAD 0eb346d, version 0.2.4, dépôt hdmain/immich-desktop).
Périmètre : src/core/*, src/main.cpp, tests/, CMakeLists.txt, .github/workflows/*, snap/, flatpak/, aur/. Rien n'a été compilé ni exécuté.
Les références sont au format fichier:ligne (chemins relatifs à src/core/ sauf mention contraire).

Vue d'ensemble : application Qt 6 Widgets monolithique (une lib statique `immich_app` + un exécutable). Le "coeur" est mélangé au Qt (QNetworkAccessManager, QObject/signaux, QSettings, QThreadPool). Il n'y a aucune couche domaine indépendante du framework : `ImmichClient` (3209 lignes) fait à la fois client HTTP, orchestrateur de pagination, cache, file d'upload, sondage réseau et décodage d'images. Pour un portage Rust, il faut redécouper.

---

## 1. Inventaire des endpoints Immich utilisés

### 1.1 Authentification, URL, réseau

- Authentification : uniquement une clé API dans l'en-tête `x-api-key` (ImmichClient.cpp:849 et 3122, VideoStreamServer.cpp:298). Aucune session, aucun login email/mot de passe, aucun OAuth, aucun refresh. `isConfigured()` = serverUrl et apiKey non vides (AppSettings.h:42).
- Construction des URL : `apiUrlForBase()` (ImmichClient.cpp:482) prend l'URL serveur, retire les `/` finaux, ajoute `/api` si absent, puis le chemin. Donc l'utilisateur peut saisir `https://host` ou `https://host/api` ou un sous-chemin de reverse proxy. Le partage public (`/share/<key>`) retire `/api` (ImmichClient.cpp:2894).
- Deux URL : `serverUrl` (distante, WAN) et `localServerUrl` (LAN, optionnelle). Sondage toutes les 12 s (`m_endpointProbeTimer`, ImmichClient.cpp:409) : `GET {localUrl}/api/server/ping` avec timeout 1500 ms, config TLS par défaut (jamais de certificat client sur le LAN), redirections même origine. Si réponse contient `pong`, l'URL active devient la locale, sinon la distante (ImmichClient.cpp:711-770). La reachability (en ligne/hors ligne) est un second sondage toutes les 15 s sur l'URL active, timeout 2500 ms (ImmichClient.cpp:412, 772-801), combiné avec `QNetworkInformation::reachabilityChanged` (ImmichClient.cpp:525-547).
- Les liens de partage utilisent toujours l'URL distante configurée, jamais la LAN (ImmichClient.cpp:2899).
- En-têtes personnalisés : appliqués à chaque requête vers l'hôte Immich (y compris l'URL locale), jamais aux redirections CDN/S3 (VideoStreamServer.cpp:299-302).
- Timeouts : 30 s par défaut (ImmichClient.cpp:852), 20 s pour la recherche (1074), 15 s pour les vignettes (3125), 120 s pour le flux vidéo amont (VideoStreamServer.cpp:309), 30 s pour la liste de prefetch (2114).
- Gestion d'erreur : `errorMessage()` (ImmichClient.cpp:866) extrait `message` (chaîne ou tableau) du JSON d'erreur Immich, traduit 401 en "clé API/permissions", SslHandshakeFailed en message mTLS (et rouvre la demande de mot de passe de clé). Les erreurs réseau "transitoires" (liste en ImmichClient.cpp:562-583) basculent l'état en hors ligne via `setOnline(false)`. Il n'y a PAS de retry générique ni de backoff exponentiel : seuls les uploads ont un retry (voir 2.5) et la bascule hors ligne se rétablit via le sondage de 15 s.
- Détection de version serveur : `GET /server/version` (ImmichClient.cpp:437-457), stocké en `QVersionNumber`, `serverAtLeast(major, minor)` pilote les fonctions conditionnelles. Les seuils réellement codés : 3.0 (tri par date d'ajout) et 3.2 (tri par taille/rating, filtre structuré, OR d'ids). Le seuil 3.0 n'est pas testé par `serverAtLeast` dans le code : le tri "date ajoutée" envoie `orderBy=createdAt` sur le timeline sans vérification (ImmichClient.cpp:323-329, commentaire "servers older than 3.0 (no orderBy) keep working with the default" car seuls les tris non défaut ajoutent des paramètres). Le seuil 3.2 est testé dans LibraryPage.cpp:1209 et ExplorePage.cpp:1754, 1759.

### 1.2 Tableau des endpoints

Toutes les routes sont préfixées `/api`. Permissions de clé API citées d'après le README (README.md, section "How to run") ou les messages d'erreur du code.

| Méthode | Chemin | Usage | Paramètres / corps | Pagination / stratégie | Version / permission |
|---|---|---|---|---|---|
| GET | /server/ping | Sondage LAN et reachability | aucun, réponse `{res:"pong"}` | toutes les 12 s (LAN) / 15 s (actif) | sans auth requise |
| GET | /server/version | Version serveur | réponse `{major,minor,patch}` | une fois au démarrage et au changement de config | |
| GET | /server/config | Jours de rétention de la corbeille (`trashDays`) | | ponctuel | |
| GET | /users/me | Test de connexion, nom affiché, id courant (pour "partagé par un autre utilisateur") | | ponctuel | user.read |
| GET | /timeline/buckets | Index complet de la timeline : liste de mois avec compteurs | `size=MONTH`, `order=asc` si ascendant, `orderBy=createdAt` si tri par date d'ajout ; pour une collection : `personId`, `albumId`, `tagId`, `isFavorite=true`, `visibility=archive`, `isTrashed=true` (ImmichClient.cpp:303-319) | un seul appel renvoie tous les mois (`[{timeBucket,count}]`), triés côté client (ImmichClient.cpp:236-254) | orderBy=createdAt : Immich 3.0+ |
| GET | /timeline/bucket | Contenu d'un mois, format colonnaire (tableaux parallèles `id,isImage,isFavorite,isTrashed,ratio,duration,fileCreatedAt,createdAt,visibility,status`), SANS nom de fichier (ImmichClient.cpp:176-225) | `timeBucket=YYYY-MM-01`, `size=MONTH`, mêmes filtres que ci-dessus | 1 requête par mois, 4 en parallèle côté UI (`kParallelTimelineBuckets=4`, LibraryPage.cpp:85, 806) | |
| POST | /search/metadata | Moteur principal de listes : pages de bibliothèque, recherche par nom, collections (personne, ville, album, tag, favoris, archive, corbeille), poll des nouveautés, prefetch, vérification du tri "dernier consulté" | Forme plate (ancienne) : `page,size(<=250),withExif,withStacked,order(asc/desc),visibility,personIds,city,albumIds,tagIds,isFavorite,withDeleted,trashedAfter,originalFileName/description/ocr...`. Forme structurée (3.2+) : `{filter:{field:{eq/any/all/like/matches/gte/lte/ne}}, orderBy:{field,direction}, cursor, size}` (toFilterShape, ImmichClient.cpp:335-370) | forme plate : `assets.nextPage` (page numérique). Forme structurée : `assets.nextCursor`, converti en pseudo numéro de page côté app (ImmichClient.cpp:1105-1108). Taille de page 80 (collections) ou 250 (recherche tout, prefetch) | filtre structuré, orderBy size/rating, filtre `or` d'ids : Immich 3.2+ |
| POST | /search/smart | Recherche sémantique (CLIP) quand une requête est tapée sans "mode" | `query,page,size,withExif` | pages | |
| POST | /search/random | Mélange (Shuffle) d'une collection ou de la bibliothèque | filtre de collection + `size=80,withExif` | un lot aléatoire de 80, rappelé à volonté | |
| GET | /search/explore | Page Explorer : villes (`exifInfo.city`) et ajouts récents (`createdAt`) | | un appel | |
| GET | /people | Liste des personnes | `withHidden`, `page`, `size` (100 pour Explorer, 1000 pour la liste complète) | une seule page demandée (pas de suite de pagination, limite silencieuse à 1000/100) | person.read |
| GET | /people/{id}/thumbnail | Visage de la personne | | cache disque/mémoire 256 px | person.read |
| GET | /albums | Liste des albums ; avec `?assetId=` : albums d'une photo | | tout d'un coup | album.read |
| PATCH | /albums/{id} | PATCH `{order:"asc"|"desc"}` pour enregistrer l'ordre d'un album | | | album.update |
| GET | /tags | Liste des tags | | tout d'un coup | tag.read |
| GET | /view/folder/unique-paths | Arborescence de dossiers | | tout d'un coup | folder.read |
| GET | /view/folder?path= | Contenu d'un dossier (avec détails de fichier) | | tout d'un coup ; tri fait côté client (`sortFolderAssets`, ImmichClient.cpp:271-300) | folder.read |
| GET | /activities?albumId= | Tri d'un album partagé par likes/commentaires | | tout d'un coup, puis `searchAllPages` pour les assets | activity.read (403 détecté, ImmichClient.cpp:1441) |
| GET | /assets/{id} | Détails d'un asset (EXIF, personnes, tags, propriétaire, chemin d'origine) ; aussi `fetchAssetInfo` | | ponctuel | asset.read |
| GET | /assets/{id}/thumbnail?size=thumbnail|preview|fullsize | Vignettes (256 px max côté app), preview viewer (1920 px max), repli `fullsize` si le décodage échoue (ImmichClient.cpp:3175-3193) | `size` | cache disque (voir 2.2) | asset.view |
| GET | /assets/{id}/original | Téléchargement de l'original sur disque (streaming vers fichier) ; copie presse-papier limitée à 64 Mo | | | asset.download |
| GET | /assets/{id}/video/playback | Flux vidéo, via proxy local | Range | | asset.view |
| POST | /assets (multipart) | Upload | champs `deviceAssetId, deviceId="immich-desktop", fileCreatedAt, fileModifiedAt, filename, isFavorite=false`, fichier `assetData` | un fichier à la fois | asset.upload |
| PUT | /assets | Mettre/retirer des favoris par lot | `{ids,isFavorite}` | | asset.update |
| DELETE | /assets | Supprimer par lot | `{ids, force}` (force = suppression définitive) | | asset.delete |
| POST | /trash/restore/assets | Restaurer des assets | `{ids}` | | |
| POST | /trash/restore | Tout restaurer | | | |
| POST | /trash/empty | Vider la corbeille | | | |
| POST | /shared-links | Créer un lien de partage individuel | `{type:"INDIVIDUAL",assetIds,allowUpload:false,allowDownload:true,showMetadata:false,password?}` ; la clé retournée est validée par regex puis l'URL publique construite côté client | | sharedLink.create |

Endpoints notablement ABSENTS : `/assets/bulk-upload-check` (déduplication avant upload), `/assets/exist`, `/sync/*` (Sync Stream des clients mobiles), `/server/features`, `/auth/*`, `/assets/{id}` en PUT (pas d'édition de métadonnées sauf favoris), `/albums` en POST ou PUT assets, `/map/*`, `/memories`. La carte est dans la roadmap, pas dans le code.

### 1.3 Contournements de version notables

- Le format de recherche a deux dialectes (plat et structuré) et le client bascule dynamiquement : dès qu'un `orderBy` est présent dans le corps, tout est converti en structure (ImmichClient.cpp:1062-1067, 1918-1923). Le serveur 3.2 rejette le mélange des deux (commentaire ImmichClient.cpp:331-334). Attention : la forme structurée inclut les assets en corbeille sauf `trashedAt eq null`, d'où l'ajout systématique (ImmichClient.cpp:365-367).
- Le curseur de pagination structurée est stocké dans un hash global `m_searchCursors["library"|"collection"]` : fragile, un seul curseur actif par type (hypothèse "les vues demandent les pages une par une", ImmichClient.h:~250).
- Tri par "date d'ajout" : la recherche ne peut pas, donc l'app passe par la timeline et reconstitue des "pages" de mois (environ 80 photos par page, ImmichClient.cpp:1696-1797).
- Tri "dernière date de suppression" de la corbeille : approximé par `updatedAt` (ImmichClient.cpp:1475).
- Tri "dernier consulté" : historique local (jamais côté serveur), puis un OR d'ids en forme structurée 3.2+ (ImmichClient.cpp:1559-1623).
- Un `isValidImmichId` (regex UUID, ImmichClient.cpp:227) est appliqué avant d'interpoler un id dans une URL : bonne hygiène contre l'injection de chemin.

---

## 2. Fonctionnement détaillé

### 2.1 Chargement de la timeline

Stratégie "index complet puis mois à la demande" (la plus intéressante du projet) :

1. `GET /timeline/buckets?size=MONTH` ramène la liste de tous les mois avec leur compteur. Le code réserve tout de suite la hauteur de défilement (`estimateReservedTimelineHeight`, LibraryPage.cpp:~830) à partir des compteurs : la scrollbar est stable avant que le contenu n'arrive.
2. `GET /timeline/bucket?timeBucket=YYYY-MM-01&size=MONTH` par mois, 4 requêtes en parallèle (LibraryPage.cpp:85, 804-813), du plus récent au plus ancien (ou inverse selon le tri).
3. Les mois reçus sont aplatis via `assetsFromBucketBody` (ImmichClient.cpp:180-225) : format colonnaire, filtrage côté client des éléments `isTrashed`, `status != active`, `visibility != timeline` (pour masquer par exemple la moitié vidéo d'une Live Photo). Ce format n'a pas `originalFileName`, donc le nom de fichier n'est connu qu'au chargement de détails.
4. Chaque mois reçu est mis en file pour l'écriture dans le cache hors ligne : fusion différée de 2 s (`m_cacheFlushTimer`, ImmichClient.cpp:407, 585-597).
5. Détection des nouveautés : `pollNewestAssets(24)` fait un `POST /search/metadata` des 24 plus récents toutes les N secondes tant que la vue est visible (LibraryPage.cpp:896-902), en forçant `visibility=timeline`.
6. Hors ligne : si `!m_online`, la page repose sur un snapshot plat du cache (`emitCachedLibrary`, ImmichClient.cpp:1129-1138), sans les mois.
7. Les tris "taille" et "rating" (3.2+) sortent de la timeline : liste paginée plate par curseur (`listSortActive`, LibraryPage.cpp:~740-800).

Problèmes : (a) tout le contenu d'un mois arrive d'un coup, sans limite (un mois de 20 000 photos = grosse réponse JSON non streamée) ; (b) pas de reprise partielle ; (c) l'index de mois est refetché à chaque rafraîchissement (pas de ETag ni de delta) ; (d) aucun usage du protocole de sync incrémentale d'Immich.

### 2.2 Cache de vignettes

Classe `ThumbnailCache` (ThumbnailCache.cpp), deux instances (ImmichClient.h:~232-236) :

- `thumbnails` : cache mémoire QCache avec budget de 8 Mo (en "kb" de pixmaps décodés, ThumbnailCache.h) + cache disque SANS limite (`maxDiskBytes=0`).
- `previews` : mémoire 8 Mo + disque borné à 512 Mo, utilisé pour les previews du viewer et pour les "tuiles" plus grandes que 256 px (`tile:N`).
- Disposition disque : un dossier plat par espace de noms dans `{CacheLocation}[/profiles/<id>]/thumbnails|previews/`, un fichier par asset nommé `SHA1(assetId).bin` (ThumbnailCache.cpp:431-436). Pas de sous-répertoires de sharding, donc des centaines de milliers de fichiers dans un seul dossier.
- Contenu : les vignettes sont ré-encodées en JPEG qualité 82 à 256 px max (ImmichClient.cpp:3031-3037, 3182-3205), les previews stockent l'octet-stream serveur tel quel pour les tuiles et un JPEG réencodé pour "preview".
- Écriture atomique via `QSaveFile` (ThumbnailCache.cpp:477-485).
- Éviction : seulement pour le cache `previews`. À chaque écriture, `trimDiskLocked()` liste TOUT le dossier, somme les tailles, et si dépassement trie par `lastModified` croissant et supprime (ThumbnailCache.cpp:496-516). L'étiquette "LRU" est donc fausse : la lecture (`readDisk`) ne touche pas le mtime, c'est du FIFO par date d'écriture. Et le coût est O(n) par écriture de preview.
- Ordonnancement des fetchs réseau (ImmichClient.cpp:2967-3206) : un `QThreadPool` de 4 threads (ImmichClient.cpp:404), priorités : lecture de cache d'abord (INT_MAX), preview du viewer ensuite (INT_MAX-1), puis tuiles à l'écran (bande 1<<29), puis le reste ; au sein d'une bande, "dernier demandé d'abord" (sérial croissant). Dédoublonnage par clé `taille:assetId`. Le `TileImageScheduler` côté UI applique des délais de présence (250 ms visible, 600 ms proche) et plafonne à 16 fetchs et 64 vérifs de cache par passe (TileImageScheduler.cpp:12-16).
- Préchargement complet optionnel : `prefetchAllThumbnails()` pagine `POST /search/metadata` par 250 (visibility=timeline, withExif=false), ignore ce qui est déjà sur disque (`hasDisk`, un `stat` par asset), télécharge le reste (ImmichClient.cpp:2053-2189). Hors ligne : s'appuie sur la liste du cache hors ligne.
- Photos de personnes : même `ThumbnailCache`, clé `person:<id>`, 256 px.

### 2.3 Magasin hors ligne (OfflineStore)

Il n'y a PAS de base de données : uniquement du JSON (OfflineStore.cpp). Schéma par fichier, clé = `SHA1(serverUrl)` :

- `{cache}/offline/<sha1>-library.json` : `{serverUrl, savedAt, query, assets:[{id,type,fileName,duration,favorite,aspectRatio,takenAt?,addedAt?}]}`. Ne contient ni `rating`, ni `fileSize`, ni `updatedAt`, ni `modifiedAt`, ni `localDateTime` (donc les tris par taille/rating/modification ne marchent pas hors ligne) (OfflineStore.cpp:69-83).
- `{cache}/offline/<sha1>-explore.json` : `{people,places,recentAssets}`.
- `{data}/history/<sha1>-viewed.json` : historique des 300 derniers assets consultés (ImmichClient.cpp:1639) ; `...-views.json` : statistiques `{asset, views:[epochSec...]}` non plafonnées (le tableau des vues grossit sans limite).
- Écriture atomique `QSaveFile` en JSON compact (OfflineStore.cpp:101-114).
- `mergeLibrary` : lit TOUT le fichier, fusionne via un hash id vers index, réécrit TOUT le fichier (OfflineStore.cpp:238-259). Déclenché toutes les 2 s pendant un chargement de timeline : coût quadratique sur grosses bibliothèques (le commentaire ImmichClient.h:~237 l'admet : "rewriting the whole cache for each of hundreds of timeline months made Library loading quadratic", atténué par le batch de 2 s mais pas supprimé).
- Aucune suppression : un asset supprimé sur le serveur reste dans le cache (le fichier est remplacé seulement quand la page 1 d'une recherche réussit, ImmichClient.cpp:1114-1118 ; la timeline, elle, ne fait que fusionner). `clearCachedLibrary()` existe mais n'est appelée nulle part dans src (grep). Les fantômes persistent donc hors ligne.
- "Hors ligne" ne veut pas dire fichiers originaux : seuls vignettes + métadonnées minimales. Les originaux téléchargés ne sont pas gérés par un magasin (le README promet des "full-resolution offline packs" en roadmap).

### 2.4 Services annexes de persistance

- `UploadQueueStore` : un seul fichier `{data}/upload-queue.json` `{"paths":[...]}` ; à chaque modification (`persistUploadQueue`) tout le tableau est réécrit ; `remove()` et `enqueue()` relisent + réécrivent tout le fichier (UploadQueueStore.cpp:609-627) donc O(n) par fichier uploadé, O(n²) pour un gros import. On ne stocke que les chemins, pas d'état par fichier (tentatives, taille, hash, erreur). `load()` filtre les chemins qui n'existent plus (silencieusement, sans notification).
- `AppSettings` : QSettings INI. Racine globale `…/Immich/immich.ini` (profils, langue, apparence) ; un `profiles/<uuid>.ini` par profil contenant `immich/*`, `folderSync/*`, `download/*`, `timeline/*`, `playback/*` (AppSettings.cpp:17-70). Migration automatique d'une config historique vers le profil `legacy` (AppSettings.cpp:44-71).
- Profils (multi-compte) : listés dans `profiles/names/<id>` ; `profileDataDirectory()` et `profileCacheDirectory()` ajoutent `profiles/<id>` sauf pour `legacy` (AppSettings.cpp:238-250). Le changement de profil se fait en quittant la boucle Qt avec le code 42 puis en reconstruisant `ImmichClient`, `FolderSyncService` et la fenêtre dans une boucle `do/while` dans `main` (main.cpp:105-141). Un seul compte actif à la fois : pas de multi-compte simultané.

### 2.5 File d'upload et reprise

- `uploadAssets()` ajoute les chemins absolus à `m_uploadQueue` (QStringList), persiste, et lance `processUploadQueue()` si en ligne (ImmichClient.cpp:2239-2264).
- Un seul upload à la fois (`m_uploadInFlight`), multipart `QHttpMultiPart` avec le fichier en flux (`setBodyDevice`) : pas de chargement complet en mémoire.
- `deviceAssetId` = `"{chemin absolu}-{taille}-{mtime en secondes}"` (ImmichClient.cpp:2392-2395) et `deviceId` fixe `"immich-desktop"` : l'identifiant d'appareil est le même pour toutes les machines d'un utilisateur, et le chemin absolu fuit au serveur. Utiliser `fileCreatedAt` = birthTime, sinon mtime (ImmichClient.cpp:2387-2391) ; birthTime n'est pas disponible partout sous Linux (fallback sur mtime).
- Réponse : le serveur renvoie `{id, status}` ; `duplicate` détecté via `duplicate:true` ou `status:"duplicate"` (ImmichClient.cpp:2472-2475). La déduplication est donc entièrement côté serveur (SHA-1 du contenu calculé par Immich) : le fichier entier est ENVOYÉ avant de savoir qu'il est en doublon. Aucun `x-immich-checksum`, aucun `POST /assets/bulk-upload-check`.
- Reprise après échec : erreur réseau transitoire = remise en tête de file, retour hors ligne (`setOnline(false)`), le sondage de 15 s relance (ImmichClient.cpp:2442-2461). Erreur d'ouverture de fichier = 5 essais avec délai 2 s x tentative (ImmichClient.cpp:2353-2375). Erreur HTTP non transitoire (4xx, 5xx) = abandon définitif du fichier (retiré de la file, signal d'échec). Pas de reprise partielle d'un upload (pas de TUS/chunk), pas de pause/reprise, pas de backoff exponentiel.
- Piège : `UnknownNetworkError` et `OperationCanceledError` comptent comme "transitoires" (ImmichClient.cpp:571, 578) : un timeout de 30 s (`setTransferTimeout(30000)` hérité de `authenticatedRequest`, ImmichClient.cpp:852, non surchargé pour l'upload, ImmichClient.cpp:2419-2420) coupe un gros fichier qui n'a pas de trafic pendant 30 s. QNetworkAccessManager applique `transferTimeout` comme inactivité, ce qui laisse passer les uploads actifs, mais un upload bloqué en attente de réponse du serveur après l'envoi (traitement serveur d'une grosse vidéo) peut dépasser 30 s et sera REJOUÉ en boucle comme "transitoire" : risque de doublon/boucle (à vérifier à l'exécution, non testé).

### 2.6 Synchronisation de dossiers (FolderSyncService)

Algorithme (FolderSyncService.cpp) :

1. Réglages : liste de dossiers (chemins canonisés, dédup insensible à la casse sous Windows, FolderSyncService.cpp:185-225), `enabled`, `localNetworkOnly`.
2. Détection : `QFileSystemWatcher` sur le dossier racine et TOUS les sous-dossiers énumérés récursivement à chaque reconstruction (FolderSyncService.cpp:559-583), plus un scan périodique complet toutes les 8 s (`kScanIntervalMs`, ligne 131) qui re-énumère tout l'arbre avec `QDirIterator` à chaque tick (`scanNow` -> `collectMediaFiles`) et reconstruit le watcher (`periodicScan`, ligne 410).
3. Filtre : extension dans une liste fixe (jpg, jpeg, png, gif, webp, heic, heif, tif, tiff, bmp, raw, dng, cr2, nef, arw, mp4, mov, m4v, avi, mkv, webm, 3gp ; FolderSyncService.cpp:416-434), fichiers cachés ignorés. Pas de .orf, .rw2, .raf, .jxl, .avif, .mts etc.
4. Stabilité : un fichier n'est mis en file qu'après 1,5 s sans changement de taille (`kStabilityMs`, timer de 500 ms, FolderSyncService.cpp:130, 366-408). Détecte une copie en cours, mais pas un fichier verrouillé ou une écriture à taille constante.
5. Déduplication LOCALE : "empreinte" = `"{taille}:{mtime en ms}"` par chemin absolu (FolderSyncService.cpp:446-452, 498-503). Un fichier est considéré synchronisé si le chemin est dans l'état ET l'empreinte correspond. Il n'y a AUCUN hash de contenu côté client. Conséquences : renommer/déplacer un fichier le fait ré-uploader (le serveur répondra "duplicate" après transfert complet) ; toucher le mtime idem ; copier le même fichier dans deux dossiers l'envoie deux fois. L'état est conservé dans `{data}/folder-sync-state.json` (`{folders, files:{chemin: empreinte}}`) et réécrit INTÉGRALEMENT après chaque fichier (`markSynced` -> `saveSyncedState`, FolderSyncService.cpp:514-557) : quadratique sur un premier import de dizaines de milliers de fichiers.
6. Envoi : `ImmichClient::uploadAssets` (file unique partagée avec les uploads manuels). Succès (y compris `duplicate`) = `markSynced`. Échec = erreur mémorisée en RAM (`m_errors`), jamais persistée, et le fichier sera retenté au prochain scan de 8 s tant que son empreinte n'est pas enregistrée (boucle de re-tentative infinie toutes les 8 s sur un fichier rejeté en 4xx, FolderSyncService.cpp:269-282).
7. Contrainte réseau : option `localNetworkOnly` : l'upload n'est lancé que si `usingLocalEndpoint()` (FolderSyncService.cpp:505-512). La file d'upload globale, elle, ne connaît pas ce critère : des chemins déjà en file continuent de partir sur l'URL distante si le LAN disparaît (le critère n'est testé qu'à l'enfilage, ligne 587).
8. Affichage des fichiers non synchronisés dans la bibliothèque comme "assets locaux" (`pendingLibraryAssets`) avec états Unsaved/Saving/Error, ce qui lance un scan complet du dossier à chaque `publishLibraryPending` (FolderSyncService.cpp:678-719, appelé sur presque chaque signal, y compris `uploadProgress`) : coût disque élevé (énumération récursive + `QImageReader` pour le ratio sur chaque fichier non synchronisé).
9. Suppressions et modifications côté local ne sont jamais propagées au serveur ; rien n'est téléchargé du serveur vers le dossier : c'est un envoi unidirectionnel.

### 2.7 Streaming vidéo : pourquoi un serveur local

`VideoStreamServer` (VideoStreamServer.cpp) est un mini serveur HTTP/1.1 sur `127.0.0.1` port éphémère (ligne 135) qui proxifie `GET /assets/{id}/video/playback`. Raisons visibles dans le code et les commentaires :

- Le lecteur multimédia Qt (GStreamer/FFmpeg) ne sait pas envoyer l'en-tête `x-api-key`, les en-têtes personnalisés, ni présenter un certificat client mTLS : le proxy les ajoute côté amont et expose au lecteur une URL simple `http://127.0.0.1:PORT/video/<uuid>?token=<uuid-de-session>` (VideoStreamServer.cpp:149-160).
- Le proxy suit lui-même les redirections Immich vers un CDN/S3 (ManualRedirectPolicy, max 5) et retire la clé API et le TLS client sur les sauts vers un autre hôte (VideoStreamServer.cpp:279-339), pour ne pas divulguer les secrets à un tiers.
- Il transmet `Range` (validé par regex) et renvoie `Content-Range`, `Content-Length`, `Accept-Ranges: bytes`, avec contre-pression (écrit au plus 512 Ko en file, chunks de 64 Ko, VideoStreamServer.cpp:356-375).
- Protection : écoute seulement sur localhost, jeton de session aléatoire obligatoire en query (comparaison simple `!=`), chemin validé par regex UUID, en-têtes limités à 32 Ko, méthodes GET/HEAD seulement.
- main.cpp:31-38 force le backend GStreamer sous Linux parce que FFmpeg bloque le thread UI à l'ouverture d'un flux HTTP ; snap/local/immich-launch le force aussi.

Faiblesses : l'URL avec jeton est dans une chaîne QUrl passée au lecteur (visible dans les logs GStreamer) ; pas de limite de connexions concurrentes ; statut HTTP "Error" générique ; pas de gestion des plages 416 ; pas de transcodage (Immich ne le fait qu'à la demande côté serveur). En Rust/Tauri, ce proxy reste pertinent pour la balise `<video>` du webview (qui n'envoie pas d'en-tête personnalisé ni mTLS) : à garder, avec une implémentation hyper/axum.

### 2.8 mTLS et en-têtes personnalisés

- `TlsConfig::build` (TlsConfig.cpp:535-575) assemble une `QSslConfiguration` : certificat client PKCS#12 (`importPkcs12`, mot de passe obligatoire, chaîne complète présentée, TlsConfig.cpp:459-497) ou PEM (cert + clé RSA puis EC, mot de passe optionnel, TlsConfig.cpp:499-531), plus une CA personnalisée PEM ou DER ajoutée AU MAGASIN de confiance (TlsConfig.cpp:551-572).
- Le certificat client ne s'applique qu'à l'URL distante : `sslConfigForActiveEndpoint()` renvoie la config par défaut dès qu'on est sur l'endpoint local (ImmichClient.cpp:679-684). Idem pour la sonde LAN.
- Piège historique corrigé : les requêtes envoyées avant la construction de la config empoisonnaient le pool de connexions de QNAM (ImmichClient.cpp:415-420), d'où `clearConnectionCache()` (ImmichClient.cpp:661).
- Le mot de passe de clé : saisi à la demande (`sslPasswordNeeded`) ou mémorisé (`rememberKeyPassword`, stocké dans l'INI, DPAPI sous Windows, clair ailleurs, AppSettings.cpp:386-391). Mot de passe de session conservé en mémoire (`m_sessionKeyPassword`).
- En-têtes personnalisés (CustomHeaders.cpp) : liste de paires, validation de nom RFC 7230 token, valeurs sans CR/LF, noms réservés (host, content-length, transfer-encoding, connection, range, x-api-key), sérialisation JSON dans l'INI en CLAIR (README.md avertit explicitement). Appliqués à toute requête Immich, y compris sonde LAN.
- Risque : mêmes en-têtes pour l'URL locale en HTTP clair : un secret de type Cloudflare Access fuit sur le LAN en clair (le README le signale).

---

## 3. UpdateManager

Fichier : UpdateManager.cpp (750 lignes). Désactivable à la compilation (`-DIMMICH_ENABLE_UPDATER=OFF` ajoute `IMMICH_UPDATER_DISABLED`, CMakeLists.txt:23, 253-255) : utilisé par snap et AUR.

### 3.1 Vérification

- `GET https://api.github.com/repos/hdmain/immich-desktop/releases/latest` (UpdateManager.cpp:179), en-tête `Accept: application/vnd.github+json`, User-Agent versionné. L'endpoint `/releases/latest` exclut les prereleases : le canal "rolling beta" n'est PAS atteignable par l'updater intégré (la release `beta` est `prerelease: true, make_latest: false`, package.yml:296-298). Il s'obtient seulement par téléchargement manuel. Aucun réglage de canal dans `UpdateSettings` (AppSettings.h:23-27 : autoCheck, skippedVersion, lastCheckUtc).
- Vérification automatique si dernier contrôle > 24 h (UpdateManager.cpp:151-159). Version "ignorée" mémorisée pour les contrôles silencieux (ligne 367).
- Comparaison : `QVersionNumber::fromString` après retrait du préfixe `v` (lignes 37-42). Un tag type `0.2.5-rc1` serait tronqué à 0.2.5.

### 3.2 Choix de l'artefact (par type d'installation)

`preferredInstallKind()` (UpdateManager.cpp:557-568) : Windows = toujours `WindowsExe` (même si installé par MSI) ; Linux : variable `SNAP` = Snap, variable `APPIMAGE` = AppImage, sinon DEB (même sur Fedora/Arch, qui ne peuvent pas installer un .deb : aucune détection du gestionnaire de paquets). Repli : EXE vers MSI, DEB vers AppImage. Appariement par nom : suffixe d'extension + contient "immich" + score d'architecture par tokens (x86_64/amd64/x64/win64, arm64/aarch64, i386..., rejet si jeton d'une autre architecture, UpdateManager.cpp:44-83).

Validations sur l'asset : nom conforme à `^[A-Za-z0-9][A-Za-z0-9._+-]{0,199}$`, URL https, hôte `github.com`/`*.github.com`/`githubusercontent.com`/`*.githubusercontent.com` (note : n'importe quel sous-domaine github.com ou githubusercontent.com est accepté, donc du contenu hébergé par n'importe quel utilisateur de GitHub Pages `*.github.io` non, mais `raw.githubusercontent.com` de n'importe quel dépôt oui), pas d'identifiants dans l'URL, taille annoncée 0 < n <= 1 Go (UpdateManager.cpp:535-552).

### 3.3 Téléchargement

Vers `{Temp}/immich-desktop-updates/<nom>` via `QSaveFile` (écriture atomique à la fin), `transferTimeout(0)` (la limite de 30 s tuait presque tous les téléchargements, commentaire ligne 259), redirections `NoLessSafeRedirectPolicy`. La taille reçue est plafonnée par la taille annoncée, et la taille finale doit être égale à `sizeBytes` (lignes 384-440). Pas de reprise (Range), pas de téléchargement différentiel.

### 3.4 Vérification d'intégrité et de signature

AUCUNE. Ni SHA-256 du fichier comparé à un manifeste signé, ni signature minisign/GPG/Authenticode vérifiée par l'app, ni `.sha256` publié par la CI. La seule garantie est TLS vers GitHub + contrôle de taille. Quiconque compromet le compte GitHub ou le workflow de release distribue du code exécuté, sur Windows avec élévation (UAC `-Verb RunAs`). Les binaires Windows ne sont pas signés Authenticode dans la CI (aucune étape de signature dans package.yml). Le seul contrôle d'intégrité signé du projet concerne les traductions (SHA-256 dans un manifeste servi par le même hôte, donc sans valeur d'authenticité, voir 6).

### 3.5 Installation par plateforme

- Windows EXE (NSIS) ou MSI : l'app écrit `%TEMP%/immich-desktop-apply-update.ps1` et le lance détaché avec `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` (UpdateManager.cpp:594-674). Le script attend la fin du PID (90 s max), tue tout processus nommé `immich`, lance l'installeur (`/S` silencieux en `RunAs` pour NSIS ; `msiexec /i … /passive /norestart` pour MSI), puis relance `immich.exe` trouvé dans `%ProgramFiles%\immich desktop\bin\`. Piège corrigé dans la dernière version : `cmd start /wait` revenait avant la fin de l'installeur élevé (commit 4f69f72). Défauts : le chemin du script est prédictible dans un `%TEMP%` utilisateur (un autre processus du même utilisateur peut le remplacer entre l'écriture et le démarrage élevé du `Start-Process -Verb RunAs`, mais il tourne déjà avec les droits utilisateur, le vrai risque est que l'installeur téléchargé soit remplaçable dans `immich-desktop-updates` pendant l'attente d'UAC) ; le premier Start-Process en cas d'échec rejoue sans `/S` ; aucune gestion du "installer par utilisateur sans élévation" ni du mode portable.
- Linux AppImage : copie le fichier téléchargé vers `$APPIMAGE` (le chemin de l'AppImage courant) via un script `/bin/sh` `cp -> chmod +x -> mv ->exec` (UpdateManager.cpp:687-717). Le téléchargement prend `chmod +x` à la fin. Les chemins sont insérés dans le script entre guillemets doubles SANS échappement (`$APPIMAGE` n'est pas contrôlé par l'app, un chemin avec `"` ou `$()` casse ou injecte, bien que ce soit la variable du lanceur AppImage). Pas de zsync/AppImageUpdate, mise à jour par remplacement complet, pas de rollback. Si `APPIMAGE` est vide (déb. lancé hors AppImage, repli DEB->AppImage) la cible est `~/Applications/<nom>`.
- Linux DEB : script `sh` qui lance `pkexec dpkg -i <pkg> || dpkg -i <pkg>` puis relance `/usr/bin/<exe>` (UpdateManager.cpp:719-745). Pas de résolution de dépendances (`apt install ./pkg.deb` serait préférable), échoue sur les distros non Debian.
- Snap : l'app n'installe rien, ouvre `snap refresh <nom>` ou `snap://` (UpdateManager.cpp:676-685), mais le build snap désactive l'updater (`-DIMMICH_ENABLE_UPDATER=OFF`, snapcraft.yaml) : ce code ne tourne jamais dans le snap officiel.
- Flatpak et AUR : updater désactivé ou non applicable.

### 3.6 Bilan UpdateManager

C'est un updater "maison" minimal : bon filtrage d'URL et de nom de fichier, mais pas de signature, pas de canal beta, pas de rollback, pas d'installation en place pour le mode portable, scripts temporaires non durcis. À remplacer par `tauri-plugin-updater` (signatures minisign obligatoires, manifeste `latest.json` multi-plateforme, support AppImage natif) si la contrainte AppImage est acceptable ; sinon refaire le même schéma en Rust avec vérification Ed25519 obligatoire.

---

## 4. Packaging et CI

### 4.1 Build système

CMake 3.21, C++20, Qt 6.2+ (Core Gui Multimedia MultimediaWidgets Network Svg Widgets LinguistTools). Options : `IMMICH_ENABLE_UPDATER`, `IMMICH_EXECUTABLE_NAME`, `IMMICH_BUILD_INFO`, `IMMICH_GITHUB_REPO` (CMakeLists.txt:20-26). Traductions .ts compilées en .qm et intégrées dans les ressources (CMakeLists.txt:178-215). CPack : NSIS et WiX v3 sous Windows, DEB sous Linux (CMakeLists.txt:313-346). Version dans `project(VERSION 0.2.4)` ; `changelog.json` sert de notes de release.

### 4.2 Workflows GitHub

- `package.yml` (déclencheurs : push sur main/master, tags `v*`, PR, dispatch ; `concurrency` annule les runs précédents d'un même ref) :
  - `windows` (windows-2022) : Qt 6.8.3 msvc2022_64 via `jurplel/install-qt-action`, installe NSIS et WiX par choco, `cmake` + `ctest` (offscreen), `cpack -G NSIS` et `-G WIX`, artefacts `.exe` et `.msi`. Pas de signature de code.
  - `linux` (matrice amd64 ubuntu-22.04 et arm64 ubuntu-24.04-arm) : paquets Qt6 système, build Ninja, ctest, `cpack -G DEB`, puis AppImage via `linuxdeploy` + `linuxdeploy-plugin-qt` téléchargés depuis le tag `continuous` (non épinglés : build non reproductible et risque chaîne d'approvisionnement). Nom `immich-desktop-{x86_64|aarch64}.AppImage`.
  - `snap` (amd64, arm64) : `snapcore/action-build@v1`, publication sur le Snap Store (stable, candidate, beta, edge) sur tag `v*`, et `beta,edge` à chaque push main.
  - `beta` : à chaque push main, supprime la release `beta` et son tag (`gh release delete beta --cleanup-tag`) puis la recrée en prerelease avec tous les artefacts et les notes de changelog.json. C'est un canal "rolling" manuel : sans l'updater.
  - `release` : sur tag `v*`, publie la release avec exe, msi, deb, AppImage, snap ; notes depuis `changelog.json` (échec si vide).
  - `aur` : workflow réutilisable (aur.yml) qui met à jour `pkgver/pkgrel` du PKGBUILD et pousse vers l'AUR via `KSXGitHub/github-actions-deploy-aur` (épinglé par SHA), avec build de test dans un conteneur Arch.
- `translations.yml` : sur modification des `.ts`, compile les .qm avec `tools/publish_translations.py` et pousse `index.json` + catalogues adressés par hash dans la branche orpheline `translations`.
- Tests (tests/, ~1067 lignes C++ + 2 tests Python) : TlsConfigTest (289 l, certificats générés), CustomHeadersTest, LocalizationTest/UiTest, TranslationUpdaterTest, ProfileTest (221 l), catalogues PL/ZH. AUCUN test de ImmichClient, OfflineStore, ThumbnailCache, FolderSyncService, UpdateManager, VideoStreamServer (celui-ci n'est compilé que pour les fonctions statiques de TlsConfigTest). Les parties les plus risquées ne sont pas testées.

### 4.3 Autres paquets

- Snap : core24, confinement strict, extension gnome, plugs audio-playback/home/network/network-bind/opengl/removable-media, lanceur `bin/immich-launch` qui réaffirme les chemins de plugins Qt et force GStreamer (snap/local/immich-launch) ; updater désactivé.
- Flatpak : manifeste `io.github.hdmain.immich-desktop.yml` (runtime KDE 6.11, accès xdg-pictures/videos/download/documents, notifications et StatusNotifier), source épinglée au tag v0.1.8 (retard sur la 0.2.4 : manifeste obsolète).
- AUR : PKGBUILD `immich-desktop` 0.2.4, build depuis les sources, updater désactivé, nom d'exécutable `immich-desktop`.

---

## 5. Autres modules du coeur

- `SingleInstance` (SingleInstance.cpp) : `QLocalServer` nommé `immich-desktop-` + SHA1(app, hostname, utilisateur)[:16], message `activate\n` pour remonter la fenêtre existante, nettoyage du socket périmé. Équivalent Rust : `single-instance` ou `tauri-plugin-single-instance`.
- `AutoStart` : Windows = clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` valeur `ImmichDesktop` ; Linux = `~/.config/autostart/immich-desktop.desktop` avec `Exec` ; sous snap/flatpak utilise le lanceur stable (AutoStart.cpp:32-50). L'app réécrit la commande à chaque démarrage pour suivre la mise à jour (main.cpp:84-86). Argument `--autostart` : démarre caché dans le tray si fermer-vers-tray.
- `Localization` : catalogues Qt `.qm` intégrés + téléchargés (`immich_<lang>.qm`), sélection avec repli région vers langue (Localization.cpp:41-90). Langues : en, de, fr, pl, zh_CN.
- `TranslationUpdater` : télécharge `index.json` (version 1, jusqu'à 64 catalogues, 2 Mo chacun, 16 Mo au total) depuis `raw.githubusercontent.com/<repo>/translations/`, compare SHA-256 et taille, vérifie que le `.qm` se charge, écrit atomiquement. Délai global de 5 s. L'app BLOQUE le démarrage dans un `QEventLoop` pendant ce temps avec une boîte de progression après 300 ms (main.cpp:65-82) : lenteur de démarrage jusqu'à 5 s hors ligne. La confiance repose sur TLS et l'hôte GitHub : le hash du manifeste n'authentifie rien puisqu'il vient du même endroit (juste de l'intégrité face à un cache CDN périmé).
- `ProcessMemory::trimProcessMemory` : `EmptyWorkingSet` (Windows) / `malloc_trim(0)` (Linux glibc) appelé quand la fenêtre est cachée dans le tray (MainWindow.cpp:445-447), avec `releaseMemoryCaches()`. Idée utile pour une appli de fond.
- `main.cpp` : Qt style Fusion, DPI PassThrough, backend multimédia GStreamer par défaut sous Linux, boucle `do/while` de profils, vérification de mise à jour programmée au premier lancement.

---

## 6. Liste "À garder" (idées à reproduire)

1. Timeline en deux temps : index de mois (compteurs) puis mois à la demande, parallélisme borné à 4, réservation de la hauteur de scroll à partir des compteurs. À améliorer avec une sync incrémentale.
2. Double URL LAN/WAN avec sonde `ping` rapide (1,5 s) et bascule automatique, avec mTLS réservé à l'URL distante. Très utile pour le profil de l'utilisateur (accès maison/hors maison). À garder, en ajoutant la détection par SSID ou réseau, et en rendant la fréquence adaptative.
3. Nommage des états de réseau : "en ligne" distinct de "endpoint actif", et `serverAtLeast(major, minor)` pour activer les fonctions par version Immich. Garder et étendre avec `GET /server/features` et un registre de capacités.
4. Mapper les permissions de clé API aux fonctions : un 403 donne un message précis "il faut la permission asset.update" et masque la fonction (ImmichClient.cpp:2884-2885, 1685-1688, 1439-1443). Documenter la liste de permissions minimale.
5. Proxy vidéo local avec jeton de session, validation d'UUID, suppression de la clé API et du TLS client sur les sauts de redirection CDN/S3.
6. Ordonnancement d'images par priorité (preview viewer > tuiles à l'écran > prefetch, "dernier demandé d'abord"), dédoublonnage des requêtes, délais de présence avant fetch (TileImageScheduler).
7. Normalisation des vignettes à une taille bornée avant cache, écriture atomique des fichiers de cache.
8. Séparation données/cache par profil (arbre `profiles/<id>`), identifiants de profil validés (`isSafeProfileId`, AppSettings.cpp:25-35), migration automatique de l'ancienne config.
9. En-têtes personnalisés : validation stricte du nom/valeur, liste de noms réservés, séparation entre hôte Immich et sauts CDN.
10. Validation systématique des identifiants (UUID) avant d'être interpolés dans une URL, de la clé de lien de partage par regex, des noms d'asset de mise à jour.
11. Lien de partage généré côté client avec l'URL publique distante et validation de la clé retournée.
12. Détection de fichier "en cours de copie" avant upload (stabilité de taille 1,5 s) : à garder, avec en plus une vérification de verrou.
13. Option "n'envoyer que sur le réseau local" pour la sync de dossiers (idée du profil NAS/maison).
14. Fichiers "locaux non encore envoyés" affichés dans la bibliothèque avec badge d'état : bon concept pour la sync bidirectionnelle (état local/serveur visible).
15. Libération de mémoire en arrière-plan (tray) : `malloc_trim`/`EmptyWorkingSet`.
16. Chargement de traductions mises à jour hors binaire avec vérification de hash, taille bornée, validité du catalogue.
17. Ré-écriture de la commande d'autostart à chaque lancement pour survivre aux changements d'emplacement.
18. Mise à jour : filtrage strict de l'URL (https + hôte), du nom, de la taille annoncée, plafond de 1 Go, écriture atomique du téléchargement.
19. Architecture de tests : fixtures de certificats pour mTLS, tests des validateurs d'en-têtes.

## 7. Liste "À éviter" (faiblesses, bugs, failles, pièges de perf)

Sécurité
1. Clé API stockée en CLAIR dans un fichier INI sur Linux (`protectSecret` ne fait rien hors Windows, AppSettings.cpp:106-110 et 134-136). Le mot de passe de clé mTLS mémorisé idem. Sur Windows : DPAPI (bien), mais les en-têtes personnalisés (qui peuvent contenir des secrets Cloudflare Access) restent en clair partout (AppSettings.cpp:376-378, README.md). À faire en Rust : trousseau système (crate `keyring` : Secret Service/KWallet, Windows Credential Manager) avec repli chiffré par une phrase de passe en mode portable, et séparer clairement le mode portable.
2. Mise à jour sans signature ni hash vérifié ; binaires Windows non signés ; scripts PowerShell/sh écrits dans `%TEMP%`/`/tmp` avec chemin fixe et `ExecutionPolicy Bypass` ; chemins insérés sans échappement dans un script `sh` (UpdateManager.cpp:702-710, 727-735) ; `Stop-Process -Force` sur tout processus nommé "immich" (y compris celui d'un autre utilisateur/instance).
3. Liste blanche d'hôtes d'update trop large (`*.github.com`, `*.githubusercontent.com`, UpdateManager.cpp:85-90).
4. `deviceAssetId` contient le chemin absolu local (fuite d'informations vers le serveur et logs) et `deviceId` est constant "immich-desktop" (collisions entre machines).
5. Jeton du proxy vidéo comparé sans temps constant et placé en query (visible en journaux) ; pas de limite de connexions.
6. Traductions téléchargées depuis un dépôt GitHub : un `.qm` n'exécute pas de code, mais un catalogue malveillant peut tromper l'utilisateur (hameçonnage dans l'UI). Intégrité ne vaut pas authenticité.
7. Les mêmes en-têtes personnalisés sont envoyés à l'URL locale même en HTTP clair.
8. Pas de pinning, pas de notion d'empreinte de certificat serveur ; CA personnalisée ajoutée au magasin global de la config (OK) mais sans option "faire confiance à ce certificat auto-signé" ciblé.

Fiabilité / exactitude
9. Pas de déduplication par hash côté client : pas de `bulk-upload-check`, pas de `x-immich-checksum` ; un fichier renommé ou déplacé est réuploadé en entier avant que le serveur dise "duplicate". Pour la reprise et la bande passante, le hash SHA-1 local + `POST /assets/bulk-upload-check` (200 éléments par appel) évite des téraoctets d'envoi redondant.
10. Fichiers rejetés (4xx) retentés toutes les 8 s indéfiniment (erreur non persistée) ; échec définitif d'un upload non récupérable ; pas de backoff exponentiel ni de limite globale.
11. Upload séquentiel unique, sans reprise intra-fichier, sans pause, sans plafond de débit ; un timeout de transfert de 30 s hérité peut classer un upload lent comme "transitoire" et le rejouer.
12. Aucune propagation de suppression ou renommage ; rien de bidirectionnel : la sync est strictement locale vers serveur.
13. Cache hors ligne : jamais purgé des assets supprimés ; champs essentiels absents (rating, taille, updatedAt, nom de fichier pour les mois de timeline) ; clé de cache = hash de l'URL serveur (pas de l'id utilisateur) donc deux comptes sur le même serveur dans un même profil partageraient un cache.
14. Pas d'invalidation de vignettes : clé = id d'asset seul (SHA1), sans `thumbhash`/`checksum`/`updatedAt` : une photo pivotée ou rééditée côté serveur garde sa vignette obsolète indéfiniment (cache vignettes illimité). Immich expose `thumbhash` et `checksum` dans ses réponses : à utiliser comme clé de version.
15. Pagination silencieusement tronquée : `/people` limitée à une page de 1000 (ou 100 pour Explorer) sans suite (ImmichClient.cpp:1160-1165, 1855-1858) ; `/albums`, `/tags` entiers d'un coup.
16. Curseurs de recherche stockés dans un map global à deux clés : deux vues concurrentes se piétinent (ImmichClient.h:~250, ImmichClient.cpp:1066, 1922).
17. Le sondage `pong` accepte `body.contains("pong")` sans vérifier le statut HTTP : une page captive ou un proxy qui renvoie "pong" dans le HTML passe pour Immich (ImmichClient.cpp:754-755).
18. Détection de version basée sur un unique appel au démarrage ; si le serveur est mis à jour, la version cachée n'est pas rafraîchie jusqu'à reconfiguration/offline-online.
19. `unknown network error` et `OperationCanceledError` traités comme coupure réseau : un abort volontaire bascule l'app hors ligne (ImmichClient.cpp:571, 578).
20. Le décodage d'image sur 4 threads utilise `QNetworkAccessManager` créé à CHAQUE vignette dans un thread worker avec `QEventLoop` local (ImmichClient.cpp:3119-3144) : aucune réutilisation de connexion HTTP/keep-alive, une poignée de main TLS (avec mTLS en plus) par vignette, pas de HTTP/2 ; aucun retry ; aucune annulation quand la tuile quitte l'écran (la requête en file part quand même).

Performance
21. Persistance en JSON complet réécrit à chaque changement (bibliothèque offline, file d'upload, état de sync de dossier, historique de vues) : O(n) à chaque écriture, O(n²) cumulé ; pas de SQLite ; à remplacer par SQLite (WAL) via `rusqlite`/`sqlx`.
22. `trimDiskLocked` liste et trie tout le dossier de previews à chaque écriture ; vignettes en dossier plat de centaines de milliers de fichiers ; éviction FIFO nommée LRU ; le cache vignettes n'a pas de plafond.
23. Scan complet récursif du dossier de sync toutes les 8 s, plus à chaque `publishLibraryPending` (appelé à chaque signal de progression d'upload), avec `QImageReader` par fichier non synchronisé : très coûteux sur de gros dossiers ou des disques réseau. Préférer `notify` (inotify/ReadDirectoryChangesW) avec re-scan complet rare, et un index SQLite des fichiers (chemin, taille, mtime, hash).
24. Réponse d'un mois entier de timeline parsée d'un bloc en mémoire (QJsonDocument), puis converties en liste de gros objets (ImmichAsset contient 4 QDateTime, 2 QString, etc.).
25. Démarrage bloqué jusqu'à 5 s par la mise à jour des traductions (main.cpp:65-82) : à faire en tâche de fond.
26. Pas d'arrêt propre des requêtes dans le pool d'images au changement de compte (le destructeur attend `waitForDone`).

Architecture
27. Un seul gros objet `ImmichClient` (3209 lignes) mélangeant HTTP, parsing, cache, pagination, état réseau, upload, prefetch ; signaux Qt partout ; difficile à tester (aucun test).
28. Multi-compte = redémarrage logique complet de l'application (code de sortie 42) ; un seul compte actif ; impossible de synchroniser plusieurs comptes en parallèle ; ne convient pas à l'objectif "multi-compte" d'un client moderne (comptes simultanés, un moteur de sync par compte).
29. Aucun mode portable (chemins `QStandardPaths`, QSettings UserScope), aucun mécanisme pour tout ranger à côté de l'exécutable.
30. Manifeste Flatpak figé à v0.1.8 ; AppImage construit avec `linuxdeploy` du tag `continuous` non épinglé ; le job Linux amd64 sur ubuntu-22.04 (glibc 2.35) limite la compatibilité vers le bas mais pas vers le haut, correct.
31. Le canal "beta" roulant n'est pas intégré à l'updater et supprime/recrée le tag à chaque push (les liens de téléchargement cassent entre deux builds).
32. Détection du type d'installation Linux trop simpliste (DEB par défaut même sur Fedora/Arch).

## 8. Recommandations pour la réécriture Rust (Tauri 2 + Svelte)

- Crate coeur sans dépendance UI : `immich-core` avec modules `api` (client reqwest typé, rustls, mTLS via `reqwest::Identity`), `net` (sonde LAN/WAN, état en ligne), `store` (SQLite : assets, buckets, albums, personnes, file d'upload, état de sync, avec migrations), `thumbs` (cache disque sharded par hash, clé = thumbhash/checksum, éviction LRU réelle sur taille totale suivie en base), `sync` (moteur par compte : upload avec hash local + bulk-upload-check, descente sélective, fichiers virtuels, détection de conflits), `video_proxy` (axum local), `updater`.
- API : s'appuyer sur la sync incrémentale de la version serveur ciblée (endpoints `/sync/*`) plutôt que de re-télécharger la timeline, avec repli sur `timeline/buckets` pour les serveurs plus anciens. Garder `server/features` + `serverAtLeast` comme registre de capacités.
- Upload : SHA-1 en flux, `x-immich-checksum`, `POST /assets/bulk-upload-check`, `deviceAssetId` = hash/identifiant stable sans chemin, `deviceId` unique par installation, pool d'envois parallèles borné, reprise avec backoff exponentiel et file persistante par fichier (état, tentatives, dernière erreur).
- Secrets : trousseau OS, en-têtes personnalisés secrets aussi dans le trousseau ; mode portable = chiffrement avec phrase de passe (ou refus de stocker).
- Mise à jour : manifeste signé (minisign/Ed25519) vérifié avant tout lancement, `tauri-plugin-updater` ou équivalent, canaux stable/beta dans les réglages (l'API `releases` permet de lister les prereleases, ici non utilisé), support AppImage (remplacement atomique à côté de `$APPIMAGE`, rollback .old), portable Windows (zip auto-extractible sans élévation), MSI/NSIS avec signature Authenticode.
- Tests : couvrir en priorité l'API (serveur factice avec `wiremock`), le moteur de sync (dossiers temporaires), le cache (éviction), l'updater (vérification de signature), ce que l'original ne teste pas.
