<script setup lang="ts">
import { computed, ref } from "vue";
import { useMcpStore } from "@/stores/mcpStore";

const store = useMcpStore();

const showWarn = ref(true);
const showError = ref(true);

const filteredLogs = computed(() => {
  const allowed = new Set<string>();
  if (showWarn.value) allowed.add("warn");
  if (showError.value) allowed.add("error");

  return store.logs
    .filter((entry) => allowed.has(entry.level.toLowerCase()))
    .slice()
    .reverse();
});

const warnCount = computed(
  () => store.logs.filter((entry) => entry.level.toLowerCase() === "warn").length,
);

const errorCount = computed(
  () => store.logs.filter((entry) => entry.level.toLowerCase() === "error").length,
);
</script>

<template>
  <div class="p-6 max-w-6xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold text-surface-900">Logs</h1>
        <p class="text-sm text-surface-500 mt-0.5">
          Application-wide warnings and errors.
        </p>
      </div>
      <div class="flex items-center gap-2 text-xs">
        <span class="px-2 py-1 rounded-full bg-amber-100 text-amber-700">
          Warn {{ warnCount }}
        </span>
        <span class="px-2 py-1 rounded-full bg-red-100 text-red-700">
          Error {{ errorCount }}
        </span>
      </div>
    </div>

    <div class="bg-white rounded-lg border border-surface-200 p-4 mb-4">
      <div class="flex items-center gap-4 text-sm">
        <label class="flex items-center gap-2">
          <input
            v-model="showWarn"
            type="checkbox"
            class="w-4 h-4 rounded border-surface-300 text-amber-600 focus:ring-amber-600"
          />
          <span class="text-surface-700">Warn</span>
        </label>
        <label class="flex items-center gap-2">
          <input
            v-model="showError"
            type="checkbox"
            class="w-4 h-4 rounded border-surface-300 text-red-600 focus:ring-red-600"
          />
          <span class="text-surface-700">Error</span>
        </label>
      </div>
    </div>

    <div
      v-if="filteredLogs.length === 0"
      class="bg-white rounded-lg border border-dashed border-surface-200 p-8 text-center"
    >
      <p class="text-sm text-surface-500">No warnings or errors yet.</p>
    </div>

    <div v-else class="bg-white rounded-lg border border-surface-200 divide-y">
      <div
        v-for="(entry, index) in filteredLogs"
        :key="`${entry.timestamp}-${index}`"
        class="px-4 py-3 text-sm"
      >
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <div class="flex items-center gap-2 mb-1">
              <span
                v-if="entry.level.toLowerCase() === 'warn'"
                class="px-2 py-0.5 rounded-full text-xs bg-amber-100 text-amber-700"
                >WARN</span
              >
              <span
                v-else
                class="px-2 py-0.5 rounded-full text-xs bg-red-100 text-red-700"
                >ERROR</span
              >
              <span class="text-xs text-surface-400">{{ entry.target }}</span>
            </div>
            <p class="text-surface-800 break-words">{{ entry.message }}</p>
          </div>
          <div class="text-xs text-surface-400 whitespace-nowrap">
            {{ new Date(entry.timestamp).toLocaleString() }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
