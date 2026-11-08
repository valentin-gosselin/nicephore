# Lot 2 : préchargement des vignettes, recherche, Explorer (08/11/2026)

## Préchargement des vignettes

- Tâche de fond au lancement (réglage « Garder toutes les vignettes », actif par défaut) : parcourt
  la chronologie et l'archive, télécharge chaque vignette absente du disque, 4 à la fois, en pause
  2 s après chaque défilement, arrêtée à la déconnexion ou quand on coupe le réglage.
- Mesures sur la bibliothèque réelle : 15 555 vignettes en 57 s pour 251 Mo ; passage suivant en 3 s.
  46 éléments n'ont pas de vignette sur le serveur (réponse 404), comptés à part.
- Budgets de cache séparés : 2 Go de vignettes, 4 Go d'aperçus (les aperçus ne chassent plus les
  vignettes).
- Quand le dernier passage était complet (retenu dans `prechargement.json`), les vignettes s'affichent
  aussi pendant un défilement éclair : 100 % affichées à 8 écrans/s, 60 i/s, au lieu de 0 %.
- Les descriptions des mois se chargent aussi pendant les défilements rapides (requêtes légères).

## Recherche

- Champ en haut de la fenêtre (Ctrl+K ou `/`), page `/recherche` dont les critères vivent dans l'URL.
- Modes : contenu (recherche intelligente CLIP, `/search/smart`), nom de fichier, description, texte
  dans l'image (OCR), ces trois-là par `/search/metadata`.
- Filtres : photos ou vidéos, période, favoris, archive, ville, personnes (pastilles retirables).
- Résultats dans la même grille, avec visionneuse et sélection, chargés par pages de 150 en
  approchant du bas (`ChronologieListe`).

## Explorer

- Personnes nommées (117 sur 943 chez Valentin), favoris d'abord, visages sans nom sur demande ;
  visages servis par `vignette://localhost/personne/<id>` (cache disque).
- Lieux : une photo par ville (`/search/explore`), clic vers la recherche filtrée sur la ville.
- Page d'une personne : sa chronologie (`personId` de `/timeline/buckets`).

## Ajouts demandés après essai

- Boutons retour et avant dans l'en-tête (pas de barre de navigateur dans l'appli), aussi par
  Alt+flèches et par les boutons latéraux de la souris. Retour ferme d'abord la visionneuse. Les
  boutons se grisent au bout de l'historique (position suivie depuis `afterNavigate`) ; un
  changement de filtre de recherche remplace l'entrée au lieu d'en ajouter une.
- Tri des personnes et des lieux dans Explorer : nombre de photos (par défaut), nom de A à Z, de Z
  à A, retenu d'une fois sur l'autre. Nombres de photos lus par `/people/{id}/statistics` et
  `/search/statistics` (filtre ville), 8 requêtes à la fois, affichés sous chaque nom.

## Vérifié au banc (KWin virtuel, lecture seule)

Recherche « concert », vidéos depuis 2025, Explorer, page d'une personne (181 éléments), défilement
à 60 i/s sans saccade. Tests d'intégration : recherche, lieux, personnes, chronologie d'une personne.

## À tester à la main

1. Réglages : la ligne « Vignettes sur cet ordinateur » progresse au premier lancement puis indique
   « à jour » ; un défilement rapide dans les années anciennes montre les vignettes tout de suite.
2. Recherche : quelques recherches par contenu, puis les filtres (dates, vidéos, favoris).
3. Explorer : une personne, un lieu ; la visionneuse et la sélection marchent dans les résultats.
