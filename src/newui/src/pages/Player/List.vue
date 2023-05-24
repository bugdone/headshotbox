<script setup lang="ts">
import { date } from 'quasar';
import { onMounted, ref } from 'vue';

import { ROUTES } from 'src/router/routes';
import { RANKS } from 'src/constants/ranks';
import type { DataTableHeader, DataTablePagination, DataTableRequestDetails } from '@/types/dataTable';
import type { PlayerResponse } from '@/types/player';

import { PlayerApi } from 'src/api/player';
import debounce from 'lodash/debounce';

/* ====================== Data ====================== */

const tableRef = ref();
const players = ref([] as PlayerResponse[]);
const columns = [
  { label: 'Name', name: 'steamInfo', field: 'steamInfo', align: 'left', style: 'width: 35%' },
  { label: 'Rank', name: 'lastRank', field: 'lastRank', align: 'center' },
  { label: 'Demos', name: 'demos', field: 'demos', align: 'center', sortable: true },
  { label: 'Last Played', name: 'lastTimestamp', field: 'lastTimestamp', align: 'center', sortable: true },
] as DataTableHeader[];
const pagination = ref({
  page: 1,
  rowsPerPage: 20,
  sortBy: '',
  descending: false,
  rowsNumber: 0,
} as DataTablePagination);
const isLoading = ref(false);

/* ====================== Methods ====================== */

const getData = debounce(async (tableProps: DataTableRequestDetails) => {
  isLoading.value = true;
  pagination.value.page = tableProps.pagination?.page || pagination.value.page;
  pagination.value.rowsPerPage = tableProps.pagination?.rowsPerPage || pagination.value.rowsPerPage;
  pagination.value.sortBy = tableProps.pagination?.sortBy || pagination.value.sortBy;
  pagination.value.descending = tableProps.pagination?.descending || pagination.value.descending;

  if (pagination.value.page && pagination.value.rowsPerPage) {
    const data = await PlayerApi.get({
      limit: pagination.value.rowsPerPage,
      offset: (pagination.value.page - 1) * pagination.value.rowsPerPage,
    });

    players.value = data.players;
    pagination.value.rowsNumber = data.playerCount;
  }

  isLoading.value = false;
}, 50);

/* ====================== Hooks ====================== */

onMounted(async () => {
  tableRef.value.requestServerInteraction();
});
</script>

<template>
  <q-page class="p-6">
    <q-table
      ref="tableRef"
      :rows="players"
      v-model:pagination="pagination"
      :columns="columns"
      loading-label="Loading ... Please wait"
      no-results-label="No results"
      class="mt-3"
      :pagination="pagination"
      :rows-per-page-options="[10, 20, 30, 40, 50]"
      @request="getData"
      :loading="isLoading"
      color="primary"
      binary-state-sort
    >
      <template #top>
        <div class="my-2" v-show="pagination.rowsNumber > 0">
          <span>{{ pagination.rowsNumber }} Results</span>
        </div>
      </template>

      <template #body-cell-steamInfo="props">
        <q-td :props="props">
          <div class="row no-wrap">
            <q-img
              :src="props.row.steamInfo.avatarfull"
              fit="cover"
              width="40px"
              height="40px"
              class="mx-2 rounded-md"
            />
            <router-link
              class="text-base self-center"
              :to="{ name: ROUTES.playerDetails, params: { id: props.row.steamInfo.steamid } }"
            >
              {{ props.row.steamInfo ? props.row.steamInfo.personaname : 'Anonymous' }}
            </router-link>
          </div>
        </q-td>
      </template>

      <template #body-cell-lastRank="props">
        <q-td :props="props">
          <q-img
            fit="cover"
            class="mx-2 my-1 rounded-lg"
            width="50%"
            height="80%"
            :src="`images/ranks/${props.row.lastRank}.png`"
          >
            <q-tooltip class="bg-sky-500/95 text-sm shadow-4 text-black" anchor="top middle" self="bottom middle">
              {{ RANKS[props.row.lastRank] }}
            </q-tooltip>
          </q-img>
        </q-td>
      </template>

      <template #body-cell-demos="props">
        <q-td :props="props">
          <span class="text-base">{{ props.row.demos }}</span>
        </q-td>
      </template>

      <template #body-cell-lastTimestamp="props">
        <q-td :props="props">
          {{ date.formatDate(props.row.lastTimestamp * 1000, 'D MMM YYYY') }}
        </q-td>
      </template>

      <template #loading>
        <q-inner-loading showing color="primary" class="mt-4" />
      </template>
    </q-table>
  </q-page>
</template>

<style scoped lang="scss"></style>
