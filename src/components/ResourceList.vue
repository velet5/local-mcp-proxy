<script setup lang="ts">
import type { Resource } from "@/types";

const props = defineProps<{
  resources: Resource[];
  editing?: boolean;
  selectedResources?: Set<string>;
  disabledResources?: string[];
}>();

const emit = defineEmits<{
  toggle: [resourceUri: string];
}>();

function isDisabled(uri: string): boolean {
  return props.disabledResources?.includes(uri) ?? false;
}
</script>

<template>
  <div class="space-y-2">
    <div
      v-if="resources.length === 0"
      class="text-center py-8 text-surface-400 text-sm"
    >
      No resources available
    </div>

    <div
      v-for="resource in resources"
      :key="resource.uri"
      class="border border-surface-200 rounded-lg p-4 transition-colors"
      :class="
        !editing && isDisabled(resource.uri)
          ? 'opacity-50'
          : 'hover:bg-surface-50'
      "
    >
      <div class="flex items-start gap-3">
        <!-- Checkbox in edit mode -->
        <label
          v-if="editing"
          class="flex items-center shrink-0 mt-1.5 cursor-pointer"
          @click.stop
        >
          <input
            type="checkbox"
            :checked="selectedResources?.has(resource.uri)"
            @change="emit('toggle', resource.uri)"
            class="w-4 h-4 rounded border-surface-300 text-surface-900 focus:ring-surface-500 cursor-pointer"
          />
        </label>

        <div
          class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 mt-0.5"
          :class="
            !editing && isDisabled(resource.uri)
              ? 'bg-surface-100 text-surface-400'
              : 'bg-violet-50 text-violet-600'
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
              d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"
            />
          </svg>
        </div>
        <div class="min-w-0 flex-1">
          <div
            v-if="resource.name"
            class="text-sm font-semibold"
            :class="
              !editing && isDisabled(resource.uri)
                ? 'text-surface-500 line-through'
                : 'text-surface-900'
            "
          >
            {{ resource.name }}
          </div>
          <div
            class="text-xs font-mono text-surface-500 break-all"
          >
            {{ resource.uri }}
          </div>
          <div
            v-if="resource.description"
            class="text-xs text-surface-500 mt-1"
          >
            {{ resource.description }}
          </div>
          <div v-if="resource.mime_type" class="mt-1.5">
            <span
              class="text-[10px] px-1.5 py-0.5 bg-surface-100 rounded text-surface-500 font-mono"
            >
              {{ resource.mime_type }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
