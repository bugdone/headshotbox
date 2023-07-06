<script setup lang="ts">
import { ref } from 'vue';

/* ====================== Data ====================== */

const type = ref({
  value: '',
  label: 'All',
});
const demoTypes = ref([
  {
    value: 'All',
    label: 'All',
  },
  {
    value: 'valve',
    label: 'Valve',
    image: 'images/demoTypes/valve.png',
  },
  {
    value: 'esea',
    label: 'ESEA',
    image: 'images/demoTypes/esea.png',
  },
  {
    value: 'faceit',
    label: 'FACEIT',
    image: 'images/demoTypes/faceit.png',
  },
  {
    value: 'cevo',
    label: 'CEVO',
    image: 'images/demoTypes/cevo.png',
  },
  {
    value: 'esportal',
    label: 'Esportal',
    image: 'images/demoTypes/esportal.png',
    class: 'bg-blue-500 rounded',
    style: 'height: 20px',
  },
  {
    value: 'custom',
    label: 'Custom',
    image: 'images/demoTypes/custom.png',
    style: 'height: 20px',
  },
]);

const emits = defineEmits(['changed']);

/* ====================== Methods ====================== */

const changed = () => {
  emits('changed', { demoType: type.value });
};
</script>

<template>
  <q-select
    v-model="type"
    :options="demoTypes"
    label="Demo types"
    dense
    emit-value
    map-options
    input-debounce="200"
    @update:modelValue="changed"
  >
    <template #selected-item="scope">
      <div class="row no-wrap">
        <q-img
          v-if="scope.opt.image"
          :src="scope.opt.image"
          width="50px"
          :class="scope.opt.class ?? ''"
          :style="scope.opt.style ?? ''"
        />
        <div v-else>{{ scope.opt.label }}</div>
      </div>
    </template>
    <template #option="scope">
      <q-item v-bind="scope.itemProps">
        <q-item-section>
          <div class="row no-wrap">
            <q-img
              v-if="scope.opt.image"
              :src="scope.opt.image"
              width="50px"
              :class="scope.opt.class ?? ''"
              :style="scope.opt.style ?? ''"
            />
            <div v-else>{{ scope.opt.label }}</div>
          </div>
        </q-item-section>
      </q-item>
    </template>
  </q-select>
</template>

<style scoped lang="scss"></style>
