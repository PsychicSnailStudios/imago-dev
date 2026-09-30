<script lang="ts">
	import { Funnel } from "lucide-svelte";

	import * as Popover from "$shadcn/popover/index.js";
	import { Button, buttonVariants } from "$shadcn/button/index.js";
	import { Badge } from "$shadcn/badge/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Slider } from "$shadcn/slider/index.js";

	import { tagStore } from "$ts/store/tagManager.svelte";
	import { libraryStore } from "$ts/store/library.svelte";
	import { FILE_FORMATS, countActiveFilters, createEmptyFilters, type SearchFilters } from "$ts/util/searchFilters";

	let { filters = $bindable() } = $props<{ filters: SearchFilters }>();

	const count = $derived(countActiveFilters(filters));
	const genreNames = $derived(tagStore.genres.map((g) => g.name));
	const tagNames = $derived(tagStore.tags.map((t) => t.name));
	const libraries = $derived(libraryStore.libraries);

	function toggle(list: string[], value: string): string[] {
		return list.includes(value) ? list.filter((v) => v !== value) : [...list, value];
	}

	function toYear(value: string): number | null {
		const n = parseInt(value, 10);
		return Number.isNaN(n) ? null : n;
	}

	function reset() {
		filters = createEmptyFilters();
	}
</script>

<Popover.Root>
	<Popover.Trigger class="{buttonVariants({ variant: 'outline', size: 'icon' })} relative shrink-0">
		<Funnel />
		{#if count > 0}
			<Badge class="absolute -top-2 -right-2 h-5 min-w-5 justify-center rounded-full px-1 text-xs">{count}</Badge>
		{/if}
	</Popover.Trigger>
	<Popover.Content align="end" class="flex max-h-[70vh] w-80 flex-col gap-4 overflow-y-auto">
		<div class="flex items-center justify-between">
			<h3 class="text-sm font-medium">Filters</h3>
			<Button variant="ghost" size="sm" disabled={count === 0} onclick={reset}>Reset</Button>
		</div>

		<div class="flex flex-col gap-2">
			<p class="text-xs font-medium text-muted-foreground">Genres</p>
			{#if genreNames.length === 0}
				<p class="text-xs text-muted-foreground">No genres yet</p>
			{:else}
				<div class="flex max-h-24 flex-wrap gap-1 overflow-y-auto">
					{#each genreNames as name (name)}
						<Button
							type="button"
							size="sm"
							variant={filters.genres.includes(name) ? "default" : "outline"}
							class="h-6 px-2 text-xs"
							onclick={() => (filters.genres = toggle(filters.genres, name))}
						>
							{name}
						</Button>
					{/each}
				</div>
			{/if}
		</div>

		<div class="flex flex-col gap-2">
			<p class="text-xs font-medium text-muted-foreground">Tags</p>
			{#if tagNames.length === 0}
				<p class="text-xs text-muted-foreground">No tags yet</p>
			{:else}
				<div class="flex max-h-24 flex-wrap gap-1 overflow-y-auto">
					{#each tagNames as name (name)}
						<Button
							type="button"
							size="sm"
							variant={filters.tags.includes(name) ? "default" : "outline"}
							class="h-6 px-2 text-xs"
							onclick={() => (filters.tags = toggle(filters.tags, name))}
						>
							{name}
						</Button>
					{/each}
				</div>
			{/if}
		</div>

		<div class="flex flex-col gap-2">
			<p class="text-xs font-medium text-muted-foreground">Year</p>
			<div class="flex items-center gap-2">
				<Input
					type="number"
					placeholder="From"
					value={filters.yearMin ?? ""}
					oninput={(e) => (filters.yearMin = toYear(e.currentTarget.value))}
				/>
				<Input
					type="number"
					placeholder="To"
					value={filters.yearMax ?? ""}
					oninput={(e) => (filters.yearMax = toYear(e.currentTarget.value))}
				/>
			</div>
		</div>

		<div class="flex flex-col gap-2">
			<div class="flex items-center justify-between">
				<p class="text-xs font-medium text-muted-foreground">Minimum rating</p>
				<span class="text-xs text-muted-foreground">{filters.minRating > 0 ? filters.minRating.toFixed(1) : "Any"}</span>
			</div>
			<Slider type="single" bind:value={filters.minRating} min={0} max={10} step={0.5} />
		</div>

		<div class="flex flex-col gap-2">
			<p class="text-xs font-medium text-muted-foreground">Format</p>
			<div class="flex flex-wrap gap-1">
				{#each FILE_FORMATS as format (format)}
					<Button
						type="button"
						size="sm"
						variant={filters.formats.includes(format) ? "default" : "outline"}
						class="h-6 px-2 text-xs"
						onclick={() => (filters.formats = toggle(filters.formats, format))}
					>
						{format}
					</Button>
				{/each}
			</div>
		</div>

		{#if libraries.length > 1}
			<div class="flex flex-col gap-2">
				<p class="text-xs font-medium text-muted-foreground">Library</p>
				<div class="flex flex-wrap gap-1">
					{#each libraries as lib (lib.uid)}
						<Button
							type="button"
							size="sm"
							variant={filters.libraries.includes(lib.uid) ? "default" : "outline"}
							class="h-6 px-2 text-xs"
							onclick={() => (filters.libraries = toggle(filters.libraries, lib.uid))}
						>
							{lib.name}
						</Button>
					{/each}
				</div>
			</div>
		{/if}
	</Popover.Content>
</Popover.Root>
