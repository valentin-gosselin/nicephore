# Lot 10 : réception, la synchro descendante (09/11/2026)

Premier lot de l'étape 3. Une sélection d'Immich est copiée dans un dossier de l'ordinateur et
tenue à jour.

## Interface

- La page « Envoi » devient « Synchro » (barre latérale, « Cet ordinateur »). Chaque dossier montre
  son sens (icône dossier qui monte ou qui descend).
- « Ajouter un dossier » commence par le sens : envoyer un dossier, ou recevoir des photos. Pour
  recevoir : un album, une personne, les favoris, une période (du, au) ou toute la photothèque ;
  rangement « tout dans le dossier » ou « par année et mois » (proposé d'office pour une période ou
  la photothèque). Avant de valider : nombre de photos, volume et place libre sur le disque ; le
  bouton porte le volume (« Recevoir 12 Go ») et reste grisé s'il n'y a pas la place (avec 1 Go de
  marge).
- Sur la page : reçus, à recevoir, retirés sur le serveur et gardés ici, et un lien « recevoir de
  nouveau » pour ce qui a été effacé ici ou a échoué.

## Moteur

- Base au schéma 2 : sens des règles, sélection, rangement, table des fichiers reçus. Migration
  depuis le schéma 1 avec copie `envoi-<clé>.sqlite.v1.sauvegarde`.
- Un passage de réception : liste complète de la sélection sur le serveur (une page en échec fait
  échouer tout le passage, jamais de liste partielle) ; noms réservés sans collision (sans tenir
  compte de la casse, comme sous Windows ; caractères interdits sous Windows remplacés) ; fichier
  déjà présent sous son nom avec la même empreinte adopté sans téléchargement ; place libre
  vérifiée avant de commencer ; téléchargement dans un fichier caché `.nicephore-<id>.part`,
  empreinte SHA-1 comparée à celle du serveur, date de modification d'origine, puis mise en place
  par renommage, jamais par-dessus un fichier existant.
- Ce qui ne se fait jamais : supprimer ou écraser un fichier local. Une photo effacée ici n'est pas
  retéléchargée (sauf demande) ; une photo retirée de la sélection sur le serveur reste ici ; un
  dossier absent ou illisible (disque débranché) arrête la règle sans rien conclure.
- La pause et les changements de règles arrêtent la réception aussitôt ; les téléchargements en
  cours effacent leur fichier temporaire, et un passage commence en effaçant ceux d'un arrêt
  brutal.
- Les dossiers de réception ne sont pas surveillés (le serveur est leur source) : un passage toutes
  les 15 minutes, ou « Vérifier maintenant ».

## Vérifié

- Tests unitaires : migration du schéma 1 avec sauvegarde, noms libres et sûrs, sous-dossiers par
  année et mois.
- Contre le serveur jetable : album reçu à plat (copie identique adoptée, homonyme différent jamais
  écrasé et photo arrivée à côté, aucun fichier temporaire restant), fichier effacé ici non
  retéléchargé puis retéléchargé sur demande, photo retirée de l'album gardée ici ; période rangée
  par année et mois ; pause en pleine réception de 60 photos de 2 Mo (rien de plus, aucun fichier
  partiel).
- Dans l'appli (48/48) : boîte d'ajout jusqu'à l'estimation, période de février 2025 reçue en
  entier (6 photos dans `2025/02/`), affichage sur la page, retrait. Portable, comptes et lecture
  seule sans régression.
