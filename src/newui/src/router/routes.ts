import { RouteRecordRaw } from 'vue-router';

export const ROUTES = {
  playersList: 'players-list',
  playerDetails: 'player-details',
  roundsSearch: 'rounds-search',
  settings: 'settings',
};

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      { path: '', component: () => import('pages/IndexPage.vue') },
      {
        path: 'players',
        children: [
          {
            path: '',
            component: () => import('pages/Player/List.vue'),
            name: ROUTES.playersList,
          },
          {
            path: ':id',
            component: () => import('pages/Player/Details.vue'),
            name: ROUTES.playerDetails,
          },
        ],
      },
      {
        path: 'rounds-search',
        component: () => import('pages/Round/Search.vue'),
        name: ROUTES.roundsSearch,
      },
      {
        path: 'settings',
        component: () => import('pages/Config/Settings.vue'),
        name: ROUTES.settings,
      },
    ],
  },

  // Always leave this as last one,
  // but you can also remove it
  {
    path: '/:catchAll(.*)*',
    component: () => import('pages/ErrorNotFound.vue'),
  },
];

export default routes;
