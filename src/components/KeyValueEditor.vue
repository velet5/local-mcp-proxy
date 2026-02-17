<script setup lang="ts">
import { ref, watch } from "vue";

interface Entry {
  key: string;
  value: string;
  hidden: boolean;
}

const props = withDefaults(
  defineProps<{
    modelValue: Record<string, string>;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    /** Keys whose values are hidden by default */
    defaultHiddenKeys?: string[];
  }>(),
  {
    keyPlaceholder: "KEY",
    valuePlaceholder: "Value",
    defaultHiddenKeys: () => [],
  }
);

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, string>): void;
}>();

const entries = ref<Entry[]>([]);

// Determine if a key should be hidden by default
function shouldHide(key: string): boolean {
  const lower = key.toLowerCase();
  for (const pattern of props.defaultHiddenKeys) {
    if (lower.includes(pattern.toLowerCase())) return true;
  }
  return false;
}

// Sync from modelValue prop → entries (only on mount / external change)
let skipSync = false;
watch(
  () => props.modelValue,
  (val) => {
    if (skipSync) return;
    const keys = Object.keys(val || {});
    if (keys.length === 0) {
      entries.value = [];
    } else {
      entries.value = keys.map((k) => ({
        key: k,
        value: val[k],
        hidden: shouldHide(k),
      }));
    }
  },
  { immediate: true, deep: true }
);

function emitUpdate() {
  skipSync = true;
  const result: Record<string, string> = {};
  for (const entry of entries.value) {
    const k = entry.key.trim();
    if (k) result[k] = entry.value;
  }
  emit("update:modelValue", result);
  // Re-enable sync on next tick
  setTimeout(() => {
    skipSync = false;
  }, 0);
}

function addEntry() {
  entries.value.push({ key: "", value: "", hidden: false });
}

function removeEntry(index: number) {
  entries.value.splice(index, 1);
  emitUpdate();
}

function toggleHidden(index: number) {
  entries.value[index].hidden = !entries.value[index].hidden;
}

function updateKey(index: number, val: string) {
  entries.value[index].key = val;
  emitUpdate();
}

function updateValue(index: number, val: string) {
  entries.value[index].value = val;
  emitUpdate();
}

</script>

<template>
  <div class="space-y-2">
    <div
      v-for="(entry, i) in entries"
      :key="i"
      class="flex items-center gap-2"
    >
      <!-- Key input -->
      <input
        :value="entry.key"
        @input="updateKey(i, ($event.target as HTMLInputElement).value)"
        :placeholder="keyPlaceholder"
        class="w-[38%] px-2.5 py-1.5 border border-surface-300 rounded-md text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent bg-white"
      />

      <!-- Value input -->
      <div class="flex-1 relative">
        <input
          :value="entry.value"
          @input="updateValue(i, ($event.target as HTMLInputElement).value)"
          :type="entry.hidden ? 'password' : 'text'"
          :placeholder="valuePlaceholder"
          class="w-full px-2.5 py-1.5 pr-8 border border-surface-300 rounded-md text-sm font-mono focus:outline-none focus:ring-2 focus:ring-surface-900 focus:border-transparent bg-white"
        />
        <!-- Toggle visibility button -->
        <button
          type="button"
          @click="toggleHidden(i)"
          class="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 text-surface-400 hover:text-surface-600 transition-colors"
          :title="entry.hidden ? 'Show value' : 'Hide value'"
        >
          <!-- Eye icon (visible) -->
          <svg
            v-if="!entry.hidden"
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
            />
          </svg>
          <!-- Eye-off icon (hidden) -->
          <svg
            v-else
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"
            />
          </svg>
        </button>
      </div>

      <!-- Remove button -->
      <button
        type="button"
        @click="removeEntry(i)"
        class="p-1.5 text-surface-400 hover:text-red-500 transition-colors shrink-0"
        title="Remove"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    </div>

    <!-- Add button -->
    <button
      type="button"
      @click="addEntry"
      class="inline-flex items-center gap-1.5 text-xs text-surface-500 hover:text-surface-700 transition-colors pt-1"
    >
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 4v16m8-8H4"
        />
      </svg>
      Add {{ keyPlaceholder.toLowerCase() === "key" ? "entry" : keyPlaceholder.toLowerCase() }}
    </button>
  </div>
</template>
