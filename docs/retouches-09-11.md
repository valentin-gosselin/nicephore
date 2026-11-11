# Retouches du 09/11 (retours de Valentin)

- Plus de sélection « façon page web » : en faisant défiler ou en cliquant, les images et les
  textes ne se surlignaient plus en bleu comme dans un navigateur. Sélection désactivée dans toute
  l'appli, sauf dans les champs de saisie ; plus d'image fantôme au glisser.
- Fermer la fenêtre la range dans la zone de notification dès que l'option est active (elle l'est
  par défaut), même sans dossier en cours d'envoi : avant, sans dossier synchronisé, fermer
  quittait l'appli. On quitte depuis le menu de l'icône. Lancé avec la session, Nicéphore démarre
  rangé dans la zone de notification.
- Vérifié dans l'appli (59/59) : sélection désactivée sur la page, fenêtre fermée sans aucun dossier
  rangée dans la zone de notification puis rouverte ; visionneuse, portable, comptes et lecture
  seule sans régression.

## Vidéos de l'ordinateur dans la visionneuse

- Une vidéo ouverte depuis le gestionnaire de fichiers (Nicéphore étant la visionneuse par défaut)
  restait noire : WebKitGTK ne lit pas les vidéos servies par le protocole interne de Tauri. Elles
  passent désormais par le relais local déjà utilisé pour les vidéos du serveur (127.0.0.1, jeton
  aléatoire), avec la lecture par plages (`Range`) qu'exige le lecteur ; seuls les fichiers ouverts
  dans la visionneuse y sont enregistrés, par numéro, jamais par chemin.
- Toutes les vidéos sont essayées (MP4, MOV, WebM, MKV, AVI…) ; si le système ne sait pas en
  décoder une, un message clair remplace l'écran noir.
- La vidéo de Valentin (`essai.mp4`) est en H.264 4K, que ses décodeurs GStreamer savent lire :
  c'était bien le protocole, pas le format.
- Vérifié : plages (tests unitaires), banc `--fichier` 14/14 avec une vidéo H.264 960 × 720 lue et
  la fenêtre à ses proportions ; vidéos du serveur toujours lues.

## Icône de la zone de notification sous Plasma

- Rangée dans la fenêtre contextuelle de la zone de notification (icônes masquées), l'icône de
  Nicéphore disparaissait et l'entrée s'appelait « nicephore ». L'icône de Tauri passe par
  libappindicator, qui transmet à Plasma un chemin de fichier temporaire et aucun titre.
- Sous Linux, Nicéphore publie désormais lui-même son icône par le protocole standard
  (StatusNotifierItem, crate `ksni`) : titre « Nicéphore », images de l'icône en 32, 64, 128 et
  256 pixels, info-bulle avec l'état de l'envoi, même menu qu'avant. Le clic gauche ouvre la
  fenêtre. Windows garde l'icône de Tauri.
- Sans hôte de zone de notification (bureau qui n'en a pas), l'icône n'est pas créée et fermer la
  fenêtre quitte, comme avant.
- Le banc lance un hôte de zone minimal sur son bus privé (`examples/hote-zone.rs`) qui relit ce
  qu'un panneau verrait. Vérifié : banc `--jetable` 59/59 et `--fichier` 14/14, avec titre, quatre
  tailles d'images et menu lus par l'hôte.

## Deux sens avec toute la photothèque

- Le double sens ne reliait un dossier qu'à un album. Il peut désormais relier un dossier de
  l'ordinateur à toute la photothèque (ou à ce qui a été pris depuis une date) : le dossier part
  sur le serveur comme un envoi (photothèque seule ou un album par sous-dossier), et les photos
  du serveur absentes du dossier descendent dans un sous-dossier « Depuis Immich » (ou directement
  dans le dossier), par année et mois.
- Pas de doublon : une photo est reconnue à son contenu (SHA-1), où qu'elle soit dans le dossier
  et quel que soit son nom. Elle n'est ni renvoyée ni retéléchargée. Les photos reçues sont notées
  comme connues : elles ne sont ni ré-analysées au passage suivant, ni rangées dans des albums.
- Rien n'est supprimé ni écrasé : pas de suppressions répercutées avec la photothèque (refusé par
  le moteur). Une photo reçue effacée ou déplacée ici n'est pas retéléchargée ; « recevoir de
  nouveau » ne reprend que les échecs pour un double sens.
- Tant qu'un fichier du dossier est en cours d'écriture, la réception attend le passage suivant :
  sinon sa copie du serveur pourrait descendre en double.
- Avant d'ajouter : nombre et volume du dossier, du serveur, et place nécessaire (au moins ce que
  le dossier n'a pas déjà) ; refus si le disque ne suffit pas.
- Schéma de la base : version 5 (colonne `dossier_recus`), sauvegarde automatique avant migration.
- Vérifié : test d'intégration `double_sens_avec_la_phototheque` (copie renommée au fond d'un
  sous-dossier non retéléchargée, nouvelle photo envoyée seule, pas d'album pour les reçues, fichier
  déplacé ni retéléchargé ni renvoyé), suite jetable du moteur 13/13, banc `--jetable` 62/62.

## Arrêt du 9 novembre à 19 h 07 et veille de l'interface

- Nicéphore s'est fermé pendant la grande synchro : KWin a coupé sa connexion (« Data too big for
  buffer… error in client communication »), le fil principal ne lisant plus les messages de
  l'affichage. La cause du blocage n'est pas encore établie.
- Nicéphore surveille désormais son fil principal (`veille_interface.rs`) : au-delà de 3 s sans
  réponse, « interface figée » dans le journal, puis la durée du gel quand il repart. Le banc
  `--fichier` provoque un gel de 5 s et vérifie les deux messages.

## Visionneuse : RAW sans message transitoire

- En passant d'une photo à un RAW, « Aperçu indisponible pour ce format » s'affichait environ une
  seconde avant l'aperçu. La photo précédente reste maintenant affichée jusqu'à ce que la suivante
  soit prête (aperçu du RAW ou vidéo compris).
- L'aperçu déjà extrait ne relit plus tout le RAW (25 Mo et plus) : taille lue dans le petit JPEG
  en cache, orientation dans le premier Mo du RAW. Les aperçus des fichiers précédent et suivant
  sont préparés à l'avance.
- Vérifié : le banc `--fichier` guette le message pendant la navigation ; il échoue sur l'ancienne
  visionneuse et passe (15/15) sur la nouvelle.

## RAW laissés de côté : plus de téléchargements en retard, interrupteur visible

- Les RAW mis en file de téléchargement avant que l'option soit décochée descendaient quand même
  (259 DNG du téléphone chez Valentin). Décocher l'option les oublie, et chaque réception les
  oublie aussi par sécurité ; les RAW déjà reçus restent connus. Test unitaire
  `sans_raw_oublie_les_raw_a_recevoir`, suite jetable du moteur 13/13.
- Sur la carte d'un dossier, l'option RAW était un lien discret, pas reconnu comme cliquable :
  c'est maintenant un interrupteur « Inclure les fichiers RAW (DNG, ARW, CR2…) ». Banc
  `--jetable` 62/62.

## Cause des gels et de l'arrêt du 9 novembre : trouvée et corrigée

- Capturé sur le vif par la veille de l'interface (gels de 3,7 et 4 s vers 1 h, pile d'appels du
  fil principal relevée avec eu-stack) : une commande de l'interface (`regles_envoi`, appelée à
  chaque fichier envoyé par la page Synchro) tournait sur le fil principal et attendait le verrou de
  la base d'envoi, tenu par le moteur pendant ses longues opérations. Le fil principal ne lisait
  plus les messages de l'affichage ; au-delà d'1 Mo en attente, KWin coupe la connexion (19 h 07).
- Correctifs :
  - toutes les commandes qui touchent la base d'envoi ou le disque passent hors du fil principal
    (`#[tauri::command(async)]`) ;
  - le moteur ne tient plus la base pendant les accès disque (présence des fichiers reçus vérifiée
    avant), et regroupe ses écritures en transactions (vérification des empreintes, liens
    dossier/serveur, réceptions).
- Vérifié : le banc `--fichier` fait tenir la base 5 s par un autre fil pendant qu'une commande la
  demande ; la commande attend bien 4,8 s, sans gel signalé. Avec la commande remise sur le fil
  principal, le banc compte 2 gels et échoue. Banc `--jetable` 62/62, suite jetable du moteur 13/13.

## Clic droit

- Plus de menu du navigateur (recharger, inspecter…) au clic droit, sauf dans les champs de texte
  (copier, coller). Vérifié au banc `--fichier`.

## Fichiers déplacés dans un dossier synchronisé (10/11)

- Constaté après le rangement d'Images : 16 522 photos liées à leur copie du serveur passaient en
  « effacé ici ». Le moteur les cherchait à leur ancien chemin, et la table des envois gardait
  l'ancien chemin à côté du nouveau (16 525 photos comptées deux fois).
- Plus grave, vérifié au banc : dans un dossier en miroir, une photo simplement déplacée était
  retirée de l'album après le délai de grâce.
- Correctifs, valables pour tout dossier synchronisé :
  - les chemins qui ont quitté le dossier sont oubliés à chaque passage. Rien n'est oublié si le
    dossier paraît vide (disque débranché) ou là où il est illisible ;
  - une photo reçue ou liée qui n'est plus à son chemin est retrouvée par son contenu sous son
    nouveau chemin ; elle n'est ni « effacée ici », ni retéléchargée, ni retirée de l'album ;
  - les photos marquées à tort « effacé ici » reviennent d'elles-mêmes à « reçu ».
  - Côté serveur, rien ne change : les photos y sont suivies par leur identifiant Immich, qu'Immich
    range ou renomme ses fichiers.
- Vérifié : deux nouveaux cas dans la suite jetable du moteur (déplacement en double sens avec la
  photothèque, déplacement en miroir), qui échouent tous les deux sur l'ancien moteur. Suite 13/13.

## Contrôles façon Immich (10/11)

- Vidéo : la barre de lecture du vrai Immich (media-chrome, licence MIT) remplace celle du moteur
  web. Elle a lecture, temps, volume qui s'ouvre au survol, plein écran, vitesse dans « … » et
  barre de progression fine, dans la visionneuse et pour les vidéos de l'ordinateur. Les flèches
  gauche et droite restent à la navigation entre fichiers.
- Listes déroulantes (tri des albums, des lieux, des personnes, type de la recherche) : le menu
  Select d'@immich/ui.
- Dates (recherche, dialogue de synchro, date de prise de vue) : le calendrier DatePicker et
  l'heure TimeInput d'@immich/ui, en français (`setLocale` et `setTranslations` dans le layout).
- Vérifié : banc `--fichier` 19/19 (commandes du lecteur, pause par son bouton), banc `--jetable`
  65/65 (aucune liste du système, menu de tri, saisie de la date de prise de vue et d'une période
  au clavier).

## Types de synchronisation nommés court (11/11)

Les quatre choix de « Ajouter un dossier synchronisé » étaient des phrases. Ils prennent des noms courts, avec les mots de l'app mobile Immich (« Sauvegarde ») et des clients de synchro habituels (« Fichiers virtuels ») : Sauvegarde (Ce dossier → Immich), Copie locale (Immich → ce dossier), Fichiers virtuels (À la demande), Synchro complète (Immich ↔ ce dossier). La légende devient « Type de synchronisation ». Vérifié par le banc (« types de synchro nommés court », capture e11-types-synchro).

## Panneau de recherche et panneau latéral façon Immich web (11/11)

Recherche, repris du client web Immich (SearchBar, SearchFilters et leurs sections) :
- Le champ du haut ouvre le même panneau : recherches récentes (5, effaçables), « Filtrer par » (Rechercher par type, Personnes, Date, Lieux, Étiquettes, Média), « Filtres avancés » (Appareil photo : marque, modèle, objectif ; Étoile d'évaluation ; Bibliothèque : Favoris, Archive, Pas dans un album), « Effacer tout » et « Recherche ». Textes repris de la traduction française officielle d'Immich.
- Les boutons de « Filtrer par » prennent la valeur choisie (« Alice + 1 personne », « Cette année », la ville...), comme dans Immich.
- Les listes de pays, régions, villes, marques, modèles et objectifs viennent de `GET /search/suggestions`, filtrées par les choix déjà faits (commande `suggestions_recherche`). Champ à liste filtrable `ChampChoix`, au style du Combobox d'Immich.
- Nouveaux critères envoyés au serveur : région (`state`), marque, modèle, objectif, pas dans un album, note, étiquettes, et le type « Chemin complet ou dossier » (`originalPath`). Ils passent par l'adresse `/recherche?...` comme les autres.
- La page de résultats perd sa barre de filtres : les critères actifs y sont des puces façon Immich (libellé, valeur, croix pour retirer le filtre).

Panneau latéral, repris de UserSidebar (NavbarItem et NavbarGroup d'@immich/ui) :
- Les trois albums modifiés en dernier sous « Albums », dépliables (choix retenu).
- En bas, le stockage (quota de l'utilisateur s'il en a un, sinon le disque du serveur, comme Immich), « Serveur en ligne » et sa version, et l'encadré « Nouvelle mise à jour ! » quand le serveur connaît une version plus récente (administrateurs seulement). Commande `infos_serveur`, rafraîchie chaque minute.
- Le mode replié (fenêtre étroite) garde les icônes seules.

Banc (`--jetable`) : étapes 15 et 16 (albums récents, stockage, serveur en ligne ; panneau de recherche, filtres avancés, suggestions, recherche lancée, puces, retrait d'une puce), captures e10 à e12.

Banc : le nettoyage des services du bus privé (ajouté le 11/11 contre la fuite mémoire) tuait aussi le service des dossiers virtuels, que le banc vérifie justement vivant après la fermeture de l'appli. Le nettoyage (`nettoyer_bus`) passe après ce contrôle, et remplace le `pkill` par chemin du binaire, qui aurait pu tuer le service de la vraie session.

### Au pixel près (11/11, après comparaison avec Immich web)

- État par défaut identique à Immich : « Rechercher par type » ouvert, filtres avancés repliés (dépliage animé par grid-template-rows, contenu inerte une fois replié).
- Les filtres suivent le compte et le serveur comme dans Immich (commande `options_recherche`) : « Étiquettes » et la note seulement si l'utilisateur les a activées dans ses préférences, « Contexte » et « OCR » seulement si le serveur a la recherche intelligente et l'OCR.
- Le bouton « Rechercher par type » affiche le type choisi quand ce n'est pas « Contexte ».
- Classes du client web reprises telles quelles : champ `py-4 ps-14`, `rounded-3xl` puis `rounded-t-3xl` à l'ouverture, ombre `0 8px 20px rgba(0,0,0,.12)`, panneau `rounded-b-3xl` sur `immich-dark-gray` (#212121) en texte gray-300, séparateurs `bg-light-200 / dark:bg-dark-600`, ouverture en `fly` (25 px, 250 ms), une seule rangée de visages jusqu'à « Voir toutes les personnes », filtre des personnes et historique au style d'Immich.
- En-tête disposé comme la barre de navigation d'Immich : 76 px de haut, une première colonne de la largeur du panneau latéral (256 px), puis la recherche jusqu'à 64rem.
- Police d'Immich dans toute l'appli : Google Sans (SIL OFL 1.1, licence dans `app/static/polices/OFL.txt`), le fichier livré par Immich réduit au latin, aux flèches et à la ponctuation (89 Ko en woff2 au lieu de 4,8 Mo), mêmes réglages (`size-adjust` et `ascent-override` à 106,25 %, espacement 0,1 px). Couleurs `immich-*` déclarées dans `app.css`.
- Listes des champs à choix (pays, appareil...) en `fixed`, placées depuis le champ comme le Combobox d'Immich : plus rognées par le panneau qui défile, ouvertes vers le haut s'il manque 225 px en bas, refermées quand le champ sort de la vue (retour de Valentin, 11/11).
- Cadre de la barre focalisable (`tabindex="-1"`, comme Immich) : un clic sur le texte du panneau ne le referme plus.

## Interface multilingue (11/11)

- Mécanique maison sans dépendance, `app/src/lib/i18n.svelte.ts` : le texte français sert de clé (`t('Rechercher par type')`, `tp(n, '{n} photo', '{n} photos')`), `locale()` pour les dates et nombres. Repli : langue choisie, puis anglais, puis français. Langues chargées à la demande.
- Anglais écrit pour Nicéphore (`traductions/en.json`, 621 textes), formulation officielle d'Immich pour ce qui est commun.
- Autres langues : traductions de la communauté Immich (Weblate), pour les 186 textes que Nicéphore partage avec Immich. `npm run traductions` (`app/scripts/traductions-immich.mjs`) les récupère depuis immich-app/immich et écrit un fichier par langue (76 langues). Correspondances automatiques par texte français ou anglais identique, plus `traductions/immich-cles.json` pour forcer une clé (avec renommage des valeurs, `{n}` vers `{count}`) ou en refuser une. Les pluriels ICU d'Immich (`{count, plural, one {...} few {...} other {...}}`) sont évalués par `t`/`tp` avec les catégories de la langue.
- La publication (`.github/workflows/publication.yml`) lance `npm run traductions` avant de construire : chaque version embarque les dernières traductions de la communauté.
- Réglages : liste « Langue » (Comme le système, Français, English, puis les langues d'Immich avec la part traduite).
- Conversion des 59 fichiers faite par trois agents Sonnet sur des lots disjoints, d'après un fichier modèle (`BarreLaterale.svelte`). Bancs en français inchangés (80/80, 19/19) ; banc en anglais pour les captures (ses contrôles cherchent du français, d'où les échecs attendus).
- Reste en français : les messages d'erreur venant de Rust, le nom du dossier réel « Depuis Immich ».
- Alignés sur le français d'Immich pour hériter de ses traductions (11/11, accord de Valentin) : « Cacher les personnes non nommées », « Rechercher des albums », « Rechercher une personne », « Aucune personne nommée « {nom} » » (avec le nom cherché), « Réduire » (bouton de fenêtre). Format `{count, number}` d'Immich pris en charge (« ajouté(s) aux favoris »). Non alignés car Immich perdrait une information : nom de l'album, « toujours dans la photothèque », explication du GPS, nom du tag, titre de la fiche fusionnée. 191 textes communs.
- Traductions automatiques (11/11, budget validé par Valentin) : les 456 textes sans équivalent Immich traduits dans 26 langues (de, es, pt-BR, it, nl, zh-Hans, ru, pl, ja, uk, cs, sv, tr, ko, zh-Hant, da, nb-NO, fi, hu, ro, ca, el, vi, id, pt, he) par 13 agents Haiku, deux langues chacun, depuis un fichier source numéroté (les agents ne recopient pas les clés françaises). Rangées dans `traductions/auto/<langue>.json`, que le script Immich ne touche jamais ; à l'exécution, la traduction de la communauté Immich passe devant, puis la traduction automatique, puis l'anglais. Contrôle automatique : toutes les clés, mêmes valeurs entre accolades, pluriels en deux formes ou en bloc ICU (formes multiples du russe, polonais, ukrainien, tchèque, roumain, hébreu). Les 26 langues sont à 100 %, les 50 autres à environ 31 % (communauté Immich) avec le reste en anglais. Pour un texte ajouté plus tard, il faudra traduire seulement les nouveautés.

### Homonymes des traductions Immich (11/11, soir)

Un même texte français a souvent plusieurs clés chez Immich, avec des sens
différents : « Annuler » est `cancel` et `undo`, « Modèle » `model` et
`template`, « Personnes » `people` et le titre en capitales `PEOPLE`. Le
script prenait la dernière clé trouvée, d'où « Rückgängig » pour Annuler ou
« PERSONEN » dans la barre latérale en allemand. Il départage maintenant par
notre anglais (`en.json`) : la clé dont l'anglais Immich est le nôtre, casse
égale d'abord, puis la clé qui porte le nom du mot (`trash` avant `to_trash`),
puis la plus courte (`location` avant `edit_location_dialog_title`). Dix
textes changent de clé ; dans quelques petites langues encore peu traduites,
la bonne clé n'a pas de traduction et le texte retombe en anglais plutôt que
de garder un faux sens.

### Préparation de la v0.1.0 (11/11, soir)

- Version 0.1.0 partout (Cargo, package.json, tauri.conf.json).
- Clé de signature des mises à jour générée : clé privée et mot de passe
  hors du dépôt (`~/.config/claude-secrets/nicephore-signature/`, à ranger
  dans Vaultwarden), clé publique posée dans la variable GitHub
  `NICEPHORE_CLE_PUBLIQUE`. Les deux secrets `TAURI_SIGNING_PRIVATE_KEY` et
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sont posés par Valentin.
- Paquet `.deb` en plus de l'AppImage et du rpm ; deb et rpm dépendent de
  `fuse3` (fusermount3, pour les fichiers virtuels).
- README réécrit avec des captures : `scripts/vitrine.sh demarrer` monte un
  second serveur Immich local (port 2298, projet Docker
  `nicephore-immich-vitrine`) rempli de 51 photos CC0 de Wikimedia Commons
  (`tests/vitrine/photos.json` : sources, auteurs, lieux, dates de
  « voyages »), puis `vitrine.sh captures` lance `banc.sh --vitrine` en
  anglais : fenêtre agrandie, frise, recherche, albums, visionneuse, carte,
  explorer, ajout d'un dossier synchronisé (`banc/<date>/v*.png`). Le profil
  du banc vitrine est entièrement privé (config, données et cache), pour que
  l'historique de recherche réel n'apparaisse pas.
- Écart repéré avec Immich web : la frise met chaque jour sur sa propre
  ligne, Immich range plusieurs jours côte à côte quand ils tiennent.
- Premier passage de la CI Windows depuis longtemps, trois défauts propres à
  Windows corrigés : le composant `Notifications.svelte` masquait le module
  `notifications.svelte.ts` (Windows ignore la casse : renommé
  `PileNotifications.svelte`) ; l'outil de banc `hote-zone` (zbus) est
  réservé à Linux ; les noms de fichiers téléchargés sont découpés à la main
  (`Path` lisait « a:b.jpg » comme le lecteur a:) et les noms réservés de
  Windows (CON, NUL, COM1...) reçoivent un « _ » devant, aussi pour la copie
  locale et les fichiers virtuels. Clippy et les tests compilent aussi en
  croisé pour Windows dans la toolbox
  (`cargo clippy --target x86_64-pc-windows-gnu ...`).

## Notes de version par fichier

La première publication a posé le texte provisoire du workflow (« Notes de version à écrire avant de publier ») sur la page de la version et dans `latest.json`, que la mise à jour intégrée affiche. Les notes vivent maintenant dans `docs/versions/<tag>.md`, obligatoire : sans lui la publication s'arrête avant de construire. Le texte avant la ligne `<!-- release page -->` est du texte brut, repris dans `latest.json` pour la fenêtre de mise à jour ; le fichier entier, en Markdown, devient la page de la version (un dernier job, une fois Linux et Windows construits). Notes de la 0.1.0 rangées dans `docs/versions/v0.1.0.md`.
