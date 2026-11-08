// The timeline views of the sidebar and what each one allows.
import type { Filtre } from './api';

export type IdVue = 'photos' | 'favoris' | 'archive' | 'corbeille' | 'recherche' | 'personne' | 'album';

export type Vue = {
  id: IdVue;
  titre: string;
  chemin: string;
  filtre: Filtre;
  /** Message when the view has nothing to show. */
  vide: string;
};

export const VUES: Record<'photos' | 'favoris' | 'archive' | 'corbeille', Vue> = {
  photos: { id: 'photos', titre: 'Photos', chemin: '/', filtre: {}, vide: 'Aucune photo pour le moment.' },
  favoris: {
    id: 'favoris',
    titre: 'Favoris',
    chemin: '/favoris',
    filtre: { favori: true },
    vide: 'Aucun favori. Le cœur d’une photo ou la barre de sélection en ajoutent.',
  },
  archive: {
    id: 'archive',
    titre: 'Archive',
    chemin: '/archive',
    filtre: { visibilite: 'archive' },
    vide: 'L’archive est vide. Les photos archivées disparaissent de la chronologie sans être supprimées.',
  },
  corbeille: {
    id: 'corbeille',
    titre: 'Corbeille',
    chemin: '/corbeille',
    filtre: { corbeille: true },
    vide: 'La corbeille est vide.',
  },
};
