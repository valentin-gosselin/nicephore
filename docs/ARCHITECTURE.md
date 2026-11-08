# Client desktop Immich en Rust : synthèse et architecture cible

Nom retenu le 08/11/2026 : **Nicéphore**, en hommage à Nicéphore Niépce (Niepce est déjà le nom d'une
appli photo de GNOME). Le nom ne reprend ni la marque ni le logo Immich. Licence : AGPL 3.

Document rédigé le 08/11/2026 à partir de l'analyse de deux projets open source :

- **immich-desktop** (hdmain, C++/Qt 6, MIT, v0.2.4, 24 000 lignes) : rapports `analyse/01` et `analyse/02`.
- **Mimick** (nicx17, Rust/GTK4, GPL 3, v9.13.0, 74 000 lignes) : rapports `analyse/03` et `analyse/04`.

Serveur de référence : Immich v3.3.6 sur le .18 (port 2283).

Règle de licence : immich-desktop est MIT, on peut s'en inspirer et en reprendre des morceaux en
citant la source. Mimick est GPL 3 : on reprend les idées, jamais le code, sauf accord de son auteur
ou si notre projet est lui-même sous GPL/AGPL.

---

## 1. Ce que l'analyse nous apprend

### immich-desktop

Bon sur la forme, fragile dans le fond. Écrit en très peu de temps (v0.1 en juillet, v0.2.4 fin
octobre), un seul gros client HTTP de 3 200 lignes, aucun test sur les modules à risque.

À reprendre :
- timeline par index de mois (`/timeline/buckets`) puis mois à la demande (`/timeline/bucket`, format
  colonnaire), hauteur de défilement réservée d'après les compteurs ;
- ordonnanceur de vignettes : délai de présence à l'écran (250 ms) avant de télécharger, priorité au
  centre de l'écran, résolutions étagées selon le zoom ;
- double adresse LAN / Internet avec sonde `/server/ping` et bascule automatique ;
- registre de capacités selon la version du serveur, messages précis quand la clé API manque d'un droit ;
- proxy vidéo local avec jeton (le lecteur ne sait pas envoyer la clé API) ;
- vue Années, liens contextuels depuis le panneau d'infos (date, lieu, personne, album, tag) ;
- aperçu vidéo muet au survol, badges d'état de synchro sur les vignettes ;
- en-têtes HTTP personnalisés et certificat client (mTLS) pour les serveurs derrière un proxy.

À ne surtout pas reproduire :
- clé API et mot de passe mTLS **en clair** dans un fichier INI sous Linux ;
- mise à jour **sans aucune signature** : script PowerShell en `ExecutionPolicy Bypass`, remplacement
  d'AppImage par un script shell non échappé, `dpkg -i` proposé même sur Fedora ;
- persistance en gros fichiers JSON réécrits en entier (bibliothèque, file d'envoi, état de synchro) ;
- cache de vignettes sans invalidation, faux LRU, scan complet du dossier à chaque écriture, une
  connexion TLS neuve par vignette ;
- synchro sans empreinte de contenu (un fichier renommé est renvoyé en entier), retentatives infinies
  toutes les 8 s, un seul envoi à la fois ;
- un widget par photo, boucles sur toute la bibliothèque à chaque défilement ;
- visionneuse en petite fenêtre, sans plein écran, qui ne charge jamais l'original (flou au zoom) ;
- aucune sélection multiple, organisation en lecture seule (albums, tags, personnes).

### Mimick

Beaucoup plus mûr (687 commits), excellent sur la performance de la grille et sur la synchro montante.

À reprendre (idées) :
- grille : rangées justifiées avec recherche par bisection, hauteurs estimées pour les mois pas encore
  chargés, défilement ancré, 16 chargements au plus triés par distance au centre, annulation de tout
  ce qui sort de la zone, suspension pendant un défilement rapide ;
- cache : déduplication des requêtes en vol, deux files séparées petites et grandes images, repli de
  qualité (original, aperçu, vignette) ;
- visionneuse à deux niveaux : taille de l'écran d'abord, pleine résolution seulement au zoom ;
- synchro : index comme source de vérité avec empreinte taille + date, SHA-1 (celle d'Immich),
  vérification groupée `bulk-upload-check` par lots de 500 avant envoi, attente qu'un fichier ait
  fini d'être écrit, anti-boucle sur ses propres écritures, verrou par album ;
- prudence : corbeille Immich seulement, retrait d'album plutôt que corbeille quand la photo est
  ailleurs, double confirmation des suppressions en masse, détection des renommages par empreinte ;
- pauses : réseau facturé à l'usage, batterie, heures calmes.

Les risques de perte de données relevés chez lui, qu'on doit rendre impossibles chez nous :
1. au démarrage, un disque non monté ou un dossier illisible fait passer **tous** les fichiers pour
   supprimés, donc envoyés à la corbeille du serveur ;
2. chaque suppression locale part à la corbeille sans délai ni seuil ;
3. en cas d'erreur de lecture d'un album, le repli est la corbeille au lieu de ne rien faire ;
4. téléchargement non atomique et non vérifié : un fichier tronqué peut être renvoyé comme nouveau ;
5. un fichier modifié pendant son envoi n'est jamais renvoyé ;
6. un index illisible est remplacé par un index vide sans sauvegarde ;
7. le nom d'album sert de clé alors qu'Immich autorise deux albums du même nom.

Ce qu'aucun des deux ne fait : fichiers virtuels, synchro descendante sélective sérieuse, vraie
synchro à trois voies (avec l'état « dernière synchro »), mode portable, mise à jour signée,
multi-comptes simultanés, flux de changements `/api/sync/*`.

---

## 2. Choix techniques

| Sujet | Choix | Pourquoi |
|---|---|---|
| Cœur | Rust, espace de travail Cargo en plusieurs crates | sûreté du moteur qui touche aux photos, tests isolés |
| Interface | Tauri 2 + SvelteKit statique + Svelte 5 | même langage que la webapp Immich |
| Composants | `@immich/ui` (MIT, Svelte 5, Tailwind) | l'apparence de la webapp sans la redessiner |
| Base locale | SQLite (`rusqlite`, WAL) | fin des JSON réécrits, reprise après plantage |
| Client API | sous-ensemble typé à la main, guidé par l'OpenAPI 3.3.6 | `progenitor` à évaluer, le schéma Immich est très gros |
| Secrets | `keyring` (trousseau système) ; fichier chiffré en mode portable | jamais en clair |
| Carte | MapLibre GL JS avec le style fourni par le serveur | regroupement natif, pas de dépendance système |
| Vidéo | `<video>` + proxy local avec jeton (ou protocole Tauri si Range marche) | à trancher au prototype |
| Mise à jour | `tauri-plugin-updater`, manifeste signé (minisign) obligatoire | AppImage et NSIS remplacés en place |
| Fichiers virtuels | Windows : `cloud-filter` (API Cloud Files) ; Linux : FUSE (`fuser`) | même mécanisme que OneDrive et Nextcloud |

Point de vigilance n°1 : sous Linux, Tauri utilise WebKitGTK. C'est lui qui posait souci avec
Music Assistant sous Wayland. Le prototype doit valider le défilement de milliers de vignettes et la
lecture vidéo sous KDE Wayland avant d'aller plus loin. Repli possible : garder le cœur Rust et passer
l'interface en Slint.

---

## 3. Découpage

```
nicephore/
  crates/
    api/        client Immich : auth x-api-key, LAN/WAN, capacités par version, erreurs typées,
                retentatives avec délai croissant, flux /api/sync/*, WebSocket d'événements
    store/      SQLite : comptes, miroir des assets, index de timeline, index du cache de vignettes,
                état de synchro à trois voies, file d'envoi et de téléchargement persistante
    cache/      vignettes et aperçus sur disque, clé = id + thumbhash, vrai LRU élagué en tâche de fond,
                requêtes en vol dédupliquées, deux sémaphores, écriture atomique
    sync/       moteur : règles, surveillance (notify), stabilisation, SHA-1, bulk-upload-check,
                réconciliation à trois voies, garde-fous de suppression, journal des actions
    vfs/        fichiers virtuels : Windows (cloud-filter), Linux (FUSE, lecture seule d'abord)
    platform/   trousseau, batterie, réseau facturé, démarrage auto, mode portable, notifications
  app/          Tauri 2 : commandes, événements, protocole thumb://, proxy vidéo, plateau, mises à jour
  ui/           SvelteKit + @immich/ui : timeline virtualisée, visionneuse, explorer, carte, réglages
  docs/
```

### Règles de synchro

Une règle relie une source et une destination, avec un sens :

- **montante** : dossier local vers bibliothèque, vers un album choisi, ou vers un album au nom du dossier ;
- **descendante** : sélection Immich vers dossier local. La sélection peut être toute la bibliothèque,
  un album, une personne, une période (« les 12 derniers mois »), les favoris ou une recherche
  enregistrée. On choisit pour chaque règle : copie complète, fichiers virtuels ou vignettes seules ;
- **dans les deux sens** : dossier et album liés, avec l'état de la dernière synchro mémorisé pour
  savoir qui a changé quoi. En cas de conflit, on garde les deux versions, jamais d'écrasement silencieux.

### Garde-fous (non négociables)

1. Aucune suppression définitive, ni locale ni distante : corbeille du système, corbeille Immich.
2. Suppression miroir **désactivée par défaut**, et quand elle est active :
   - un fichier témoin à la racine du dossier : s'il est absent (disque non monté), la règle se met en pause ;
   - un dossier illisible n'est jamais compté comme supprimé ;
   - un seuil (par exemple plus de 20 fichiers ou plus de 5 %) déclenche une demande de confirmation
     dans l'interface, avec la liste ;
   - un délai de grâce avant d'appliquer une suppression ;
   - toute erreur de lecture d'état mène à l'abstention, jamais à la corbeille.
3. Téléchargements dans un fichier temporaire, empreinte SHA-1 vérifiée, puis renommage atomique.
4. Empreinte recalculée si le fichier a changé pendant l'envoi.
5. Albums identifiés par leur id, jamais par leur nom.
6. Base SQLite sauvegardée avant chaque migration, jamais remplacée en silence.
7. Journal consultable de toutes les actions de synchro, avec annulation des envois et des corbeilles.

---

## 4. Mode portable et mises à jour

- **Portable** : la présence d'un fichier `portable` à côté de l'exécutable fait tout ranger dans un
  dossier `donnees/` voisin : base, cache, réglages. Les secrets sont chiffrés par une phrase de passe
  (ou mis dans le trousseau du PC si l'utilisateur l'accepte). Rien n'est écrit dans le registre ni
  dans le profil. Pas de démarrage auto ni de fichiers virtuels en mode portable (ils demandent une
  inscription auprès du système).
- **Mises à jour dans l'appli** :
  - AppImage : remplacement en place après vérification de la signature, ancienne version gardée
    pour revenir en arrière ;
  - Windows installé (NSIS) : téléchargement, vérification, installation silencieuse, relance ;
  - Windows portable : nouvel exe téléchargé et vérifié à côté de l'ancien, échange au redémarrage ;
  - rpm, Flatpak : mise à jour laissée au gestionnaire de paquets (Discover, dnf), l'appli prévient seulement ;
  - canaux stable et bêta, manifeste signé obligatoire, liste des nouveautés affichée.
- **Livrables** : Windows (installeur NSIS + exe portable), Linux (AppImage, rpm, puis Flatpak sur Flathub).
  Signature de code Windows à prévoir (SignPath propose une offre gratuite pour les projets open source)
  pour éviter les blocages SmartScreen.

---

## 5. Ce qu'on ajoute par rapport aux deux projets

Côté visionnage, viser la parité avec la webapp :
- sélection multiple et actions groupées ;
- visionneuse plein écran, diaporama, pleine résolution au zoom ;
- défilement par frise de dates (scrubber) en plus de la vue Années ;
- carte, souvenirs, albums partagés, partenaires, piles, Live Photos, doublons, dossier verrouillé ;
- édition des métadonnées (date, lieu, description, note, tags) et nommage ou fusion des personnes ;
- thème qui suit le système.

Le reste :
- fichiers virtuels (Windows puis Linux) avec « libérer de l'espace » / « toujours garder sur cet appareil » ;
- import de carte SD ou d'appareil photo : empreintes comparées au serveur, seuls les nouveaux partent ;
- intégration au gestionnaire de fichiers : icônes d'état, clic droit « Envoyer vers Immich » ;
- glisser une photo vers un mail ou un document, avec redimensionnement à la volée ;
- plusieurs comptes ouverts en même temps (pas de redémarrage pour changer) ;
- rappels « il y a X ans », diaporama en économiseur d'écran ;
- passerelle avec les ateliers souvenir (Madeleine) : ouvrir une sélection, renvoyer un montage.

---

## 6. Étapes proposées

| Étape | Contenu | Critère de sortie |
|---|---|---|
| 0. Prototype | connexion par clé API, timeline complète virtualisée sur le serveur du .18, cache thumb://, visionneuse, une vidéo | fluide sur Fedora KDE Wayland et dans la VM Windows, sinon bascule Slint |
| 1. Visionneuse | parité immich-desktop + sélection multiple, plein écran, scrubber, carte, souvenirs | utilisable au quotidien à la place de la webapp |
| 2. Envoi | règles montantes, SQLite, file persistante, plateau, mode portable, mises à jour signées, CI | premières versions publiques |
| 3. Synchro descendante et double sens | sélections, trois voies, garde-fous, journal | tests sur serveur simulé, zéro perte sur scénarios de panne |
| 4. Fichiers virtuels | Windows d'abord, puis FUSE sous Linux | hydratation à la demande dans l'Explorateur et Dolphin |
| 5. Extras | import SD, intégration fichiers, multi-comptes, rappels, Madeleine | à prioriser ensemble |

Les tests du moteur de synchro tournent contre un serveur Immich simulé (et une instance Immich
jetable en Docker pour l'intégration), jamais contre la vraie bibliothèque.

---

## 7. Décisions à prendre

1. **Nom public** et identité visuelle (pas le logo Immich : un membre de l'équipe Immich a déjà
   ouvert une issue à ce sujet chez immich-desktop).
2. **Licence** : AGPL 3 (même famille qu'Immich, autorise à reprendre le code GPL de Mimick avec
   l'accord de son auteur) ou MIT/Apache (plus permissive, mais Mimick reste alors une simple source
   d'idées).
3. **Compilation sur le PC fixe** : installer les paquets de développement WebKitGTK avec dnf, ou
   compiler dans une toolbox pour ne rien toucher au système.
4. **Dépôt** : privé jusqu'au prototype validé, ou public dès le départ.
5. **Prise de contact** avec l'auteur de Mimick pour un cœur de synchro commun : maintenant ou
   après le prototype.

---

## 8. Chantiers notés en route

- **Découpage réel** (08/11) : `crates/api_immich` (client), `crates/cache_vignettes` (cache disque
  des images), `crates/envoi` (moteur d'envoi : base SQLite, analyse, boucle ; lot 6). Les crates
  `store`, `sync`, `vfs` et `platform` prévues plus haut naîtront avec les étapes 3 et 4.
- **Étape 2 découpée** : lot 6 envoi (fait), lot 7 zone de notification et démarrage automatique (fait),
  lot 8 mode portable (fait), lot 9 mises à jour signées, paquets et intégration continue (fait, clé de publication à créer).

- **Connexion OAuth** (demandé le 08/11) : ouvrir la page du fournisseur d'identité dans le navigateur
  du système, pour profiter du gestionnaire de mots de passe, puis récupérer le jeton par une
  redirection vers l'appli (adresse de rappel en boucle locale ou schéma `nicephore://`). Immich ne
  passe la main au navigateur que pour OAuth : la connexion par e-mail et mot de passe n'a pas de
  mécanisme équivalent. Le serveur de test n'a pas OAuth activé.
