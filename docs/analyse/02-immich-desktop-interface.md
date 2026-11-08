# Analyse de immich-desktop (client Qt/C++ non officiel, MIT) : interface et UX

Source lue : /home/goss/.cache/immich-ref/immich-desktop (v0.2.4, 50 commits, du 23 au 31/10/2026). Chemins relatifs à `src/` sauf mention contraire. Rien n'a été compilé ni exécuté.

Résumé en une phrase : une application Qt Widgets de 13 000 lignes d'UI, centrée sur une grille justifiée façon Google Photos / Immich, avec un bon ordonnanceur de miniatures mais une architecture « un QWidget par photo » qui ne passera pas à l'échelle, et sans sélection multiple, carte, souvenirs, albums partagés, partenaires, piles, doublons ni dossier verrouillé.

---

## 1. Inventaire fonctionnel, écran par écran

### 1.1 Coquille de l'application (MainWindow, TopBar, Sidebar)

- Fenêtre sans cadre natif, barre de titre maison de 52 px (`ui/widgets/TopBar.cpp:18-40`) : logo, fil « Library », menu de profils (visible seulement s'il y a plus d'un profil, `TopBar.cpp` refreshProfiles), bouton « Update to vX » conditionnel, boutons réduire/agrandir/fermer dessinés. Glisser la barre = `startSystemMove`, double-clic = agrandir. Poignées de redimensionnement maison (`MainWindow.h` ResizeHandle).
- Pages empilées avec fondu (120 ms sortie, 180 ms entrée, `ui/widgets/AnimatedStackedWidget.cpp:13-43`) : 0 Library, 1 Explore (qui héberge aussi People/Albums/Tags/Folders/Favorites/Archive/Trash), 2 Settings (`MainWindow.cpp:454-486`).
- Barre latérale (`ui/widgets/Sidebar.cpp:228-262`), trois groupes avec petits titres en majuscules :
  - MAIN : Library, Explore, People, Favorites
  - COLLECTIONS : Archive, Albums (dépliable), Tags (dépliable), Folders (dépliable)
  - TOOLS : Trash, Settings
  - Albums/Tags/Folders se déplient en arbre animé (160 ms) sous le bouton ; chevron pour replier ; Folders et Tags sont de vrais arbres (hiérarchie de tags par `parentId`, chemins de dossiers) (`ui/widgets/BrowseTree.cpp`).
  - Mode Settings : la barre change de contenu (Immich Server, Folder Sync, Appearance, Update, About) (`Sidebar.cpp:263-282`).
  - Repliable (largeur 224 vers 68 px, icônes seules, `Sidebar.cpp:179`), raccourci F9 (`MainWindow.cpp:354`), repli automatique sous 1000 px de fenêtre (`MainWindow.cpp:684`), état mémorisé (`WindowSettings.sidebarCollapsed`). Option « UI minimaliste » = icônes seules partout.
  - Pied de barre : « fan-made immich desktop v0.2.3 ».
- Tray système, fermeture vers le tray (défaut oui), démarrage automatique, instance unique, Ctrl+Q pour quitter (`MainWindow.cpp:350`).
- Toasts (`ui/widgets/Toast.cpp`), notification de mise à jour dans le tray.

### 1.2 Library (page 0), `ui/pages/LibraryPage.cpp` (3031 lignes)

Barre d'outils (`LibraryPage.cpp:152-248`) : titre « Photos & videos » + sous-titre d'état (« 2687 items · local », « Loading library… 12/48 months »), champ de recherche, bouton réglages de recherche (n'apparaît qu'une fois le champ activé), puis boutons icônes : Upload, Refresh, Years (calendrier), Sort (menu), Shuffle, curseur de zoom avec boutons − et +.

Timeline :
- Groupes par jour, en-têtes « Today », « Yesterday », sinon date longue localisée (`LibraryPage.cpp:1347-1358`). Pas de groupement par mois, pas d'en-tête de mois.
- Disposition : lignes justifiées (« justified rows ») à la Flickr/Google Photos, hauteur cible = taille de tuile, bornée à 0,75x - 1,15x (`ui/GridZoom.h:22-37`), écart de 3 px, marge latérale 12 px (`LibraryPage.cpp:81-82`). Mode « Compact grid » alternatif : carrés uniformes sans en-têtes de date (`LibraryPage.cpp:2115-2140`, réglage Appearance).
- Zoom : 96 à 720 px, défaut 180, pas multiplicatif 1,15 (`GridZoom.h:12-27`). Ctrl+molette (`LibraryPage.cpp:2941-2950`), Ctrl+/Ctrl-/Ctrl+0 (`LibraryPage.cpp:665-671`), curseur + boutons. La position de scroll est conservée en proportion (`LibraryPage.cpp:702-719`). Valeur persistée.
- Chargement progressif : on récupère la liste des « buckets » mensuels (`/timeline/buckets`) puis jusqu'à 4 mois en parallèle (`kParallelTimelineBuckets = 4`, `LibraryPage.cpp:85`, `kickTimelineBucketFetches` 802-813), ajoutés dans l'ordre dès qu'un préfixe contigu est prêt (`flushReadyTimelineBuckets` 855-873). La hauteur de scroll restante est réservée par estimation à partir des comptes des buckets (`estimateReservedTimelineHeight` 823-840), donc la barre de défilement a d'emblée la bonne taille. Statut « Loading library… n/N months ».
- Détection de nouvelles photos : sondage toutes les 30 s des 24 plus récentes (`kAutoCheckIntervalMs`, `LibraryPage.cpp:83`, `checkForNewPhotos` 896-903).
- Retour en haut : Home/End (et Ctrl+Home/End) (`LibraryPage.cpp:262-270`), bouton flottant « back to top » avec fondu (`ui/widgets/ScrollToTopButton.cpp`), re-cliquer « Library » remonte en haut (`scrollToTop` 987).
- Pas de scrubber (curseur de date latéral). Seul repère de navigation temporelle : la vue Years.

Vue Years (`LibraryPage.cpp:1627-1960`) :
- Grille de cartes année (220x150 px, image de couverture = première photo du premier mois, nombre d'éléments) construite à partir des buckets (`computeYearSummaries`, `buildYearGrid`).
- Clic sur une année : vue « année » qui charge tous ses mois et affiche une carte par jour (110 px, vignette du premier média du jour, numéro du jour), groupées par nom de mois (`buildYearDetailUi` 1825-1874). Clic sur un jour : `jumpToDate` revient à la timeline et fait défiler jusqu'à l'en-tête du jour, après avoir chargé les mois intermédiaires si besoin (`jumpToDate` 1913-1958, `scrollToPendingDate` 1960-1980). Les liens de date du panneau d'info utilisent le même chemin.
- Hors ligne : « Years needs a connection » (`LibraryPage.cpp:1654`).

Tri / shuffle (`LibraryPage.cpp:1199-1345`, `ui/widgets/SortButton.cpp`) :
- Champs : date de prise de vue, date d'ajout, taille, note (Immich 3.2+), pertinence (recherche intelligente), dernière consultation (Immich 3.2+ et historique local), nom/type/modifié dans les dossiers, activité (albums partagés), date de suppression (corbeille). Sens croissant/décroissant avec libellés adaptés (« Largest first », « A to Z »).
- Chaque vue retient son tri (`AppSettings::loadSort("library"|"search"|"album:<id>")`).
- Trier par taille ou note remplace la timeline par une liste paginée sans en-têtes de jour (`listSortActive` 1240).
- Shuffle : `/search/random`, lots qui s'ajoutent au défilement, pas de recherche mélangée (`LibraryPage.cpp:1290-1330`).

Recherche (`LibraryPage.cpp:905-985`, panneau 190-330) :
- Champ unique avec anti-rebond 350 ms (`kSearchDebounceMs`, 84) ; Entrée lance tout de suite.
- Panneau « Refine your search » animé (hauteur + fondu 180 ms) : Search in (All/Favorites/Archive), People (liste issue d'Explore), Places (ville en texte libre), Media (photos/vidéos), plage de dates (cases + deux sélecteurs), « Search by » à 5 boutons : Context (recherche intelligente CLIP `/search/smart`), File name, Description, Full path or folder, OCR (`/search/metadata`). Résumé des filtres et « Reset filters ».
- Les résultats réutilisent la même grille, avec un tri de recherche séparé ; effacer la recherche rebascule sur la timeline en buckets.

Actions sur une tuile (`ui/widgets/MediaTile.cpp`) :
- Clic = ouvre le visualiseur ; un déplacement de plus de 8 px annule le clic (`MediaTile.cpp:~285`) ; Entrée/Espace ouvre ; Ctrl+C copie l'image.
- Menu contextuel (clic droit) : Open, Info, Copy (images seulement), Download, Share link…, Add/Remove favorites, Move to trash, Delete permanently. En corbeille : Open, Info, Download, Restore, Delete permanently (`MediaTile.cpp:~310-340`).
- Surcouches : icône lecture + durée en haut à droite pour les vidéos, cœur en bas à gauche pour les favoris, badge cloud (non sauvegardé / en cours / erreur) pour les fichiers du Folder Sync. Anneau violet de focus.
- Aucune sélection multiple, aucune case à cocher, aucun lasso, aucune action en lot (confirmé : seul résultat de grep « selection » hors BrowseTree est un commentaire de `SummaryTile.h:13` disant « no selection »). Roadmap README : « Bulk actions » en « Near term ».
- Survol d'une vidéo : lecture muette d'un extrait de 2,5 s après 450 ms de survol, via un lecteur unique sur un thread dédié, images réduites à 240 px max et poussées à 10 i/s sur l'UI (`ui/widgets/VideoHoverPreview.cpp:18-21`, moteur 25-85). Désactivable (Appearance), désactivé dans le Snap.

Upload / presse-papiers :
- Glisser-déposer de fichiers sur la page (surcouche « Drop photos or videos to upload »), Ctrl+V colle fichiers ou image du presse-papiers (`LibraryPage.cpp:661-663, 2543-2620`), bouton Upload avec sélecteur de fichiers, file d'attente d'upload persistante avec reprise, statut « Upload finished: n ok, m failed ».
- Les fichiers du Folder Sync apparaissent immédiatement dans la grille avec un badge « unsaved » (`prependLocalAsset` 1432).

### 1.3 Visualiseur (`ui/widgets/MediaViewerDialog.cpp`, 284 lignes)

- Un QDialog 960x700 (fenêtre séparée, non modale, pas de plein écran, pas de fond noir immersif), créé à chaque ouverture (`LibraryPage::openAsset` 2388-2416).
- Zone média + panneau d'info optionnel de 300 px à droite + rangée de boutons texte+icône en bas : Précédent, Suivant, Favori, Copier, Télécharger, Lien de partage, Corbeille, Restaurer, Supprimer définitivement, Info, Fermer (`MediaViewerDialog.cpp:55-90`).
- Raccourcis : flèches gauche/droite (précédent/suivant), F (favori), I (info), Espace (lecture vidéo), Échap (ferme, défaut QDialog) (`MediaViewerDialog.cpp:151-163`, `VideoPlayerWidget.cpp:85`). Pas de Suppr, pas de plein écran, pas de diaporama.
- Navigation : le visualiseur ne connaît pas la liste, il appelle un « fournisseur de voisins » (`adjacentAsset`, `LibraryPage.cpp:2418-2444`) qui reconstruit à chaque pas la liste plate de toutes les tuiles (O(N) par pas) et déclenche `loadMore()` à l'approche de la fin d'une liste paginée.
- Préchargement : l'aperçu de l'élément voisin dans la direction du dernier pas est préchargé (`updateNavigation`, 175-183).
- Aperçu image : on ne charge que l'image « preview » d'Immich (taille moyenne), jamais l'original ; zoom molette de x1 à x8 par facteur 1,12 centré sur le curseur, glisser pour déplacer, double-clic pour réinitialiser (`ui/widgets/ZoomPanWidget.cpp:12-15, 108-130, 188-251`). La molette zoome, elle ne navigue pas. À x8 l'image est donc floue (pas de montée en résolution).
- Vidéo : `QMediaPlayer` + `QVideoSink`, images copiées vers un widget logiciel (`SoftwareVideoWidget`), barre de position (0-1000), volume (curseur 0-100, défaut 80), temps, statut « Buffering… », bouton lecture central de 72 px, le zoom/pan marche aussi sur la vidéo (`ui/widgets/VideoPlayerWidget.cpp`). Seul l'élément ouvert depuis la grille se lance tout seul. Flux servi par un petit serveur TCP local (`core/VideoStreamServer.cpp`) qui ajoute clé API, en-têtes personnalisés et mTLS, car QMediaPlayer ne sait pas envoyer d'en-têtes.
- « Vue » comptée après 2 s d'ouverture (`MediaViewerDialog.cpp:39`), alimente « Recently viewed » local (300 max, `ImmichClient.cpp:1639`).
- Mode corbeille : boutons Restaurer / Supprimer définitivement à la place.
- L'état « info ouverte » est mémorisé (`WindowSettings.viewerInfoOpen`).

### 1.4 Panneau d'info (`ui/widgets/AssetInfoPanel.cpp`, 300 px fixes)

Sections avec icône à gauche, dans l'ordre : Date taken (lien cliquable vers ce jour dans Library + heure + fuseau), File (nom, dimensions, mégapixels, taille), Camera (marque/modèle dédoublonnés, objectif, ƒ/ouverture, vitesse, ISO, focale), Location (ville cliquable vers la vue Place, latitude/longitude, et un cadre « Map coming soon », ligne 230), Shared by, People (grille de 3 avatars ronds de 48 px, cliquables vers la fiche personne), Description, Albums (liens, requiert album.read), Tags (liens), Folder (lien vers le dossier). Lecture seule : aucune édition (description, date, lieu, tags), pas de nom de personne modifiable, pas de note par étoiles.

### 1.5 Explore et collections (`ui/pages/ExplorePage.cpp`, 1984 lignes)

- Vue d'accueil « Explore » (« People, places, and recent media ») : quatre rangées à défilement horizontal : People (cartes rondes), Places (cartes villes, image d'exemple), Recently added (lien « View all » qui ouvre Library triée par date d'ajout), Recently viewed (local, lien « View all ») (`ExplorePage.cpp:243-256`). Alimentée par `/search/explore`. Pas de carte géographique : « Places » est une grille de villes ; la feuille de route mentionne « map improvements » mais rien n'existe.
- Pages d'index (People, Albums, Tags, Folders) : grilles de cartes zoomables par curseur (`applyIndexZoom`), people avec bascule « Show hidden (n) » pour les personnes masquées.
- Page « collection » (personne, lieu, album, tag, dossier, favoris, archive, corbeille, récemment vu) : en-tête avec vignette héros (ronde pour une personne), titre, sous-titre (« 12 items »), fil d'Ariane cliquable (People > Prénom, Albums > nom, chaîne des tags parents, chemins de dossier) et « chips » des sous-tags / sous-dossiers (`ExplorePage.cpp:983-1083, 1237-1295`). Réutilise la même grille justifiée par jours, même zoom, tri (propre à chaque collection), shuffle, chargement de page suivante au défilement (avec « Load more » / « Try again » en secours, `ExplorePage.cpp:403, 946`).
- Albums : ordre de l'album repris d'Immich et sauvegardable (`album.update`), albums partagés triables par likes/commentaires (`activity.read`). Aucune création/édition/ajout d'éléments (README « Near term : create albums and tags, add/remove assets »).
- Corbeille : « Restore all », « Empty trash », « deleted permanently after n day(s) » (`ExplorePage.cpp:323-340, 1341`).
- Personnes : pas de renommage, fusion, masquage, ni suppression de visages (README « Next »).

### 1.5 bis Raccourcis clavier (liste complète trouvée dans le code)

| Contexte | Touche | Action |
|---|---|---|
| Global | Ctrl+Q, F9 | Quitter, replier la barre latérale |
| Grilles | Ctrl+molette, Ctrl+/-/0 (et pavé numérique) | Zoom des vignettes |
| Grilles | Home / End (avec ou sans Ctrl) | Haut / bas de grille |
| Grilles | Ctrl+V, Ctrl+C | Coller pour envoyer, copier l'image de la tuile active |
| Tuile | Entrée, Espace | Ouvrir |
| Visualiseur | Gauche/Droite, F, I, Espace, Échap | Naviguer, favori, info, lecture, fermer |
| Réglages | Échap | Retour |

Il n'y a ni navigation clavier entre tuiles (flèches dans la grille), ni Suppr, ni A (archive), ni sélection Maj/Ctrl, ni raccourci de recherche (Ctrl+K ou /).

### 1.6 Réglages (`ui/pages/*Page.cpp`)

- Immich Server (`ConnectionPage.cpp`) : URL distante, URL locale optionnelle (bascule automatique sur le LAN, indicateur « Using local endpoint »), clé d'API collée (pas de login/mot de passe, pas d'OAuth), « Save & test connection », en-têtes HTTP personnalisés (Cloudflare Access, etc.), certificat client mTLS (PKCS#12 ou PEM + clé + CA, mot de passe mémorisable), profils d'utilisateurs (créer, renommer, supprimer, « utiliser au lancement », isolation complète des caches, files d'upload, dossiers).
- Folder Sync (`SyncFolderPage.cpp`, `core/FolderSyncService.cpp`) : surveille plusieurs dossiers (QFileSystemWatcher + scan toutes les 8 s, délai de stabilité 1,5 s, `FolderSyncService.cpp:27-28`), option « upload only on local network », « Scan now », légende des badges Saved/Saving/Unsaved/Error.
- Appearance (`AppearancePage.cpp`) : langue (système ou choix), préréglage Light/Dark/Custom, palette personnalisée à 7 couleurs (Background, Panels, Buttons, Borders, Text, Muted text, Accent) avec réinitialisation, pack d'icônes (Lucide, Tabler, Bootstrap), UI minimaliste, bureau (fermer vers le tray, démarrer à l'ouverture de session), lecture (aperçu au survol), timeline (grille compacte, mémoriser les photos ouvertes, effacer l'historique), cache (rafraîchir, « Download all thumbnails »).
- Update (`UpdatesPage.cpp`) : mise à jour automatique (installeur Windows, MAJ par gestionnaire de paquets/Snap sinon), barre de progression, invite d'étoile GitHub.
- About : pile technique, liens, copie du pseudo Discord.
- Traductions : catalogues Qt `.ts` (626 chaînes), téléchargés depuis GitHub au démarrage avec repli embarqué ; en 0.2.4 : EN 100 %, PL et ZH 87 %, DE et FR 25 % (`README.md` tableau de traduction).

---

## 2. Techniques de performance et transposition au web

### 2.1 Ce que fait l'application

1. Index complet en mémoire, chargé mois par mois. Pas d'accès aléatoire : la timeline charge tous les buckets en ordre (4 en parallèle) et, pour atteindre une date ancienne, il faut attendre que les mois précédents soient arrivés (`jumpToDate` 1948-1957). Pour 100 000 photos, c'est 100 000 objets en mémoire et tout le chargement à chaque démarrage (une copie JSON du tout est aussi réécrite pour le mode hors ligne, `core/OfflineStore.cpp:59`, et fusionnée par lots, commit « perf(library) #13 »).
2. Un QWidget par photo (`LibraryPage.cpp:1401`, `new MediaTile`), tous enfants du même hôte géant. Seuls les widgets proches de la fenêtre sont `setVisible` (zone = 3 hauteurs de viewport, `LibraryPage.cpp:2230-2260`), mais la boucle de visibilité parcourt toutes les tuiles à chaque défilement (O(N)) et la mise en page `layoutTimeline` repositionne toutes les tuiles à chaque nouvelle donnée (`LibraryPage.cpp:2111-2210`). Les commentaires du code reconnaissent les symptômes : « showing a tile for the first time is costly (style polish), a 700-photo month would otherwise stall the UI » (2237) et « relayouting on every thumbnail freezes scrolling » (1999).
3. Rapport d'aspect : issu de l'asset s'il est connu, puis corrigé par la vignette reçue ; si l'écart dépasse 0,03, relayout complet (`LibraryPage.cpp:1982-2007`). Donc des sauts de mise en page pendant le défilement possible. (Avec l'API de buckets colonnaires, le ratio est fourni d'emblée : `core/ImmichClient.cpp:176-210`.)
4. Ordonnanceur de miniatures (`ui/TileImageScheduler.cpp`), la meilleure idée du projet :
   - Vérification du cache disque dès qu'une tuile est proche (jusqu'à 64 par passe).
   - Demande réseau seulement après un temps de présence : 250 ms visible, 600 ms proche mais hors écran (`TileImageScheduler.cpp:12-14`). Un défilement rapide ne déclenche donc aucun téléchargement.
   - Max 16 requêtes par passe (`:16`), traitées de la plus proche du centre de l'écran à la plus éloignée ; on envoie en ordre inverse pour que la dernière demandée (la plus centrale) parte la première (`:87-91` commentaire et `:119-128`).
   - Une demande déjà lancée est conservée même si la tuile sort (pour le cache) ; une demande préchargée est promue dès que la tuile devient visible (`:80-92`).
   - Relance périodique par minuteur (60 ms si plafonné, sinon à l'échéance du dwell).
5. File de décodage : pool de 4 threads (`ImmichClient.cpp:404`), priorités par numéro de série (les plus récentes d'abord) + bande « à l'écran » (2^29) + aperçu du visualiseur tout en haut, lectures du cache toujours prioritaires (`ImmichClient.cpp:2976-3000`). Le décodage et le ré-encodage JPEG 82 % se font dans le pool, la conversion en QPixmap sur le thread UI.
6. Résolution adaptative : la vignette serveur de 256 px suffit jusqu'à un besoin de 384 px ; au-delà on demande par paliers de 256 px jusqu'à 1536 px, pris sur l'image « preview » (`GridZoom.h:40-46`, `TileImageScheduler.cpp:46-66`).
7. Mémoire : tampon RAM de pixmaps de 8 Mo seulement pour les miniatures (`ThumbnailCache.h:20`, `ThumbnailCache.cpp:24`), 8 Mo + 512 Mo disque pour les aperçus (`ImmichClient.h:273-274`), disque des miniatures sans limite par défaut (`maxDiskBytes = 0`). En plus, toute tuile hors zone (1 viewport de chaque côté) abandonne son pixmap (`LibraryPage.cpp:2262-2273`). Le disque est purgé par tri de tous les fichiers à chaque écriture quand un plafond existe (`ThumbnailCache.cpp:98-120`, O(n) par écriture). Libération de mémoire lors du passage en arrière-plan (`MainWindow.cpp:442-448`, `trimProcessMemory`).
8. Réessais des 404 juste après un upload (miniature pas encore générée) à 5 s, 20 s, 60 s (`LibraryPage.cpp:2023-2040`).
9. Préchargement complet optionnel de toutes les miniatures (Réglages > Appearance > Cache).
10. Survol vidéo : un seul lecteur, un seul thread, images réduites, 10 i/s.
11. Le visualiseur calcule la liste voisine en O(N) à chaque pas (`LibraryPage.cpp:2418-2444`).

### 2.2 Transposition à Svelte (Tauri 2 + cœur Rust)

| Technique du projet | Transposable ? | Comment |
|---|---|---|
| Grille justifiée | Oui, mais autrement | Calcul de la géométrie pure (JS ou Rust/WASM) à partir des (id, ratio) fournis par les buckets ; on ne crée jamais un nœud DOM par photo. |
| Un widget par photo, visibilité O(N) | À éviter absolument | Virtualisation par rangées : on précalcule les positions (y, hauteur de rangée) par bucket, on ne rend que les rangées dans le viewport + marge (comme le webapp Immich). Pas d'`IntersectionObserver` par tuile à 100 000 photos ; un seul écouteur de scroll + recherche binaire sur les offsets. |
| Index complet chargé mois par mois | À améliorer | Charger la liste des buckets (comptes) → hauteur totale connue immédiatement (idée déjà dans le projet, `estimateReservedTimelineHeight`) → ne charger que les mois proches du viewport, à la demande, et pouvoir sauter à n'importe quelle date (scrubber). Les tailles de mois sont connues, donc la position d'un mois = somme des hauteurs estimées, corrigée quand il est chargé. |
| Ordonnanceur à dwell (250/600 ms), priorité au centre, annulation | Oui, excellente idée | `<img loading="lazy">` ne suffit pas. Reproduire côté JS : on ne pose `src` qu'après ~150-250 ms de présence, file de priorité par distance au centre, `AbortController` pour annuler ce qui sort. Dans Tauri, mieux : un protocole personnalisé `immich://thumb/<id>` géré par le cœur Rust (cache disque, déduplication, limite de concurrence) ; le webview émet les requêtes, Rust les trie. |
| Pool de 4 threads de décodage | Rendu inutile en JS | Le navigateur décode hors thread ; en revanche garder une limite de requêtes simultanées (6-8) côté Rust. Possibilité `img.decode()` + `content-visibility`. |
| Palier de résolution (256 → 1536) | Oui | `srcset`-like manuel : miniature de 250 px par défaut, `preview` quand la tuile dépasse ~300 px device. |
| Pixmap RAM 8 Mo + disque 512 Mo | Oui, côté Rust | Cache disque SQLite ou fichiers adressés par hash avec index (pas de scan du dossier à chaque écriture) et éviction LRU par date d'accès ; RAM gérée par le webview (cache HTTP du protocole custom avec `Cache-Control` immuable). |
| Abandon des images hors zone | Automatique avec la virtualisation | Les nœuds sont détruits en sortant de la fenêtre. |
| Aperçu précharge voisine | Oui | Précharger N±1 (voire ±2) dans le visualiseur. |
| Relayout si ratio change | Éviter | Utiliser le ratio d'abord fourni par l'API (colonnes `ratio` du bucket) ; pas de correction a posteriori. |
| Survol vidéo avec un lecteur partagé | Oui, plus simple | Un `<video muted>` unique repositionné, ou mieux une courte vidéo d'aperçu fournie par le serveur si disponible ; même délai 450 ms. |
| Serveur TCP local pour la vidéo | Oui, mais via le cœur Rust | Protocole custom ou petit proxy 127.0.0.1 qui injecte clé d'API/en-têtes/mTLS, supporte `Range`. `<video src>` ne peut pas envoyer `x-api-key`. |
| Hors ligne par gros JSON | Non | Utiliser SQLite avec les métadonnées par bucket. |

Chiffres à retenir pour la conception : défilement fluide exige O(1) par frame (rangées visibles seulement), toute mise en page est calculée par bucket et mémorisée, la hauteur totale ne bouge pas quand un mois arrive (réservation exacte puis remplacement).

---

## 3. Design visuel

- Look général : sombre bleu-ardoise par défaut, fenêtre sans cadre à coins arrondis, barre latérale et barre de titre en « cartes » flottantes (rayon 16 px, bordure 1 px) sur fond uni, boutons arrondis (rayon 10 px), espacements généreux (`core/ThemeManager.cpp:95-130`). Police Inter embarquée (Regular/Medium/SemiBold/Bold), 10,5 pt (`ThemeManager.cpp:95-100`, `ui/FontLoader.cpp`). Icônes SVG Lucide, remplaçables par Tabler ou Bootstrap, recolorées selon le thème (`ui/IconUtils.cpp`). Capture : `docs/screenshots/immich-desktop-library.png`.
- Thème : sept rôles de couleur seulement (`core/Theme.h:11-19`). Sombre : fond #0C1018, panneau #151B26, bouton #202838, accent #818CF8 (indigo), texte #F5F7FB, texte atténué #98A2B3, bordure #293244. Clair : fond #F4F6FA, panneau #FFFFFF, accent #6366F1 (`core/Theme.cpp:5-30`). Un thème « Custom » par défaut turquoise #2DD4BF. Une feuille de style Qt générée par chaînage de chaînes (`ThemeManager::buildStyleSheet`), pas de variables : tout est recalculé et réappliqué à chaque changement.
- Écart avec Immich : ce n'est pas la palette d'Immich (primaire bleu #4250af clair / #adcbfa sombre, fond gris neutre) ; l'ancienne version utilisait le logo à roue colorée d'Immich (`docs/screenshots/immich-desktop-library-OLD.png`, v0.1.3), remplacé par un logo maison pour éviter la confusion de marque. Pour notre projet « comme le webapp Immich », il faudra reprendre ses jetons de couleur ou assumer une identité propre.
- Barre latérale : boutons pleine largeur, sélection indigo plein, étiquettes de groupe en petites majuscules, filets guides sous les groupes (commits du 26/10), pied « fan-made ».
- Grille : tuiles aux angles droits, écart de 3 px, fond #141414 en attente, texte « … » ; surcouches sobres (pastilles noires translucides). En-têtes de jour en gras sur fond de page, sans ligne de séparation ni compteur.

Ce qui fait bon : grille dense et nette, barre latérale claire et regroupée, icônes seules dans la barre d'outils, recherche qui s'étend au focus, panneau de filtres animé, thèmes entièrement personnalisables à sept couleurs, bulles d'info partout, repli automatique sous 1000 px, fondus de page discrets, cartes d'années avec dégradé et gros chiffre.

Ce qui fait daté / « application Qt » : boutons texte+icône du visualiseur en rangée basse (loin du modèle Immich : barre d'actions translucide en haut), visualiseur en fenêtre séparée classique avec titre de fichier, curseurs Qt standard pour position/volume vidéo, dialogues Qt natifs (confirmations, sélecteur de fichier), combos et date-pickers Qt dans le panneau de recherche, titre de fenêtre maison sans effets (pas de mica/acrylique), « fan-made » en pied, pas de chargement squelette/animations de tuiles (carré uni avec « … »), pas de transitions de partage d'éléments (grille vers visualiseur).

---

## 4. Évolution (changelog.json, historique git, README)

`changelog.json` ne contient que les notes de la dernière version (0.2.4, 2 976 octets) ; l'historique des demandes se lit surtout dans les 50 commits (du 23 au 31/10/2026, deux versions publiées : v0.2.3 le 23/10, v0.2.4 le 30/10, plus une prérelease « beta » à chaque push sur main) et dans les numéros de PR/issue cités.

Chronologie utile :
- v0.1.x : Library + Explore + Settings seulement, boutons « Upload » et « Refresh » en texte (capture OLD).
- v0.2.3 : grilles zoomables, années, panneau d'info, visualiseur navigable, barre latérale repliable, aperçu vidéo au survol (contributeur majeur « gardar »).
- 23/10 PR #13 « perf(library) : fix freezes and reload loops on large libraries » : écritures hors ligne par lots, tuiles visibles seulement près du viewport, retries de miniatures 404 : le premier signal de souffrance liée à de grandes bibliothèques.
- 24/10 : en-têtes HTTP personnalisés (proxies d'auth), i18n Qt (issue #17), polonais, chinois (PR #18).
- 25/10 PR #19 : sidebar People/Favorites/Archive/Albums/Tags/Folders/Trash. Palette personnalisable étendue (PR #20).
- 26/10 PR #22 : favoris depuis le visualiseur/le menu. 27/10 PR #23 « thumbnails load faster when scrolling » (TileImageScheduler) : deuxième passe de perf. Issue #24 : profils multi-comptes (PR #24).
- 28/10 PR #26 : retour en haut (Home/End, bouton) : demande liée aux longues grilles. PR #27 : chargement au défilement dans Explore (au lieu du bouton). « Download all thumbnails » (cache hors ligne). Correctifs de mise à jour automatique (timeout 30 s, mauvaise architecture).
- 29/10 PR #28 : tri, shuffle, récemment consulté.
- 30/10 : v0.2.4 ; correctif Windows de relance de l'ancien binaire après installation.

Ce que les utilisateurs demandent (déduit des issues et des PR) : performance sur grandes bibliothèques (PR #13, #23, #26), accès à toute l'organisation (albums, tags, dossiers, corbeille, personnes), multi-compte, authentification derrière proxy (en-têtes, mTLS), langues, et packaging (Snap, AUR, deb, AppImage, Flatpak à venir). Aucune demande de carte ou de souvenirs n'apparaît dans les commits, mais la roadmap les liste (« Memories & faces », « map improvements »).

Feuille de route officielle du README (`README.md` sections Roadmap) : Near term = création d'albums et tags, actions en lot, fiabilité des uploads (pause/reprise, progression par fichier), polissage d'Explore (carte, nommer/fusionner des personnes), plus de raccourcis ; Next = partage d'albums, partenaires, souvenirs, gestion des visages, doublons, filtres avancés, notifications, santé de la synchro ; Later = multi-serveur (déjà fait via profils), packs hors ligne en pleine résolution, « ouvrir avec », parité API, Flatpak.

---

## 5. Garder / éviter / manques

### 5.1 « Garder » (meilleures idées d'UX)

1. Grille justifiée + zoom continu par Ctrl+molette avec conservation de la position (`LibraryPage.cpp:702-719`) et raccourcis Ctrl +/-/0. Garder le curseur de zoom visible dans la barre.
2. Réservation de la hauteur de scroll depuis les comptes de buckets, avec statut « n/N mois » (`LibraryPage.cpp:823-840, 842-853`).
3. Vue Years : cartes d'année, puis cartes de jour avec vignette (`LibraryPage.cpp:1627-1980`) : bonne alternative à un scrubber, à garder en complément de celui-ci.
4. Lien vers le contexte depuis le panneau d'info : date → jour de la timeline, lieu → vue lieu, personne → fiche, album, tag, dossier (`AssetInfoPanel.cpp`). Très naturel.
5. Barre latérale à groupes (Main / Collections / Tools) avec arbres dépliables pour albums, tags (hiérarchiques) et dossiers, repli par F9, repli automatique sous 1000 px.
6. Fil d'Ariane + chips de sous-éléments dans les collections (`ExplorePage.cpp:1237-1295`).
7. Tri par vue mémorisé, menu de tri avec libellés adaptés au champ, Shuffle, « Recently viewed » local avec option d'oubli.
8. Recherche : un champ, panneau de filtres qui s'étend, anti-rebond 350 ms, bascule explicite de mode (Context / nom de fichier / description / chemin / OCR) avec description du mode (`LibraryPage.cpp:213-290`).
9. Ordonnanceur de miniatures par dwell et distance au centre (`TileImageScheduler.cpp`), avec résolution adaptative.
10. Aperçu vidéo au survol (délai 450 ms, 2,5 s, muet), badge durée.
11. Dépôt de fichiers par glisser-déposer et Ctrl+V pour envoyer, badges de statut « non sauvegardé » du Folder Sync directement sur les tuiles.
12. Mode hors ligne explicite : « n cached items · offline », file d'upload qui attend.
13. URL locale + URL distante avec bascule automatique ; en-têtes personnalisés ; mTLS ; profils isolés. Précieux pour des utilisateurs self-hosted derrière proxy.
14. Panneau d'info qui mémorise son ouverture.
15. Personnalisation de couleurs (7 rôles) et mode « minimaliste ».

### 5.2 « Éviter / améliorer » (faiblesses de l'UX)

Architecture et performances
1. Un widget par photo et une boucle O(N) à chaque défilement : à remplacer par la virtualisation par rangées (voir 2.2).
2. Chargement séquentiel de tous les mois, pas d'accès direct par date : aller à « mars 2015 » oblige à tout charger avant (`LibraryPage.cpp:1948-1957`). Ajouter un scrubber (voir plus bas) et un chargement par bucket à la demande.
3. Relayout complet à chaque lot de données ou changement de ratio (`LibraryPage.cpp:2111`), risques de saut de défilement.
4. Cache hors ligne en un seul JSON par serveur (`OfflineStore.cpp`) ; purge de cache disque O(n) à chaque écriture. Prévoir SQLite et éviction LRU.
5. Le visualiseur reconstruit la liste de voisins à chaque pas (`LibraryPage.cpp:2418`).
6. Pas de zoom en pleine résolution : l'aperçu seul est chargé, donc flou à x8. Charger l'original (ou une image 4K) lors du zoom, avec indicateur de netteté.

Visualiseur
7. Fenêtre séparée non immersive, boutons texte en bas, pas de plein écran (F), pas de diaporama, pas de rotation/miroir, pas de raccourcis Suppr/A (archive)/E, pas de « télécharger l'original » en un clic avec progression visible, pas de glisser-déposer hors de la fenêtre, molette = zoom (au lieu de naviguer comme dans le webapp), pas de geste tactile (pincer, balayer), pas de contrôles qui s'estompent, pas de vignettes en filmstrip.
8. Vidéo : lecture par copie d'images logicielles (`SoftwareVideoWidget`), vitesse de lecture et plein écran absents, pas de sous-titres, pas de sélecteur de qualité/transcodage.
9. Vignettes carrées à angles droits, pas d'effets de survol (case de sélection, sélection par cercle comme Immich), pas de transitions du clic vers le visualiseur.

Sélection et organisation
10. Aucune sélection multiple (clic sur cercle, Maj-clic plage, Ctrl-clic, lasso, Ctrl+A, glisser sur une rangée), donc aucune action en lot : favori, archive, corbeille, ajout à un album, téléchargement zip, étiquettes. C'est le manque n°1 par rapport au webapp.
11. Albums et tags en lecture seule (pas de création, ajout, retrait, renommage, couverture, tri manuel).
12. Personnes : pas de nom, fusion, masquage depuis l'UI, pas de suggestion de visages ; pas de page des visages d'une photo avec cadres.
13. Recherche : la ville est un champ libre (pas d'autocomplétion pays/état/ville), pas de filtres appareil, objectif, note, taille, pas de recherches enregistrées, pas d'historique de recherche, pas de bascule « ctrl+K ». Les champs natifs (QDateEdit, QComboBox) jurent avec le reste.

Fonctions Immich absentes (confirmées par recherche dans `src/`)
- Carte (vue géographique, clusters, Places) : placeholder « Map coming soon » (`AssetInfoPanel.cpp:230`).
- Souvenirs (« On this day », mémoires) : absent.
- Albums partagés (création, invitation, rôles, activité likes/commentaires) : seulement le tri par activité ; partage de liens individuels uniquement (`ShareLinkDialog`), sans gestion des liens existants (liste, expiration, droits).
- Partage avec partenaire : absent.
- Piles (stacks), photos en direct (Live Photos), RAW+JPEG associés : absents (le code filtre seulement les vidéos cachées des Live Photos pour les contrôles de nouveauté, `ImmichClient.cpp:1045`).
- Doublons (utilitaire de détection) : absent.
- Dossier verrouillé (locked folder), archive avec raccourci d'action, restauration individuelle limitée.
- Édition (rotation, recadrage, filtres, correction de date/lieu/description, étoiles de note) : aucune.
- Scrubber de timeline à droite avec libellés de dates : absent (le seul repère est la barre de défilement).
- Groupement par mois ou année, regroupement par lieu : absent (seulement par jour).
- Gestion de la bibliothèque : bibliothèques externes, jobs, utilisateurs, stockage, administration : absent (normal pour un client).
- Notifications serveur, quotas, statistiques d'usage : absent.
- Authentification : uniquement clé d'API collée à la main, pas de login/mot de passe, OAuth/SSO, ni 2FA ; stockage de la clé en clair dans le fichier de réglages (README « Custom HTTP headers : stored locally in the app's plaintext settings file »). À remplacer par le trousseau du système.
- Upload : pas de reprise par fichier ni de progression détaillée (roadmap), pas de dédoublonnage visible avant envoi, pas de « sauvegarde de l'appareil » style mobile.
- Accessibilité et clavier : navigation par flèches dans la grille absente ; thèmes sans mode « suivre le système » (le thème est choisi à la main, `Appearance`) ; pas de réduction des animations.
- i18n : 25 % pour FR/DE ; format de date longue du système.

### 5.3 Recommandations directes pour la réécriture Tauri/Svelte

1. Garder le vocabulaire et l'ordre de la navigation actuelle (Library, Explore, People, Favorites, Archive, Albums, Tags, Folders, Trash), en y ajoutant Map, Memories, Sharing, Partners, Duplicates, Locked folder selon le webapp Immich.
2. Concevoir le cœur de la timeline autour de buckets (comptes + ratios) et d'une virtualisation par rangées, avec scrubber à droite, repère de date flottant, saut par année, et conserver la vue Years comme aperçu.
3. Sélection multiple dès la première version, y compris lasso et Maj-clic, avec barre d'actions contextuelle (comme le webapp).
4. Visualiseur plein cadre, barre translucide en haut, molette = navigation, Ctrl+molette ou pincement = zoom, montée en résolution automatique, panneau d'info à droite avec édition, filmstrip optionnel, plein écran, diaporama.
5. Reprendre l'ordonnanceur de vignettes (dwell, distance au centre, annulation, résolutions étagées), le déplacer côté Rust (protocole custom) et y mettre le cache disque indexé.
6. Reprendre profils, URL locale/distante, en-têtes et mTLS (utiles) ; ajouter login/OAuth et stockage du secret dans le trousseau.
7. Reprendre Folder Sync avec badges sur les tuiles, en ajoutant file de reprise, progression par fichier, et exclusions.
8. Thème : variables CSS à partir des jetons du webapp Immich (clair/sombre/auto), accent personnalisable, plutôt que sept couleurs en dur.
