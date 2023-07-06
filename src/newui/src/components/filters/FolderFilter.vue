<script setup lang="ts">
import { ref } from 'vue';

import { ConfigApi } from 'src/api/config';

/* ====================== Data ====================== */

const selectedFolder = ref('All');
let folders = ref([] as string[]);

const emits = defineEmits(['changed']);

/* ====================== Methods ====================== */

const changed = () => {
  emits('changed', { folder: selectedFolder.value });
};

/* ====================== Hooks ====================== */

(async () => {
  folders.value = await ConfigApi.folders();
  folders.value.unshift('All');
})();
</script>

<template>
  <q-select v-model="selectedFolder" :options="folders" label="Folders" dense @update:modelValue="changed" />
</template>

<style scoped lang="scss"></style>
