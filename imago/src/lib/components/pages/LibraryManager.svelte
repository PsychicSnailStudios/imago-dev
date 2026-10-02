<script lang="ts">
	import { Trash, CloudDownload, FolderInput, Loader2 } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import * as Tooltip from "$shadcn/tooltip/index.js";
	import { Checkbox } from "$shadcn/checkbox/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { Button, buttonVariants } from "$shadcn/button/index.js";

	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";
	import TrackRow from "$lib/components/pages/library-manager/TrackRow.svelte";
	import AlbumRow from "$lib/components/pages/library-manager/AlbumRow.svelte";
	import ArtistRow from "$lib/components/pages/library-manager/ArtistRow.svelte";
	import DuplicateGroupCard from "$lib/components/pages/library-manager/DuplicateGroupCard.svelte";
	import AddToAlbumDialog from "$lib/components/dialogs/AddToAlbumDialog.svelte";
	import TagManager from "$lib/components/pages/library-manager/TagManager.svelte";
	import LibraryDatabaseManager from "$lib/components/pages/library-manager/LibraryDatabaseManager.svelte";
	import BlocklistDialog from "$lib/components/dialogs/BlocklistDialog.svelte";

	import {
		removeTracksFromLibrary,
		removeAlbums,
		removeArtists,
		getDuplicates,
	} from "$ts/library/libraryManager";
	import { enrichAlbums, enrichArtists, enrichTracks, enrichAllAlbums, enrichAllArtists, enrichAllTracks } from "$ts/library/enrichment";
	import { searchTracks, searchAlbums, searchArtists, onLibraryChange } from "$ts/store/library.svelte";
	import type { DuplicateGroup, Track, Album, Artist } from "$ts/util/types";
	import { scanState } from "$ts/store/session.svelte";

	// ─── Search ───────────────────────────────────────────────────────────────────
	let trackSearch = $state("");
	let albumSearch = $state("");
	let artistSearch = $state("");

	// ─── Track filter mode ────────────────────────────────────────────────────────
	type TrackFilterMode = "all" | "ghosts" | "remote" | "local";
	let trackFilterMode = $state<TrackFilterMode>("all");

	// ─── Duplicates ───────────────────────────────────────────────────────────────
	let duplicates = $state<DuplicateGroup[]>([]);
	let duplicatesLoaded = $state(false);

	// ─── Add to album dialog ──────────────────────────────────────────────────────
	let addToAlbumOpen = $state(false);

	// ─── Selections ───────────────────────────────────────────────────────────────
	let trackSelections = $state<Record<string, boolean>>({});
	let albumSelections = $state<Record<string, boolean>>({});
	let artistSelections = $state<Record<string, boolean>>({});

	let lastTrackIndex = $state<number | null>(null);
	let lastAlbumIndex = $state<number | null>(null);
	let lastArtistIndex = $state<number | null>(null);

	// ─── Async filtered lists ─────────────────────────────────────────────────────
	let filteredTracks = $state<Track[]>([]);
	let filteredAlbums = $state<Album[]>([]);
	let filteredArtists = $state<Artist[]>([]);

	async function loadTracks() {
		let pool = await searchTracks(trackSearch);
		if (trackFilterMode === "ghosts") pool = pool.filter((t) => !t.remote_path && (!t.path || t.path === t.uid));
		else if (trackFilterMode === "remote") pool = pool.filter((t) => !!t.remote_path);
		else if (trackFilterMode === "local") pool = pool.filter((t) => !!t.path && t.path !== t.uid && !t.remote_path);
		filteredTracks = pool;
	}

	async function loadAlbums() {
		filteredAlbums = await searchAlbums(albumSearch);
	}

	async function loadArtists() {
		filteredArtists = await searchArtists(artistSearch);
	}

	$effect(() => { trackSearch; trackFilterMode; loadTracks(); });
	$effect(() => { albumSearch; loadAlbums(); });
	$effect(() => { artistSearch; loadArtists(); });

	$effect(() => {
		const u1 = onLibraryChange("tracks:changed", loadTracks);
		const u2 = onLibraryChange("albums:changed", loadAlbums);
		const u3 = onLibraryChange("artists:changed", loadArtists);
		return () => { u1(); u2(); u3(); };
	});

	// ─── Track selection derived ──────────────────────────────────────────────────
	const selectedTrackUids = $derived(
		Object.entries(trackSelections).filter(([, v]) => v).map(([k]) => k)
	);
	const anyTracksSelected = $derived(selectedTrackUids.length > 0);
	const allTracksSelected = $derived(
		filteredTracks.length > 0 && filteredTracks.every((t) => trackSelections[t.uid])
	);

	// ─── Album selection derived ──────────────────────────────────────────────────
	const selectedAlbumUids = $derived(
		Object.entries(albumSelections).filter(([, v]) => v).map(([k]) => k)
	);
	const anyAlbumsSelected = $derived(selectedAlbumUids.length > 0);
	const allAlbumsSelected = $derived(
		filteredAlbums.length > 0 && filteredAlbums.every((a) => albumSelections[a.uid])
	);

	// ─── Artist selection derived ─────────────────────────────────────────────────
	const selectedArtistUids = $derived(
		Object.entries(artistSelections).filter(([, v]) => v).map(([k]) => k)
	);
	const anyArtistsSelected = $derived(selectedArtistUids.length > 0);
	const allArtistsSelected = $derived(
		filteredArtists.length > 0 && filteredArtists.every((a) => artistSelections[a.uid])
	);

	// ─── Shift-select helpers ─────────────────────────────────────────────────────
	function shiftSelectRange<T extends { uid: string }>(
		list: T[],
		clickedIndex: number,
		lastIndex: number | null,
		selections: Record<string, boolean>
	): Record<string, boolean> {
		const next = { ...selections };
		const from = lastIndex ?? clickedIndex;
		const lo = Math.min(from, clickedIndex);
		const hi = Math.max(from, clickedIndex);
		for (let i = lo; i <= hi; i++) {
			next[list[i].uid] = true;
		}
		return next;
	}

	function toggleTrack(uid: string, index: number) {
		trackSelections = { ...trackSelections, [uid]: !(trackSelections[uid] ?? false) };
		lastTrackIndex = index;
	}

	function shiftTrack(index: number) {
		trackSelections = shiftSelectRange(filteredTracks, index, lastTrackIndex, trackSelections);
		lastTrackIndex = index;
	}

	function toggleAlbum(uid: string, index: number) {
		albumSelections = { ...albumSelections, [uid]: !(albumSelections[uid] ?? false) };
		lastAlbumIndex = index;
	}

	function shiftAlbum(index: number) {
		albumSelections = shiftSelectRange(filteredAlbums, index, lastAlbumIndex, albumSelections);
		lastAlbumIndex = index;
	}

	function toggleArtist(uid: string, index: number) {
		artistSelections = { ...artistSelections, [uid]: !(artistSelections[uid] ?? false) };
		lastArtistIndex = index;
	}

	function shiftArtist(index: number) {
		artistSelections = shiftSelectRange(filteredArtists, index, lastArtistIndex, artistSelections);
		lastArtistIndex = index;
	}

	// ─── Bulk actions ─────────────────────────────────────────────────────────────
	async function bulkRemoveTracks() {
		await removeTracksFromLibrary(selectedTrackUids);
		trackSelections = {};
	}

	async function bulkRemoveAlbums() {
		await removeAlbums(selectedAlbumUids);
		albumSelections = {};
	}

	async function bulkRemoveArtists() {
		await removeArtists(selectedArtistUids);
		artistSelections = {};
	}

	async function bulkEnrichTracks() {
		scanState.enriching = true;
		scanState.enrichDone = 0;
		scanState.enrichTotal = 0;
		scanState.enrichErrors = 0;
		try {
			await enrichTracks(selectedTrackUids);
		} catch (e) {
			scanState.status = `Enrich error: ${e}`;
		} finally {
			scanState.enriching = false;
		}
	}

	async function bulkEnrichAlbums() {
		scanState.enriching = true;
		scanState.enrichDone = 0;
		scanState.enrichTotal = 0;
		scanState.enrichErrors = 0;
		try {
			await enrichAlbums(selectedAlbumUids);
		} catch (e) {
			scanState.status = `Enrich error: ${e}`;
		} finally {
			scanState.enriching = false;
		}
	}

	async function bulkEnrichArtists() {
		scanState.enriching = true;
		scanState.enrichDone = 0;
		scanState.enrichTotal = 0;
		scanState.enrichErrors = 0;
		try {
			await enrichArtists(selectedArtistUids);
		} catch (e) {
			scanState.status = `Enrich error: ${e}`;
		} finally {
			scanState.enriching = false;
		}
	}

	async function enrich(type: "tracks" | "albums" | "artists") {
		scanState.enriching = true;
		scanState.enrichDone = 0;
		scanState.enrichTotal = 0;
		scanState.enrichErrors = 0;
		try {
			switch (type) {
				case "tracks":  await enrichAllTracks();  break;
				case "albums":  await enrichAllAlbums();  break;
				case "artists": await enrichAllArtists(); break;
			}
		} catch (e) {
			scanState.status = `Enrich error: ${e}`;
		} finally {
			scanState.enriching = false;
		}
	}

	// ─── Duplicates ───────────────────────────────────────────────────────────────
	async function loadDuplicates() {
		duplicatesLoaded = false;
		duplicates = await getDuplicates();
		duplicatesLoaded = true;
	}

	function onDuplicateResolved(uid: string) {
		duplicates = duplicates.filter(g => g.tracks[0]?.uid !== uid);
	}
</script>

<div class="flex flex-col gap-2 p-4 border-2 h-full w-full overflow-hidden rounded-md">
	<h1 class="h1">Library Manager</h1>

	<ScrollArea class="h-full min-h-0 min-w-0 pr-4">
		<div class="flex flex-col gap-4 pb-4">
			<Tabs.Root value="database">
				<Tabs.List>
					<Tabs.Trigger value="database">Database</Tabs.Trigger>
					<Tabs.Trigger value="tags">Tags</Tabs.Trigger>
					<Tabs.Trigger value="tracks">Tracks</Tabs.Trigger>
					<Tabs.Trigger value="albums">Albums</Tabs.Trigger>
					<Tabs.Trigger value="artists">Artists</Tabs.Trigger>
					<Tabs.Trigger value="duplicates" onclick={loadDuplicates}>Duplicates</Tabs.Trigger>
				</Tabs.List>

				<!-- TRACKS -->
				<Tabs.Content value="tracks">
					<div class="flex flex-col gap-2 pt-2">
						<div class="flex gap-2 justify-between items-center">
							<span class="text-sm text-muted-foreground">
								{filteredTracks.length} {filteredTracks.length === 1 ? "track" : "tracks"}
							</span>
							<SearchBar bind:search={trackSearch} searchCount={filteredTracks.length} />
						</div>

						<div class="flex gap-1 flex-wrap">
							{#each (["all","ghosts","remote","local"] as const) as mode}
								<Button variant={trackFilterMode === mode ? "default" : "outline"} size="sm" onclick={() => trackFilterMode = mode}>
									{mode.charAt(0).toUpperCase() + mode.slice(1)}
								</Button>
							{/each}
						</div>

						{#if anyTracksSelected}
							<div class="flex gap-2 justify-between items-center">
								<div class="flex gap-2 items-center">
									<Checkbox
										checked={allTracksSelected}
										onCheckedChange={(v) => {
											const next: Record<string, boolean> = {};
											filteredTracks.forEach(t => next[t.uid] = !!v);
											trackSelections = next;
										}}
									/>
									<span class="text-xs text-muted-foreground">{selectedTrackUids.length} selected</span>
								</div>
								<div class="flex gap-1">
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "destructive", size: "sm" })}
											onclick={bulkRemoveTracks}
										>
											<Trash class="w-4 h-4 mr-1" /> Remove
										</Tooltip.Trigger>
										<Tooltip.Content><p>Remove selected from library</p></Tooltip.Content>
									</Tooltip.Root>
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "outline", size: "sm" })}
											onclick={bulkEnrichTracks}
											disabled={scanState.enriching}
										>
											{#if scanState.enriching}
												<Loader2 class="w-4 h-4 mr-1 animate-spin" />
											{:else}
												<CloudDownload class="w-4 h-4 mr-1" />
											{/if}
											Enrich
										</Tooltip.Trigger>
										<Tooltip.Content><p>Fetch metadata for selected</p></Tooltip.Content>
									</Tooltip.Root>
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "outline", size: "sm" })}
											onclick={() => (addToAlbumOpen = true)}
										>
											<FolderInput class="w-4 h-4 mr-1" /> Add to Album
										</Tooltip.Trigger>
										<Tooltip.Content><p>Add selected to an album</p></Tooltip.Content>
									</Tooltip.Root>
								</div>
							</div>
						{/if}

						<div class="flex flex-col gap-2">
							{#each filteredTracks as track, i (track.uid)}
								<TrackRow
									{track}
									selected={trackSelections[track.uid] ?? false}
									onToggle={() => toggleTrack(track.uid, i)}
									onShiftClick={() => shiftTrack(i)}
								/>
							{/each}
						</div>
					</div>
				</Tabs.Content>

				<!-- ALBUMS -->
				<Tabs.Content value="albums">
					<div class="flex flex-col gap-2 pt-2">
						<div class="flex gap-2 justify-between items-center">
							<span class="text-sm text-muted-foreground">
								{filteredAlbums.length} {filteredAlbums.length === 1 ? "album" : "albums"}
							</span>
							<SearchBar bind:search={albumSearch} searchCount={filteredAlbums.length} />
						</div>

						{#if anyAlbumsSelected}
							<div class="flex gap-2 justify-between items-center">
								<div class="flex gap-2 items-center">
									<Checkbox
										checked={allAlbumsSelected}
										onCheckedChange={(v) => {
											const next: Record<string, boolean> = {};
											filteredAlbums.forEach(a => next[a.uid] = !!v);
											albumSelections = next;
										}}
									/>
									<span class="text-xs text-muted-foreground">{selectedAlbumUids.length} selected</span>
								</div>
								<div class="flex gap-1">
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "destructive", size: "sm" })}
											onclick={bulkRemoveAlbums}
										>
											<Trash class="w-4 h-4 mr-1" /> Remove
										</Tooltip.Trigger>
										<Tooltip.Content><p>Remove selected albums</p></Tooltip.Content>
									</Tooltip.Root>
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "outline", size: "sm" })}
											onclick={bulkEnrichAlbums}
											disabled={scanState.enriching}
										>
											{#if scanState.enriching}
												<Loader2 class="w-4 h-4 mr-1 animate-spin" />
											{:else}
												<CloudDownload class="w-4 h-4 mr-1" />
											{/if}
											Enrich
										</Tooltip.Trigger>
										<Tooltip.Content><p>Fetch metadata for selected</p></Tooltip.Content>
									</Tooltip.Root>
								</div>
							</div>
						{/if}

						<div class="flex flex-col gap-2">
							{#each filteredAlbums as album, i (album.uid)}
								<AlbumRow
									{album}
									selected={albumSelections[album.uid] ?? false}
									onToggle={() => toggleAlbum(album.uid, i)}
									onShiftClick={() => shiftAlbum(i)}
								/>
							{/each}
						</div>
					</div>
				</Tabs.Content>

				<!-- ARTISTS -->
				<Tabs.Content value="artists">
					<div class="flex flex-col gap-2 pt-2">
						<div class="flex gap-2 justify-between items-center">
							<span class="text-sm text-muted-foreground">
								{filteredArtists.length} {filteredArtists.length === 1 ? "artist" : "artists"}
							</span>
							<SearchBar bind:search={artistSearch} searchCount={filteredArtists.length} />
						</div>

						{#if anyArtistsSelected}
							<div class="flex gap-2 justify-between items-center">
								<div class="flex gap-2 items-center">
									<Checkbox
										checked={allArtistsSelected}
										onCheckedChange={(v) => {
											const next: Record<string, boolean> = {};
											filteredArtists.forEach(a => next[a.uid] = !!v);
											artistSelections = next;
										}}
									/>
									<span class="text-xs text-muted-foreground">{selectedArtistUids.length} selected</span>
								</div>
								<div class="flex gap-1">
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "destructive", size: "sm" })}
											onclick={bulkRemoveArtists}
										>
											<Trash class="w-4 h-4 mr-1" /> Remove
										</Tooltip.Trigger>
										<Tooltip.Content><p>Remove selected artists</p></Tooltip.Content>
									</Tooltip.Root>
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "outline", size: "sm" })}
											onclick={bulkEnrichArtists}
											disabled={scanState.enriching}
										>
											{#if scanState.enriching}
												<Loader2 class="w-4 h-4 mr-1 animate-spin" />
											{:else}
												<CloudDownload class="w-4 h-4 mr-1" />
											{/if}
											Enrich
										</Tooltip.Trigger>
										<Tooltip.Content><p>Fetch metadata for selected</p></Tooltip.Content>
									</Tooltip.Root>
								</div>
							</div>
						{/if}

						<div class="flex flex-col gap-2">
							{#each filteredArtists as artist, i (artist.uid)}
								<ArtistRow
									{artist}
									selected={artistSelections[artist.uid] ?? false}
									onToggle={() => toggleArtist(artist.uid, i)}
									onShiftClick={() => shiftArtist(i)}
								/>
							{/each}
						</div>
					</div>
				</Tabs.Content>

				<Tabs.Content value="tags">
					<TagManager />
				</Tabs.Content>

				<Tabs.Content value="duplicates">
					<div class="flex flex-col gap-2 pt-2">
						{#if !duplicatesLoaded}
							<p class="text-sm text-muted-foreground">Loading…</p>
						{:else if duplicates.length === 0}
							<p class="text-sm text-muted-foreground">No duplicates found.</p>
						{:else}
							<span class="text-sm text-muted-foreground">
								{duplicates.length} duplicate {duplicates.length === 1 ? "group" : "groups"}
							</span>
							<div class="flex flex-col gap-3">
								{#each duplicates as group (group.tracks[0]?.uid)}
									<DuplicateGroupCard {group} onresolved={() => onDuplicateResolved(group.tracks[0]?.uid ?? "")} />
								{/each}
							</div>
						{/if}
					</div>
				</Tabs.Content>

				<Tabs.Content value="database">
					<LibraryDatabaseManager />
				</Tabs.Content>
			</Tabs.Root>
		</div>
	</ScrollArea>
</div>

<AddToAlbumDialog bind:open={addToAlbumOpen} trackUids={selectedTrackUids} />
<BlocklistDialog />