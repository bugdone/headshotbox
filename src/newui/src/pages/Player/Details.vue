<script setup lang="ts">
import { isArray, isEmpty, isString } from 'lodash';
import { onMounted, provide, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { RANKS } from 'src/constants/ranks';
import { Dictionary } from '@/types/common';
import type { PlayerInfoResponse, PlayerStats } from '@/types/player';

import { PlayerApi } from 'src/api/player';
import { ROUTES } from 'src/router/routes';
import { Format } from 'src/utils/formatters';
import Loader from 'src/utils/loader';

import StatsCard from 'components/Player/StatsCard.vue';
import Help from 'components/Player/Help.vue';
import FolderFilter from 'components/filters/FolderFilter.vue';
import DemoTypeFilter from 'components/filters/DemoTypeFilter.vue';
import MapFilter from 'components/filters/MapFilter.vue';
import RoundFilter from 'components/filters/RoundFilter.vue';
import TeamMatesFilter from 'components/filters/TeamMatesFilter.vue';
import DateRangeFilter from 'components/filters/DateRangeFilter.vue';
import PlayerBanned from 'components/Player/Banned.vue';
import PlayerCharts from 'components/Player/Charts.vue';
import PlayerDemos from 'components/Player/Demos.vue';
import PlayerWeaponStats from 'components/Player/WeaponStats.vue';

/* ====================== Data ====================== */

const route = useRoute();
const router = useRouter();
const stats = ref({} as PlayerStats);
const playerInfo = ref({} as PlayerInfoResponse);
const isLoading = ref(false);
const filtersExpanded = ref(false);
const filters = ref({} as Dictionary);
const tab = ref('demos');
const splitterModel = ref(20);

let steamId = '';

/* ====================== Methods ====================== */

const filtersChanged = async (newFilter: Dictionary) => {
  const filterKeys = Object.keys(newFilter);

  filterKeys.forEach((filterKey) => {
    if (newFilter[filterKey]) {
      filters.value[filterKey] = newFilter[filterKey];
    }

    if (
      (isString(newFilter[filterKey]) && (newFilter[filterKey] === 'All' || isEmpty(newFilter[filterKey]))) ||
      (isArray(newFilter[filterKey]) && newFilter[filterKey].length === 0)
    ) {
      delete filters.value[filterKey];
    }
  });

  stats.value = await PlayerApi.stats(steamId, filters.value);
};

/* ====================== Hooks ====================== */

onMounted(async () => {
  Loader.withMessage('Gathering player information');
  isLoading.value = true;

  if (!route.params.id) {
    await router.push({ name: ROUTES.playersList });
  }

  steamId = route.params.id as string;
  stats.value = await PlayerApi.stats(steamId, {});
  playerInfo.value = await PlayerApi.getPlayerInfo(steamId);

  Loader.hide();
  isLoading.value = false;
});

provide('steamId', route.params.id);
</script>

<template>
  <q-page class="px-6 py-1 column items-center main-container">
    <q-list bordered class="rounded-borders w-11/12 my-4" dense>
      <q-expansion-item
        expand-separator
        icon="mdi-filter-cog-outline"
        :label="filtersExpanded ? 'Hide Filters' : 'Show filters'"
        dense
        v-model="filtersExpanded"
      >
        <q-card>
          <q-card-section class="q-gutter-x-md row justify-center">
            <FolderFilter @changed="filtersChanged" class="filter-field" />
            <DemoTypeFilter @changed="filtersChanged" class="filter-field" />
            <MapFilter @changed="filtersChanged" class="filter-field" />
            <RoundFilter @changed="filtersChanged" class="filter-field" />
            <TeamMatesFilter @changed="filtersChanged" class="filter-teammates" />
            <DateRangeFilter @changed="filtersChanged" class="filter-date" />
          </q-card-section>
        </q-card>
      </q-expansion-item>
    </q-list>

    <div class="row justify-center" v-if="!isLoading && stats.rating">
      <div class="self-center column items-center q-gutter-md mr-3 player-info">
        <q-img
          :src="playerInfo[steamId]['avatarfull']"
          fit="contain"
          ratio="1"
          class="rounded-md"
          width="140px"
          height="140px"
        />
        <a
          :href="`https://steamcommunity.com/profiles/${steamId}`"
          target="_blank"
          class="text-xl break-normal text-center"
        >
          {{ playerInfo[steamId]['personaname'] }}
        </a>
        <q-img
          fit="cover"
          class="mx-2 my-2 rounded-lg"
          width="100px"
          height="40px"
          :src="`images/ranks/${stats.lastRank}.png`"
        >
          <q-tooltip class="bg-sky-500/95 text-sm shadow-4 text-black" anchor="top middle" self="bottom middle">
            {{ RANKS[stats.lastRank] }}
          </q-tooltip>
        </q-img>
      </div>

      <StatsCard
        class="mr-3"
        :header="{
          icon: 'mdi-trophy',
          label: 'Win Rate',
          color: 'green',
          value: Format.number((stats.won / (stats.won + stats.lost)) * 100) + '%',
        }"
        :stats="[
          { label: 'Played', value: stats.won + stats.tied + stats.lost },
          { label: 'Won', value: stats.won },
          { label: 'Lost', value: stats.lost },
          { label: 'Draw', value: stats.tied },
        ]"
      />

      <StatsCard
        class="mr-3"
        :header="{
          icon: 'mdi-chart-bar',
          label: 'Performance',
          color: 'primary',
        }"
        :stats="[
          {
            label: 'Rating',
            value: Format.number(stats.rating),
            tooltip: 'HLTV rating',
            needsColor: true,
          },
          {
            label: 'KDR',
            value: Format.number(stats.kills / stats.deaths),
            needsColor: true,
            tooltip: 'Kill/Death ratio',
          },
          {
            label: 'ADR',
            value: Format.number(stats.damage / stats.roundsWithDamageInfo),
            tooltip: 'Average damage per round. Takes into account only games after the 30th of June 2015 patch.',
          },
          {
            label: 'KPR',
            value: Format.number(stats.kills / stats.rounds),
            tooltip: 'Kills per round',
            needsColor: true,
          },
          {
            label: 'DPR',
            value: Format.number(stats.deaths / stats.rounds),
            tooltip: 'Deaths per round',
          },
          {
            label: 'APR',
            value: Format.number(stats.assists / stats.rounds),
            tooltip: 'Assists per round',
          },
        ]"
      >
        <template #Rating>
          <Help />
        </template>
      </StatsCard>

      <StatsCard
        :header="{
          icon: 'mdi-camera',
          label: 'Outstanding',
          color: 'purple',
        }"
        :stats="[
          {
            label: 'HS%',
            value: Format.number(stats.hsPercent),
            tooltip: 'Head Shots percent',
          },
          {
            label: '3k',
            value: stats.roundsWithKills[3],
            tooltip: 'Rounds with 3 kills',
          },
          {
            label: '4k',
            value: stats.roundsWithKills[4],
            tooltip: 'Rounds with 4 kills',
          },
          {
            label: '5k',
            value: stats.roundsWithKills[5],
            tooltip: 'Rounds with 5 kills',
          },
          {
            label: 'Entry Kill Win',
            value: Format.number((stats.entryKills / stats.entryKillsAttempted) * 100),
            tooltip: `First duel as T win percent (${stats.entryKills} / ${stats.entryKillsAttempted})`,
          },
          {
            label: 'First Duel Win',
            value: Format.number((stats.openKills / stats.openKillsAttempted) * 100),
            tooltip: `First duel as T or CT win percent (${stats.openKills} / ${stats.openKillsAttempted})`,
          },
        ]"
      />
    </div>

    <q-splitter v-model="splitterModel" class="py-6" disable style="min-width: 80%">
      <template #before>
        <q-tabs v-model="tab" class="text-primary" vertical>
          <q-tab name="demos" icon="mdi-history" label="Demos" />
          <q-tab name="weapon-stats" icon="mdi-bullseye" label="Weapon Stats" />
          <q-tab name="banned-players" icon="mdi-cancel" label="Banned Players" />
          <q-tab name="charts" icon="mdi-chart-bar" label="Charts" />
        </q-tabs>
      </template>

      <template #after>
        <q-tab-panels
          v-model="tab"
          animated
          class="min-w-full"
          swipeable
          transition-next="jump-up"
          transition-prev="jump-up"
          vertical
        >
          <q-tab-panel name="demos">
            <PlayerDemos />
          </q-tab-panel>

          <q-tab-panel name="weapon-stats">
            <PlayerWeaponStats />
          </q-tab-panel>

          <q-tab-panel name="banned-players">
            <PlayerBanned />
          </q-tab-panel>

          <q-tab-panel name="charts">
            <PlayerCharts />
          </q-tab-panel>
        </q-tab-panels>
      </template>
    </q-splitter>
  </q-page>
</template>

<style scoped lang="scss">
body.screen--lg,
body.screen--xl {
  .main-container {
    margin: 0 auto;
    max-width: 1200px;
  }
}

.filter-field {
  min-width: 100px;
}

.filter-date {
  width: 235px;
}

.filter-teammates {
  width: 250px;
}

.player-info {
  width: 250px;
}
</style>
