<script lang="ts">
	import { onMount } from "svelte";
	import Fuse from "fuse.js";
	import { History, X, ArrowUpDown, Check } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";

	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";
	import SearchFilterPopover from "$lib/components/custom/search/SearchFilterPopover.svelte";
	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import TrackTable from "$lib/components/custom/track-table/TrackTable.svelte";
	import DefultPlaylistArt from "$lib/components/pages/playlist/DefultPlaylistArt.svelte";
	import MediaGrid from "$lib/layouts/MediaGrid.svelte";

	import {
		getAlbums,
		getArtists,
		getPlaylists,
		searchTracks,
		onLibraryChange,
		libraryStore,
		loadLibraryRegistry,
	} from "$ts/store/library.svelte";
	import { searchHistory, type HistorySort } from "$ts/store/searchHistory.svelte";
	import { parseTracks } from "$ts/audio/playlistManager.svelte";
	import { parseArtists, parseArtistsToString, parseTags } from "$ts/util/parsers";
	import { createColumnState } from "$ts/util/columnConfig.svelte";
	import { SortState } from "$ts/util/sortConfig.svelte";
	import {
		countActiveFilters,
		createEmptyFilters,
		filterTracks,
		filterAlbums,
		filterArtists,
		filterPlaylists,
		type SearchFilters,
	} from "$ts/util/searchFilters";
	import type { Track, Album, Artist, Playlist } from "$ts/util/types";

	const ROW_LIMIT = 20;
	const HISTORY_LIMIT = 12;

	const sortLabels: Record<HistorySort, string> = {
		recent: "Most recent",
		alpha: "A to Z",
		frequent: "Most searched",
	};

	const cols = createColumnState("library");
	const trackSort = new SortState();

	let query = $state("");
	let debouncedQuery = $state("");
	let tab = $state("all");
	let filters = $state<SearchFilters>(createEmptyFilters());

	let allAlbums = $state.raw<Album[]>([]);
	let allArtists = $state.raw<Artist[]>([]);
	let allPlaylists = $state.raw<Playlist[]>([]);
	let trackResults = $state.raw<Track[]>([]);
	let trackSeq = 0;

	const albumFuse = $derived(
		new Fuse<Album>(allAlbums, {
			keys: [
				{ name: "title", weight: 0.5, getFn: (a) => a.title ?? "" },
				{ name: "album_artist", weight: 0.25, getFn: (a) => a.album_artist ?? "" },
				{ name: "artists", weight: 0.15, getFn: (a) => parseArtists(a.artists) },
				{ name: "release_date", weight: 0.1, getFn: (a) => a.release_date ?? "" },
				{ name: "tags", weight: 0.05, getFn: (a) => parseTags(a.tags) },
				{ name: "genres", weight: 0.05, getFn: (a) => parseTags(a.genres) },
			],
			threshold: 0.35,
			ignoreLocation: true,
			minMatchCharLength: 2,
		})
	);

	const artistFuse = $derived(
		new Fuse<Artist>(allArtists, {
			keys: [
				{ name: "name", weight: 0.5, getFn: (a) => a.name ?? "" },
				{ name: "aka", weight: 0.35, getFn: (a) => parseTags(a.aka) },
				{ name: "tags", weight: 0.05, getFn: (a) => parseTags(a.tags) },
				{ name: "genres", weight: 0.05, getFn: (a) => parseTags(a.genres) },
			],
			threshold: 0.35,
			ignoreLocation: true,
			minMatchCharLength: 2,
		})
	);

	const playlistFuse = $derived(
		new Fuse<Playlist>(allPlaylists, {
			keys: [
				{ name: "title", weight: 0.5, getFn: (p) => p.title ?? "" },
				{ name: "owner", weight: 0.25, getFn: (p) => p.owner ?? "" },
				{ name: "description", weight: 0.1, getFn: (p) => p.description ?? "" },
			],
			threshold: 0.35,
			ignoreLocation: true,
			minMatchCharLength: 2,
		})
	);

	const hasQuery = $derived(debouncedQuery.length >= 2);
	const filterCount = $derived(countActiveFilters(filters));
	const hasSearch = $derived(hasQuery || filterCount > 0);
	const defaultLibUid = $derived(libraryStore.libraries.find((l) => l.is_default)?.uid ?? null);

	const tracks = $derived(hasQuery ? filterTracks(trackResults, filters, defaultLibUid) : []);

	const albums = $derived.by(() => {
		if (!hasSearch) return [];
		const base = hasQuery ? albumFuse.search(debouncedQuery).map((r) => r.item) : allAlbums;
		return filterAlbums(base, filters, defaultLibUid);
	});

	const artists = $derived.by(() => {
		if (!hasSearch) return [];
		const base = hasQuery ? artistFuse.search(debouncedQuery).map((r) => r.item) : allArtists;
		return filterArtists(base, filters, defaultLibUid);
	});

	const playlists = $derived.by(() => {
		if (!hasSearch) return [];
		const base = hasQuery ? playlistFuse.search(debouncedQuery).map((r) => r.item) : allPlaylists;
		return filterPlaylists(base, filters);
	});

	const total = $derived(tracks.length + albums.length + artists.length + playlists.length);
	const history = $derived(searchHistory.entries.slice(0, HISTORY_LIMIT));

	async function loadAlbums() {
		allAlbums = await getAlbums();
	}

	async function loadArtists() {
		allArtists = await getArtists();
	}

	async function loadPlaylists() {
		allPlaylists = await getPlaylists();
	}

	function playlistArtTracks(playlist: Playlist): Track[] {
		return parseTracks(playlist.tracks) as unknown as Track[];
	}

	function remember() {
		searchHistory.add(query);
	}

	function onSearchKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") remember();
	}

	function runHistory(text: string) {
		query = text;
		debouncedQuery = text.trim();
		tab = "all";
		searchHistory.add(text);
	}

	$effect(() => {
		const q = query.trim();
		const timer = setTimeout(() => (debouncedQuery = q), 200);
		return () => clearTimeout(timer);
	});

	$effect(() => {
		const q = debouncedQuery;
		if (q.length < 2) {
			trackResults = [];
			return;
		}
		const seq = ++trackSeq;
		searchTracks(q).then((result) => {
			if (seq === trackSeq) trackResults = result;
		});
	});

	onMount(() => {
		searchHistory.load();
		loadAlbums();
		loadArtists();
		loadPlaylists();
		if (!libraryStore.loaded) loadLibraryRegistry();

		const unsubs = [
			onLibraryChange("albums:changed", loadAlbums),
			onLibraryChange("artists:changed", loadArtists),
			onLibraryChange("playlists:changed", loadPlaylists),
		];
		return () => unsubs.forEach((unsub) => unsub());
	});
</script>

<div class="flex h-full w-full flex-col gap-3 overflow-hidden rounded-md border-2 p-4">
	<div class="flex items-center justify-center gap-2">
		<div role="presentation" class="w-full max-w-sm" onkeydown={onSearchKeydown}>
			<SearchBar bind:search={query} searchCount={total} />
		</div>
		<SearchFilterPopover bind:filters />
	</div>

	{#if !hasSearch}
		<div class="flex items-center justify-between">
			<h2 class="h2">Recent Searches</h2>
			<div class="flex items-center gap-1">
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						<Button variant="ghost" size="sm" class="gap-2">
							<ArrowUpDown class="size-3" />
							{sortLabels[searchHistory.sort]}
						</Button>
					</DropdownMenu.Trigger>
					<DropdownMenu.Content>
						{#each Object.entries(sortLabels) as [mode, label] (mode)}
							<DropdownMenu.Item onSelect={() => (searchHistory.sort = mode as HistorySort)}>
								{#if searchHistory.sort === mode}
									<Check class="size-3 text-primary" />
								{/if}
								{label}
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
				<Button variant="ghost" size="sm" disabled={history.length === 0} onclick={() => searchHistory.clear()}>Clear</Button>
			</div>
		</div>

		{#if history.length === 0}
			<p class="text-sm text-muted-foreground">Your searches will show up here</p>
		{:else}
			<div class="app-music-grid grid gap-3">
				{#each history as entry (entry.query)}
					<div class="group relative">
						<button
							type="button"
							onclick={() => runHistory(entry.query)}
							class="flex w-full cursor-pointer items-center gap-3 rounded-md bg-muted/50 p-2 text-left hover:bg-muted"
						>
							<div class="flex size-12 shrink-0 items-center justify-center rounded-sm bg-muted">
								<History class="text-muted-foreground" />
							</div>
							<div class="flex min-w-0 flex-col">
								<span class="truncate text-sm font-medium">{entry.query}</span>
								<span class="truncate text-xs text-muted-foreground">
									Searched {entry.count} {entry.count === 1 ? "time" : "times"}
								</span>
							</div>
						</button>
						<Button
							variant="ghost"
							size="icon"
							class="absolute top-1 right-1 size-6 opacity-0 group-hover:opacity-100"
							onclick={() => searchHistory.remove(entry.query)}
						>
							<X class="size-3" />
						</Button>
					</div>
				{/each}
			</div>
		{/if}
	{:else}
		<Tabs.Root bind:value={tab} class="flex min-h-0 flex-1 flex-col gap-2">
			<Tabs.List class="w-full justify-start">
				<Tabs.Trigger value="all">All</Tabs.Trigger>
				<Tabs.Trigger value="tracks">
					Tracks
					{#if tracks.length > 0}<span class="ml-1 text-xs text-muted-foreground">{tracks.length}</span>{/if}
				</Tabs.Trigger>
				<Tabs.Trigger value="albums">
					Albums
					{#if albums.length > 0}<span class="ml-1 text-xs text-muted-foreground">{albums.length}</span>{/if}
				</Tabs.Trigger>
				<Tabs.Trigger value="artists">
					Artists
					{#if artists.length > 0}<span class="ml-1 text-xs text-muted-foreground">{artists.length}</span>{/if}
				</Tabs.Trigger>
				<Tabs.Trigger value="playlists">
					Playlists
					{#if playlists.length > 0}<span class="ml-1 text-xs text-muted-foreground">{playlists.length}</span>{/if}
				</Tabs.Trigger>
			</Tabs.List>

			<div class="min-h-0 flex-1 overflow-hidden">
				{#if tab === "tracks"}
					{#if !hasQuery}
						<p class="text-sm text-muted-foreground">Type at least 2 characters to search tracks</p>
					{:else if tracks.length === 0}
						<p class="text-sm text-muted-foreground">No tracks found</p>
					{:else}
						<div role="presentation" class="h-full" onclick={remember}>
							<TrackTable {tracks} columns={cols} sort={trackSort} />
						</div>
					{/if}
				{:else if total === 0}
					<p class="text-sm text-muted-foreground">
						{hasQuery ? "No results found" : "No results match these filters. Type at least 2 characters to include tracks"}
					</p>
				{:else if tab === "all"}
					<ScrollArea class="h-full">
						<div class="flex flex-col gap-4 pr-4">
							{#if tracks.length > 0}
								<section class="flex flex-col gap-1">
									<div class="flex items-center justify-between">
										<h2 class="h2">Tracks</h2>
										{#if tracks.length > ROW_LIMIT}
											<Button variant="link" size="sm" onclick={() => (tab = "tracks")}>See all {tracks.length}</Button>
										{/if}
									</div>
									<div class="flex gap-2 overflow-x-auto pb-1">
										{#each tracks.slice(0, ROW_LIMIT) as track (track.uid)}
											<div role="presentation" class="w-36 shrink-0" onclick={remember}>
												<AudioCard title={track.title ?? "Unknown Title"} subTitle={parseArtistsToString(track.artists)} entity={track} type="track" />
											</div>
										{/each}
									</div>
								</section>
							{/if}

							{#if albums.length > 0}
								<section class="flex flex-col gap-1">
									<div class="flex items-center justify-between">
										<h2 class="h2">Albums</h2>
										{#if albums.length > ROW_LIMIT}
											<Button variant="link" size="sm" onclick={() => (tab = "albums")}>See all {albums.length}</Button>
										{/if}
									</div>
									<div class="flex gap-2 overflow-x-auto pb-1">
										{#each albums.slice(0, ROW_LIMIT) as album (album.uid)}
											<div role="presentation" class="w-36 shrink-0" onclick={remember}>
												<AudioCard title={album.title} subTitle={album.album_artist ?? "Unknown Artist"} entity={album} type="album" />
											</div>
										{/each}
									</div>
								</section>
							{/if}

							{#if artists.length > 0}
								<section class="flex flex-col gap-1">
									<div class="flex items-center justify-between">
										<h2 class="h2">Artists</h2>
										{#if artists.length > ROW_LIMIT}
											<Button variant="link" size="sm" onclick={() => (tab = "artists")}>See all {artists.length}</Button>
										{/if}
									</div>
									<div class="flex gap-2 overflow-x-auto pb-1">
										{#each artists.slice(0, ROW_LIMIT) as artist (artist.uid)}
											<div role="presentation" class="w-36 shrink-0" onclick={remember}>
												<AudioCard title={artist.name} subTitle="Artist" entity={artist} type="artist" />
											</div>
										{/each}
									</div>
								</section>
							{/if}

							{#if playlists.length > 0}
								<section class="flex flex-col gap-1">
									<div class="flex items-center justify-between">
										<h2 class="h2">Playlists</h2>
										{#if playlists.length > ROW_LIMIT}
											<Button variant="link" size="sm" onclick={() => (tab = "playlists")}>See all {playlists.length}</Button>
										{/if}
									</div>
									<div class="flex gap-2 overflow-x-auto pb-1">
										{#each playlists.slice(0, ROW_LIMIT) as playlist (playlist.uid)}
											<div role="presentation" class="w-36 shrink-0" onclick={remember}>
												<AudioCard title={playlist.title} subTitle={playlist.owner} entity={playlist} type="playlist">
													<DefultPlaylistArt tracks={playlistArtTracks(playlist)} />
												</AudioCard>
											</div>
										{/each}
									</div>
								</section>
							{/if}
						</div>
					</ScrollArea>
				{:else if tab === "albums"}
					<ScrollArea class="h-full">
						<MediaGrid>
							{#each albums as album (album.uid)}
								<div role="presentation" onclick={remember}>
									<AudioCard title={album.title} subTitle={album.album_artist ?? "Unknown Artist"} entity={album} type="album" />
								</div>
							{/each}
						</MediaGrid>
					</ScrollArea>
				{:else if tab === "artists"}
					<ScrollArea class="h-full">
						<MediaGrid>
							{#each artists as artist (artist.uid)}
								<div role="presentation" onclick={remember}>
									<AudioCard title={artist.name} subTitle="Artist" entity={artist} type="artist" />
								</div>
							{/each}
						</MediaGrid>
					</ScrollArea>
				{:else if tab === "playlists"}
					<ScrollArea class="h-full">
						<MediaGrid>
							{#each playlists as playlist (playlist.uid)}
								<div role="presentation" onclick={remember}>
									<AudioCard title={playlist.title} subTitle={playlist.owner} entity={playlist} type="playlist">
										<DefultPlaylistArt tracks={playlistArtTracks(playlist)} />
									</AudioCard>
								</div>
							{/each}
						</MediaGrid>
					</ScrollArea>
				{/if}
			</div>
		</Tabs.Root>
	{/if}
</div>

<style>
.app-music-grid {
	grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
}
</style>