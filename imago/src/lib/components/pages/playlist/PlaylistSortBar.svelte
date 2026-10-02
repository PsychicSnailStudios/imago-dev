<script lang="ts">
	import { ArrowUpDown, ArrowUp, ArrowDown } from "lucide-svelte";
	import { Button } from "$shadcn/button/index.js";
	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";

	export type PlaylistSortField = "title" | "date_created" | "custom";

	let {
		field = $bindable<PlaylistSortField>("custom"),
		direction = $bindable<"asc" | "desc">("asc"),
	} = $props<{
		field: PlaylistSortField;
		direction: "asc" | "desc";
	}>();

	const labels: Record<PlaylistSortField, string> = {
		title: "Name",
		date_created: "Date Created",
		custom: "Custom",
	};

	function setField(f: PlaylistSortField) {
		if (field === f) {
			if (f !== "custom") direction = direction === "asc" ? "desc" : "asc";
		} else {
			field = f;
			direction = "asc";
		}
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props }: { props: Record<string, unknown> })}
			<Button {...props} variant="ghost" size="sm" class="h-7 gap-1 text-xs text-muted-foreground">
				{#if field === "custom"}
					<ArrowUpDown class="size-3" />
				{:else if direction === "asc"}
					<ArrowUp class="size-3" />
				{:else}
					<ArrowDown class="size-3" />
				{/if}
				{labels[field]}
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end" class="w-40">
		{#each (["custom", "title", "date_created"] as PlaylistSortField[]) as f}
			<DropdownMenu.Item
				class="flex items-center justify-between"
				onclick={() => setField(f)}
			>
				<span>{labels[f]}</span>
				{#if field === f}
					{#if f === "custom"}
						<ArrowUpDown class="size-3 text-primary" />
					{:else if direction === "asc"}
						<ArrowUp class="size-3 text-primary" />
					{:else}
						<ArrowDown class="size-3 text-primary" />
					{/if}
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
