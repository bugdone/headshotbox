<script setup lang="ts">
import debounce from 'lodash/debounce';
import { ref, watch } from 'vue';

import { ConfigApi } from 'src/api/config';

/* ====================== Data ====================== */

const selectedFolder = ref('All');
let folders = ref([] as string[]);
const emits = defineEmits(['changed']);

/* ====================== Hooks ====================== */

(async () => {
  folders.value = await ConfigApi.folders();
  folders.value.unshift('All');
})();

watch(
  selectedFolder,
  debounce(() => {
    emits('changed', selectedFolder.value);
  }, 100)
);
</script>

<template>
  <q-select v-model="selectedFolder" :options="folders" label="Folders" dense />
</template>

<style scoped lang="scss"></style>
