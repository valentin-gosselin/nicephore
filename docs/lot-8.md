# Lot 8 : mode portable (08/11/2026)

Pour emporter Nicéphore sur une clé USB, ou le lancer sur le PC du travail sans rien y laisser.

## Principe

- Un fichier nommé `portable` posé à côté du programme (l'exe sous Windows, l'AppImage sous Linux)
  range tout dans un dossier `donnees/` voisin :
  - `config/` : réglages, connexion chiffrée, préchargement ;
  - `envoi/` : base des dossiers envoyés ;
  - `cache/` : vignettes et aperçus ;
  - `webview/` : profil du moteur web (WebKitGTK, WebView2), qui irait sinon dans le profil de
    l'utilisateur. La fenêtre est donc créée par le code et non plus par la configuration, pour lui
    donner ce dossier.
- Rien dans le trousseau du système, ni dans le registre, ni dans le profil. Pas de démarrage avec
  la session (le réglage disparaît et la commande le refuse). L'icône dans la zone de notification
  reste possible : elle n'inscrit rien.
- Retirer le fichier `portable` rend le mode normal ; les données portables restent dans `donnees/`.
- `NICEPHORE_PORTABLE=<dossier>` force le mode portable (tests, développement).

## Connexion

- « Rester connecté » demande alors une phrase de passe (8 caractères au moins, saisie deux fois).
  Seul le jeton de session (ou la clé API) est gardé, scellé avec elle : Argon2id (64 Mio, 3 passes)
  pour la clé, XChaCha20-Poly1305 pour chiffrer et authentifier, sel et nonce tirés au hasard à
  chaque fois. Jamais le mot de passe.
- Au lancement suivant, un écran « Déverrouiller » demande la phrase ; une mauvaise phrase est
  refusée proprement (« phrase de passe incorrecte ») ; « Se connecter autrement » ramène à l'écran
  de connexion.
- Les Réglages affichent « Mode portable » avec le dossier, et l'origine de la connexion (« gardée à
  côté de Nicéphore et chiffrée par votre phrase de passe »).

## Vérifié

- Tests unitaires : coffre (scellé, rouvert, mauvaise phrase, deux scellés différents, phrase trop
  courte), compte scellé lu sans trousseau puis oublié, détection du fichier `portable`.
- `scripts/banc.sh --portable`, deux lancements sur le serveur jetable, sans clé de développement :
  1. écran de connexion piloté (adresse, e-mail et mot de passe du compte de test, phrase de passe),
     champs de phrase proposés, connexion, section « Mode portable », origine chiffrée annoncée, pas
     de démarrage avec la session (5/5) ;
  2. écran de déverrouillage, mauvaise phrase refusée, bonne phrase acceptée (3/3).
  Puis le script contrôle les fichiers : tout est dans `donnees/` (config, envoi, cache, webview),
  `compte.json` contient le coffre et aucun jeton en clair, et les dossiers config, données et cache
  du vrai profil sont restés identiques (la config n'est pas redirigée pendant ce test, exprès).
- Non-régression : scénario d'écriture 38/38, banc en lecture seule à 60 i/s.
- Reste à voir sous Windows (lot 9) : le profil WebView2 dans `donnees/webview` et l'exe portable.
