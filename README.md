# Nicéphore

Unofficial desktop client for [Immich](https://immich.app), the self-hosted photo and video
management server. Not affiliated with or endorsed by the Immich project.

Named after Nicéphore Niépce, who took the oldest surviving photograph, a view from his window
at Le Gras (about 1827). The icon is a nod to it.

Status: early development. Browsing works (timeline, favorites, archive, trash, viewer, video,
multi-selection with bulk actions); sync is not there yet.

## Goals

- Browse the whole library like the web app, natively on Windows and Linux.
- Two-way sync between local folders and the server: upload, selective download, virtual files
  (placeholders downloaded on demand).
- Portable mode (no installation) and signed in-app updates, including the AppImage.

## Layout

- `crates/api_immich`: typed client for the Immich API (API key authentication).
- `crates/cache_vignettes`: on-disk image cache shared by the UI.
- `app/`: Tauri 2 shell (`src-tauri/`) and Svelte 5 UI built with `@immich/ui`.
- `docs/`: design notes (French).

## Development

Linux build dependencies (Fedora): `webkit2gtk4.1-devel gtk3-devel libsoup3-devel
javascriptcoregtk4.1-devel librsvg2-devel libappindicator-gtk3-devel openssl-devel`, plus Rust and
Node.js.

```sh
cd app
npm install
npx tauri dev          # development
npx tauri build        # release
```

For development, `IMMICH_URL` and `IMMICH_API_KEY` connect the app at startup
(`scripts/lancer.sh --dev` reads them from `~/.config/nicephore/dev.env`). Otherwise the app shows
a login screen and keeps the API key in the operating system keyring.

`scripts/banc.sh` runs a scripted benchmark in an invisible virtual KWin session (scrolling frame
times, thumbnail fill rate, screenshots), and `scripts/installer-lanceur.sh` adds a desktop entry.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
