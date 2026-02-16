<script setup lang="ts">
import { onMounted } from "vue";
import { useMcpStore } from "@/stores/mcpStore";

const store = useMcpStore();

onMounted(() => {
  store.init();
});
</script>

<template>
  <div class="flex h-screen w-full">
    <!-- Sidebar -->
    <aside class="w-56 bg-surface-900 text-white flex flex-col shrink-0 select-none">
      <div class="px-5 py-5 border-b border-surface-700">
        <h1 class="text-lg font-bold tracking-tight flex items-center gap-2">
          <svg class="w-5 h-5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
          MCP Hub
        </h1>
      </div>

      <nav class="flex-1 py-3 space-y-1 px-3">
        <router-link to="/" class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors"
          :class="$route.path === '/'
            ? 'bg-surface-700 text-white'
            : 'text-surface-300 hover:bg-surface-800 hover:text-white'
            ">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zm10 0a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zm10 0a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
          </svg>
          Dashboard
        </router-link>

        <router-link to="/add"
          class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors" :class="$route.path === '/add'
            ? 'bg-surface-700 text-white'
            : 'text-surface-300 hover:bg-surface-800 hover:text-white'
            ">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          Add MCP
        </router-link>

        <router-link to="/settings"
          class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors" :class="$route.path === '/settings'
            ? 'bg-surface-700 text-white'
            : 'text-surface-300 hover:bg-surface-800 hover:text-white'
            ">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          Settings
        </router-link>

        <router-link to="/logs"
          class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors" :class="$route.path === '/logs'
            ? 'bg-surface-700 text-white'
            : 'text-surface-300 hover:bg-surface-800 hover:text-white'
            ">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M9 17v-2a4 4 0 014-4h4" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M9 5h6a2 2 0 012 2v10a2 2 0 01-2 2H9a2 2 0 01-2-2V7a2 2 0 012-2z" />
          </svg>
          Logs
        </router-link>
      </nav>

      <!-- Status summary in sidebar footer -->
      <div class="px-5 py-4 border-t border-surface-700 text-xs space-y-1">
        <div class="flex justify-between text-surface-400">
          <span>Total</span>
          <span class="text-white font-semibold">{{ store.totalCount }}</span>
        </div>
        <div class="flex justify-between text-surface-400">
          <span>Connected</span>
          <span class="text-emerald-400 font-semibold">{{
            store.connectedCount
          }}</span>
        </div>
        <div v-if="store.errorCount > 0" class="flex justify-between">
          <span class="text-surface-400">Errors</span>
          <span class="text-red-400 font-semibold">{{
            store.errorCount
          }}</span>
        </div>
      </div>
    </aside>

    <!-- Main content -->
    <main class="flex-1 overflow-y-auto bg-surface-50">
      <router-view />
    </main>
  </div>
</template>
