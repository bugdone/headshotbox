<script setup lang="ts">
import { inject } from 'vue';

/* ====================== Vars ====================== */

export interface EssentialLinkProps {
  title: string;
  caption?: string;
  link?: string;
  icon?: string;
  route?: string;
}

withDefaults(defineProps<EssentialLinkProps>(), {
  caption: '',
  link: '#',
  icon: '',
});

const drawerMiniState = inject('drawerMiniState', false);
</script>

<template>
  <template v-if="route">
    <q-item clickable :to="{ name: route }">
      <q-item-section v-if="icon" avatar>
        <q-icon :name="icon" size="lg">
          <q-tooltip
            v-if="drawerMiniState"
            class="bg-white text-black text-base shadow-4"
            anchor="top middle"
            self="bottom middle"
          >
            {{ title }}
          </q-tooltip>
        </q-icon>
      </q-item-section>

      <q-item-section>
        <q-item-label>{{ title }}</q-item-label>
        <q-item-label caption>{{ caption }}</q-item-label>
      </q-item-section>
    </q-item>
  </template>
  <template v-else>
    <q-item clickable tag="a" :href="link" target="_blank">
      <q-item-section v-if="icon" avatar>
        <q-icon :name="icon" size="lg">
          <q-tooltip
            v-if="drawerMiniState"
            class="bg-white text-black text-base shadow-4 min-w-fit"
            anchor="top middle"
            self="bottom middle"
          >
            {{ title }}
          </q-tooltip>
        </q-icon>
      </q-item-section>

      <q-item-section>
        <q-item-label>{{ title }}</q-item-label>
        <q-item-label caption>{{ caption }}</q-item-label>
      </q-item-section>
    </q-item>
  </template>
</template>
