<script setup lang="ts">
import { now } from 'lodash';
import { date } from 'quasar';
import { ref } from 'vue';

import { DEMO_TYPE_IMAGES } from 'src/constants/demos';
import { RANKS } from 'src/constants/ranks';
import { ApiRequestParams } from '@/types/api';
import { DataTableHeader } from '@/types/dataTable';
import { DemoResponse } from '@/types/demo';

import { DemoApi } from 'src/api/demo';
import { Format } from 'src/utils/formatters';

import DataTable from 'components/common/DataTable.vue';
import MatchDetails from 'components/Player/MatchDetails.vue';

/* ====================== Data ====================== */

const props = defineProps<{
  steamId: string;
}>();

const tableRef = ref();
const columns = [
  { label: 'Date', name: 'timestamp', align: 'left', style: 'width: 75px' },
  // { label: 'Type', name: 'type' },
  { label: 'Rank', name: 'mmRankUpdate' },
  { label: 'Map', name: 'map' },
  { label: 'Score', name: 'score' },
  { label: 'K', name: 'kills' },
  { label: 'A', name: 'assists' },
  { label: 'D', name: 'deaths' },
  { label: 'KDD', name: 'kdd' },
  { label: 'ADR', name: 'adr' },
] as DataTableHeader[];
const matchDetailsRef = ref();

/* ====================== Methods ====================== */

const getDemos = async ({ limit, offset, orderBy }: ApiRequestParams) => {
  const { demos, demoCount } = await DemoApi.list(props.steamId, { ...{ limit, offset, orderBy } });

  return {
    results: demos,
    count: demoCount,
  };
};

const formatDate = (timestamp: number) => {
  const timestampYear = date.formatDate(timestamp * 1000, 'YYYY');
  const currentYear = date.formatDate(now(), 'YYYY');

  const dateFormat = timestampYear === currentYear ? 'D MMM, H:mm' : 'D MMM YY, H:mm';

  return date.formatDate(timestamp * 1000, dateFormat);
};

const showMatchDetails = (row: DemoResponse) => {
  matchDetailsRef.value.open(row.demoid);
};
</script>

<template>
  <q-page class="">
    <DataTable
      ref="tableRef"
      :apiCall="getDemos"
      :columns="columns"
      entityName="demo"
      rowsPerPage="25"
      @click:row="showMatchDetails"
    >
      <template #timestamp="item: DemoResponse">
        {{ formatDate(item.timestamp) }}
      </template>

      <template #type="item: DemoResponse">
        <q-img v-if="item.type" :src="DEMO_TYPE_IMAGES[item.type] as string" width="50px" />
      </template>

      <template #mmRankUpdate="{ mmRankUpdate }: DemoResponse">
        <q-img
          :src="`images/ranks/${(mmRankUpdate && mmRankUpdate.rankNew) ?? 0}.png`"
          class="my-1 rounded"
          fit="fill"
          height="19px"
          width="45px"
        >
          <q-tooltip class="bg-sky-500/95 text-sm shadow-4 text-black" anchor="top middle" self="bottom middle">
            {{ RANKS[(mmRankUpdate && mmRankUpdate.rankNew) ?? 0] }}
          </q-tooltip>
        </q-img>
      </template>

      <template #score="{ score }: DemoResponse">
        <span
          :class="score[0] < score[1] ? 'text-red' : score[0] === score[1] ? 'text-blue-500' : 'text-green-500'"
          class="font-semibold"
        >
          {{ score[0] }} - {{ score[1] }}
        </span>
      </template>

      <template #kdd="{ kills, deaths }: DemoResponse">
        <span :class="kills - deaths < 0 ? 'text-red' : 'text-green-500'" class="font-semibold">
          {{ kills - deaths }}
        </span>
      </template>

      <template #adr="{ damage, roundsWithDamageInfo }: DemoResponse">
        {{ Format.number(damage / roundsWithDamageInfo) }}
      </template>
    </DataTable>
    <MatchDetails ref="matchDetailsRef" />
  </q-page>
</template>

<style scoped lang="scss"></style>
