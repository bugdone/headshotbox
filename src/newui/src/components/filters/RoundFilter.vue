<script setup lang="ts">
import { ref } from 'vue';

/* ====================== Data ====================== */

const round = ref('All');
const rounds = ref(['All', 'T', 'CT', 'Pistol']);

const emits = defineEmits(['changed']);

/* ====================== Methods ====================== */

const changed = () => {
  emits('changed', { rounds: round.value });
};
</script>

<template>
  <q-select v-model="round" :options="rounds" label="Rounds" dense @update:modelValue="changed" />
</template>

<style scoped lang="scss"></style>
