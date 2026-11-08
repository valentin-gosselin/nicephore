# Lot 4 : carte, souvenirs, diaporama (08/11/2026)

## Carte

- Page Carte (barre latérale) : les photos géolocalisées (`/map/markers`, 3 938 chez Valentin) sur
  MapLibre GL 6.15 avec les styles du serveur (`/server/config`, tuiles d'Immich par défaut), clair
  ou sombre selon le thème. Groupes avec leur nombre, photos isolées en points.
- Clic sur un groupe ou une photo : panneau à droite avec la grille habituelle (visionneuse,
  sélection, actions), jusqu'à 300 photos par groupe, lues par `/assets/{id}` 8 à la fois.
- Pièges trouvés au banc :
  - MapLibre ne devine l'adresse de son worker que sur une page http(s) ; Tauri sert
    `tauri://localhost`, donc le worker est importé par Vite (`?url`) et déclaré avec `setWorkerUrl`.
  - `maplibre-gl.css` impose `position: relative` au conteneur : un conteneur `absolute inset-0`
    tombait à 0 px de haut. Taille explicite à 100 %.
  - Les libellés des groupes réutilisent une police déjà fournie par le style du serveur.

## Souvenirs

- Bandeau « Il y a N ans » en haut de la page Photos (`/memories?for=<jour local>`), comme les
  applis Immich ; anniversaires gérés (« Anniversaire de X »).
- Page d'un souvenir : ses photos et un bouton « Diaporama ».

## Diaporama

- Dans toute visionneuse : bouton lecture et touche D ; 5 s par photo avec une barre de progression,
  les vidéos jusqu'à leur fin ; arrêt après la dernière.

## Vérifié au banc (lecture seule sur la vraie bibliothèque)

Souvenir du jour (2022, 2 éléments), page du souvenir, diaporama qui avance, carte affichée en
1048 × 764 avec ses groupes, clic sur un groupe qui ouvre 26 photos de Strasbourg. Défilement de la
chronologie toujours à 60 i/s.
