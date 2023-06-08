<script setup lang="ts">
import { useQuasar } from 'quasar';
import { computed, provide, ref } from 'vue';

import { ROUTES } from 'src/router/routes';

import EssentialLink, { EssentialLinkProps } from 'components/EssentialLink.vue';

/* ====================== Vars ====================== */

const essentialLinks: EssentialLinkProps[] = [
  {
    title: 'Players',
    caption: 'View players statistics',
    icon: 'mdi-account-multiple',
    route: ROUTES.playersList,
  },
  {
    title: 'Search Rounds',
    caption: 'Search rounds by 3k, 4k, etc.',
    icon: 'mdi-text-box-search-outline',
    route: ROUTES.roundsSearch,
  },
  {
    title: 'Settings',
    caption: 'Adjust UI settings',
    icon: 'mdi-cog-outline',
    route: ROUTES.settings,
  },
  {
    title: 'View Source',
    icon: 'mdi-github',
    link: 'https://github.com/bugdone/headshotbox',
  },
];
const $q = useQuasar();

const version = computed(() => import.meta.env.VITE_VERSION);

const userMiniState = computed(() => {
  if ($q.localStorage.has('menuMiniState')) {
    return $q.localStorage.getItem('menuMiniState');
  }

  return false;
});
const miniState = ref(userMiniState.value as boolean);

/* ====================== Functions ====================== */

const saveMiniState = () => {
  miniState.value = !miniState.value;
  $q.localStorage.set('menuMiniState', miniState.value);
};

/* ====================== Hooks ====================== */

provide('drawerMiniState', miniState);
</script>

<template>
  <q-layout view="hHh Lpr lff">
    <q-header elevated>
      <q-toolbar>
        <q-btn flat dense round icon="menu" aria-label="Menu" @click="saveMiniState" />

        <q-toolbar-title> HeadshotBox </q-toolbar-title>

        <div>v {{ version }}</div>
      </q-toolbar>
    </q-header>

    <q-drawer show-if-above bordered :mini="miniState">
      <q-list class="mt-4">
        <EssentialLink v-for="link in essentialLinks" :key="link.title" v-bind="link" />
      </q-list>
    </q-drawer>

    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<style lang="scss" scoped></style>
