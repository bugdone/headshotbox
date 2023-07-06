<script setup lang="ts">
import { date, DateOptions } from 'quasar';
import { computed, ref } from 'vue';
import addToDate = date.addToDate;

/* ====================== Data ====================== */

const dateRange = ref({} as { from: string; to: string });

const emits = defineEmits(['changed']);

const humanDate = computed(() => {
  if (dateRange.value) {
    if (dateRange.value.from && dateRange.value.to) {
      return dateRange.value.from + ' - ' + dateRange.value.to;
    }

    if (dateRange.value.from) {
      return dateRange.value.from + ' - ?';
    }
  }

  return '';
});

/* ====================== Methods ====================== */

const changed = () => {
  if (dateRange.value && dateRange.value.from && dateRange.value.to) {
    emits('changed', {
      startDate: date.formatDate(dateRange.value.from, 'X'),
      endDate: date.formatDate(dateRange.value.to, 'X'),
    });
  }
};

const pickStart = (from: DateOptions) => {
  dateRange.value.from = date.formatDate(date.buildDate(from), 'DD MMM YYYY');
  dateRange.value.to = date.formatDate(addToDate(date.buildDate(from), { year: 1 }), 'DD MMM YYYY');

  emits('changed', {
    startDate: from ? date.formatDate(`${from.year}-${from.month}-${from.day}`, 'X') : '',
  });
};

const clear = () => {
  dateRange.value = { from: '', to: '' };

  emits('changed', {
    startDate: '',
    endDate: '',
  });
};
</script>

<template>
  <q-input v-model="humanDate" clearable dense @clear="clear">
    <template #append>
      <q-icon class="cursor-pointer" name="event">
        <q-popup-proxy cover transition-hide="scale" transition-show="scale">
          <q-date v-model="dateRange" mask="DD MMM YYYY" range @update:modelValue="changed" @range-start="pickStart">
            <div class="row items-center justify-end">
              <q-btn v-close-popup color="primary" flat label="Close" />
            </div>
          </q-date>
        </q-popup-proxy>
      </q-icon>
    </template>
  </q-input>
</template>

<style scoped lang="scss"></style>
