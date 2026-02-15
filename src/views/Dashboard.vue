<script setup lang="ts">
import { useRouter } from "vue-router";
import { useMcpStore } from "@/stores/mcpStore";
import McpCard from "@/components/McpCard.vue";

const router = useRouter();
const store = useMcpStore();

async function handleConnect(id: string) {
  try {
    await store.connectMcp(id);
  } catch (e) {
    console.error("Connect failed:", e);
  }
}

async function handleDisconnect(id: string) {
  try {
    await store.disconnectMcp(id);
  } catch (e) {
    console.error("Disconnect failed:", e);
  }
}
</script>

<template>
  <div class="p-6 max-w-6xl mx-auto">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold text-surface-900">Dashboard</h1>
        <p class="text-sm text-surface-500 mt-0.5">
          Monitor and manage your MCP servers
        </p>
      </div>
      <button
        @click="router.push('/add')"
        class="inline-flex items-center gap-2 px-4 py-2 bg-surface-900 text-white rounded-lg text-sm font-medium hover:bg-surface-800 transition-colors"
      >
        <svg
          class="w-4 h-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 4v16m8-8H4"
          />
        </svg>
        Add MCP
      </button>
    </div>

    <!-- Stats row -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-8">
      <div class="bg-white rounded-lg border border-surface-200 p-4">
        <div class="text-2xl font-bold text-surface-900">
          {{ store.totalCount }}
        </div>
        <div class="text-xs text-surface-500 uppercase tracking-wider mt-1">
          Total MCPs
        </div>
      </div>
      <div
        class="bg-white rounded-lg border border-surface-200 p-4 border-l-4 border-l-emerald-500"
      >
        <div class="text-2xl font-bold text-emerald-600">
          {{ store.connectedCount }}
        </div>
        <div class="text-xs text-surface-500 uppercase tracking-wider mt-1">
          Connected
        </div>
      </div>
      <div
        class="bg-white rounded-lg border border-surface-200 p-4 border-l-4 border-l-amber-500"
      >
        <div class="text-2xl font-bold text-amber-600">
          {{ store.reconnectingCount }}
        </div>
        <div class="text-xs text-surface-500 uppercase tracking-wider mt-1">
          Reconnecting
        </div>
      </div>
      <div
        class="bg-white rounded-lg border border-surface-200 p-4 border-l-4 border-l-red-500"
      >
        <div class="text-2xl font-bold text-red-600">
          {{ store.errorCount }}
        </div>
        <div class="text-xs text-surface-500 uppercase tracking-wider mt-1">
          Errors
        </div>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="store.loading && store.totalCount === 0" class="text-center py-16">
      <div
        class="w-8 h-8 border-2 border-surface-300 border-t-surface-600 rounded-full animate-spin mx-auto mb-3"
      ></div>
      <p class="text-sm text-surface-500">Loading MCPs...</p>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="store.totalCount === 0"
      class="text-center py-16 bg-white rounded-xl border border-dashed border-surface-300"
    >
      <svg
        class="w-12 h-12 text-surface-300 mx-auto mb-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.5"
          d="M13 10V3L4 14h7v7l9-11h-7z"
        />
      </svg>
      <h3 class="text-base font-semibold text-surface-700 mb-1">
        No MCPs configured
      </h3>
      <p class="text-sm text-surface-500 mb-4">
        Add your first MCP server to get started.
      </p>
      <button
        @click="router.push('/add')"
        class="inline-flex items-center gap-2 px-4 py-2 bg-surface-900 text-white rounded-lg text-sm font-medium hover:bg-surface-800 transition-colors"
      >
        Add MCP
      </button>
    </div>

    <!-- MCP grid -->
    <div
      v-else
      class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
    >
      <McpCard
        v-for="status in store.statuses"
        :key="status.id"
        :status="status"
        @view-detail="router.push(`/mcp/${status.id}`)"
        @connect="handleConnect(status.id)"
        @disconnect="handleDisconnect(status.id)"
      />
    </div>

    <!-- Error banner -->
    <div
      v-if="store.error"
      class="mt-6 bg-red-50 border border-red-200 rounded-lg p-4 text-sm text-red-700"
    >
      {{ store.error }}
    </div>
  </div>
</template>
