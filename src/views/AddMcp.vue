<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useMcpStore } from "@/stores/mcpStore";
import { TransportType, TRANSPORT_LABELS } from "@/types";
import type { McpServerConfig } from "@/types";
import KeyValueEditor from "@/components/KeyValueEditor.vue";

const route = useRoute();
const router = useRouter();
const store = useMcpStore();

const editId = computed(() => (route.query.edit as string) || null);
const isEditing = computed(() => !!editId.value);

const form = ref<McpServerConfig>({
  id: "",
  name: "",
  transport_type: TransportType.Stdio,
  command: "",
  args: [],
  url: "",
  env: {},
  headers: {},
  enabled: true,
});

const argsInput = ref("");
const envMap = ref<Record<string, string>>({});
const headersMap = ref<Record<string, string>>({});

// Auth fields for HTTP transports
const authType = ref<"none" | "basic">("none");
const authUser = ref("");
const authPass = ref("");

const submitting = ref(false);
const formError = ref("");

// Generate a short random ID
function generateId(): string {
  return crypto.randomUUID().slice(0, 8);
}

// Check if transport is HTTP-based
const isHttpTransport = computed(
  () =>
    form.value.transport_type === TransportType.Sse ||
    form.value.transport_type === TransportType.StreamableHttp
);

// Sync auth fields from headers when loading
function syncAuthFromHeaders(headers: Record<string, string>) {
  const authHeader = Object.entries(headers).find(
    ([k]) => k.toLowerCase() === "authorization"
  );
  if (authHeader) {
    const val = authHeader[1];
    if (val.toLowerCase().startsWith("basic ")) {
      authType.value = "basic";
      try {
        const decoded = atob(val.slice(6));
        const colonIdx = decoded.indexOf(":");
        if (colonIdx >= 0) {
          authUser.value = decoded.slice(0, colonIdx);
          authPass.value = decoded.slice(colonIdx + 1);
        }
      } catch {
        // If decoding fails, keep raw
        authUser.value = "";
        authPass.value = "";
      }
    } else {
      authType.value = "none";
    }
    // Remove Authorization from headersMap so it's not duplicated
    const cleaned: Record<string, string> = {};
    for (const [k, v] of Object.entries(headers)) {
      if (k.toLowerCase() !== "authorization") cleaned[k] = v;
    }
    headersMap.value = cleaned;
  }
}

// Validate form before submission
function validate(): string | null {
  if (!form.value.name.trim()) return "Name is required.";

  if (form.value.transport_type === TransportType.Stdio) {
    if (!form.value.command?.trim())
      return "Command is required for Stdio transport.";
  } else {
    if (!form.value.url?.trim())
      return "URL is required for this transport type.";
    try {
      new URL(form.value.url!);
    } catch {
      return "URL is not valid.";
    }
  }

  // Check for duplicate keys in env
  const envKeys = Object.keys(envMap.value);
  if (new Set(envKeys).size !== envKeys.length)
    return "Duplicate environment variable keys.";

  // Check for duplicate keys in headers
  const headerKeys = Object.keys(headersMap.value);
  if (new Set(headerKeys).size !== headerKeys.length)
    return "Duplicate header keys.";

  // Validate basic auth fields
  if (authType.value === "basic" && !authUser.value.trim())
    return "Username is required for Basic auth.";

  return null;
}

async function handleSubmit() {
  formError.value = "";
  const err = validate();
  if (err) {
    formError.value = err;
    return;
  }

  submitting.value = true;
  try {
    // Parse args
    if (argsInput.value.trim()) {
      form.value.args = argsInput.value
        .split(/\s+/)
        .filter((a) => a.length > 0);
    } else {
      form.value.args = [];
    }

    // Set env from key-value map
    form.value.env = { ...envMap.value };

    // Build headers from key-value map + auth
    const finalHeaders: Record<string, string> = { ...headersMap.value };
    if (isHttpTransport.value && authType.value === "basic") {
      finalHeaders["Authorization"] =
        "Basic " + btoa(authUser.value + ":" + authPass.value);
    }
    form.value.headers = finalHeaders;

    // Auto-generate ID for new MCPs
    if (!isEditing.value && !form.value.id) {
      form.value.id = generateId();
    }

    if (isEditing.value) {
      await store.updateMcp(form.value);
    } else {
      await store.addMcp(form.value);
    }

    router.push("/");
  } catch (e) {
    formError.value = `${e}`;
  } finally {
    submitting.value = false;
  }
}

// Reset auth when switching away from HTTP
watch(
  () => form.value.transport_type,
  (newType) => {
    if (
      newType !== TransportType.Sse &&
      newType !== TransportType.StreamableHttp
    ) {
      authType.value = "none";
      authUser.value = "";
      authPass.value = "";
    }
  }
);

// Load existing MCP data when editing
onMounted(async () => {
  if (editId.value) {
    const detail = await store.fetchDetail(editId.value);
    if (detail) {
      form.value = { ...detail.config };
      argsInput.value = (form.value.args || []).join(" ");
      envMap.value = { ...(form.value.env || {}) };

      const hdrs = { ...(form.value.headers || {}) };
      syncAuthFromHeaders(hdrs);
      // If no auth was detected, just use all headers
      if (authType.value === "none") {
        headersMap.value = hdrs;
      }
    }
  }
});
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto">
    <!-- Back -->
    <button @click="router.push('/')"
      class="inline-flex items-center gap-1.5 text-sm text-surface-500 hover:text-surface-800 mb-4 transition-colors">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
      </svg>
      Back to Dashboard
    </button>

    <h1 class="text-2xl font-bold text-surface-900 mb-6">
      {{ isEditing ? "Edit MCP" : "Add New MCP" }}
    </h1>

    <form @submit.prevent="handleSubmit"
      class="bg-white rounded-lg border border-surface-200 divide-y divide-surface-100">
      <!-- Name -->
      <div class="p-5">
        <label class="block text-sm font-medium text-surface-700 mb-1.5">Name *</label>
        <input v-model="form.name" type="text" placeholder="My MCP Server"
          class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent" />
      </div>

      <!-- Transport type -->
      <div class="p-5">
        <label class="block text-sm font-medium text-surface-700 mb-1.5">Transport Type *</label>
        <div class="grid grid-cols-3 gap-2">
          <button v-for="(label, type) in TRANSPORT_LABELS" :key="type" type="button"
            @click="form.transport_type = type as TransportType"
            class="px-3 py-2.5 rounded-lg text-sm font-medium border transition-colors text-center" :class="form.transport_type === type
              ? 'bg-surface-900 text-white border-surface-900'
              : 'bg-white text-surface-600 border-surface-300 hover:bg-surface-50'
              ">
            {{ label }}
          </button>
        </div>
      </div>

      <!-- Stdio fields -->
      <div v-if="form.transport_type === TransportType.Stdio" class="p-5 space-y-4">
        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">Command *</label>
          <input v-model="form.command" type="text" placeholder="npx -y @modelcontextprotocol/server-everything"
            class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent" />
          <p class="text-xs text-surface-400 mt-1">
            The executable to run as an MCP server process.
          </p>
        </div>

        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">Arguments</label>
          <input v-model="argsInput" type="text" placeholder="--port 3000 --verbose"
            class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent" />
          <p class="text-xs text-surface-400 mt-1">
            Space-separated command arguments.
          </p>
        </div>

        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">Environment Variables</label>
          <KeyValueEditor
            v-model="envMap"
            key-placeholder="VARIABLE"
            value-placeholder="value"
            :default-hidden-keys="['key', 'secret', 'token', 'password', 'auth']"
          />
        </div>
      </div>

      <!-- HTTP/SSE fields -->
      <div v-if="isHttpTransport" class="p-5 space-y-4">
        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">URL *</label>
          <input v-model="form.url" type="url" placeholder="http://localhost:3000/mcp"
            class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent" />
        </div>

        <!-- Auth section -->
        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">Authentication</label>
          <div class="flex gap-2 mb-3">
            <button
              type="button"
              @click="authType = 'none'"
              class="px-3 py-1.5 rounded-md text-xs font-medium border transition-colors"
              :class="authType === 'none'
                ? 'bg-surface-900 text-white border-surface-900'
                : 'bg-white text-surface-600 border-surface-300 hover:bg-surface-50'"
            >
              None
            </button>
            <button
              type="button"
              @click="authType = 'basic'"
              class="px-3 py-1.5 rounded-md text-xs font-medium border transition-colors"
              :class="authType === 'basic'
                ? 'bg-surface-900 text-white border-surface-900'
                : 'bg-white text-surface-600 border-surface-300 hover:bg-surface-50'"
            >
              Basic
            </button>
          </div>

          <!-- Basic auth fields -->
          <div v-if="authType === 'basic'" class="space-y-2 mb-3">
            <input
              v-model="authUser"
              type="text"
              placeholder="Username"
              class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent"
            />
            <input
              v-model="authPass"
              type="password"
              placeholder="Password"
              class="w-full px-3 py-2 border border-surface-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent"
            />
          </div>
        </div>

        <div>
          <label class="block text-sm font-medium text-surface-700 mb-1.5">Headers</label>
          <KeyValueEditor
            v-model="headersMap"
            key-placeholder="Header"
            value-placeholder="value"
            :default-hidden-keys="['authorization', 'token', 'secret', 'key', 'password', 'auth']"
          />
        </div>
      </div>

      <!-- Enabled toggle -->
      <div class="p-5">
        <label class="flex items-center gap-3 cursor-pointer">
          <input v-model="form.enabled" type="checkbox"
            class="w-4 h-4 rounded border-surface-300 text-surface-900 focus:ring-surface-900" />
          <div>
            <span class="text-sm font-medium text-surface-700">Enabled</span>
            <p class="text-xs text-surface-400">
              Connect automatically on app start.
            </p>
          </div>
        </label>
      </div>

      <!-- Error -->
      <div v-if="formError" class="mx-5 my-0 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-700">
        {{ formError }}
      </div>

      <!-- Submit -->
      <div class="p-5 flex gap-3">
        <button type="submit" :disabled="submitting"
          class="flex-1 px-4 py-2.5 bg-surface-900 text-white rounded-lg text-sm font-medium hover:bg-surface-800 transition-colors disabled:opacity-50">
          {{ submitting ? "Saving..." : isEditing ? "Update MCP" : "Add MCP" }}
        </button>
        <button type="button" @click="router.push('/')"
          class="px-4 py-2.5 bg-surface-100 text-surface-700 rounded-lg text-sm font-medium hover:bg-surface-200 transition-colors">
          Cancel
        </button>
      </div>
    </form>
  </div>
</template>
