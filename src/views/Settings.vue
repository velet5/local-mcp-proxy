<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useMcpStore } from "@/stores/mcpStore";
import type { AppConfig } from "@/types";

const store = useMcpStore();

const form = ref<AppConfig>({
  proxy_port: 3001,
  health_check_interval_secs: 30,
  auto_reconnect: true,
  max_reconnect_attempts: 5,
  mcps: [],
});

const saving = ref(false);
const saved = ref(false);
const error = ref("");

async function loadConfig() {
  await store.fetchAppConfig();
  if (store.appConfig) {
    form.value = { ...store.appConfig };
  }
}

async function handleSave() {
  error.value = "";
  saving.value = true;

  try {
    if (form.value.proxy_port < 1024 || form.value.proxy_port > 65535) {
      throw new Error("Port must be between 1024 and 65535.");
    }
    if (form.value.health_check_interval_secs < 5) {
      throw new Error("Health check interval must be at least 5 seconds.");
    }

    await store.updateAppConfig(form.value);
    saved.value = true;
    setTimeout(() => {
      saved.value = false;
    }, 2000);
  } catch (e) {
    error.value = `${e}`;
  } finally {
    saving.value = false;
  }
}

onMounted(loadConfig);
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto">
    <h1 class="text-2xl font-bold text-surface-900 mb-1">Settings</h1>
    <p class="text-sm text-surface-500 mb-6">
      Configure Local MCP Proxy global settings.
    </p>

    <div
      class="bg-white rounded-lg border border-surface-200 divide-y divide-surface-100"
    >
      <!-- Proxy port -->
      <div class="p-5">
        <label class="block text-sm font-medium text-surface-700 mb-1.5"
          >Proxy Server Port</label
        >
        <input
          v-model.number="form.proxy_port"
          type="number"
          min="1024"
          max="65535"
          class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent"
        />
        <p class="text-xs text-surface-400 mt-1">
          The local port for the SSE proxy server. External apps connect here.
          Requires restart to take effect.
        </p>
      </div>

      <!-- Health check interval -->
      <div class="p-5">
        <label class="block text-sm font-medium text-surface-700 mb-1.5"
          >Health Check Interval (seconds)</label
        >
        <input
          v-model.number="form.health_check_interval_secs"
          type="number"
          min="5"
          max="300"
          class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent"
        />
        <p class="text-xs text-surface-400 mt-1">
          How often to ping each connected MCP server.
        </p>
      </div>

      <!-- Auto reconnect -->
      <div class="p-5">
        <label class="flex items-center gap-3 cursor-pointer">
          <input
            v-model="form.auto_reconnect"
            type="checkbox"
            class="w-4 h-4 rounded border-surface-300 text-surface-900 focus:ring-surface-900"
          />
          <div>
            <span class="text-sm font-medium text-surface-700"
              >Auto Reconnect</span
            >
            <p class="text-xs text-surface-400">
              Automatically reconnect to MCPs when they disconnect or error.
            </p>
          </div>
        </label>
      </div>

      <!-- Max reconnect attempts -->
      <div class="p-5">
        <label class="block text-sm font-medium text-surface-700 mb-1.5"
          >Max Reconnect Attempts</label
        >
        <input
          v-model.number="form.max_reconnect_attempts"
          type="number"
          min="1"
          max="100"
          class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent"
        />
        <p class="text-xs text-surface-400 mt-1">
          Stop reconnecting after this many consecutive failures.
        </p>
      </div>

      <!-- Save -->
      <div class="p-5 flex items-center gap-3">
        <button
          @click="handleSave"
          :disabled="saving"
          class="px-4 py-2.5 bg-surface-900 text-white rounded-lg text-sm font-medium hover:bg-surface-800 transition-colors disabled:opacity-50"
        >
          {{ saving ? "Saving..." : "Save Settings" }}
        </button>
        <span v-if="saved" class="text-sm text-emerald-600 font-medium">
          Settings saved!
        </span>
        <span v-if="error" class="text-sm text-red-600"> {{ error }} </span>
      </div>
    </div>
  </div>
</template>
