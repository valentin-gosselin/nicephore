#!/bin/sh
# Runs the benchmark mode in an invisible virtual KWin session (nothing opens
# on the user's screen), with a private D-Bus session so screenshots only see
# the virtual screen. Results land in banc/<date>/ (journal + screenshots).
#
# Usage: scripts/banc.sh [binary] [width] [height]
set -eu

racine=$(cd "$(dirname "$0")/.." && pwd)
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

sortie="$racine/banc/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$sortie"
export NICEPHORE_BANC_DOSSIER="$sortie"

interne="$sortie/lancer.sh"
cat > "$interne" <<FIN
#!/bin/sh
# Never fall back to the real X server.
unset DISPLAY
export GDK_BACKEND=wayland
exec "$binaire"
FIN
chmod +x "$interne"

/usr/bin/dbus-run-session -- /usr/bin/kwin_wayland --virtual --no-lockscreen \
    --width "$largeur" --height "$hauteur" --socket "banc-$$" --exit-with-session \
    "$interne" >"$sortie/sortie.txt" 2>&1 || true
rm -f "$interne"

grep -a 'banc' "$sortie/sortie.txt" | sed 's/^.*banc: //' > "$sortie/mesures.txt" || true
cat "$sortie/mesures.txt"
ls "$sortie"
