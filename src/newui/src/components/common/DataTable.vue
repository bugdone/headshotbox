<script setup lang="ts">
import { AxiosError } from 'axios';
import debounce from 'lodash/debounce';
import get from 'lodash/get';
import has from 'lodash/has';
import snakeCase from 'lodash/snakeCase';
import { useQuasar } from 'quasar';
import { computed, onMounted, ref } from 'vue';

import { DataTableHeader, DataTablePagination, DataTableRequestDetails } from '@/types/dataTable';

import notification from 'src/utils/notification';

/* ====================== Data ====================== */

const props = withDefaults(
  defineProps<{
    columns: DataTableHeader[];
    apiCall: (params: any) => any;
    entityName: string;
    rowsPerPage?: number | string;
    addActionsSlot?: boolean;
  }>(),
  {
    rowsPerPage: 20,
    addActionsSlot: false,
  }
);

const tableRef = ref();
const items = ref<any[]>([]);
const pagination = ref({
  page: 1,
  rowsPerPage: props.rowsPerPage,
  sortBy: '',
  descending: false,
} as DataTablePagination);
const isLoading = ref(false);
const $q = useQuasar();
const entityListSortKey = props.entityName + 'ListSortBy';

if (props.entityName && $q.localStorage.has(entityListSortKey)) {
  pagination.value.sortBy = $q.localStorage.getItem(entityListSortKey) as string;
}

const headers = computed(() => {
  let headers: DataTableHeader[] = props.columns.map((el) => {
    if (!has(el, 'align')) {
      el.align = 'center';
    }

    if (!has(el, 'field')) {
      el.field = el.name;
    }

    return el;
  }) as DataTableHeader[];

  if (props.addActionsSlot) {
    headers = headers.concat([{ classes: 'hs-action', field: 'actions', label: '', name: 'actions', sortable: false }]);
  }

  return headers;
});

/* ====================== Methods ====================== */

const emit = defineEmits<{
  (e: 'click:row', item: any): void;
}>();

const getData = debounce(async (tableProps: DataTableRequestDetails) => {
  isLoading.value = true;
  pagination.value.page = tableProps.pagination?.page;
  pagination.value.rowsPerPage = tableProps.pagination?.rowsPerPage;
  pagination.value.sortBy = tableProps.pagination?.sortBy;
  pagination.value.descending = tableProps.pagination?.descending;

  if (props.entityName && tableProps.pagination?.sortBy && tableProps.pagination?.sortBy.length > 0) {
    $q.localStorage.set(entityListSortKey, tableProps.pagination?.sortBy);
  }

  if (pagination.value.page && pagination.value.rowsPerPage) {
    await props
      .apiCall({
        ...{
          limit: pagination.value.rowsPerPage,
          offset: (pagination.value.page - 1) * pagination.value.rowsPerPage,
        },
        ...(pagination.value.sortBy && pagination.value.sortBy.length > 0
          ? { orderBy: snakeCase(pagination.value.sortBy) }
          : {}),
      })
      .then(
        (response: any) => {
          items.value = response.results;
          pagination.value.rowsNumber = response.count;
        },
        (error: AxiosError) => {
          // no results found, retry starting from page 1
          if (error.response?.status === 404) {
            pagination.value.page = 1;
            return tableRef.value.requestServerInteraction();
          }

          notification.error(error.response?.statusText as string);
        }
      );
  }

  isLoading.value = false;
}, 50);

const refresh = () => {
  tableRef.value.requestServerInteraction();
};

const rowClicked = (evt: Event, row: any) => {
  emit('click:row', row);
};

/* ====================== Hooks ====================== */

defineExpose({
  refresh,
});

onMounted(async () => {
  refresh();
});
</script>

<template>
  <q-table
    ref="tableRef"
    v-model:pagination="pagination"
    :columns="headers"
    :loading="isLoading"
    :pagination="pagination"
    :rows="items"
    :rows-per-page-options="[10, 20, 25, 30, 40, 50]"
    binary-state-sort
    class="mt-3"
    color="primary"
    dense
    flat
    loading-label="Loading ... Please wait"
    no-results-label="No results"
    separator="horizontal"
    wrap-cells
    @request="getData"
    @row-click="rowClicked"
  >
    <template #top>
      <div class="flex items-baseline justify-between w-full mb-2" v-show="pagination.rowsNumber > 0">
        <div>{{ pagination.rowsNumber }} Results</div>
        <slot name="top-Filter">
          <div></div>
        </slot>
      </div>
    </template>

    <template #header-cell="props">
      <q-th :props="props">
        <span class="text-base font-bold">{{ props.col.label }}</span>
      </q-th>
    </template>

    <template #loading>
      <q-inner-loading showing color="primary" class="mt-4" />
    </template>

    <template v-for="{ name: columnName } in columns" #[`body-cell-${columnName}`]="props" :key="columnName">
      <q-td :props="props" auto-width>
        <slot :name="columnName" v-bind="props.row">{{ get(props.row, columnName) }}</slot>
      </q-td>
    </template>
  </q-table>
</template>

<style scoped lang="scss"></style>
