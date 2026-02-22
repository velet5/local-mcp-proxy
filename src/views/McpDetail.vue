<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
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
const claudeDesktopAdded = ref(false);
const addingToClaudeDesktop = ref(false);
const claudeDesktopDropdownOpen = ref(false);
const updatingClaudeDesktop = ref(false);
const removingFromClaudeDesktop = ref(false);

// Manage mode state
const editingMode = ref(false);
const selectedTools = ref<Set<string>>(new Set());
const selectedResources = ref<Set<string>>(new Set());
const saving = ref(false);

const disabledTools = computed(() => detail.value?.config.disabled_tools ?? []);
const disabledResources = computed(() => detail.value?.config.disabled_resources ?? []);

const enabledToolsCount = computed(() => {
  if (!detail.value) return 0;
  return detail.value.tools.length - disabledTools.value.length;
});

const enabledResourcesCount = computed(() => {
  if (!detail.value) return 0;
  return detail.value.resources.length - disabledResources.value.length;
});

const sortedTools = computed(() => {
  if (!detail.value) return [];
  if (editingMode.value) return detail.value.tools;
  const disabled = disabledTools.value;
  return [...detail.value.tools].sort((a, b) => {
    const aDisabled = disabled.includes(a.name);
    const bDisabled = disabled.includes(b.name);
    if (aDisabled !== bDisabled) return aDisabled ? 1 : -1;
    return 0;
  });
});

const sortedResources = computed(() => {
  if (!detail.value) return [];
  if (editingMode.value) return detail.value.resources;
  const disabled = disabledResources.value;
  return [...detail.value.resources].sort((a, b) => {
    const aDisabled = disabled.includes(a.uri);
    const bDisabled = disabled.includes(b.uri);
    if (aDisabled !== bDisabled) return aDisabled ? 1 : -1;
    return 0;
  });
});

function enterEditMode() {
  if (!detail.value) return;
  const disabled = disabledTools.value;
  selectedTools.value = new Set(
    detail.value.tools
      .map((t) => t.name)
      .filter((name) => !disabled.includes(name)),
  );
  const disabledRes = disabledResources.value;
  selectedResources.value = new Set(
    detail.value.resources
      .map((r) => r.uri)
      .filter((uri) => !disabledRes.includes(uri)),
  );
  editingMode.value = true;
}

function cancelEditMode() {
  editingMode.value = false;
}

function selectAll() {
  if (!detail.value) return;
  if (activeTab.value === "tools") {
    selectedTools.value = new Set(detail.value.tools.map((t) => t.name));
  } else if (activeTab.value === "resources") {
    selectedResources.value = new Set(
      detail.value.resources.map((r) => r.uri),
    );
  }
}

function selectNone() {
  if (activeTab.value === "tools") {
    selectedTools.value = new Set();
  } else if (activeTab.value === "resources") {
    selectedResources.value = new Set();
  }
}

function toggleTool(name: string) {
  const s = new Set(selectedTools.value);
  if (s.has(name)) {
    s.delete(name);
  } else {
    s.add(name);
  }
  selectedTools.value = s;
}

function toggleResource(uri: string) {
  const s = new Set(selectedResources.value);
  if (s.has(uri)) {
    s.delete(uri);
  } else {
    s.add(uri);
  }
  selectedResources.value = s;
}

async function saveDisabledItems() {
  if (!detail.value) return;
  saving.value = true;
  try {
    const newDisabledTools = detail.value.tools
      .map((t) => t.name)
      .filter((name) => !selectedTools.value.has(name));
    const newDisabledResources = detail.value.resources
      .map((r) => r.uri)
      .filter((uri) => !selectedResources.value.has(uri));
    await store.setDisabledItems(id.value, newDisabledTools, newDisabledResources);
    editingMode.value = false;
  } catch (e) {
    alert(`Failed to save: ${e}`);
  } finally {
    saving.value = false;
  }
}

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
  try {
    claudeDesktopAdded.value = await invoke<boolean>("check_claude_desktop", {
      mcpId: id.value,
    });
  } catch {
    // Claude Desktop config may not exist
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

async function handleAddToClaudeDesktop() {
  addingToClaudeDesktop.value = true;
  try {
    await invoke("add_to_claude_desktop", { mcpId: id.value });
    claudeDesktopAdded.value = true;
  } catch (e) {
    alert(`Failed to add to Claude Desktop: ${e}`);
  } finally {
    addingToClaudeDesktop.value = false;
  }
}

async function handleUpdateInClaudeDesktop() {
  updatingClaudeDesktop.value = true;
  claudeDesktopDropdownOpen.value = false;
  try {
    await invoke("update_in_claude_desktop", { mcpId: id.value });
  } catch (e) {
    alert(`Failed to update in Claude Desktop: ${e}`);
  } finally {
    updatingClaudeDesktop.value = false;
  }
}

async function handleRemoveFromClaudeDesktop() {
  claudeDesktopDropdownOpen.value = false;
  removingFromClaudeDesktop.value = true;
  try {
    await invoke("remove_from_claude_desktop", { mcpId: id.value });
    claudeDesktopAdded.value = false;
  } catch (e) {
    alert(`Failed to remove from Claude Desktop: ${e}`);
  } finally {
    removingFromClaudeDesktop.value = false;
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
  if (!seconds) return "\u2014";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${seconds}s`;
}

function formatTime(iso?: string): string {
  if (!iso) return "\u2014";
  return new Date(iso).toLocaleString();
}

function handleClickOutside(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (!target.closest(".claude-desktop-dropdown")) {
    claudeDesktopDropdownOpen.value = false;
  }
}

onMounted(() => {
  loadDetail();
  document.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", handleClickOutside);
});
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
              <span class="text-surface-500">Tools</span>
              <span class="font-medium">
                <template v-if="disabledTools.length > 0">
                  {{ enabledToolsCount }} / {{ detail.tools.length }}
                </template>
                <template v-else>
                  {{ detail.tools.length }}
                </template>
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-surface-500">Resources</span>
              <span class="font-medium">
                <template v-if="disabledResources.length > 0">
                  {{ enabledResourcesCount }} / {{ detail.resources.length }}
                </template>
                <template v-else>
                  {{ detail.resources.length }}
                </template>
              </span>
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

          <!-- Claude Desktop -->
          <div class="mt-4 pt-4 border-t border-surface-100">
            <!-- Add button (not yet added) -->
            <button
              v-if="!claudeDesktopAdded"
              @click="handleAddToClaudeDesktop"
              :disabled="addingToClaudeDesktop"
              class="w-full px-3 py-2 text-sm font-medium rounded-lg transition-colors flex items-center justify-center gap-2 bg-surface-900 text-white hover:bg-surface-800 disabled:opacity-50"
            >
              {{ addingToClaudeDesktop ? "Adding..." : "Add to Claude Desktop" }}
            </button>

            <!-- Added state with dropdown -->
            <div v-else class="relative claude-desktop-dropdown">
              <button
                @click="claudeDesktopDropdownOpen = !claudeDesktopDropdownOpen"
                class="w-full px-3 py-2 text-sm font-medium rounded-lg transition-colors flex items-center justify-center gap-2 bg-emerald-50 text-emerald-700 border border-emerald-200 hover:bg-emerald-100"
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
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                Added to Claude Desktop
                <svg
                  class="w-3.5 h-3.5 ml-auto transition-transform"
                  :class="claudeDesktopDropdownOpen ? 'rotate-180' : ''"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 9l-7 7-7-7"
                  />
                </svg>
              </button>
              <div
                v-if="claudeDesktopDropdownOpen"
                class="absolute left-0 right-0 mt-1 bg-white border border-surface-200 rounded-lg shadow-lg z-10 overflow-hidden"
              >
                <button
                  @click="handleUpdateInClaudeDesktop"
                  :disabled="updatingClaudeDesktop"
                  class="w-full px-3 py-2 text-sm text-left text-surface-700 hover:bg-surface-50 transition-colors disabled:opacity-50"
                >
                  {{ updatingClaudeDesktop ? "Updating..." : "Update" }}
                </button>
                <button
                  @click="handleRemoveFromClaudeDesktop"
                  :disabled="removingFromClaudeDesktop"
                  class="w-full px-3 py-2 text-sm text-left text-red-600 hover:bg-red-50 transition-colors disabled:opacity-50"
                >
                  {{ removingFromClaudeDesktop ? "Removing..." : "Remove" }}
                </button>
              </div>
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
        <div class="flex items-center border-b border-surface-200">
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

          <!-- Manage button (right-aligned) -->
          <div class="ml-auto pr-3" v-if="activeTab !== 'logs'">
            <button
              v-if="!editingMode"
              @click="enterEditMode"
              class="px-3 py-1.5 text-xs font-medium text-surface-600 bg-surface-100 rounded-lg hover:bg-surface-200 transition-colors"
            >
              Manage
            </button>
          </div>
        </div>

        <!-- Edit mode toolbar -->
        <div
          v-if="editingMode && activeTab !== 'logs'"
          class="flex items-center gap-2 px-5 py-3 bg-surface-50 border-b border-surface-200"
        >
          <button
            @click="selectAll"
            class="px-2.5 py-1 text-xs font-medium text-surface-600 bg-white border border-surface-300 rounded hover:bg-surface-50 transition-colors"
          >
            Select All
          </button>
          <button
            @click="selectNone"
            class="px-2.5 py-1 text-xs font-medium text-surface-600 bg-white border border-surface-300 rounded hover:bg-surface-50 transition-colors"
          >
            Select None
          </button>
          <div class="flex-1"></div>
          <button
            @click="cancelEditMode"
            class="px-3 py-1.5 text-xs font-medium text-surface-600 bg-white border border-surface-300 rounded-lg hover:bg-surface-50 transition-colors"
          >
            Cancel
          </button>
          <button
            @click="saveDisabledItems"
            :disabled="saving"
            class="px-3 py-1.5 text-xs font-medium text-white bg-surface-900 rounded-lg hover:bg-surface-800 transition-colors disabled:opacity-50"
          >
            {{ saving ? "Saving..." : "Save" }}
          </button>
        </div>

        <div class="p-5">
          <ToolList
            v-if="activeTab === 'tools'"
            :tools="sortedTools"
            :editing="editingMode"
            :selected-tools="selectedTools"
            :disabled-tools="disabledTools"
            @toggle="toggleTool"
          />
          <ResourceList
            v-if="activeTab === 'resources'"
            :resources="sortedResources"
            :editing="editingMode"
            :selected-resources="selectedResources"
            :disabled-resources="disabledResources"
            @toggle="toggleResource"
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
