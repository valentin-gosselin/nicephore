# Lot 9 : mises à jour signées, paquets, intégration continue (08/11/2026)

Dernier lot de l'étape 2 : de quoi publier de vraies versions.

## Mises à jour dans l'appli

- `tauri-plugin-updater` : manifeste `latest.json` publié avec chaque version sur GitHub, fichiers
  signés (minisign), signature vérifiée avec la clé publique avant toute installation.
- La clé publique n'est pas dans le dépôt : la CI la donne à la publication (`--config`). Une copie
  compilée localement n'a donc pas de mises à jour, et le dit dans les Réglages.
- Réglages, section « Mises à jour » : version actuelle, « Rechercher », notes de la nouvelle
  version, « Installer et redémarrer » avec la progression du téléchargement. Recherche
  automatique 20 s après le lancement ; une pastille « Nicéphore x.y.z disponible » apparaît alors
  dans l'en-tête.
- Qui se remplace tout seul :
  - l'AppImage, en place, l'ancienne gardée à côté (`Nicephore.AppImage.ancienne`) pour revenir en
    arrière à la main ;
  - la version Windows installée (installeur NSIS, mode « passif ») ;
  - pas le rpm (Discover ou dnf s'en chargent), ni la copie portable (le programme se remplace à
    la main sur le support, un installeur la mettrait dans le profil) : ces copies sont seulement
    prévenues, avec l'adresse de la page des versions.
- Pas encore : canal bêta, mise à jour de l'exe portable Windows par échange au redémarrage.

## Paquets

- Linux : AppImage (environ 115 Mo, WebKitGTK embarqué) et rpm ; Windows : installeur NSIS et
  zip portable (exe + fichier `portable`). Cibles dans `tauri.conf.json`, artefacts de mise à jour
  signés (`createUpdaterArtifacts`).

## Intégration continue (GitHub Actions)

- `.github/workflows/ci.yml`, à chaque envoi sur `main` et chaque demande de fusion, sous Ubuntu
  24.04 et Windows : vérification de l'interface (svelte-check), construction de l'interface,
  clippy sans avertissement toléré, tests unitaires. C'est la première compilation du code Windows
  (registre, batterie, WebView2) ; il a seulement été vérifié ici dans une petite crate isolée
  contre la cible `x86_64-pc-windows-gnu`.
- `.github/workflows/publication.yml`, sur une étiquette `v*` : construction Ubuntu 22.04 (pour
  que l'AppImage tourne aussi sur les systèmes plus anciens) et Windows, version en brouillon avec
  `latest.json`, puis zip portable Windows ajouté. Il faut d'abord, dans les réglages du dépôt :
  les secrets `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, et la variable
  `NICEPHORE_CLE_PUBLIQUE`.
- À prévoir : signature de code Windows (SignPath, gratuit pour les projets libres) contre
  SmartScreen.

## Vérifié

- `scripts/essai-mise-a-jour.sh --construire` : une paire de clés de test jetable (jamais la vraie),
  deux AppImage 0.0.1 et 0.0.2 signées avec elle, un manifeste servi sur 127.0.0.1:8799 (http permis
  pour ce seul essai), puis l'AppImage 0.0.1 lancée dans la session KWin virtuelle : section Mises à
  jour, 0.0.2 trouvée avec ses notes, installable, installée, redémarrée en 0.0.2 ; ensuite le script
  constate que le fichier est bien la 0.0.2 et que l'ancienne est gardée à côté, identique à la
  0.0.1. Une seule récupération de la 0.0.2 sur le serveur local.
- `--signature-fausse` : la signature de la 0.0.1 présentée pour le fichier 0.0.2 ; l'installation
  est refusée, l'erreur s'affiche et l'AppImage reste la 0.0.1.
- Non-régression : tests unitaires, scénario d'écriture, banc en lecture seule.

## Décision qui t'attend

La vraie clé de signature des mises à jour : qui la génère et où elle est gardée (elle ne doit
jamais être perdue, sinon les copies installées ne pourront plus se mettre à jour). Le circuit est
prêt ; il ne manque que la clé, ses deux secrets et la variable dans GitHub.
