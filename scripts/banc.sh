#!/bin/sh
# Runs the benchmark mode in an invisible virtual KWin session (nothing opens
# on the user's screen), with a private D-Bus session so screenshots only see
# the virtual screen. Results land in banc/<date>/ (journal + screenshots).
#
# Usage: scripts/banc.sh [binary] [width] [height]
set -eu

racine=$(cd "$(dirname "$0")/.." && pwd)
# --jetable: in-app write scenario against the disposable Immich server
# (scripts/immich-jetable.sh demarrer first). Never against a real library.
# --portable: portable mode scenario, two launches on a private `donnees`
# folder, logging in through the login screen of the disposable server.
jetable=""
portable=""
if [ "${1:-}" = "--jetable" ]; then
    jetable=1
    shift
elif [ "${1:-}" = "--portable" ]; then
    jetable=1
    portable=1
    shift
fi
# --mise-a-jour <AppImage>: in-app update scenario (see
# scripts/essai-mise-a-jour.sh, which builds the versions and serves them).
# --comptes: multi-account scenario, portable mode (no keyring in the
# private session), one launch.
comptes=""
if [ "${1:-}" = "--comptes" ]; then
    jetable=1
    portable=1
    comptes=1
    shift
fi
# --fichier: default viewer scenario, Nicéphore launched with a photo like
# the file manager does.
fichier=""
if [ "${1:-}" = "--fichier" ]; then
    jetable=1
    fichier=1
    shift
fi
mise_a_jour=""
if [ "${1:-}" = "--mise-a-jour" ]; then
    jetable=1
    mise_a_jour=1
    shift
fi
binaire=${1:-"$racine/target/release/nicephore"}
largeur=${2:-1600}
hauteur=${3:-1000}

# Development connection: IMMICH_URL and IMMICH_API_KEY (or IMMICH_KEY), read
# from $NICEPHORE_ENV (default ~/.config/nicephore/dev.env).
fichier_env="${NICEPHORE_ENV:-${XDG_CONFIG_HOME:-$HOME/.config}/nicephore/dev.env}"
if [ -f "$fichier_env" ]; then
    set -a
    . "$fichier_env"
    set +a
fi
export IMMICH_API_KEY="${IMMICH_API_KEY:-${IMMICH_KEY:-}}"
if [ -n "$jetable" ]; then
    donnees_jetable="${NICEPHORE_IMMICH_JETABLE:-${XDG_CACHE_HOME:-$HOME/.cache}/nicephore-immich-jetable}"
    set -a
    . "$donnees_jetable/test.env"
    set +a
    export IMMICH_URL="$IMMICH_TEST_URL" IMMICH_API_KEY="$IMMICH_TEST_API_KEY" NICEPHORE_BANC_SCENARIO=ecriture
    # Virtual folders go through their service, in the private config.
    export NICEPHORE_BANC_SERVICE=1
fi
if [ -n "$mise_a_jour" ]; then
    export NICEPHORE_BANC_SCENARIO=mise-a-jour
fi
if [ -n "$fichier" ]; then
    export NICEPHORE_BANC_SCENARIO=fichier
fi
if [ -n "$portable" ]; then
    # No development key: the login screen is the way in.
    unset IMMICH_URL IMMICH_API_KEY IMMICH_KEY
fi

sortie="$racine/banc/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$sortie"
export NICEPHORE_BANC_DOSSIER="$sortie"
arguments=""
if [ -n "$fichier" ]; then
    # A photo already on the server, a new one (another checksum), another.
    mkdir -p "$sortie/local"
    python3 -I - "$donnees_jetable/images/essai-00.jpg" "$sortie/local" <<'PY'
import pathlib, sys, time
source = pathlib.Path(sys.argv[1]).read_bytes()
sortie = pathlib.Path(sys.argv[2])
(sortie / "a-sur-serveur.jpg").write_bytes(source)
for nom in ("b-nouvelle", "c-autre"):
    texte = f"{nom} {time.time()}".encode()
    (sortie / f"{nom}.jpg").write_bytes(source[:2] + b"\xff\xfe" + (len(texte) + 2).to_bytes(2, "big") + texte + source[2:])
# A made-up RAW: TIFF header with orientation 6 (rotated), a JPEG inside.
entete = b"II*\0\x08\0\0\0" + bytes([1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0])
(sortie / "d-raw.arw").write_bytes(entete + b"\0" * 64 + source + b"\0" * 64)
PY
    arguments="\"$sortie/local/b-nouvelle.jpg\""
elif [ -n "$portable" ]; then
    export NICEPHORE_PORTABLE="$sortie/donnees"
elif [ -n "$jetable" ]; then
    # Photos for the upload scenario: variants of a seeded photo (new
    # checksums), one copy already on the server, one file to skip.
    python3 -I "$racine/tests/immich-jetable/dossier-envoi.py" "$donnees_jetable/images" "$sortie/photos"
fi

# The write scenario gets its own config directory: settings it toggles
# (session autostart...) never reach the user's real session.
config_privee=""
if [ -n "$jetable" ] && [ -z "$portable" ]; then
    config_privee="export XDG_CONFIG_HOME=\"$sortie/config\""
fi

interne="$sortie/lancer.sh"
cat > "$interne" <<FIN
#!/bin/sh
# Never fall back to the real X server.
unset DISPLAY
export GDK_BACKEND=wayland
$config_privee
if [ -n "$mise_a_jour" ]; then
    # The update restarts Nicéphore: the session lasts while a copy runs.
    "$binaire"
    sleep 2
    while pgrep -f "^$binaire" >/dev/null; do sleep 1; done
else
    exec "$binaire" $arguments
fi
FIN
chmod +x "$interne"

lancer() {
    /usr/bin/dbus-run-session -- /usr/bin/kwin_wayland --virtual --no-lockscreen \
        --width "$largeur" --height "$hauteur" --socket "banc-$$" --exit-with-session \
        "$interne" >>"$sortie/sortie.txt" 2>&1 || true
}

if [ -n "$comptes" ]; then
    NICEPHORE_BANC_SCENARIO=comptes lancer
    echo "banc: bases d'envoi par compte : $(ls "$sortie/donnees/envoi" | grep -c '^envoi-.*[.]sqlite$')" >>"$sortie/sortie.txt"
elif [ -n "$portable" ]; then
    # The real profile folders, watched for any write (none expected; the
    # real config is not redirected here, on purpose).
    identifiant=io.github.valentin-gosselin.nicephore
    profil() {
        for d in "${XDG_CONFIG_HOME:-$HOME/.config}/$identifiant" "${XDG_DATA_HOME:-$HOME/.local/share}/$identifiant" \
            "${XDG_CACHE_HOME:-$HOME/.cache}/$identifiant"; do
            ls -l --time-style=full-iso "$d" 2>/dev/null
        done | md5sum
    }
    avant=$(profil)
    NICEPHORE_BANC_SCENARIO=portable-1 lancer
    NICEPHORE_BANC_SCENARIO=portable-2 lancer
    # What landed where: everything in donnees/, nothing in the real profile.
    {
        echo "banc: fichiers du mode portable :"
        (cd "$sortie/donnees" && find . -maxdepth 2 | sort | sed 's/^/banc:   /')
        if grep -q '"coffre"' "$sortie/donnees/config/comptes.json" && ! grep -qi 'token' "$sortie/donnees/config/comptes.json"; then
            echo "banc: ok : connexion scellée dans donnees/config/comptes.json"
        else
            echo "banc: ÉCHEC : connexion non scellée"
        fi
        if [ "$avant" = "$(profil)" ]; then
            echo "banc: ok : profil réel inchangé (config, données et cache de $identifiant)"
        else
            echo "banc: ÉCHEC : le profil réel a changé"
        fi
    } >>"$sortie/sortie.txt"
else
    lancer
    if [ -n "$jetable" ] && [ -z "$portable" ] && [ -z "$fichier" ] && [ -z "$mise_a_jour" ]; then
        # The virtual folders service outlives the app: its folder is still
        # served, then stopping the service unmounts it.
        if mount | grep -q "$sortie/persistant"; then
            echo "banc: ok : dossier virtuel encore servi après la fermeture ($(ls "$sortie/persistant" | wc -l) fichiers)" >>"$sortie/sortie.txt"
        else
            echo "banc: ÉCHEC : dossier virtuel plus servi après la fermeture" >>"$sortie/sortie.txt"
        fi
        pkill -TERM -f "^$binaire --dossiers-virtuels" || true
        sleep 2
        mount | grep -q "$sortie/persistant" || echo "banc: ok : démonté à l'arrêt du service" >>"$sortie/sortie.txt"
    fi
    # A virtual folder must never stay mounted after the app.
    if mount | grep -q "$sortie"; then
        echo "banc: ÉCHEC : un montage reste dans $sortie" >>"$sortie/sortie.txt"
        mount | grep "$sortie" | awk '{print $3}' | xargs -r -n1 fusermount3 -uz
    fi
fi
rm -f "$interne"

grep -a 'banc' "$sortie/sortie.txt" | sed 's/^.*banc: //' > "$sortie/mesures.txt" || true
cat "$sortie/mesures.txt"
ls "$sortie"
