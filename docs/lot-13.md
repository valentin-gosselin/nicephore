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
