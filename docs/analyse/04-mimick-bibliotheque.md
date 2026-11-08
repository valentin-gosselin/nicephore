# Analyse du navigateur de bibliothèque de Mimick (src/library/**)

Source : /home/goss/.cache/immich-ref/mimick (GPL-3.0-or-later, lecture seule, rien n'a été compilé ni exécuté).
Les chemins ci-dessous sont relatifs à `src/library/` sauf mention contraire. Les captures de `docs/screenshots` n'ont pas été ouvertes (l'analyse porte sur le code, 53 600 lignes dans `src/library`).

Avertissement de licence : Mimick est GPL-3.0-or-later. Reprendre du code tel quel contaminerait la nouvelle app. Ce rapport sert à reprendre des idées et des algorithmes (non protégés en tant que tels), pas à copier le code, sauf si la nouvelle app est elle aussi GPL.

---

## 0. Constat structurant : deux sources d'images très différentes

Le fichier géant `texture_decode.rs` (1772 lignes) ne sert PAS à afficher la bibliothèque Immich distante. Il sert aux fichiers LOCAUX : sources `LocalAll`, `Unified`, `AlbumLocal` (state.rs:36-60), la vue "Staging" (fichiers ouverts via "Ouvrir avec" ou `--upload`), et le lightbox quand la photo existe déjà sur disque (rendering.rs:282-330). Mimick est d'abord un client de synchronisation de dossiers vers Immich, et la bibliothèque affiche aussi les dossiers surveillés avec leur état de sync.

Pour le contenu distant, le serveur Immich fournit déjà des JPEG/WebP prêts (thumbnail ~250 px, preview ~1440 px, fullsize optionnel) et le client n'a qu'à décoder des formats simples. La complexité de `texture_decode.rs` n'est donc pas nécessaire pour une app Immich pure, sauf si on veut un mode "dossier local".

---

## 1. Inventaire fonctionnel / UX (Mimick vs webapp Immich)

### 1.1 Navigation (sidebar, sidebar/rows.rs:7-80, raccourcis shortcuts_dialog.rs)
Groupes : Recherche ; Bibliothèque (Photos, Explorer, Carte, Personnes, Favoris, Archive, Corbeille) ; Albums et partage (Albums, Albums partagés, Liens partagés, Partage = partenaires) ; Parcourir (Tags, Dossiers) ; Utilitaires (Doublons). Pied de sidebar : état de connexion et stats serveur (server_stats_dialog.rs). Albums récents dans la sidebar (sidebar.rs:22-23, 3 récents). Ctrl+1..9 pour changer de vue, F9 sidebar, F5 refresh, Ctrl+K recherche, Alt+Gauche/Droite et bouton souris "retour" (nav_history.rs, navigation.rs). Responsive : breakpoints libadwaita, layout téléphone avec bottom_nav.rs, sheets de menus qui montent du bas.

### 1.2 Sources de contenu (enum `LibrarySource`, state.rs:20-106)
AllAssets (timeline), Explore (random), Album, SmartSearch (CLIP), MetadataSearch, OcrSearch, AdvancedSearch (filtres), Trash, Favorites, Archive, Person, Partner, Tag, Folder, Place, Thing, Memory, Similar ("voir photos similaires"), MapArea (IDs d'un cluster de carte). Plus les sources locales : LocalAll, LocalSearch, Unified, UnifiedSearch, AlbumLocal, AlbumUnified.
Tri : plus récent, plus ancien, nom de fichier, type (state.rs:162-167). Les tris nom/type sont faits côté client sur les pages déjà chargées (state.rs:170 `server_order` renvoie None).

### 1.3 Grille
- Mise en page justifiée "façon Google Photos / Immich web" dessinée sur un canvas custom (`MasonryCanvas`), pas de widgets par tuile.
- Timeline par mois avec en-têtes (modes Off / Mois / Jour, `DateHeadings`), scrubber à droite avec années/mois/poignée, sauts clavier jour/mois/année (`m`/`Shift+m`, `y`/`Shift+y`, timeline/nav.rs).
- Mode sélection (clic long/touche), sélection multiple, barre d'actions groupées (window_layout/bulk_bar.rs, actions.rs) : télécharger, corbeille, ajouter à un album, changer date/heure en lot (bulk_date.rs), archiver, favori, retirer de l'album, lien de partage.
- Bordures configurables (gap + couleur), qualité de grille (Auto/Thumbnail/Preview/Fullsize), "fit tiles" (décoder à la taille d'affichage).
- Fondu + dé-flou à l'arrivée de chaque vignette (masonry/reveal.rs:3-4 : 220 ms, flou max 10).
- Navigation clavier complète sur la grille (masonry/canvas/keyboard.rs, `Step` Left/Right/Up/Down/First/Last, layout.rs:~240), anneau de focus, menu contextuel au clavier (Shift+F10).
- Badges (vidéo, live, favori, pile, édité, état de sync).
- Drag and drop de fichiers pour importer (drop_target.rs, upload_picker.rs, staging_view.rs).
- Mémoires au-dessus de la timeline : cartes "il y a N ans" qui s'ouvrent en diaporama (memories.rs, rafraîchies toutes les 30 min).

### 1.4 Lightbox (lightbox.rs + lightbox/**)
Zoom (Ctrl +/-/0, molette, pinch), pan, plein écran, carrousel Prev/Current/Next avec swipe et ressort (lightbox/strip.rs), préchargement des voisins (interaction/neighbours.rs), volet de détails (EXIF, caméra, lieu, mini-carte, description, tags, personnes, albums, activité), rating, favori, archive, corbeille, téléchargement, "Ouvrir avec", Live Photo (clip qui boucle sur la photo, lightbox/live.rs), piles (strip des membres, lightbox/stack.rs), OCR (calque de texte sélectionnable, "Copier tout", lightbox/ocr/*), visages (cadres sur la photo, renommer/déplacer/retirer/dessiner un visage, lightbox/faces.rs), diaporama (options de transition, durée, aléatoire, contrôles auto-masqués, lightbox/slideshow*), éditeur (voir §4), changer l'emplacement (map/location_dialog.rs), changer date/description/favori (asset_edit.rs), bascule preview/original.
Vidéo : GStreamer `gst_play` (video_player.rs), vitesse, volume, saut 10 s (j/l), pistes audio/sous-titres (video_player/tracks.rs), en-tête `x-api-key` injecté dans le pipeline (video_player/pipeline.rs).

### 1.5 Pages
- Explorer (explore_view.rs:1-25) : Personnes (avatars ronds), Récemment ajoutées, Lieux (tuiles de villes), Choses (tags) ; calque sur l'onglet Explore du web.
- Personnes (people_view.rs) : grille d'avatars, recherche, afficher les masqués, renommer, fusionner (person_merge_dialog.rs), favori, date de naissance.
- Albums (albums_view.rs) : sections Récents / Possédés / Partagés, création, édition, partage (album_share_dialog.rs), liens (album_link.rs), activité/commentaires (album_activity/*), couverture, téléchargement (album_download.rs), suppression.
- Recherche (search_view.rs, search_filters.rs, search_panel.rs) : modes Smart (CLIP), Métadonnées, OCR ; panneau de filtres latéral (nom, description, OCR, type, favori, motion, hors album, supprimés, visibilité, note, dates prises/créées, marque/modèle/objectif, pays/état/ville, tags, personnes). Les résultats retombent dans la même grille.
- Carte (map/**, libshumate) : tuiles OSM raster ou styles vectoriels MapLibre fournis par le serveur Immich (map/style.rs convertit le style en rendu libshumate), clustering radial en pixels Web Mercator (map/cluster.rs:13 `CLUSTER_RADIUS_PX = 60`), recherche de lieux Photon puis lieux Immich en repli (map/place_search.rs), satellite, rotation/nord, "ma position".
- Dossiers (folders_view.rs, tree_page.rs), Tags (tags_view.rs, asset_tags.rs avec tags imbriqués Parent/Enfant), Doublons (duplicates_view.rs, un groupe à la fois, garder/trasher), Corbeille (restaurer, supprimer définitivement), Archive, Partage (partenaires, sharing_view.rs), Liens partagés (shared_links_view.rs, shared_link_dialog.rs).

### 1.6 Hors périmètre Immich web mais présent dans Mimick
Sources locales et état de sync par asset, liaison dossier local et album avec diff bidirectionnel (album_sync.rs, album_sync/diff.rs), RAW/HEIC/JXL/PSD/SVG locaux, ouverture "avec l'app par défaut", glisser un asset vers un autre programme (`drag_export_path`, lightbox/details.rs:79).

### 1.7 Écarts par rapport à l'Immich web (à tester, non vérifiés ligne à ligne)
Absent ou non vu dans le périmètre : dossier verrouillé (locked folder), vue "Utilisateurs/Admin", gestion des API keys, upload en navigateur avec file de progression type web, partage d'albums avec rôles fins (présent partiellement), "Utilitaires" (gestion de stockage, vérification de checksums). Les Mémoires sont présentes mais sans la vue "plein écran type stories" de l'app mobile. À l'inverse l'édition locale avancée (courbes de tonalité, rédaction/floutage) dépasse l'éditeur web d'Immich, qui ne fait que crop/rotation/miroir.

---

## 2. Techniques de performance

### 2.1 Chargement de la timeline (timeline/mod.rs, api_client/timeline.rs)
- **Time buckets** : `GET /api/timeline/buckets?visibility=timeline&order=desc&withStacked=true&withPartners=true` renvoie `[{timeBucket, count}]` par mois (api_client/timeline.rs:56-76, `TimeBucket`). Un seul appel donne la structure complète de la timeline : N mois avec leur nombre de photos.
- Mimick construit un `TimelineIndex` (timeline/index.rs) : `months: Vec<Month{key,count}>` et un état par mois `Unloaded | Loading | Loaded(len)`. `offset(month)` = somme des longueurs chargées avant ce mois, `month_at(position)` pour l'inverse.
- **Chargement d'un mois à la demande** : `load_month` (timeline/mod.rs:~290) n'est déclenché que pour les mois dont la section intersecte la bande visible (`request_visible_months`, masonry/canvas/timeline.rs:~272-300), jamais pendant un fling (`flinging`) ni si une requête est déjà en attente (`month_request_pending`). Les requêtes sont différées en `idle_add_local_once` pour ne pas travailler dans le snapshot.
- **Détail d'un mois** : étonnamment, Mimick n'utilise PAS la réponse colonnaire de `/api/timeline/bucket` pour les assets. Il fait `POST /search/metadata` avec `takenAfter/takenBefore` élargis de 14 h (`ZONE_SLACK_HOURS`, timeline.rs:13-30) par pages de 1000 (`MONTH_PAGE_SIZE`, timeline.rs:11, jusqu'à 200 pages), puis un second appel `GET /api/timeline/bucket?timeBucket=...` uniquement pour récupérer les ids et les couvertures de piles (`BucketIds`, `apply_stacks`, timeline.rs:~33-60), puis filtre par mois local (`keep_local_month`). C'est lourd et fragile (décalage de fuseau). Pour la nouvelle app : utiliser directement la réponse colonnaire de `/timeline/bucket` (id, ratio, fileCreatedAt, isFavorite, isImage, duration, livePhotoVideoId, stack, thumbhash, localOffsetHours...), c'est ce que fait le web et ça évite l'aller-retour.
- Repli : si `fetch_time_buckets` échoue, chargement page par page de 50 (`PAGE_SIZE = 50`, mod.rs:121 et state.rs:8) via `load_pages`.
- Les autres sources (album, recherche, personne, tag...) utilisent une pagination classique de 50 éléments avec défilement infini (pas de buckets), `MAP_AREA_PAGE_SIZE = 32` pour les clusters de carte (mod.rs:123).
- Rechargement : `reload` conserve l'ancre (clé de mois + offset dans le mois, timeline/mod.rs:~205-225) et `anchored_scroll` (timeline/layout.rs:195) recale le scroll quand la hauteur d'un mois change (estimation devient mesure).

### 2.2 Algorithme de mise en page (masonry/layout.rs)
- **Rangées justifiées gloutonnes** (`pack_rows`, layout.rs:82) : on remplit une rangée avec des vignettes à hauteur max (`aspect * max_row_height`) tant que la somme tient dans la largeur (`collect_row_indices`, :123), puis `scale_to_fit` réduit la hauteur pour justifier exactement la largeur. Si la hauteur obtenue passe sous `min_row_height`, la dernière vignette est renvoyée à la rangée suivante (`build_row`, :146). La dernière rangée est clampée entre min et max (pas d'étirement).
- Trois configs selon la largeur (layout.rs:10-15) : compact < 400 px (48-80), narrow (60-120), wide (180-360). Gap configurable.
- Ratio inconnu : repli 4:3 (`FALLBACK_W/H`, layout.rs:5-6). Les dimensions viennent des champs `width/height` des assets.
- **Recherche de rangée en O(log n)** : `first_row_at_or_after` (layout.rs:203), `row_at_y`, par bisection sur `y`. `item_at_x` est linéaire dans la rangée (petite).
- **Timeline par sections** (timeline/layout.rs) : chaque mois (ou une série de mois sans en-tête, `pack_run`) est packé indépendamment en `PackedSection` ; `compose` (layout.rs:~120) concatène les sections packées et des sections **estimées** pour les mois non chargés. L'estimation (`estimated_height`, :172) suppose un ratio 1,5 (`ESTIMATED_ASPECT`) et 8 photos/jour pour les en-têtes. La hauteur totale est donc connue dès les buckets, ce qui rend la barre de défilement et le scrubber cohérents avant tout chargement.
- **Cache de sections packées** (`section_cache` par mois, `CachedSection{key,months,packed}`, masonry/canvas/timeline.rs:~140-185) : invalidé seulement si la largeur/config (`SectionKey`) change ou si la série de mois chargés change. Un redimensionnement de fenêtre repack tout, mais un chargement de mois ne repack que lui.
- `schedule_relayout` (timeline.rs:~328) coalesce les relayouts en un seul `idle_add_local_full(HIGH_IDLE)` avant la frame suivante.

### 2.3 Virtualisation
- Le "canvas" est un seul widget GTK qui dessine dans `snapshot()` uniquement les rangées dans la fenêtre visible : `rows_between(rows, top, bottom)` (canvas_helpers.rs:90) puis `append_texture` ou `append_color` (placeholder) par tuile (canvas_paint.rs:~95-175). Pas de widget par photo : 50 000 photos = 50 000 `LaidItem` légers (index u32 + x + w) et un modèle `LibraryAssetModel` (GListModel d'objets GObject `AssetObject`).
- Bande de prélecture : 0,5 viewport derrière et 1 viewport devant (`VIEWPORTS_BEHIND/AHEAD`, canvas.rs:22-23, canvas_paint.rs:65-66) pour décider quoi charger, alors que seul le viewport exact est peint.
- Un mois chargé = tous ses éléments insérés dans le modèle à la position `offset(month)` (insert_month), les mois non chargés n'ont aucun objet, seulement des rectangles gris estimés (`paint_estimated_sections`, timeline.rs:~190-240, placeholders par rangée estimée).
- Limite notable : le modèle ne décharge jamais un mois (pas d'éviction de métadonnées). Pour 50 000 photos c'est de l'ordre de dizaines de Mo, acceptable, mais à surveiller à 500 000.

### 2.4 Ordonnanceur de chargement (masonry/schedule.rs)
- `LoadScheduler` : au plus 16 chargements en vol (`MAX_LOADS`, schedule.rs:8). À chaque snapshot, `plan(wanted)` : (1) marque annulés (drapeau `Rc<Cell<bool>>`) tous les chargements en vol dont l'asset n'est plus voulu, (2) retire des demandes celles déjà en vol, (3) trie par priorité = distance du centre de la tuile au centre du viewport (canvas.rs:321), (4) tronque au budget restant. C'est le coeur de la fluidité : ce qui est au milieu de l'écran passe en premier, ce qui sort de l'écran est annulé, y compris en pleine file d'attente de sémaphore (`check_cancelled` avant et après `acquire`, thumbnail_cache.rs:~389-395).
- **Détection de fling** (`is_fling`, schedule.rs:76) : vitesse > 3 viewports/s (`FLING_VIEWPORTS_PER_SEC`) => aucune demande de vignette ni de mois (`wanted_loads` renvoie vide, canvas_paint.rs:92 ; `request_visible_months` idem). Un timer de 150 ms (`SETTLE_DELAY`) remet `flinging` à faux quand le scroll s'arrête. Les tuiles non chargées restent en gris pendant le fling puis se remplissent d'un coup au repos. Sans ça, un glisser du scrubber à travers 10 ans déclencherait des milliers de requêtes inutiles.
- `failed` : set d'ids en échec pour ne pas retenter en boucle (canvas.rs:~335). Message unique sur permissions API manquantes (`asset.view`, canvas_helpers.rs:127).

### 2.5 Sélection du bucket de qualité et "fit tiles" (masonry/quality.rs)
- `GridQuality` Auto/Thumbnail/Preview/Fullsize ; en Auto, hauteur de rangée <= 600 px => `thumbnail`, sinon `preview` (quality.rs:9). Défaut `Thumbnail`.
- Chaîne de repli sur 404 (`fallback_bucket`, quality.rs:68 ; `load_with_fallback`, load.rs:~30) : fullsize -> preview -> thumbnail, car le fullsize n'existe que si le job serveur est activé.
- **Fit tiles** (config `library_grid_fit_tiles`, true par défaut, config/schema.rs:78/176) : pour les buckets preview/fullsize, le décodage se fait à la taille réellement affichée arrondie à un palier `[320, 480, 640, 960, 1280, 1920]` (quality.rs:46, `fit_edge`). Les octets réseau et disque restent ceux du serveur, seule la texture en RAM est réduite. `fit_candidates` cherche en cache le palier voisin avant de redécoder.

### 2.6 Cache de vignettes (thumbnail_cache.rs)
- **Deux niveaux** : RAM (LRU borné en octets) + disque (octets bruts serveur, un fichier par asset/taille/édité).
- **RAM** : `SizedLruCache` (thumbnail_cache.rs:~73-200) sur `lru::LruCache` non borné + compteur d'octets ; taille estimée = largeur x hauteur x 4 (`estimate_texture_bytes`, :694). Éviction `pop_lru` jusqu'à repasser sous le budget. Statistiques hits/misses/evictions loguées. Budget : `cache_memory_cap_mb` défaut **2048 Mo** (config/schema.rs:100-102, :231), plafonné à **la moitié de la RAM** lue dans `/proc/meminfo` (`memory_budget`, :699, Linux seulement). Vidé à la fermeture de la fenêtre bibliothèque (`clear_memory`, :599) pour que l'app en tray ne garde pas de RAM.
- **Disque** : `<cache_dir>/thumbnails/<bucket>[-edited]:<asset_id>` (memory_key/cache_key, :635-650), écriture atomique (`util::atomic_write`, :681). Plafond global `cache_disk_cap_mb` défaut **2000 Mo** (schema.rs:98, :227). Élagage **une seule fois au démarrage**, par mtime croissant (pseudo-LRU, jamais mis à jour à la lecture donc en réalité FIFO) sur tous les sous-dossiers (cache_manager.rs:60-86). Pas de vérification d'intégrité ni d'invalidation par etag/checksum : seul `forget(asset_id)` (:572) supprime après une édition.
- **Déduplication des requêtes en vol** : table `inflight: HashMap<key, watch::Receiver>` ; le premier demandeur charge, les suivants attendent via `tokio::sync::watch` (:36-70, `enter_inflight`, :619). `InflightGuard` nettoie à la destruction.
- **Concurrence** : deux sémaphores Tokio : petites (thumbnail) `min(16, 2xCPU)` min 2, grandes (preview/fullsize) `min(6, CPU)` min 2 (:29-31, :219-247). Lecture disque et décodage dans `spawn_blocking`.
- Ordre de résolution (:362-428) : RAM -> inflight -> sémaphore -> re-test RAM -> disque -> réseau (écrit le disque puis décode).
- Clés RAM avec palier de fit : `bucket@edge:id` (:639-650), et `screen@edge:id` pour les images plein écran du lightbox (:668).
- Vignettes locales : décodées en 256 px, ré-encodées PNG et stockées (`local-thumbnail-v2:<hash>`, :674, :535-560). La clé locale est le hash du seul id/chemin, donc pas d'invalidation si le fichier change (point faible).
- Taille cibles de décodage : thumbnail 256, preview 1440, fullsize 2560 (thumbnail_cache/decode.rs:191-197).
- Décodage des octets serveur : `Pixbuf::from_stream_at_scale` (gdk-pixbuf) dans `spawn_blocking` (decode.rs:199-225) puis `MemoryTexture` (RGB sans alpha quand possible, 3 octets/pixel au lieu de 4 pour le transfert).

### 2.7 Pipeline de décodage des fichiers locaux (texture_decode.rs et texture_decode/codecs.rs)
Pourquoi 1772 lignes :
1. **Routage par extension** (`texture_decoder_for_path`, :185-200) vers 10 décodeurs : Raw, Heif (heic/heif/hif/avif), JpegXl, Svg, Jpeg, Webp, Jpeg2k, Psd, Pixbuf (bmp/gif/png/tif), ImageFallback (crate `image`), avec **chaîne de repli** si le décodeur primaire échoue (`decode_with_fallbacks`, :113-130) et journalisation de la route gagnante.
2. **JPEG** : libjpeg-turbo via `turbojpeg` avec **décodage réduit** (DCT scaling 1/2, 1/4, 1/8) choisi pour que le côté long reste >= cible (`jpeg_scale_for`, :1293 ; `decode_jpeg`, :1221). C'est ce qui permet d'ouvrir des JPEG de 200 Mpx vite (wiki/Performance-Tuning.md). L'orientation EXIF est lue à la main (parseur TIFF/JPEG maison, :549-646) et appliquée.
3. **RAW** : libraw (`rsraw-sys`) ; stratégie "d'abord le JPEG embarqué" (`extract_libraw_thumb`, :753 ; plus gros JPEG embarqué choisi avec un scanner de marqueurs JPEG fait main, :348-545, vérifie que le JPEG est complet et sans perte de rotation, `is_thumbnail_prerotated`, :668) ; sinon demosaïquage demi-taille ; démosaïquage complet optionnel (`RAW_FULL_DECODE`). Cache disque optionnel du RAW décodé en PNG pleine résolution (`RAW_CACHE_ENABLED`, clé blake3 de chemin+mtime+taille, :261-336), désactivé par défaut car lourd.
4. **HEIC/AVIF** : libheif-rs ; **JXL** : jxl-oxide (pur Rust) ; **SVG** : resvg ; **JPEG2000** : jpeg2k ; **PSD** : crate psd ; **WebP** : crate webp.
5. **Vidéo** : miniature par `ffmpeg` externe (seek 1 s puis 0 s, fichier PNG temporaire, thumbnail_cache/decode.rs:~35-60).
6. Conversions de formats de pixels (bitmap thumbs libraw 8/16 bits, normalisation JXL), orientation, redimensionnement (`fit_pixbuf`, `fit_texture`), PNG atomique.
Tout tourne dans `spawn_blocking` (thread pool Tokio), jamais sur le thread GTK.

### 2.8 Prélecture, lightbox, mémoire plein écran
- Voisins : prev et next remplis et préchargés pour que le swipe soit instantané (lightbox/interaction/neighbours.rs).
- Image du lightbox : source par ordre (rendering.rs:282-330) poster vidéo / fichier local (`load_screen` décodé à la taille de l'écran, `screen_edge`, :171-185 = côté long du moniteur x échelle) / original téléchargé et mis en cache (`original_preview_cache_path`) / preview serveur 1440 px. Option "Open Originals in Lightbox" (défaut off).
- **Affinage au zoom** (`sharpen_for_zoom`, :202-237) : le décodage plein écran est rapide, la pleine résolution n'est chargée que si le zoom dépasse la texture affichée ; au changement de photo, `drop_full_resolution` (:471) repasse à la copie écran pour libérer la RAM.
- Spinner de chargement retardé (`arm_loader_spinner`, 250 ms) : pas de clignotement sur les chargements rapides.

### 2.9 Scrubber (timeline/scrubber.rs, timeline/marks.rs)
Widget custom 64 px de large à droite. `marks()` (marks.rs:~15-45) projette le haut de chaque section sur la piste en proportion de la hauteur totale (donc mois estimés inclus), affiche un label d'année à chaque changement d'année et des points pour les mois, **épure** les marques trop proches (`MIN_GAP` 14 px, une marque d'année remplace un point voisin). Poignée 28x44 qui suit le scroll (`handle_top`, marks.rs:~55), visible pendant le scroll puis masquée après 1,5 s avec fondu 300 ms (scrubber.rs:~20-25), pastille de date au survol/drag. Le drag convertit `track_to_canvas` en position de scroll. Combiné avec la détection de fling, un drag rapide ne déclenche aucun chargement avant le relâchement.

---

## 3. Transposition vers Tauri 2 + Svelte et ce qui reste côté Rust

### 3.1 Se transpose bien au web (frontend)
| Technique Mimick | Équivalent webview |
|---|---|
| Buckets de mois + état Unloaded/Loading/Loaded | Identique : un store Svelte `months[]` avec états, requêtes à la demande |
| Hauteur estimée des sections non chargées | Identique : conteneur de hauteur calculée, `position:absolute` ou `transform` par section ; les placeholders peuvent être du CSS (pas de DOM par tuile) |
| Pack de rangées justifiées glouton | Algorithme pur, portable tel quel en TS (ou en Rust compilé WASM, mais inutile) ; à exécuter dans un Web Worker pour 50 000 éléments |
| Bisection O(log n) des rangées visibles | Identique : ne rendre que les rangées intersectant le viewport +/- marge ; aussi bien en DOM virtualisé qu'en Canvas 2D |
| Cache de sections packées par mois | Mémoïsation par (mois, largeur, config) |
| Détection de fling + annulation hors écran | `AbortController` par image ; `IntersectionObserver` ou calcul explicite sur scroll ; vitesse = deltaY/dt |
| Ordonnanceur 16 en vol, priorité au centre | Inutile côté `<img>` seule (le navigateur gère), mais utile pour limiter la file : ne poser `src` que pour la bande visible et poser `loading` ; ou piloter via `fetch` + file de priorité |
| Fondu + dé-flou à l'arrivée | CSS transition (opacité + `filter: blur`) ; mieux encore : ThumbHash/BlurHash comme placeholder (le serveur Immich fournit `thumbhash` dans les buckets) |
| Scrubber avec marques épurées | Composant Svelte pur ; mêmes calculs (proportion de hauteur, épuration par `MIN_GAP`) |
| Navigation clavier de grille (`Step`) | Portable (le `locate` par asset_index est O(rangées), à indexer) |
| Ancrage de scroll au rechargement (clé de mois + offset) | Identique ; en web on peut aussi utiliser `overflow-anchor` en complément |
| Chaîne de repli de qualité | Identique |
| Palier de fit `[320..1920]` | Remplacé par `srcset`/dimensions CSS : demander au cache Rust la bonne taille |

### 3.2 Choix de rendu pour 50 000+ photos
Mimick dessine tout dans un canvas, parce que GTK n'a pas de DOM léger. En webview, deux voies : (a) DOM virtualisé avec `<img>` positionnés (recyclage de ~200 noeuds), solide, accessible, sélection/clic natifs ; (b) `<canvas>` 2D comme Mimick, plus de contrôle (fondus, sélection), mais perte d'accessibilité et de menus natifs. Recommandation : (a) avec recyclage de noeuds et un seul layer par rangée, en réservant le canvas pour des cas extrêmes. Les placeholders des mois non chargés en CSS (dégradé répété) coûtent peu.

### 3.3 À garder côté Rust (cœur Tauri)
1. **Client API Immich** avec `x-api-key`/jeton : une webview ne peut pas poser d'en-tête sur `<img src>`, et on évite CORS ; tout passe par le cœur.
2. **Cache de vignettes disque** servi par un **protocole personnalisé** (`register_asynchronous_uri_scheme_protocol`, par exemple `thumb://<id>/<size>`) : le webview fait un `<img src="thumb://...">`, Rust répond depuis le disque (mmap/sendfile) ou va chercher au serveur, écrit atomiquement, déduplique les requêtes en vol, applique les en-têtes `Cache-Control: immutable`. Cela remplace avantageusement le cache RAM de textures : le cache mémoire de décodage est géré par le navigateur, et le cache HTTP du webview joue le rôle du LRU RAM (taille non contrôlable finement, donc garder un petit LRU Rust d'octets bruts pour les plus récents si besoin).
3. **Déduplication en vol** (map clé -> futur partagé) et **sémaphores** de concurrence réseau/disque (reprendre la logique du `ThumbnailCache` : petites vs grandes, `2xCPU`/`CPU`). Annulation : le protocole reçoit l'abandon de la requête quand l'`<img>` est retiré (à vérifier selon la plateforme), sinon prévoir une commande `cancel`.
4. **Élagage du cache disque** : améliorer Mimick : vrai LRU (toucher le mtime/atime à la lecture ou index SQLite avec `last_access` et taille), élagage en tâche de fond périodique et pas seulement au démarrage, plafond configurable, métadonnées (etag, checksum) pour l'invalidation après édition.
5. **Index local** (SQLite) des buckets et métadonnées de timeline pour ouvrir l'app instantanément avant le réseau (Mimick n'a pas ça, il refetch les buckets à chaque ouverture) : stockage `id, ratio, date, thumbhash`, avec synchro incrémentale. Gros gain d'UX, aussi pour la recherche locale rapide.
6. **Génération de miniatures et décodage des formats exotiques** pour une éventuelle bibliothèque locale (si cette fonction est voulue) : JPEG réduit (turbojpeg ou `zune-jpeg`/`fast_image_resize`), HEIC (libheif-rs), RAW (libraw/`rawler`, JPEG embarqué d'abord), JXL (jxl-oxide), vidéo (ffmpeg externe ou `ffmpeg-next`). C'est exactement ce que fait `texture_decode.rs` ; le webview ne sait pas afficher HEIC (sauf Safari/WebKit récent), RAW, JXL (sauf Safari).
7. **Téléchargement de l'original** vers un cache puis ouverture/zoom : le protocole `original://id` ou un fichier temporaire.
8. **Vidéo** : ne pas copier GStreamer. Le webview a `<video>` ; Immich fournit un transcodage `/api/assets/{id}/video/playback` (pas de souci de codecs en général). Prévoir un fallback "ouvrir dans le lecteur système" pour les formats non lisibles. Les en-têtes d'auth passent par un protocole personnalisé (support des Range requests indispensable pour le seek).
9. **Carte** : MapLibre GL JS en frontend (le serveur Immich fournit le style via `/api/server/config` -> `mapLightStyleUrl`/`mapDarkStyleUrl`) ; le clustering radial pixel de Mimick (`cluster.rs`) est inutile, MapLibre fait du clustering GeoJSON natif. Mimick a dû convertir le style pour libshumate (style.rs), ce que MapLibre GL JS fait nativement.
10. **Rendu de l'édition** (voir §4) : le pipeline pixel peut vivre côté Rust si l'on veut une copie plein résolution cohérente avec l'aperçu.

### 3.4 Non transposable ou inutile
Canvas GTK snapshot, `GListModel`, widgets libadwaita, sémaphore par thread GTK (`Rc<Cell>`), `/proc/meminfo` (non portable : Windows/macOS, et le webview ne permet pas de borner finement sa mémoire), GStreamer, libshumate, Pixbuf.

---

## 4. Édition de photo : local ou via l'API

Deux chemins, décidés par `EditSession::needs_copy` (lightbox/edit/saving.rs:~22-38, photo_editor.rs:290).

### 4.1 Édité côté serveur Immich (non destructif, API d'édition d'asset)
- Recadrage, rotation (90/180/270), miroir H/V. Format : `{"action":"crop|rotate|mirror","parameters":{...}}` (api_client/edits.rs:6-30). Immich applique le crop d'abord (en pixels de l'original) puis miroirs et rotation dans l'ordre de la liste.
- `GET /api/assets/{id}/edits` (fetch_edits, edits.rs:50), `PUT /api/assets/{id}/edits` avec `{"edits":[...]}` (save_edits, :63), revert (`revert_edits`, :~100, DELETE du jeu d'éditions). Rendu asynchrone par le serveur : le client **sonde** toutes les 1,5 s, 40 fois max (`RENDER_POLL`, `RENDER_POLLS`, saving.rs:18-19, `wait_for_render` :~275-295) en comparant le `thumbhash` jusqu'à ce qu'il change, puis `forget(id)` du cache de vignettes et redessin (saving.rs `show_rendered_edit`).
- Lecture du rendu édité : `?edited=true` sur thumbnail et original (api_client/library.rs:83-97) ; clés de cache avec suffixe `-edited`.
- L'aperçu de crop en édition est dessiné sur la preview locale (`saved_crop`, `build_edits` : conversion entre pixels aperçu et pixels de l'original, photo_editor.rs:~419-460 ; tests en :469-500).

### 4.2 Édité localement puis envoyé comme NOUVELLE photo
Tout le reste : redressement (straighten, jusqu'à 45 deg, `MAX_STRAIGHTEN`), 10 curseurs Adjust (exposition, luminosité, contraste, hautes lumières, ombres, saturation, chaleur, teinte, vignettage, netteté : photo_editor/look.rs:8-37), 10 filtres avec intensité (look.rs:129+, `Recipe` combinée), rédaction (pixelisation, flou, "givré" = flou + grain, barre pleine ; photo_editor/redaction.rs:7-21).
- Aperçu temps réel : pipeline pixel en Rust (photo_editor/pixels.rs + pixels/tone.rs, parallélisé avec `rayon`) sur la preview ; vignettes de filtres rendues à 96 px (`FILTER_PREVIEW_EDGE`, photo_editor.rs:~28).
- Sauvegarde : l'**original est décodé en pleine taille** (via `texture_decode`), développé avec les mêmes `Steps` mis à l'échelle (`CopyJob::run`, photo_editor/copy.rs:~45-100), encodé en JPEG qualité 92 (`JPEG_QUALITY`, copy.rs:12) ou PNG, avec EXIF/XMP/ICC ré-injectés à la main (`embed.rs` : segments JPEG, XMP dans PNG, `copy_exif.rs`, `carried.rs`), puis `POST /api/assets` multipart (`upload_copy`, edits.rs:~70-95, `fileCreatedAt` de l'original pour rester voisin dans la timeline). La copie est **empilée** (stack) avec l'original et ajoutée aux mêmes albums (saving.rs en-tête, `match_record`). Option "garder les métadonnées" ou nouvelle photo vierge.
- Autres métadonnées (non pixel) via l'API : description, date/heure avec fuseau (asset_edit.rs, "n'envoie que les champs modifiés", asset_edit.rs:3-4 ; `DateChange`, zones IANA, time_zones.rs), favori, rating, tags, emplacement (PUT lat/lon), visages (création/déplacement/suppression), changement de date en lot (bulk_date.rs). Rien n'est écrit dans le fichier original.

### 4.3 Pistes pour la nouvelle app
- Aperçu des curseurs : en webview, WebGL/Canvas 2D (shaders) est plus fluide que de renvoyer des images depuis Rust à chaque mouvement ; garder le pipeline Rust seulement pour l'export plein résolution, avec une spécification commune des paramètres (même `Recipe`) pour que l'aperçu et l'export concordent. Alternative simple : tout en Rust avec `rayon` sur une preview 1440 px et transfert via protocole personnalisé (latence à mesurer).
- Garder la distinction nette : "éditions d'asset Immich" (crop/rotate/mirror) vs "copie retouchée" (nouvel asset empilé), et expliquer clairement à l'utilisateur laquelle est créée.

---

## 5. Garder

1. L'API `timeline/buckets` + chargement de mois à la demande avec états `Unloaded/Loading/Loaded` et hauteurs estimées pour les mois non chargés (timeline/index.rs, timeline/layout.rs:172).
2. Rangées justifiées avec 3 plages min/max de hauteur selon la largeur, dernière rangée clampée, et bisection O(log n) (masonry/layout.rs:82-203).
3. Cache de sections packées par mois, relayout coalescé, ancrage de scroll (section + offset) lors des recalculs de hauteur (timeline/layout.rs:195, timeline/mod.rs:~205).
4. Ordonnanceur à priorité par distance au centre avec annulation de tout ce qui sort de la bande (masonry/schedule.rs:35-60).
5. Détection de fling : aucune requête pendant un défilement rapide ni pendant un drag du scrubber (schedule.rs:76 ; canvas_paint.rs:92).
6. Déduplication des chargements en vol et sémaphores séparés petites/grandes images (thumbnail_cache.rs:36-70, :219-247).
7. Chaîne de repli de qualité sur 404 (fullsize -> preview -> thumbnail) car le fullsize est optionnel côté serveur (quality.rs:68).
8. Plafond RAM à la moitié de la mémoire système et libération à la fermeture de la fenêtre (thumbnail_cache.rs:699, :599).
9. Lightbox : décoder à la taille de l'écran d'abord, affiner seulement au zoom, relâcher au changement de photo, spinner retardé de 250 ms, préchargement des voisins (rendering.rs:171-237, :471).
10. Scrubber : marques proportionnelles à la hauteur totale, épuration par distance minimale, année prioritaire sur mois, poignée qui s'efface (timeline/marks.rs).
11. Navigation clavier complète et raccourcis (Ctrl+1..9, j/l pour vidéo, i/f/a/e/s dans le lightbox, m/y pour sauter mois/années), historique avant/arrière avec le bouton souris.
12. Modèle d'édition à deux voies (éditions Immich vs copie empilée) et "n'envoyer que les champs modifiés" pour les métadonnées (asset_edit.rs:3-4).
13. Sources paramétrées par un enum (`LibrarySource`) qui sert à la fois de navigation, d'historique et de requête (state.rs:20-106) : un seul rendu de grille réutilisé pour tout (recherche, albums, personnes, lieux, carte, mémoires).
14. Fondu + dé-flou 220 ms à l'arrivée des vignettes (reveal.rs) ; à remplacer par un placeholder ThumbHash.
15. Extraction du JPEG embarqué des RAW avant tout démosaïquage (texture_decode.rs:753, :249-260) si un mode local est prévu.

## 6. Éviter

1. **Chargement d'un mois via `search/metadata` par pages de 1000 puis second appel `timeline/bucket` pour les piles** (api_client/timeline.rs:103-130) : deux requêtes, objets complets, hack de fuseau +/-14 h (`ZONE_SLACK_HOURS`) et refiltrage client. Utiliser la réponse colonnaire du bucket (id, ratio, thumbhash, dates locales, pile, live).
2. **Dimensions issues de `width/height` avec repli 4:3** (layout.rs:5-6) : provoque des sauts de mise en page quand le vrai ratio arrive. Utiliser le `ratio` du bucket dès le départ.
3. **Aucune persistance de la timeline** : buckets refetchés à chaque ouverture. Prévoir un index SQLite local.
4. **Élagage disque une seule fois au démarrage, tri par mtime d'écriture** (cache_manager.rs:60-86) : ce n'est ni un LRU ni dynamique. Faire un vrai LRU avec `last_access`, élagage de fond.
5. **Pas d'invalidation du cache disque** (clé = id seul, sauf `forget` après édition). Une édition faite depuis un autre client ou un remplacement d'asset laisse une vignette périmée. Utiliser etag/checksum/`updatedAt` dans la clé.
6. **Cache de vignettes locales avec clé = hash de l'id/chemin seulement** (thumbnail_cache.rs:674) : pas de mtime. Comparer avec la clé RAW (chemin + mtime + taille, texture_decode.rs:270).
7. **`/proc/meminfo` pour borner la RAM** (thumbnail_cache.rs:707) : Linux uniquement (le code lit même une chaîne traduite `gettext("MemTotal:")`, fragile). Utiliser `sysinfo`.
8. **Modèle qui ne décharge jamais les mois** : acceptable à 50 000 photos mais non borné ; prévoir l'éviction des métadonnées de mois lointains.
9. **Sondage du rendu d'édition toutes les 1,5 s, 40 fois** (saving.rs:18-19) : préférer un évènement WebSocket Immich (le serveur émet des évènements `on_asset_update`) si disponible.
10. **Reconstruire des parseurs JPEG/EXIF/TIFF à la main** (texture_decode.rs:348-646, ~300 lignes) alors que `kamadak-exif`/`nom-exif`, `zune-jpeg`, `rawler` existent. À n'envisager que si le mode "fichiers locaux" est réellement voulu.
11. **GStreamer pour la vidéo** et **libshumate pour la carte** : spécifiques à GTK ; utiliser `<video>` et MapLibre GL JS dans le webview.
12. **Gros fichiers monolithiques** : `texture_decode.rs` (1772 lignes), `thumbnail_cache.rs` (719), `controls.rs`, `actions.rs` mêlent plusieurs responsabilités. Découper en crates : `immich-api`, `cache`, `decode`, `layout` (pur, testable, partagé avec le front en TS ou WASM).
13. **Tris nom/type uniquement sur les pages chargées** (state.rs:170) : trompeur avec une pagination ; cacher ces tris sur les sources paginées ou les faire côté serveur quand possible.
14. **Un seul canvas qui redessine tout à chaque snapshot** : en webview, ne pas reproduire ; préférer DOM virtualisé pour garder l'accessibilité, la sélection native et le menu contextuel.
15. **Écrire le PNG de vignette locale 256 px** : coûteux en octets ; WebP/AVIF ou JPEG q80 suffit et réduit le disque.
16. **Dépendance GPL** : ne pas copier du code Mimick dans une app sous licence non GPL ; réimplémenter à partir des idées de ce rapport.
