"""Seeds the showcase Immich server used for the README screenshots: an
account, the CC0 photos listed in photos.json (downloaded once from
Wikimedia Commons), their date and place, albums and favorites, and writes
the connection details to <data dir>/test.env. Standard library only.

Usage: remplir.py <server url> <data dir>"""

import json
import pathlib
import sys
import time
import urllib.error
import urllib.request
import uuid

URL = sys.argv[1].rstrip("/")
DONNEES = pathlib.Path(sys.argv[2])
LISTE = pathlib.Path(__file__).with_name("photos.json")
EMAIL = "demo@nicephore.invalid"
MOT_DE_PASSE = "nicephore-demo"
AGENT = {"User-Agent": "nicephore-vitrine/0.1 (https://github.com/valentin-gosselin/nicephore)"}


def appel(methode, chemin, corps=None, jeton=None, brut=None, entetes=None):
    donnees = brut if brut is not None else (json.dumps(corps).encode() if corps is not None else None)
    requete = urllib.request.Request(f"{URL}/api/{chemin}", data=donnees, method=methode)
    if brut is None:
        requete.add_header("Content-Type", "application/json")
    for nom, valeur in (entetes or {}).items():
        requete.add_header(nom, valeur)
    if jeton:
        requete.add_header("Authorization", f"Bearer {jeton}")
    with urllib.request.urlopen(requete, timeout=120) as reponse:
        texte = reponse.read()
        return json.loads(texte) if texte else None


def attendre_serveur():
    for _ in range(180):
        try:
            if appel("GET", "server/ping")["res"] == "pong":
                return
        except (urllib.error.URLError, ConnectionError, OSError):
            pass
        time.sleep(1)
    sys.exit("le serveur Immich de la vitrine ne répond pas")


def telecharger(photo, chemin):
    """Downloads a photo from Commons once, gently (Commons limits bots)."""
    if chemin.exists():
        return
    for essai in range(5):
        try:
            requete = urllib.request.Request(photo["source"], headers=AGENT)
            chemin.write_bytes(urllib.request.urlopen(requete, timeout=120).read())
            time.sleep(1)
            return
        except urllib.error.HTTPError as e:
            if e.code != 429:
                raise
            time.sleep(10 * (essai + 1))
    sys.exit(f"téléchargement impossible : {photo['source']}")


def envoyer(jeton, chemin, date):
    frontiere = uuid.uuid4().hex
    morceaux = [
        f'--{frontiere}\r\nContent-Disposition: form-data; name="{nom}"\r\n\r\n{valeur}\r\n'.encode()
        for nom, valeur in [("fileCreatedAt", date), ("fileModifiedAt", date), ("filename", chemin.name)]
    ]
    morceaux.append(
        f'--{frontiere}\r\nContent-Disposition: form-data; name="assetData"; filename="{chemin.name}"\r\n'
        f"Content-Type: image/jpeg\r\n\r\n".encode()
        + chemin.read_bytes()
        + b"\r\n"
    )
    morceaux.append(f"--{frontiere}--\r\n".encode())
    return appel(
        "POST",
        "assets",
        brut=b"".join(morceaux),
        jeton=jeton,
        entetes={"Content-Type": f"multipart/form-data; boundary={frontiere}"},
    )["id"]


def attendre_travaux(jeton):
    """Waits until the server is done with metadata, thumbnails and places
    (not the machine learning jobs: the showcase has no ML container)."""
    apprentissage = {"faceDetection", "facialRecognition", "smartSearch", "ocr", "duplicateDetection"}
    for _ in range(600):
        travaux = appel("GET", "jobs", jeton=jeton)
        if all(t["jobCounts"]["active"] + t["jobCounts"]["waiting"] == 0 for n, t in travaux.items() if n not in apprentissage):
            return
        time.sleep(2)


def main():
    attendre_serveur()
    try:
        appel("POST", "auth/admin-sign-up", {"email": EMAIL, "password": MOT_DE_PASSE, "name": "Camille Martin"})
        print("compte créé")
    except urllib.error.HTTPError as e:
        if e.code != 400:
            raise
    jeton = appel("POST", "auth/login", {"email": EMAIL, "password": MOT_DE_PASSE})["accessToken"]

    if not appel("POST", "search/metadata", {"size": 1}, jeton=jeton)["assets"]["items"]:
        photos = json.loads(LISTE.read_text())
        dossier = DONNEES / "photos"
        dossier.mkdir(parents=True, exist_ok=True)
        albums = {}
        identifiants = []
        for photo in photos:
            chemin = dossier / photo["fichier"]
            telecharger(photo, chemin)
            identifiant = envoyer(jeton, chemin, photo["date"] + ".000Z")
            identifiants.append(identifiant)
            if photo["album"]:
                albums.setdefault(photo["album"], []).append(identifiant)
        print(f"{len(photos)} photos envoyées")
        # Date and place set once the server has read the files, so its own
        # metadata pass does not overwrite them.
        attendre_travaux(jeton)
        for photo, identifiant in zip(photos, identifiants):
            appel(
                "PUT",
                f"assets/{identifiant}",
                {
                    "dateTimeOriginal": photo["date"] + "+00:00",
                    "latitude": photo["latitude"],
                    "longitude": photo["longitude"],
                    "isFavorite": photo["favori"],
                },
                jeton=jeton,
            )
        for nom, ids in albums.items():
            appel("POST", "albums", {"albumName": nom, "assetIds": ids}, jeton=jeton)
        print(f"{len(albums)} albums")
        attendre_travaux(jeton)
    fichier_cle = DONNEES / "cle-api"
    if not fichier_cle.exists():
        cle = appel("POST", "api-keys", {"name": "vitrine Nicéphore", "permissions": ["all"]}, jeton=jeton)["secret"]
        fichier_cle.write_text(cle)
    (DONNEES / "test.env").write_text(
        f"IMMICH_TEST_URL={URL}\nIMMICH_TEST_EMAIL={EMAIL}\nIMMICH_TEST_PASSWORD={MOT_DE_PASSE}\n"
        f"IMMICH_TEST_API_KEY={fichier_cle.read_text().strip()}\n"
    )
    print(f"connexion écrite dans {DONNEES / 'test.env'}")


main()
