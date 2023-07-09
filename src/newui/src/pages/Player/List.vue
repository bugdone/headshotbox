<script setup lang="ts">
import { date } from 'quasar';
import { ref } from 'vue';

import { RANKS } from 'src/constants/ranks';
import { ApiRequestParams } from '@/types/api';
import type { DataTableHeader } from '@/types/dataTable';
import { PlayerResponse } from '@/types/player';

import { PlayerApi } from 'src/api/player';
import { ROUTES } from 'src/router/routes';

import DataTable from 'src/components/common/DataTable.vue';
import FolderFilter from 'src/components/filters/FolderFilter.vue';

/* ====================== Data ====================== */

const tableRef = ref();
const columns = [
  { label: 'Name', name: 'steamInfo', field: 'steamInfo', align: 'left', style: 'width: 35%' },
  { label: 'MM Rank', name: 'lastRank', field: 'lastRank', align: 'center', sortable: true },
  { label: 'Demos', name: 'demos', field: 'demos', align: 'center', sortable: true },
  { label: 'Last Played', name: 'lastTimestamp', field: 'lastTimestamp', align: 'center', sortable: true },
] as DataTableHeader[];
const selectedFolder = ref('All');

/* ====================== Methods ====================== */

const getPlayers = async ({ limit, offset, orderBy }: ApiRequestParams) => {
  const { playerCount, players } = await PlayerApi.get({
    ...{ limit, offset, orderBy },
    ...(selectedFolder.value !== 'All' ? { folder: selectedFolder.value } : {}),
  });

  return {
    results: players,
    count: playerCount,
  };
};

const filterChanged = (filter: { folder: string }) => {
  selectedFolder.value = filter.folder;
  tableRef.value.refresh();
};
</script>

<template>
  <q-page class="px-6 py-1 main-container">
    <DataTable ref="tableRef" :columns="columns" entityName="player" :apiCall="getPlayers">
      <template #top-Filter>
        <div style="min-width: 100px">
          <FolderFilter @changed="filterChanged" />
        </div>
      </template>

      <template #steamInfo="item: PlayerResponse">
        <div class="row">
          <q-img
            v-if="item.steamInfo"
            :src="item.steamInfo.avatarfull || ''"
            fit="cover"
            width="40px"
            height="40px"
            class="mr-2 rounded-md"
          />
          <q-icon v-else name="mdi-account" size="60px" color="primary" />
          <router-link class="text-base self-center" :to="{ name: ROUTES.playerDetails, params: { id: item.steamid } }">
            {{ item.steamInfo ? item.steamInfo.personaname : item.name }}
          </router-link>
        </div>
      </template>

      <template #lastRank="item: PlayerResponse">
        <q-img fit="fill" class="my-1 rounded" width="90px" height="38px" :src="`images/ranks/${item.lastRank}.png`">
          <q-tooltip class="bg-sky-500/95 text-sm shadow-4 text-black" anchor="top middle" self="bottom middle">
            {{ RANKS[item.lastRank] }}
          </q-tooltip>
        </q-img>
      </template>

      <template #demos="item: PlayerResponse">
        <span class="text-base">{{ item.demos }}</span>
      </template>

      <template #lastTimestamp="item: PlayerResponse">
        {{ date.formatDate(item.lastTimestamp * 1000, 'D MMM YYYY') }}
      </template>
    </DataTable>
  </q-page>
</template>

<style scoped lang="scss">
body.screen--md,
body.screen--lg,
body.screen--xl {
  .main-container {
    margin: 0 auto;
    max-width: 906px;
  }
}
</style>
