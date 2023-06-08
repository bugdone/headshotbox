<script setup lang="ts">
import debounce from 'lodash/debounce';
import snakeCase from 'lodash/snakeCase';
import { date } from 'quasar';
import { onMounted, ref, watch } from 'vue';

import { RANKS } from 'src/constants/ranks';
import type { DataTableHeader, DataTablePagination, DataTableRequestDetails } from '@/types/dataTable';
import type { PlayerResponse } from '@/types/player';

import { ConfigApi } from 'src/api/config';
import { PlayerApi } from 'src/api/player';
import { ROUTES } from 'src/router/routes';

/* ====================== Data ====================== */

const tableRef = ref();
const players = ref([] as PlayerResponse[]);
const columns = [
  { label: 'Name', name: 'steamInfo', field: 'steamInfo', align: 'left', style: 'width: 35%' },
  { label: 'MM Rank', name: 'lastRank', field: 'lastRank', align: 'center', sortable: true },
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
const selectedFolder = ref('All');
let folders = [] as string[];

/* ====================== Methods ====================== */

const getData = debounce(async (tableProps: DataTableRequestDetails) => {
  isLoading.value = true;
  pagination.value.page = tableProps.pagination?.page || pagination.value.page;
  pagination.value.rowsPerPage = tableProps.pagination?.rowsPerPage || pagination.value.rowsPerPage;
  pagination.value.sortBy = tableProps.pagination?.sortBy || 'last_timestamp';
  pagination.value.descending = tableProps.pagination?.descending || pagination.value.descending;

  if (pagination.value.page && pagination.value.rowsPerPage) {
    const data = await PlayerApi.get({
      ...{
        limit: pagination.value.rowsPerPage,
        offset: (pagination.value.page - 1) * pagination.value.rowsPerPage,
      },
      ...(pagination.value.sortBy && pagination.value.sortBy.length > 0
        ? { orderBy: snakeCase(pagination.value.sortBy) }
        : {}),
      ...(selectedFolder.value !== 'All' ? { folder: selectedFolder.value } : {}),
    });

    players.value = data.players;
    pagination.value.rowsNumber = data.playerCount;
  }

  isLoading.value = false;
}, 50);

/* ====================== Hooks ====================== */

onMounted(async () => {
  tableRef.value.requestServerInteraction();

  folders = await ConfigApi.folders();
  folders.unshift('All');
});

watch(
  selectedFolder,
  debounce(() => {
    tableRef.value.requestServerInteraction();
  }, 100)
);
</script>

<template>
  <q-page class="px-6 py-1 main-container">
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
      dense
      flat
      separator="horizontal"
    >
      <template #top>
        <div class="flex items-baseline justify-between w-full mb-2" v-show="pagination.rowsNumber > 0">
          <div>{{ pagination.rowsNumber }} Results</div>
          <div style="min-width: 100px">
            <q-select v-model="selectedFolder" :options="folders" label="Folders" dense />
          </div>
        </div>
      </template>

      <template #header-cell="props">
        <q-th :props="props">
          <span class="text-base font-bold">{{ props.col.label }}</span>
        </q-th>
      </template>

      <template #body-cell-steamInfo="props">
        <q-td :props="props">
          <div class="row no-wrap">
            <q-img
              v-if="props.row.steamInfo"
              :src="props.row.steamInfo.avatarfull || ''"
              fit="cover"
              width="40px"
              height="40px"
              class="mr-2 rounded-md"
            />
            <q-icon v-else name="mdi-account" size="60px" color="primary" />
            <router-link
              class="text-base self-center"
              :to="{ name: ROUTES.playerDetails, params: { id: props.row.steamid } }"
            >
              {{ props.row.steamInfo ? props.row.steamInfo.personaname : props.row.name }}
            </router-link>
          </div>
        </q-td>
      </template>

      <template #body-cell-lastRank="props">
        <q-td :props="props">
          <q-img
            fit="cover"
            class="my-1 rounded-lg"
            width="90px"
            height="38px"
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
