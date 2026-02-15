<script setup lang="ts">
import { computed } from "vue";
import { ConnectionState } from "@/types";

const props = defineProps<{
  state: ConnectionState;
  size?: "sm" | "md";
}>();

const label = computed(() => {
  const map: Record<ConnectionState, string> = {
    [ConnectionState.Connected]: "Connected",
    [ConnectionState.Connecting]: "Connecting",
    [ConnectionState.Reconnecting]: "Reconnecting",
    [ConnectionState.Error]: "Error",
    [ConnectionState.Disconnected]: "Disconnected",
  };
  return map[props.state] || props.state;
});

const classes = computed(() => {
  const base =
    props.size === "sm"
      ? "text-[10px] px-1.5 py-0.5"
      : "text-xs px-2.5 py-1";

  const colorMap: Record<ConnectionState, string> = {
    [ConnectionState.Connected]: "bg-emerald-100 text-emerald-700",
    [ConnectionState.Connecting]: "bg-blue-100 text-blue-700",
    [ConnectionState.Reconnecting]: "bg-amber-100 text-amber-700",
    [ConnectionState.Error]: "bg-red-100 text-red-700",
    [ConnectionState.Disconnected]: "bg-surface-200 text-surface-600",
  };

  return `${base} ${colorMap[props.state] || "bg-surface-200 text-surface-600"}`;
});

const dotColor = computed(() => {
  const map: Record<ConnectionState, string> = {
    [ConnectionState.Connected]: "bg-emerald-500",
    [ConnectionState.Connecting]: "bg-blue-500 animate-pulse",
    [ConnectionState.Reconnecting]: "bg-amber-500 animate-pulse",
    [ConnectionState.Error]: "bg-red-500",
    [ConnectionState.Disconnected]: "bg-surface-400",
  };
  return map[props.state] || "bg-surface-400";
});
</script>

<template>
  <span
    :class="classes"
    class="inline-flex items-center gap-1.5 rounded-full font-semibold uppercase tracking-wide"
  >
    <span class="w-1.5 h-1.5 rounded-full" :class="dotColor"></span>
    {{ label }}
  </span>
</template>
