// Forwards client-side errors to the Rust log, so they show up in the
// terminal and in benchmark runs (the webview console is not visible there).
import type { HandleClientError } from '@sveltejs/kit/hooks';
import { invoke } from '@tauri-apps/api/core';

const signaler = (message: string) => {
  console.error(message);
  void invoke('journal', { message: `erreur : ${message}` }).catch(() => {});
};

export const handleError: HandleClientError = ({ error }) => {
  const e = error as Error;
  signaler(`${e?.message ?? error}\n${e?.stack ?? ''}`);
  return { message: e?.message ?? 'Erreur interne' };
};

window.addEventListener('error', (e) => signaler(`${e.message} (${e.filename}:${e.lineno})`));
window.addEventListener('unhandledrejection', (e) => signaler(`promesse rejetée : ${e.reason}`));
