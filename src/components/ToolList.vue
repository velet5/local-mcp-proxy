<script setup lang="ts">
import { ref } from "vue";
import type { Tool } from "@/types";

const props = defineProps<{
  tools: Tool[];
  editing?: boolean;
  selectedTools?: Set<string>;
  disabledTools?: string[];
}>();

const emit = defineEmits<{
  toggle: [toolName: string];
}>();

const expanded = ref<Set<string>>(new Set());

function toggleExpand(name: string) {
  if (expanded.value.has(name)) {
    expanded.value.delete(name);
  } else {
    expanded.value.add(name);
  }
}

function isDisabled(name: string): boolean {
  return props.disabledTools?.includes(name) ?? false;
}
</script>

<template>
  <div class="space-y-2">
    <div
      v-if="tools.length === 0"
      class="text-center py-8 text-surface-400 text-sm"
    >
      No tools available
    </div>

    <div
      v-for="tool in tools"
      :key="tool.name"
      class="border border-surface-200 rounded-lg overflow-hidden"
      :class="{ 'opacity-50': !editing && isDisabled(tool.name) }"
    >
      <div class="w-full flex items-center gap-3 px-4 py-3 text-left">
        <!-- Checkbox in edit mode -->
        <label
          v-if="editing"
          class="flex items-center shrink-0 cursor-pointer"
          @click.stop
        >
          <input
            type="checkbox"
            :checked="selectedTools?.has(tool.name)"
            @change="emit('toggle', tool.name)"
            class="w-4 h-4 rounded border-surface-300 text-surface-900 focus:ring-surface-500 cursor-pointer"
          />
        </label>

        <button
          class="flex items-center gap-3 flex-1 min-w-0"
          @click="toggleExpand(tool.name)"
        >
          <div
            class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
            :class="
              !editing && isDisabled(tool.name)
                ? 'bg-surface-100 text-surface-400'
                : 'bg-blue-50 text-blue-600'
            "
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
                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
              />
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
              />
            </svg>
          </div>
          <div class="min-w-0 flex-1 text-left">
            <div
              class="text-sm font-semibold"
              :class="
                !editing && isDisabled(tool.name)
                  ? 'text-surface-500 line-through'
                  : 'text-surface-900'
              "
            >
              {{ tool.name }}
            </div>
            <div
              v-if="tool.description"
              class="text-xs text-surface-500 truncate"
            >
              {{ tool.description }}
            </div>
          </div>
          <svg
            class="w-4 h-4 text-surface-400 transition-transform shrink-0"
            :class="{ 'rotate-180': expanded.has(tool.name) }"
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
      </div>

      <div v-if="expanded.has(tool.name)" class="border-t border-surface-100">
        <div class="p-4">
          <div
            v-if="tool.description"
            class="text-sm text-surface-600 mb-3"
          >
            {{ tool.description }}
          </div>
          <div class="text-xs text-surface-400 font-medium mb-1.5">
            Input Schema
          </div>
          <pre
            class="bg-surface-800 text-surface-100 rounded-lg p-3 text-xs overflow-x-auto leading-relaxed"
            >{{ JSON.stringify(tool.input_schema, null, 2) }}</pre
          >
        </div>
      </div>
    </div>
  </div>
</template>
