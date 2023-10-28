<script setup lang="ts">
import { ref } from 'vue';
import { DEMO_TYPE_IMAGES, DEMO_TYPES } from 'src/constants/demos';

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
    value: DEMO_TYPES.VALVE,
    label: 'Valve',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.VALVE],
  },
  {
    value: DEMO_TYPES.ESEA,
    label: 'ESEA',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.ESEA],
  },
  {
    value: DEMO_TYPES.FACEIT,
    label: 'FACEIT',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.FACEIT],
  },
  {
    value: DEMO_TYPES.CEVO,
    label: 'CEVO',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.CEVO],
  },
  {
    value: DEMO_TYPES.ESPORTAL,
    label: 'Esportal',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.ESPORTAL],
    class: 'bg-blue-500 rounded',
    style: 'height: 20px',
  },
  {
    value: DEMO_TYPES.CUSTOM,
    label: 'Custom',
    image: DEMO_TYPE_IMAGES[DEMO_TYPES.CUSTOM],
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
