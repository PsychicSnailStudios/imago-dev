<script lang="ts">

	// COMPONENTS
	import { ArrowDownUp, ChevronUp, ChevronDown, ArrowUpDown, ArrowUp, ArrowDown } from "lucide-svelte"

	import * as DropdownMenu from "$shadcn/dropdown-menu"
	import { Button } from "$shadcn/button"
	
	// SCRIPTS
	import type { SortState, SortField } from "$ts/util/sortConfig.svelte"

	// PROPS
	let { sort } = $props<{ sort: SortState }>()

	// VARIABLES
	const fields: { value: SortField; label: string }[] = [
		{ value: "title", label: "Title" },
		{ value: "artist", label: "Artist" },
		{ value: "album", label: "Album" },
		{ value: "year", label: "Year" },
		{ value: "rating", label: "Rating" },
		{ value: "duration", label: "Duration" },
		{ value: "label", label: "Label" },
		{ value: "number", label: "Track Number" },
	]
	
	// FUNCTIONS
	function toggleDirection() {
		if (!sort.field) return
		sort.set(sort.field, sort.direction === "asc" ? "desc" : "asc")
	}

	function setField(f: SortField) {
		if (sort.field === f) {
			sort.set(f, sort.direction === "asc" ? "desc" : "asc")
		} else {
			sort.set(f, "asc")
		}
	}
</script>

<div class="flex items-center">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			<Button variant="ghost" size="sm" class="gap-2">
				<!-- <ArrowDownUp size={14} /> -->
				{#if sort.field === "custom"}
					<ArrowUpDown class="size-3" />
				{:else if sort.direction === "asc"}
					<ArrowUp class="size-3" />
				{:else}
					<ArrowDown class="size-3" />
				{/if}
				{sort.field ? fields.find(f => f.value === sort.field)?.label : "Custom"}
			</Button>
		</DropdownMenu.Trigger>
		<DropdownMenu.Content>
			{#each fields as f}
				<DropdownMenu.Item
					class={sort.active(f.value) ? "bg-accent" : ""}
					onclick={() => setField(f.value)}
				>
					{#if sort.active(f.value)}
						{#if sort.field === "custom"}
							<ArrowUpDown class="size-3 text-primary" />
						{:else if sort.direction === "asc"}
							<ArrowUp class="size-3 text-primary" />
						{:else}
							<ArrowDown class="size-3 text-primary" />
						{/if}
					{/if}
					{f.label}
				</DropdownMenu.Item>
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>