<script setup lang="ts">
/* ====================== Data ====================== */

defineProps({
  header: {
    type: Object as () => { icon?: string; color?: string; label?: string; value?: string | number },
    default: () => ({}),
  },
  stats: {
    type: Array as () => { label: string; value: string | number; tooltip?: string; needsColor?: boolean }[],
  },
});

/* ====================== Methods ====================== */

const getSkillColor = (value: number) => {
  if (value < 0.7) {
    return 'red';
  }

  if (value >= 0.7 && value < 1) {
    return 'orange';
  }

  return 'green';
};
</script>

<template>
  <q-card class="my-card">
    <q-item dense>
      <q-item-section v-if="header.icon" avatar>
        <q-icon :name="header.icon" size="4rem" :color="header.color" />
      </q-item-section>

      <q-item-section v-if="header.label">
        <q-item-label class="text-base">{{ header.label }}</q-item-label>
      </q-item-section>
      <q-item-section v-if="header.value" side>
        <q-item-label class="text-2xl text-black text-bold font-mono">
          {{ header.value }}
        </q-item-label>
      </q-item-section>
    </q-item>

    <q-separator />

    <q-card-section>
      <q-list separator dense>
        <q-item v-for="stat in stats" :key="stat.label">
          <q-item-section class="font-medium">
            <div class="row">
              {{ stat.label }}
              <template v-if="stat.tooltip">
                <q-icon class="ml-2 self-center" name="mdi-information-outline" color="primary">
                  <q-tooltip
                    class="bg-white text-sm shadow-4 text-black max-w-xs"
                    anchor="top middle"
                    self="bottom middle"
                  >
                    {{ stat.tooltip }}
                  </q-tooltip>
                </q-icon>
              </template>
              <slot :name="stat.label"></slot>
            </div>
          </q-item-section>
          <q-item-section side class="font-mono">
            <span :class="stat.needsColor ? 'text-' + getSkillColor(stat.value as number) : 'text-black'">
              {{ stat.value }}
            </span>
          </q-item-section>
        </q-item>
      </q-list>
    </q-card-section>
  </q-card>
</template>

<style scoped lang="scss">
.my-card {
  min-width: 280px;
  max-width: 300px;
}
</style>
