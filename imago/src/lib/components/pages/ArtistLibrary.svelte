<script lang="ts">
	import { ArrowDownAZ, ArrowUpAZ, LayoutGrid, List } from "lucide-svelte";

	import { Button } from "$shadcn/button/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";
	import MediaGrid from "$lib/layouts/MediaGrid.svelte";

	import { getArtists, searchArtists, onLibraryChange } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { createPersistedViewState } from "$ts/store/session.svelte";
	import type { Artist } from "$ts/util/types";

	let search = $state("");
	let allArtists = $state<Artist[]>([]);
	let searchResults = $state<Artist[]>([]);

	const view = createPersistedViewState("artist-library", {
		sortField: "title",
		sortDir: "asc",
		colPreset: "default",
	});

	async function load() {
		allArtists = await getArtists();
	}

	async function runSearch() {
		searchResults = await searchArtists(search);
	}

	$effect(() => { load(); });
	$effect(() => { search; runSearch(); });
	$effect(() => {
		const unsub = onLibraryChange("artists:changed", load);
		return unsub;
	});

	const sortedArtists = $derived.by(() => {
		const dir = view.sort.direction === "asc" ? 1 : -1;
		return [...allArtists].sort((a, b) => a.name.localeCompare(b.name) * dir);
	});

	const filteredArtists = $derived(search.trim().length >= 2 ? searchResults : sortedArtists);
</script>

<div class="flex flex-col gap-2 pt-4 border-2 h-full w-full overflow-hidden rounded-md">

	<div class="flex justify-between items-center gap-2 pr-4 pl-4">
		<h1 class="h1">Artists</h1>
		<SearchBar bind:search searchCount={filteredArtists.length} />
	</div>

	<div class="flex flex-col h-full w-full overflow-hidden gap-3 p-0.5">
		<div class="flex justify-between pr-4 pl-4">
			<span>{allArtists.length} {allArtists.length === 1 ? "artist" : "artists"}</span>
			<div class="flex items-center gap-1">
				<Button variant="ghost" size="icon" onclick={() => view.sort.direction = view.sort.direction === "asc" ? "desc" : "asc"}>
					{#if view.sort.direction === "asc"}
						<ArrowDownAZ />
					{:else}
						<ArrowUpAZ />
					{/if}
				</Button>
				<Button variant="ghost" size="icon" onclick={() => view.compact = !view.compact}>
					{#if view.compact}
						<List />
					{:else}
						<LayoutGrid />
					{/if}
				</Button>
			</div>
		</div>

		<ScrollArea class="min-h-0 min-w-0 pr-2">
		{#if view.compact}
			<div class="pr-4 pl-4 pb-4">
				{#each filteredArtists as artist}
					<div
						class="grid items-center px-3 border-b hover:bg-muted/50"
						style="grid-template-columns: 40px 1fr; height: 56px;"
					>
						<ArtworkDisplay entity={artist} type="artist" size={40} />
						<button onclick={() => setSelection(artist.uid)} class="pl-2 text-sm truncate text-left">
							<p class="text-sm font-medium">{artist.name}</p>
						</button>
					</div>
				{/each}
			</div>
		{:else}
			<MediaGrid>
				{#each filteredArtists as artist}
					<button onclick={() => setSelection(artist.uid)} class="flex flex-col items-center gap-2 p-2 rounded-md bg-background border hover:border-primary transition-colors cursor-default">
						<div class="w-full aspect-square rounded-full bg-muted flex items-center justify-center overflow-hidden">
							<ArtworkDisplay entity={artist} />
						</div>
						<div class="min-w-0 w-full text-center">
							<p class="text-sm font-medium">{artist.name}</p>
						</div>
					</button>
				{/each}
			</MediaGrid>
		{/if}
		</ScrollArea>
	</div>

</div>