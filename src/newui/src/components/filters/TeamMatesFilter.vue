<script setup lang="ts">
import { inject, ref } from 'vue';

import { PlayerTeamMate } from '@/types/player';

import { PlayerApi } from 'src/api/player';

/* ====================== Data ====================== */

const teamMates = ref([] as PlayerTeamMate[]);
const allTeamMates = ref([] as PlayerTeamMate[]);
const steamId = inject('steamId', '');

const emits = defineEmits(['changed']);

/* ====================== Methods ====================== */

const changed = () => {
  emits('changed', {
    teammates:
      teamMates.value && teamMates.value.length ? teamMates.value.map((teamMate) => teamMate.steamid).join(',') : [],
  });
};

/* ====================== Hooks ====================== */

(async () => {
  allTeamMates.value = await PlayerApi.teamMates(steamId as string);
})();
</script>

<template>
  <q-select
    v-model="teamMates"
    :options="allTeamMates"
    label="Team mates"
    dense
    emit-value
    map-options
    input-debounce="200"
    multiple
    use-chips
    clearable
    @update:modelValue="changed"
  >
    <template #selected-item="scope">
      <div class="row wrap">
        <q-chip removable @remove="scope.removeAtIndex(scope.index)">
          {{ scope.opt.name }} <span v-show="scope.opt.demos" class="ml-2">({{ scope.opt.demos }})</span>
        </q-chip>
      </div>
    </template>
    <template #option="scope">
      <q-item v-bind="scope.itemProps">
        <q-item-section>
          <div class="row wrap">
            {{ scope.opt.name }} <span v-show="scope.opt.demos" class="ml-2">({{ scope.opt.demos }})</span>
          </div>
        </q-item-section>
      </q-item>
    </template>
  </q-select>
</template>

<style scoped lang="scss"></style>
