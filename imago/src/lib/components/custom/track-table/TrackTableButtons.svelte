<script lang="ts">

	// COMPONENTS
	import { EllipsisIcon, List, TextAlignJustify, Columns3Cog } from "lucide-svelte";

	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import Button from "$shadcn/button/button.svelte";
	import { Toggle } from "$shadcn/toggle/index.js";

	// CUSTOM COMPONENTS
	import SortDropdown from "$lib/components/custom/track-table/SortDropdown.svelte";

	// SCRIPTS
	import { SortState } from "$ts/util/sortConfig.svelte";
	import { ALL_COLUMNS, COLUMN_LABELS } from "$ts/util/columnConfig.svelte"
	import type { ColumnState, ColumnKey } from "$ts/util/columnConfig.svelte"

	// PROPS
	let { sort, cols, compact, onCompactChange } = $props<{
			sort: SortState, cols: ColumnState, compact: boolean, onCompactChange: (v: boolean) => void
		}>();

</script>

<div class="flex gap-2 flex-wrap">
	<SortDropdown sort={sort} />

	<Button variant="ghost" size="icon" onclick={() => onCompactChange(!compact)} >
		{#if compact}
		<TextAlignJustify />
		<!-- Compact -->
		{:else}
		<List />
		<!-- List -->
		{/if}
	</Button>

	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button {...props} variant="ghost" size="icon"><Columns3Cog/></Button>
			{/snippet}
		</DropdownMenu.Trigger>
	
		<DropdownMenu.Content>
			<DropdownMenu.Group>
				<DropdownMenu.Label>Display Columns</DropdownMenu.Label>
				<div class="flex flex-col flex-wrap gap-1">
					{#each ALL_COLUMNS as col (col)}
						<Toggle
							pressed={cols.visible[col as ColumnKey]}
							onPressedChange={(v) => (cols.visible[col as ColumnKey] = v)}
							size="sm"
							variant="outline"
							class="text-sm justify-start"
						>
							{COLUMN_LABELS[col as ColumnKey]}
						</Toggle>
					{/each}
				</div>
			</DropdownMenu.Group>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
