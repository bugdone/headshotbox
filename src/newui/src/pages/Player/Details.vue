<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import type { PlayerInfoResponse, PlayerStats } from '@/types/player';

import { PlayerApi } from 'src/api/player';
import { ROUTES } from 'src/router/routes';
import { Format } from 'src/utils/formatters';
import Loader from 'src/utils/loader';

import StatsCard from 'components/Player/StatsCard.vue';

/* ====================== Data ====================== */

const route = useRoute();
const router = useRouter();
const stats = ref({} as PlayerStats);
const playerInfo = ref({} as PlayerInfoResponse);
const isLoading = ref(false);

let steamId = '';

/* ====================== Hooks ====================== */

onMounted(async () => {
  Loader.withMessage('Gathering player information');
  isLoading.value = true;

  if (!route.params.id) {
    await router.push({ name: ROUTES.playersList });
  }

  steamId = route.params.id as string;
  stats.value = await PlayerApi.stats(steamId);
  playerInfo.value = await PlayerApi.getPlayerInfo(steamId);

  Loader.hide();
  isLoading.value = false;
});
</script>

<template>
  <q-page class="p-6">
    <div class="row justify-center q-gutter-lg" v-if="!isLoading && stats.rating">
      <div class="self-center column items-center q-gutter-md mx-6">
        <q-img
          :src="playerInfo[steamId]['avatarfull']"
          fit="contain"
          ratio="1"
          class="rounded-md"
          width="140px"
          height="140px"
        />
        <a :href="`https://steamcommunity.com/profiles/${steamId}`" target="_blank" class="text-2xl">
          {{ playerInfo[steamId]['personaname'] }}
        </a>
      </div>

      <StatsCard
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
      />

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
  </q-page>
</template>

<style scoped lang="scss"></style>
