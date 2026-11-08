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
fi
if [ -n "$portable" ]; then
    # No development key: the login screen is the way in.
    unset IMMICH_URL IMMICH_API_KEY IMMICH_KEY
fi

sortie="$racine/banc/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$sortie"
export NICEPHORE_BANC_DOSSIER="$sortie"
if [ -n "$portable" ]; then
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
exec "$binaire"
FIN
chmod +x "$interne"

lancer() {
    /usr/bin/dbus-run-session -- /usr/bin/kwin_wayland --virtual --no-lockscreen \
        --width "$largeur" --height "$hauteur" --socket "banc-$$" --exit-with-session \
        "$interne" >>"$sortie/sortie.txt" 2>&1 || true
}

if [ -n "$portable" ]; then
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
        if grep -q '"coffre"' "$sortie/donnees/config/compte.json" && ! grep -qi 'token' "$sortie/donnees/config/compte.json"; then
            echo "banc: ok : connexion scellée dans donnees/config/compte.json"
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
fi
rm -f "$interne"

grep -a 'banc' "$sortie/sortie.txt" | sed 's/^.*banc: //' > "$sortie/mesures.txt" || true
cat "$sortie/mesures.txt"
ls "$sortie"
