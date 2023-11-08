<script setup lang="ts">
import { ref } from 'vue';

/* ====================== Data ====================== */

// const props = defineProps<{}>();

const demoId = ref(0);
const isVisible = ref(false);

/* ====================== Methods ====================== */

const emit = defineEmits(['close']);

const open = (demo: number) => {
  demoId.value = demo;
  isVisible.value = true;
};

const close = () => {
  isVisible.value = false;
  emit('close');
};

/* ====================== Hooks ====================== */

defineExpose({ open, close });
</script>

<template>
  <q-dialog v-model="isVisible" content-class="dialog" @hide="close">
    <q-card class="hsb-min-width" flat>
      <q-card-section class="row items-center q-pb-none">
        <q-space />
        <q-btn icon="close" flat round dense v-close-popup />
      </q-card-section>

      <q-card-section> Demo ID: {{ demoId }} </q-card-section>
    </q-card>
  </q-dialog>
</template>

<style scoped lang="scss">
.hsb-min-width {
  min-width: 1040px;
}
</style>
