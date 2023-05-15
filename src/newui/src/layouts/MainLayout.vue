<script setup lang="ts">
import { computed, ref } from 'vue';

import EssentialLink, {
  EssentialLinkProps,
} from 'components/EssentialLink.vue';

/* ====================== Vars ====================== */

const essentialLinks: EssentialLinkProps[] = [
  {
    title: 'Players',
    caption: 'View players statistics',
    icon: 'mdi-account-multiple',
  },
  {
    title: 'Search',
    caption: 'Search rounds by 3k, 4k, etc.',
    icon: 'mdi-account-search',
  },
  {
    title: 'Settings',
    caption: 'Adjust UI settings',
    icon: 'mdi-cog-outline',
  },
  {
    title: 'View source',
    caption: '',
    icon: 'mdi-github',
    link: 'https://github.com/YOLO-Projects/headshotbox-ui',
  },
];
const leftDrawerOpen = ref(false);

const version = computed(() => import.meta.env.VITE_VERSION);

/* ====================== Functions ====================== */

function toggleLeftDrawer() {
  leftDrawerOpen.value = !leftDrawerOpen.value;
}
</script>

<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <q-btn
          flat
          dense
          round
          icon="menu"
          aria-label="Menu"
          @click="toggleLeftDrawer"
        />

        <q-toolbar-title> HeadshotBox </q-toolbar-title>

        <div>v {{ version }}</div>
      </q-toolbar>
    </q-header>

    <q-drawer v-model="leftDrawerOpen" show-if-above bordered>
      <q-list>
        <q-item-label header>
          <q-img src="images/hsbox.png" fit="contain" :ratio="16 / 9" />
        </q-item-label>

        <EssentialLink
          v-for="link in essentialLinks"
          :key="link.title"
          v-bind="link"
        />
      </q-list>
    </q-drawer>

    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>
