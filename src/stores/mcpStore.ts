import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  McpStatus,
  McpDetail,
  McpServerConfig,
  AppConfig,
} from "@/types";
import { ConnectionState } from "@/types";

export const useMcpStore = defineStore("mcp", () => {
  // State
  const statuses = ref<McpStatus[]>([]);
  const details = ref<Map<string, McpDetail>>(new Map());
  const appConfig = ref<AppConfig | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);

  // Computed
  const totalCount = computed(() => statuses.value.length);

  const connectedCount = computed(
    () =>
      statuses.value.filter((s) => s.state === ConnectionState.Connected).length
  );

  const errorCount = computed(
    () =>
      statuses.value.filter((s) => s.state === ConnectionState.Error).length
  );

  const reconnectingCount = computed(
    () =>
      statuses.value.filter((s) => s.state === ConnectionState.Reconnecting)
        .length
  );

  const disconnectedCount = computed(
    () =>
      statuses.value.filter((s) => s.state === ConnectionState.Disconnected)
        .length
  );

  const getMcpById = computed(() => {
    return (id: string) => statuses.value.find((s) => s.id === id);
  });

  // Actions
  async function fetchStatuses() {
    loading.value = true;
    error.value = null;
    try {
      statuses.value = await invoke<McpStatus[]>("list_mcps");
    } catch (e) {
      error.value = `Failed to fetch statuses: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  async function fetchDetail(id: string): Promise<McpDetail | null> {
    try {
      const detail = await invoke<McpDetail>("get_mcp_detail", { id });
      details.value.set(id, detail);
      return detail;
    } catch (e) {
      error.value = `Failed to fetch detail: ${e}`;
      return null;
    }
  }

  async function addMcp(config: McpServerConfig): Promise<string> {
    const id = await invoke<string>("add_mcp", { config });
    await fetchStatuses();
    return id;
  }

  async function updateMcp(config: McpServerConfig) {
    await invoke("update_mcp", { config });
    details.value.delete(config.id);
    await fetchStatuses();
  }

  async function removeMcp(id: string) {
    await invoke("remove_mcp", { id });
    details.value.delete(id);
    await fetchStatuses();
  }

  async function connectMcp(id: string) {
    await invoke("connect_mcp", { id });
    await fetchStatuses();
  }

  async function disconnectMcp(id: string) {
    await invoke("disconnect_mcp", { id });
    await fetchStatuses();
  }

  async function getProxyUrl(id: string): Promise<string> {
    return await invoke<string>("get_proxy_url", { id });
  }

  async function fetchAppConfig() {
    try {
      appConfig.value = await invoke<AppConfig>("get_app_config");
    } catch (e) {
      error.value = `Failed to fetch config: ${e}`;
    }
  }

  async function updateAppConfig(config: AppConfig) {
    await invoke("update_app_config", { config });
    appConfig.value = config;
  }

  // Initialize: fetch data + subscribe to Tauri events
  async function init() {
    if (initialized.value) return;
    initialized.value = true;

    await fetchStatuses();
    await fetchAppConfig();

    // Listen for real-time status updates from the Rust backend
    listen<McpStatus[]>("mcp-statuses-changed", (event) => {
      statuses.value = event.payload;
    });

    // Also poll every 10s as a fallback
    setInterval(() => {
      fetchStatuses();
    }, 10000);
  }

  return {
    // State
    statuses,
    details,
    appConfig,
    loading,
    error,
    // Computed
    totalCount,
    connectedCount,
    errorCount,
    reconnectingCount,
    disconnectedCount,
    getMcpById,
    // Actions
    init,
    fetchStatuses,
    fetchDetail,
    addMcp,
    updateMcp,
    removeMcp,
    connectMcp,
    disconnectMcp,
    getProxyUrl,
    fetchAppConfig,
    updateAppConfig,
  };
});
