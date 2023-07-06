<script setup lang="ts">
import { inject, ref } from 'vue';

import { PlayerApi } from 'src/api/player';

/* ====================== Data ====================== */

const map = ref('All');
const maps = ref([] as string[]);
const steamId = inject('steamId', '');

const emits = defineEmits(['changed']);

/* ====================== Methods ====================== */

const changed = () => {
  emits('changed', { mapName: map.value });
};

/* ====================== Hooks ====================== */

(async () => {
  maps.value = await PlayerApi.maps(steamId as string);
  maps.value.unshift('All');
})();
</script>

<template>
  <q-select v-model="map" :options="maps" label="Maps" dense @update:modelValue="changed" />
</template>

<style scoped lang="scss"></style>
