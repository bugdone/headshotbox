<script setup lang="ts">
import { api } from 'boot/axios'

api.get('players', { params: {limit: 50, offset: 0} }).then((data) => console.log(data));
</script>

<template>
  <q-page class="row items-center justify-evenly"> Test page </q-page>
</template>
