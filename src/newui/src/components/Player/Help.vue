<script setup lang="ts">
import { ref } from 'vue';

/* ====================== Vars ====================== */

const isModalVisible = ref(false);

const html = `
<div class="q-gutter-lg mx-4">
  <div class="text-2xl my-4">How is rating calculated?</div>

  Rating is calculated by comparing a player's average stats...
  <ul class="list-disc ml-4 leading-normal">
    <li>kills per round</li>
    <li>survived rounds per round</li>
    <li>a value based on number of rounds with multiple kills</li>
  </ul>

  ...to the same averages for Counter Strike in general (which we calculated over a period of time).

  <br/><br/>
  Average number of kills per round is 0.679, so if a player had 22 kills in 18 rounds, or 1.22 kills per round, he would have a Kill-Rating of 1.80 (1.22/0.679), meaning he did 80% better than expected on average in the kills department.
  <br/><br/>
  In the same way a Survival-Rating is calculated, as well as a RoundsWithMultipleKills-Rating for which you can see the formula below. These three values are then added together, with Survival-Rating participating with a 0.7 factor (as surviving a round is less important that getting a kill) and then divided by 2.7 to give a 1.0 value for an average performance.
  <br/><br/>
  Note: Survived rounds are used instead of deaths, as deaths are reversely proportionate to other stats (having less is better, and more is worse).

  <div class="text-2xl my-4">What's wrong with K/D ratio?</div>

  K/D ratio has several downsides, mainly, it favors players who play fewer rounds and therefore have fewer deaths, giving them high K/D ratios. That is why it isn't really useful for comparing performances from different matches.
  <br/><br/>
  For example:
  <br/><br/>
  Player 1 had 13-6 (K/D = 2.2) in a match of 18 rounds and Player 2 had 37-17 (K/D = 2.2) in a match of 30 rounds.
  <br/><br/>
  K/D ratio would suggest they had a similar performance, while Player 1 actually had an average game with 13 kills in 18 rounds, with 5 rounds with 1 kill (1K rounds) and 4 rounds with 2 kills (2K rounds), and Player 2 had a great game with 37 kills in 30 rounds of a close match with nine 2K rounds, four 3K rounds a one 4K round. If we apply the Rating on the two performances, we get that Player 1 had 1.28, while Player2 had a much higher 1.91.
  <br/><br/>
  Another problem can be the fact that K/D ratio doesn't have an actual limit, since it can go from 0 to infinity. That makes it hard to compare performances from a single match as well.
  <br/><br/>
  For example:
  <br/><br/>
  Player 1 has 22-4 (K/D = 5.5) in 18 rounds, and in the same match Player 2 has 20-9 (K/D = 2.2). K/D ratio would suggest that Player 1 had a much better game than Player 2, while that wasn't actually the case, as he had only 2 kills more, and 5 deaths less. Rating shows Player 1 had 1.87 and Player 2 had 1.56, suggesting Player 1 had around 20% better performance.

  <div class="text-2xl my-4">Advantages of Rating</div>

  Rating disregards the amount of rounds played in a match, as it considers average values, so it is very useful for comparing performances. It has a limited range, as it can go from 0 to 3 (although rarely over 2, which is then an amazing performance). It also has a well spread range of values that should reflect properly on how many average, good and great performances there are.
  <br/><br/>
  So here is how Rating is calculated in detail:

  <code style="background: #333; color: white; padding: 8px; display: block;">
    (KillRating + 0.7*SurvivalRating + RoundsWithMultipleKillsRating)/2.7
    <br/><br/>
    KillRating = Kills/Rounds/AverageKPR<br/>
    SurvivalRating = (Rounds-Deaths)/Rounds/AverageSPR<br/>
    RoundsWithMultipleKillsRating = (1K + 4*2K + 9*3K + 16*4K + 25*5K)/Rounds/AverageRMK
    <br/><br/>
    AverageKPR = 0.679 (average kills per round)<br/>
    AverageSPR = 0.317 (average survived rounds per round)<br/>
    AverageRMK = 1.277 (average value calculated from rounds with multiple kills: (1K + 4*2K + 9*3K + 16*4K + 25*5K)/Rounds)
    <br/><br/>
    1K = Number of rounds with 1 kill<br/>
    2K = Number of rounds with 2 kill<br/>
    3K = Number of rounds with 3 kill<br/>
    4K = Number of rounds with 4 kill<br/>
    5K = Number of rounds with 5 kill<br/>
  </code>
</div>
`;
</script>

<template>
  <div class="inline-block float-right -mt-2">
    <q-btn
      flat
      color="primary"
      no-caps
      icon="mdi-help-circle"
      size="sm"
      rounded
      class="p-0 m-0 mt-2 ml-1"
      @click="isModalVisible = true"
    />

    <q-dialog v-model="isModalVisible">
      <q-card>
        <q-card-section class="row items-center q-pb-none pb-4">
          <div class="text-h6">Rating calculation</div>
          <q-space />
          <q-btn icon="close" flat round dense v-close-popup />
        </q-card-section>

        <q-separator />

        <q-card-section class="content-wrapper scroll">
          <div v-html="html"></div>
        </q-card-section>

        <q-card-actions class="pr-6" align="right">
          <q-btn outline label="OK" color="primary" v-close-popup />
        </q-card-actions>
      </q-card>
    </q-dialog>
  </div>
</template>

<style lang="scss" scoped>
.q-card {
  max-width: 80vw;
  width: 900px;
}

.content-wrapper {
  font-size: 14px;
  max-height: 80vh;
}

.q-expansion-item:deep(.q-item__label) {
  margin-left: -14px;
  font-size: 16px;
  font-weight: bold;
}

code {
  background: #333;
  color: white;
  padding: 8px;
  display: block;
}

p {
  margin: 14px 0;
}

.text-error {
  color: #cd3b00;
}

.link,
.card-code strong {
  color: #00cdac;
}
</style>
