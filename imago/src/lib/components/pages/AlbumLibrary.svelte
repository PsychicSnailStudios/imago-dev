<script lang="ts">
	import { ArrowUpDown, ArrowUp, ArrowDown, LayoutGrid, List } from "lucide-svelte";

	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { Button } from "$shadcn/button/index.js";

	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";
	import MediaGrid from "$lib/layouts/MediaGrid.svelte";

	import { getAlbums, searchAlbums, onLibraryChange } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { createPersistedViewState } from "$ts/store/session.svelte";
	import type { SortField } from "$ts/util/sortConfig.svelte";
	import { parseArtistsToString } from "$ts/util/parsers";
	import type { Album } from "$ts/util/types";

	let search = $state("");
	let allAlbums = $state<Album[]>([]);
	let searchResults = $state<Album[]>([]);

	const view = createPersistedViewState("album-library", {
		sortField: "number",
		sortDir: "asc",
		colPreset: "album",
	});

	async function load() {
		allAlbums = await getAlbums();
	}

	async function runSearch() {
		searchResults = await searchAlbums(search);
	}

	$effect(() => { load(); });
	$effect(() => { search; runSearch(); });
	$effect(() => {
		const unsub = onLibraryChange("albums:changed", load);
		return unsub;
	});

	const sortedAlbums = $derived.by(() => {
		const field = view.sort.field;
		const dir = view.sort.direction === "asc" ? 1 : -1;
		return [...allAlbums].sort((a, b) => {
			let cmp = 0;
			if (field === "name" as SortField) {
				cmp = a.title.localeCompare(b.title);
			} else if (field === "artist" as SortField) {
				cmp = (a.album_artist ?? "").localeCompare(b.album_artist ?? "");
			} else if (field === "year" as SortField) {
				cmp = (a.release_date ?? "").localeCompare(b.release_date ?? "");
			}
			return cmp * dir;
		});
	});

	const filteredAlbums = $derived(search.trim().length < 2 ? sortedAlbums : searchResults);

	function toggleAlbumSort(field: "name" | "artist" | "year") {
		if (view.sort.field === field) {
			view.sort.direction = view.sort.direction === "asc" ? "desc" : "asc";
		} else {
			view.sort.field = field as SortField;
			view.sort.direction = "asc";
		}
	}
</script>

<div class="flex flex-col gap-2 pt-4 border-2 h-full w-full overflow-hidden rounded-md">

	<div class="flex justify-between items-center gap-2 pr-4 pl-4">
		<h1 class="h1">Albums</h1>
		<SearchBar bind:search searchCount={filteredAlbums.length} />
	</div>

	<div class="flex justify-between pr-4 pl-4">
		<span>{allAlbums.length} {allAlbums.length === 1 ? "album" : "albums"}</span>
		<div class="flex items-center gap-1">
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					<Button variant="ghost" size="sm" class="gap-2">
						{#if view.sort.field === null}
							<ArrowUpDown class="size-3" />
						{:else if view.sort.direction === "asc"}
							<ArrowUp class="size-3" />
						{:else}
							<ArrowDown class="size-3" />
						{/if}
						{view.sort.field}
					</Button>
				</DropdownMenu.Trigger>
				<DropdownMenu.Content>
					<DropdownMenu.Item
						class={view.sort.field === "name" as SortField ? "bg-accent" : ""}
						onSelect={() => toggleAlbumSort("name")}
					>
						{#if view.sort.field === "name" as SortField}
							{#if view.sort.direction === "asc"}
								<ArrowUp class="size-3 text-primary" />
							{:else}
								<ArrowDown class="size-3 text-primary" />
							{/if}
						{/if}
						Name
					</DropdownMenu.Item>
					<DropdownMenu.Item
						class={view.sort.field === "artist" ? "bg-accent" : ""}
						onSelect={() => toggleAlbumSort("artist")}
					>
						{#if view.sort.field === "artist"}
							{#if view.sort.direction === "asc"}
								<ArrowUp class="size-3 text-primary" />
							{:else}
								<ArrowDown class="size-3 text-primary" />
							{/if}
						{/if}
						Artist
					</DropdownMenu.Item>
					<DropdownMenu.Item
						class={view.sort.field === "year" ? "bg-accent" : ""}
						onSelect={() => toggleAlbumSort("year")}
					>
						{#if view.sort.field === "year"}
							{#if view.sort.direction === "asc"}
								<ArrowUp class="size-3 text-primary" />
							{:else}
								<ArrowDown class="size-3 text-primary" />
							{/if}
						{/if}
						Year
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			<Button variant="ghost" size="icon" onclick={() => view.compact = !view.compact}>
				{#if view.compact}
					<List />
				{:else}
					<LayoutGrid />
				{/if}
			</Button>
		</div>
	</div>

	<div class="flex flex-col h-full w-full overflow-hidden gap-3 p-0.5">
		<ScrollArea class="min-h-0 min-w-0 pr-2">
		{#if view.compact}
			<div class="pr-4 pl-4 pb-4">
				{#each filteredAlbums as album}
					<div
						class="grid items-center px-3 border-b hover:bg-muted/50"
						style="grid-template-columns: 40px 1fr 1fr 40px; height: 56px;"
					>
						<ArtworkDisplay entity={album} size={40} />
						<button onclick={() => setSelection(album.uid)} class="pl-2 text-sm truncate text-left">
							<p class="text-sm font-medium">{album.title}</p>
						</button>
						<p class="text-sm">{parseArtistsToString(album.artists)}</p>
						<p class="text-sm">{album.release_date}</p>
					</div>
				{/each}
			</div>
		{:else}
			<MediaGrid>
				{#each filteredAlbums as album}
					<AudioCard title={album.title} subTitle={album.album_artist ?? "Unknown Artist"} entity={album} type="album" />
				{/each}
			</MediaGrid>
		{/if}
		</ScrollArea>
	</div>
</div>