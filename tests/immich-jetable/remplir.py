"""Seeds the disposable Immich server: admin account, a few dozen generated
photos spread over several months, and writes the connection details to
<data dir>/test.env for the write tests. Standard library only."""

import json
import pathlib
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid

URL = sys.argv[1].rstrip("/")
DONNEES = pathlib.Path(sys.argv[2])
EMAIL = "test@nicephore.invalid"
MOT_DE_PASSE = "nicephore-test"


def appel(methode, chemin, corps=None, jeton=None, brut=None, entetes=None):
    donnees = brut if brut is not None else (json.dumps(corps).encode() if corps is not None else None)
    requete = urllib.request.Request(f"{URL}/api/{chemin}", data=donnees, method=methode)
    if brut is None:
        requete.add_header("Content-Type", "application/json")
    for nom, valeur in (entetes or {}).items():
        requete.add_header(nom, valeur)
    if jeton:
        requete.add_header("Authorization", f"Bearer {jeton}")
    with urllib.request.urlopen(requete, timeout=60) as reponse:
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
    sys.exit("le serveur Immich jetable ne répond pas")


def envoyer_photo(jeton, chemin, date):
    frontiere = uuid.uuid4().hex
    morceaux = []
    for nom, valeur in [("fileCreatedAt", date), ("fileModifiedAt", date), ("filename", chemin.name)]:
        morceaux.append(f'--{frontiere}\r\nContent-Disposition: form-data; name="{nom}"\r\n\r\n{valeur}\r\n'.encode())
    morceaux.append(
        f'--{frontiere}\r\nContent-Disposition: form-data; name="assetData"; filename="{chemin.name}"\r\n'
        f"Content-Type: image/jpeg\r\n\r\n".encode()
        + chemin.read_bytes()
        + b"\r\n"
    )
    morceaux.append(f"--{frontiere}--\r\n".encode())
    appel(
        "POST",
        "assets",
        brut=b"".join(morceaux),
        jeton=jeton,
        entetes={"Content-Type": f"multipart/form-data; boundary={frontiere}"},
    )


def main():
    attendre_serveur()
    try:
        appel("POST", "auth/admin-sign-up", {"email": EMAIL, "password": MOT_DE_PASSE, "name": "Test Nicéphore"})
        print("compte administrateur créé")
    except urllib.error.HTTPError as e:
        if e.code != 400:
            raise
        print("compte administrateur déjà présent")
    jeton = appel("POST", "auth/login", {"email": EMAIL, "password": MOT_DE_PASSE})["accessToken"]

    deja = appel("POST", "search/metadata", {"size": 1}, jeton=jeton)["assets"]["items"]
    if not deja:
        images = DONNEES / "images"
        images.mkdir(parents=True, exist_ok=True)
        couleurs = ["#3b6ea5", "#a53b4f", "#3ba56c", "#a58a3b", "#6c3ba5", "#3b9ea5"]
        for i in range(36):
            chemin = images / f"essai-{i:02}.jpg"
            if not chemin.exists():
                largeur, hauteur = (1600, 1067) if i % 3 else (1067, 1600)
                subprocess.run(
                    ["magick", "-size", f"{largeur}x{hauteur}", f"xc:{couleurs[i % len(couleurs)]}",
                     "-gravity", "center", "-pointsize", "220", "-fill", "white", "-annotate", "0", str(i),
                     "-quality", "80", str(chemin)],
                    check=True,
                )
            mois = 1 + (i // 6)
            envoyer_photo(jeton, chemin, f"2025-{mois:02}-{1 + i % 27:02}T12:00:00.000Z")
        print("36 photos envoyées sur 6 mois")
    # API key for the in-app scenarios (the app reads IMMICH_API_KEY).
    fichier_cle = DONNEES / "cle-api"
    if not fichier_cle.exists():
        cle = appel("POST", "api-keys", {"name": "banc Nicéphore", "permissions": ["all"]}, jeton=jeton)["secret"]
        fichier_cle.write_text(cle)
    (DONNEES / "test.env").write_text(
        f"IMMICH_TEST_URL={URL}\nIMMICH_TEST_EMAIL={EMAIL}\nIMMICH_TEST_PASSWORD={MOT_DE_PASSE}\n"
        f"IMMICH_TEST_API_KEY={fichier_cle.read_text().strip()}\n"
    )
    print(f"connexion écrite dans {DONNEES / 'test.env'}")


main()
