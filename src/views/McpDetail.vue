<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMcpStore } from "@/stores/mcpStore";
import { ConnectionState, TRANSPORT_LABELS } from "@/types";
import StatusBadge from "@/components/StatusBadge.vue";
import ToolList from "@/components/ToolList.vue";
import ResourceList from "@/components/ResourceList.vue";

const route = useRoute();
const router = useRouter();
const store = useMcpStore();

const id = computed(() => route.params.id as string);
const detail = computed(() => store.details.get(id.value));
const loading = ref(true);
const proxyUrl = ref("");
const copied = ref(false);
const activeTab = ref<"tools" | "resources" | "logs">("tools");

const errorSummary = computed(() => {
  const message = detail.value?.status.error_message;
  if (!message) return "";
  return message.split("\n")[0] || message;
});

const filteredLogs = computed(() => {
  return store.logs.filter(
    (entry) => entry.level === "WARN" || entry.level === "ERROR",
  );
});

async function loadDetail() {
  loading.value = true;
  await store.fetchDetail(id.value);
  try {
    proxyUrl.value = await store.getProxyUrl(id.value);
  } catch {
    // Proxy URL may not be available yet
  }
  loading.value = false;
}

async function handleConnect() {
  await store.connectMcp(id.value);
  await loadDetail();
}

async function handleDisconnect() {
  await store.disconnectMcp(id.value);
  await loadDetail();
}

async function handleDelete() {
  if (confirm(`Delete "${detail.value?.config.name}"? This cannot be undone.`)) {
    await store.removeMcp(id.value);
    router.push("/");
  }
}

async function copyProxyUrl() {
  try {
    await navigator.clipboard.writeText(proxyUrl.value);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    // fallback
  }
}

function formatUptime(seconds?: number): string {
  if (!seconds) return "—";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${seconds}s`;
}

function formatTime(iso?: string): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString();
}

onMounted(loadDetail);
</script>

<template>
  <div class="p-6 max-w-4xl mx-auto">
    <!-- Back button -->
    <button
      @click="router.push('/')"
      class="inline-flex items-center gap-1.5 text-sm text-surface-500 hover:text-surface-800 mb-4 transition-colors"
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
          d="M15 19l-7-7 7-7"
        />
      </svg>
      Back to Dashboard
    </button>

    <!-- Loading -->
    <div v-if="loading" class="text-center py-16">
      <div
        class="w-8 h-8 border-2 border-surface-300 border-t-surface-600 rounded-full animate-spin mx-auto"
      ></div>
    </div>

    <template v-else-if="detail">
      <!-- Header -->
      <div class="flex items-start justify-between mb-6">
        <div>
          <h1 class="text-2xl font-bold text-surface-900">
            {{ detail.config.name }}
          </h1>
          <p class="text-sm text-surface-500 mt-0.5">
            {{ TRANSPORT_LABELS[detail.config.transport_type] }}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <button
            @click="router.push(`/add?edit=${id}`)"
            class="px-3 py-1.5 text-sm font-medium text-surface-600 bg-white border border-surface-300 rounded-lg hover:bg-surface-50 transition-colors"
          >
            Edit
          </button>
          <button
            @click="handleDelete"
            class="px-3 py-1.5 text-sm font-medium text-red-600 bg-white border border-red-200 rounded-lg hover:bg-red-50 transition-colors"
          >
            Delete
          </button>
        </div>
      </div>

      <!-- Status & Config cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        <!-- Status card -->
        <div class="bg-white rounded-lg border border-surface-200 p-5">
          <h2
            class="text-xs font-semibold text-surface-400 uppercase tracking-wider mb-4"
          >
            Status
          </h2>
          <div class="flex items-center gap-3 mb-4">
            <StatusBadge :state="detail.status.state" />
            <span v-if="errorSummary" class="text-xs text-red-600">
              {{ errorSummary }}
            </span>
          </div>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-surface-500">Connected at</span>
              <span class="font-medium">{{
                formatTime(detail.status.connected_at)
              }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Last ping</span>
              <span class="font-medium">{{
                formatTime(detail.status.last_ping)
              }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Uptime</span>
              <span class="font-medium">{{
                formatUptime(detail.status.uptime_seconds)
              }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Tools / Resources</span>
              <span class="font-medium"
                >{{ detail.status.tools_count }} /
                {{ detail.status.resources_count }}</span
              >
            </div>
          </div>
          <div class="mt-4 pt-4 border-t border-surface-100 flex gap-2">
            <button
              v-if="detail.status.state !== ConnectionState.Connected"
              @click="handleConnect"
              class="flex-1 px-3 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg hover:bg-emerald-700 transition-colors"
            >
              Connect
            </button>
            <button
              v-else
              @click="handleDisconnect"
              class="flex-1 px-3 py-2 bg-surface-200 text-surface-700 text-sm font-medium rounded-lg hover:bg-surface-300 transition-colors"
            >
              Disconnect
            </button>
            <button
              @click="loadDetail"
              class="px-3 py-2 bg-surface-100 text-surface-600 text-sm rounded-lg hover:bg-surface-200 transition-colors"
              title="Refresh"
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
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                />
              </svg>
            </button>
          </div>
        </div>

        <!-- Config card -->
        <div class="bg-white rounded-lg border border-surface-200 p-5">
          <h2
            class="text-xs font-semibold text-surface-400 uppercase tracking-wider mb-4"
          >
            Configuration
          </h2>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-surface-500">ID</span>
              <span class="font-mono text-xs text-surface-600">{{
                detail.config.id
              }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Transport</span>
              <span class="font-medium">{{
                TRANSPORT_LABELS[detail.config.transport_type]
              }}</span>
            </div>
            <div v-if="detail.config.command" class="flex justify-between">
              <span class="text-surface-500">Command</span>
              <span class="font-mono text-xs text-surface-600 text-right max-w-[200px] truncate">{{
                detail.config.command
              }}</span>
            </div>
            <div v-if="detail.config.args?.length" class="flex justify-between">
              <span class="text-surface-500">Args</span>
              <span class="font-mono text-xs text-surface-600">{{
                detail.config.args.join(" ")
              }}</span>
            </div>
            <div v-if="detail.config.url" class="flex justify-between">
              <span class="text-surface-500">URL</span>
              <span class="font-mono text-xs text-surface-600 text-right max-w-[200px] truncate">{{
                detail.config.url
              }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Enabled</span>
              <span class="font-medium">{{
                detail.config.enabled ? "Yes" : "No"
              }}</span>
            </div>
          </div>

          <!-- Proxy URL -->
          <div v-if="proxyUrl" class="mt-4 pt-4 border-t border-surface-100">
            <div
              class="text-xs font-semibold text-surface-400 uppercase tracking-wider mb-2"
            >
              Proxy Endpoint
            </div>
            <div class="flex gap-2">
              <input
                :value="proxyUrl"
                readonly
                class="flex-1 px-3 py-2 bg-surface-50 border border-surface-200 rounded-lg text-xs font-mono text-surface-700"
              />
              <button
                @click="copyProxyUrl"
                class="px-3 py-2 text-sm font-medium rounded-lg transition-colors"
                :class="
                  copied
                    ? 'bg-emerald-100 text-emerald-700'
                    : 'bg-surface-100 text-surface-600 hover:bg-surface-200'
                "
              >
                {{ copied ? "Copied!" : "Copy" }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Error details -->
      <div
        v-if="detail.status.error_message"
        class="mb-6 bg-red-50 border border-red-200 rounded-lg p-4"
      >
        <div class="text-xs font-semibold text-red-700 uppercase tracking-wider mb-2">
          Error details
        </div>
        <pre class="text-xs text-red-800 whitespace-pre-wrap break-words">{{
          detail.status.error_message
        }}</pre>
      </div>

      <!-- Tabs: Tools / Resources -->
      <div class="bg-white rounded-lg border border-surface-200">
        <div class="flex border-b border-surface-200">
          <button
            class="px-5 py-3 text-sm font-medium border-b-2 transition-colors"
            :class="
              activeTab === 'tools'
                ? 'border-surface-900 text-surface-900'
                : 'border-transparent text-surface-500 hover:text-surface-700'
            "
            @click="activeTab = 'tools'"
          >
            Tools ({{ detail.tools.length }})
          </button>
          <button
            class="px-5 py-3 text-sm font-medium border-b-2 transition-colors"
            :class="
              activeTab === 'resources'
                ? 'border-surface-900 text-surface-900'
                : 'border-transparent text-surface-500 hover:text-surface-700'
            "
            @click="activeTab = 'resources'"
          >
            Resources ({{ detail.resources.length }})
          </button>
          <button
            class="px-5 py-3 text-sm font-medium border-b-2 transition-colors"
            :class="
              activeTab === 'logs'
                ? 'border-surface-900 text-surface-900'
                : 'border-transparent text-surface-500 hover:text-surface-700'
            "
            @click="activeTab = 'logs'"
          >
            Logs ({{ filteredLogs.length }})
          </button>
        </div>
        <div class="p-5">
          <ToolList v-if="activeTab === 'tools'" :tools="detail.tools" />
          <ResourceList
            v-if="activeTab === 'resources'"
            :resources="detail.resources"
          />
          <div v-if="activeTab === 'logs'" class="space-y-2">
            <div
              v-if="filteredLogs.length === 0"
              class="text-sm text-surface-500"
            >
              No warnings or errors yet.
            </div>
            <div
              v-else
              class="max-h-[360px] overflow-auto rounded-lg border border-surface-200"
            >
              <div
                v-for="(entry, index) in filteredLogs"
                :key="`${entry.timestamp}-${index}`"
                class="border-b border-surface-200 last:border-b-0 p-3"
              >
                <div class="flex items-center justify-between mb-1">
                  <span
                    class="text-[10px] font-semibold uppercase tracking-wider"
                    :class="
                      entry.level === 'ERROR'
                        ? 'text-red-700'
                        : 'text-amber-700'
                    "
                  >
                    {{ entry.level }}
                  </span>
                  <span class="text-[10px] text-surface-400">
                    {{ new Date(entry.timestamp).toLocaleString() }}
                  </span>
                </div>
                <div class="text-xs text-surface-700 whitespace-pre-wrap break-words">
                  {{ entry.message }}
                </div>
                <div class="text-[10px] text-surface-400 mt-1">
                  {{ entry.target }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Not found -->
    <div v-else class="text-center py-16 text-surface-500">
      MCP not found.
    </div>
  </div>
</template>
