<script setup lang="ts">
import { computed } from "vue";
import type { McpStatus } from "@/types";
import { ConnectionState, TRANSPORT_LABELS } from "@/types";
import StatusBadge from "./StatusBadge.vue";

const props = defineProps<{
  status: McpStatus;
}>();

const emit = defineEmits<{
  "view-detail": [];
  connect: [];
  disconnect: [];
}>();

const borderColor = computed(() => {
  const map: Record<ConnectionState, string> = {
    [ConnectionState.Connected]: "border-l-emerald-500",
    [ConnectionState.Connecting]: "border-l-blue-500",
    [ConnectionState.Reconnecting]: "border-l-amber-500",
    [ConnectionState.Error]: "border-l-red-500",
    [ConnectionState.Disconnected]: "border-l-surface-300",
  };
  return map[props.status.state] || "border-l-surface-300";
});

function formatUptime(seconds?: number): string {
  if (!seconds) return "—";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${seconds}s`;
}
</script>

<template>
  <div
    class="bg-white rounded-lg shadow-sm border border-surface-200 border-l-4 hover:shadow-md transition-shadow cursor-pointer"
    :class="borderColor"
    @click="emit('view-detail')"
  >
    <div class="p-4">
      <!-- Header -->
      <div class="flex items-start justify-between mb-3">
        <div class="min-w-0 flex-1">
          <h3 class="text-sm font-semibold text-surface-900 truncate">
            {{ status.name }}
          </h3>
          <p class="text-xs text-surface-400 mt-0.5">
            {{ TRANSPORT_LABELS[status.transport_type] }}
          </p>
        </div>
        <StatusBadge :state="status.state" size="sm" />
      </div>

      <!-- Stats -->
      <div class="grid grid-cols-3 gap-3 text-center">
        <div>
          <div class="text-lg font-bold text-surface-800">
            {{ status.tools_count }}
          </div>
          <div class="text-[10px] text-surface-400 uppercase tracking-wider">
            Tools
          </div>
        </div>
        <div>
          <div class="text-lg font-bold text-surface-800">
            {{ status.resources_count }}
          </div>
          <div class="text-[10px] text-surface-400 uppercase tracking-wider">
            Resources
          </div>
        </div>
        <div>
          <div class="text-lg font-bold text-surface-800">
            {{ formatUptime(status.uptime_seconds) }}
          </div>
          <div class="text-[10px] text-surface-400 uppercase tracking-wider">
            Uptime
          </div>
        </div>
      </div>
    </div>

    <!-- Footer actions -->
    <div
      class="flex border-t border-surface-100 divide-x divide-surface-100"
      @click.stop
    >
      <button
        class="flex-1 px-3 py-2 text-xs font-medium text-surface-600 hover:bg-surface-50 transition-colors"
        @click="emit('view-detail')"
      >
        Details
      </button>
      <button
        v-if="status.state !== ConnectionState.Connected"
        class="flex-1 px-3 py-2 text-xs font-medium text-emerald-600 hover:bg-emerald-50 transition-colors"
        @click="emit('connect')"
      >
        Connect
      </button>
      <button
        v-else
        class="flex-1 px-3 py-2 text-xs font-medium text-surface-500 hover:bg-surface-50 transition-colors"
        @click="emit('disconnect')"
      >
        Disconnect
      </button>
    </div>
  </div>
</template>
