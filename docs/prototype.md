# Prototype (étape 0) : résultats du 08/11/2026

## Ce qu'il fait

- Connexion par clé API (variables `IMMICH_URL` et `IMMICH_API_KEY`, ou écran de connexion ; la clé
  reste en mémoire, rien n'est écrit en clair).
- Timeline complète en rangées justifiées, groupées par mois puis par jour, virtualisée : index des
  mois d'abord, mois chargés à la demande (4 en parallèle, les plus proches de l'écran d'abord),
  hauteurs estimées en attendant, position conservée quand un mois change de hauteur.
- Vignettes servies par le protocole `vignette://` : cache disque partagé, requêtes en double fusionnées,
  16 petites et 6 grandes en parallèle, écriture atomique, clé de cache liée au thumbhash, élagage
  au-delà de 4 Go. Placeholders flous thumbhash pendant le chargement. Aucune requête pendant un
  défilement éclair (plus de 3 écrans par seconde), délai de 120 ms avant de demander une image.
- Frise des années à droite (survol = mois, clic ou glisser = saut), zoom Ctrl+molette ou Ctrl +/-/0,
  touches Début et Fin.
- Visionneuse plein cadre : aperçu écran puis pleine résolution au zoom (molette autour du pointeur,
  glisser pour déplacer, double-clic), flèches, F pour le plein écran, I pour le panneau d'infos
  (date dans le fuseau de la prise de vue, fichier, appareil, réglages, lieu, personnes), Échap.
- Vidéo lue par un proxy local (127.0.0.1, jeton aléatoire, en-tête Range transmis, clé jamais exposée).
- Thème clair ou sombre selon le système, composants `@immich/ui`.

## Mesures (banc automatique, KWin Wayland virtuel 1600 × 1000, sans carte graphique)

Bibliothèque réelle : 15 599 éléments, 198 mois (1960 à 2026), serveur Immich 3.3.6 sur le .18.

| Mesure | Résultat |
|---|---|
| Premier écran rempli | 150 à 300 ms |
| Défilement 1,5 écran/s pendant 8 s | 60 i/s, 95 % des images sous 18 ms, 0 saccade, 100 % des vignettes affichées |
| Défilement éclair 8 écrans/s pendant 3 s | 60 i/s, 0 saccade (vignettes volontairement non chargées) |
| Écran rempli après l'arrêt d'un défilement éclair | 350 ms |
| Saut à 50 % ou 90 % de la timeline | écran rempli en 300 à 470 ms |
| Images servies (cache froid) | 22 ms en moyenne, 55 ms au pire |
| Vidéo de 39 s | prête et en lecture après 4 s |

Conclusion : WebKitGTK tient sous KDE Wayland. Le repli vers Slint n'est pas nécessaire.

## Bugs trouvés et corrigés pendant les essais

1. Hauteurs de mois nulles avant la première mesure de largeur : la recherche du mois en haut de
   l'écran renvoyait le dernier, et la vue sautait tout en bas (sur les photos de 1960).
2. Vitesse de défilement calculée entre deux événements trop rapprochés : l'appli croyait à un
   défilement éclair permanent et ne chargeait que 11 % des vignettes.
3. Clé en double dans les titres de jours (un même jour peut revenir dans un mois à cause des fuseaux).
4. Infobulles de `@immich/ui` sans leur fournisseur : la visionneuse plantait.
5. Fuseaux Immich du type « UTC+1 » refusés par `toLocaleString`.

## Lancer

- Banc invisible : `scripts/banc.sh` (résultats et captures dans `banc/<date>/`).
- Appli sur le bureau : lanceur « Nicéphore » dans le menu (`scripts/installer-lanceur.sh` pose le
  fichier `.desktop` et les icônes), qui appelle `scripts/lancer.sh`. Connexion de développement lue
  dans `~/.config/nicephore/dev.env` (`IMMICH_URL`, `IMMICH_API_KEY`).
- Modes de rendu essayés dans l'ordre jusqu'à ce que l'appli tienne 15 s : `natif` (Wayland, le plus
  fluide), `dmabuf` (corrige le plantage NVIDIA « Error 71 » mais divise la fluidité par deux : 26 i/s
  et 77 saccades contre 59 i/s et 0 au banc), `compat` (X11, recette de Music Assistant). Le mode qui
  marche est retenu dans `~/.cache/nicephore/mode` ; `lancer.sh --oublier` reteste tout,
  `--mode compat` force un mode. Journal : `~/.cache/nicephore/dernier-lancement.log`.
- Compilation : `cd app && npx tauri build --no-bundle` (sur le PC fixe, dans la toolbox de dev).

## Pas encore vérifié

- Le rendu sur le vrai bureau avec la carte NVIDIA (le lanceur se replie seul en cas de plantage au
  démarrage ; un écran blanc sans plantage se règle avec `lancer.sh --mode dmabuf`).
- Windows (compilation croisée ou VM winapps).
- Le défilement à la molette réelle (le banc fait défiler par programme).

## Limites connues du prototype

- CSP désactivée (`csp: null`) : à resserrer avant toute version publique.
- Clé API en mémoire seulement (le trousseau arrive avec l'écran de réglages).
- Pas de sélection multiple, de recherche, d'albums ni de personnes : étape 1.
- Textes d'interface en français en dur : passeront par des fichiers de traduction.
