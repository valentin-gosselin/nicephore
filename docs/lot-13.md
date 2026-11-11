# Lot 13 : fichiers virtuels sous Windows (09/11/2026, non testé)

Fin de l'étape 4. Même service que sous Linux, avec l'API Cloud Files de Windows (celle des fichiers
« disponibles en ligne » de OneDrive) : le dossier choisi devient un dossier cloud de l'Explorateur.

## Fonctionnement (`crates/virtuel/src/windows.rs`)

- Le dossier (vide la première fois) est enregistré comme racine de synchronisation « Nicéphore »,
  propre à l'utilisateur ; chaque photo y est un espace réservé avec sa vraie taille et sa date.
- Ouvrir une photo l'hydrate en entier depuis le même cache vérifié que sous Linux (téléchargement
  de l'original, empreinte SHA-1 comparée, puis transfert par morceaux de 4 Mo avec la progression
  affichée par l'Explorateur).
- « Libérer de l'espace » est accepté (le contenu part, l'espace réservé reste) ; « Toujours garder
  sur cet appareil » est permis. Suppression et renommage sont refusés : lecture seule.
- Déconnexion à la sortie ou au changement de compte : les espaces réservés et les fichiers déjà
  téléchargés restent, et s'ouvrent de nouveau à la connexion suivante. Retirer le dossier le
  désenregistre (les fichiers téléchargés restent, les espaces réservés vides partent).
- À l'actualisation, les nouvelles photos de la sélection reçoivent leur espace réservé ; celles
  retirées sur le serveur restent (première version).

## Limites, à dire franchement

- Écrit d'après la crate `cloud-filter` 0.0.7, qui se déclare elle-même expérimentale, et ses
  tests. Je n'ai ni Windows ni chaîne de compilation Windows ici : le module a seulement été
  vérifié par le compilateur contre la cible `x86_64-pc-windows-gnu`, dans une crate isolée
  (types, traits, envoi entre fils). Il n'a jamais tourné.
- Première vraie compilation : l'intégration continue Windows (`ci.yml`) au premier envoi sur
  GitHub ; premier essai réel : dans la VM Windows du PC fixe, ou sur un PC Windows.

## Première compilation Windows (10/11/2026)

- Compilée ici, de façon croisée, dans la toolbox `immich-bureau` (MinGW installé par
  `podman exec -u root immich-bureau dnf install -y mingw64-gcc mingw64-gcc-c++`) :
  `npx tauri build --target x86_64-pc-windows-gnu --no-bundle` depuis `app/`. Passée du premier
  coup ; clippy propre sur les deux cibles après avoir réservé `par_service` à Linux.
- L'exe a besoin de `WebView2Loader.dll` à côté (produit dans le même dossier), et du runtime
  WebView2, présent sur Windows 10 et 11 à jour.
- Toujours jamais lancée : premier essai par Valentin sur un PC Windows au travail, en fichiers
  virtuels.

Premier essai réel (10/11/2026, PC du travail) : l'appli tourne, mais le montage du dossier virtuel paniquait (« provider name must not contain exclamation points »). L'identifiant passait la clé du dossier dans le nom du fournisseur, avec un `!`, que Windows réserve comme séparateur. Corrigé : fournisseur `Nicephore`, clé du dossier (FNV-1a du chemin en minuscules, stable d'une version de Rust à l'autre) dans le nom de compte.

Deuxième essai : « icon cannot be empty (0x80070057) ». Windows exige une icône à l'enregistrement du dossier ; on passe la première icône de notre exe (`<chemin de l'exe>,0`).

Troisième essai : enregistrement et connexion passent, mais la création du premier dossier fantôme échoue (« L'opération de cloud n'est pas valide », 0x8007017C). Le dossier est enregistré « toujours complet » (AlwaysFull) : Windows ne demande jamais le contenu d'un dossier, donc chaque dossier fantôme doit être créé avec DISABLE_ON_DEMAND_POPULATION (`has_no_children`), comme dans l'exemple CloudMirror de Microsoft. Les notifications passent aussi au-dessus des fenêtres (z-index 1000 au lieu de 60, les dialogues sont à 70) : l'erreur était cachée sous la fenêtre de synchro.
