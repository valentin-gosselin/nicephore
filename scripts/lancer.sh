#!/bin/sh
# Launches Nicéphore from the source tree.
#
# Rendering modes, tried in this order until one survives its first seconds:
#   natif   native Wayland, full GPU path (smoothest)
#   dmabuf  native Wayland without WebKit's DMA-BUF renderer (fixes the
#           NVIDIA "Error 71 dispatching to Wayland display" crash, but
#           halves scrolling smoothness)
#   compat  X11 through XWayland without GPU compositing (last resort, same
#           recipe as the Music Assistant desktop app on this machine)
# The working mode is remembered in ~/.cache/nicephore/mode.
#   lancer.sh --mode compat   forces a mode;  lancer.sh --oublier  retries all.
#   lancer.sh --dev [...]     development connection (see below).
set -eu
racine=$(cd "$(dirname "$0")/.." && pwd)
binaire="${NICEPHORE_BINAIRE:-$racine/target/release/nicephore}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/nicephore"
mkdir -p "$cache"

# With --dev, connects with IMMICH_URL and IMMICH_API_KEY (or IMMICH_KEY) read
# from $NICEPHORE_ENV (default ~/.config/nicephore/dev.env). Without it, the
# app behaves as installed: login screen, key remembered in the OS keyring.
if [ "${1:-}" = "--dev" ]; then
    shift
    fichier_env="${NICEPHORE_ENV:-${XDG_CONFIG_HOME:-$HOME/.config}/nicephore/dev.env}"
    if [ -f "$fichier_env" ]; then
        set -a
        . "$fichier_env"
        set +a
    fi
    export IMMICH_API_KEY="${IMMICH_API_KEY:-${IMMICH_KEY:-}}"
else
    unset IMMICH_URL IMMICH_API_KEY
fi

modes="natif dmabuf compat"
case "${1:-}" in
    --mode) modes="$2"; shift 2 ;;
    --oublier) rm -f "$cache/mode"; shift ;;
    *) [ -f "$cache/mode" ] && modes="$(cat "$cache/mode")" ;;
esac

lancer() {
    (
        case "$1" in
            dmabuf) export WEBKIT_DISABLE_DMABUF_RENDERER=1 ;;
            compat)
                export GDK_BACKEND=x11
                export WEBKIT_DISABLE_DMABUF_RENDERER=1
                export WEBKIT_DISABLE_COMPOSITING_MODE=1
                ;;
        esac
        exec "$binaire" "$@"
    )
}

for mode in $modes; do
    journal="$cache/dernier-lancement.log"
    echo "mode $mode, $(date)" >"$journal"
    debut=$(date +%s)
    statut=0
    lancer "$mode" >>"$journal" 2>&1 || statut=$?
    duree=$(( $(date +%s) - debut ))
    if [ "$statut" -eq 0 ] || [ "$duree" -ge 15 ]; then
        # Survived its start: remember this mode.
        echo "$mode" >"$cache/mode"
        exit "$statut"
    fi
    cp "$journal" "$cache/echec-$mode.log"
done
exit 1
