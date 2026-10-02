<script lang="ts">
	import TrackTable from "$lib/components/custom/track-table/TrackTable.svelte";
	import TrackTableSettings from "$lib/components/custom/track-table/TrackTableButtons.svelte";
	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";

	import { createPersistedViewState } from "$ts/store/session.svelte";
	import { getTracks, onLibraryChange } from "$ts/store/library.svelte";
	import { createColumnState } from "$ts/util/columnConfig.svelte";
	import { parseAlbumEntries } from "$ts/util/parsers";
	import type { Track } from "$ts/util/types";

	const cols = createColumnState("library");
	const view = createPersistedViewState("track-library", {
		sortField: "year",
		sortDir: "desc",
		colPreset: "library",
	});

	let search = $state("");
	let allTracks = $state<Track[]>([]);

	const filteredTracks = $derived.by(() => {
		const q = search.trim().toLowerCase();
		if (q.length < 2) return allTracks;
		return allTracks.filter(t =>
			(t.title?.toLowerCase().includes(q)) ||
			(t.album_artist?.toLowerCase().includes(q)) ||
			(parseAlbumEntries(t.albums)[0]?.name?.toLowerCase().includes(q)) ||
			(t.year?.toLowerCase().includes(q))
		);
	});

	async function load() {
		allTracks = await getTracks();
	}

	$effect(() => { load(); });
	$effect(() => {
		const unsub = onLibraryChange("tracks:changed", load);
		return unsub;
	});
</script>

<div class="flex flex-col gap-2 p-4 border-2 h-full w-full overflow-hidden rounded-md">

	<div class="flex justify-between items-center gap-2">
		<h1 class="h1">Tracks</h1>
		<SearchBar bind:search searchCount={filteredTracks.length} />
	</div>

	<div class="flex justify-between items-center gap-2">
		<span>{allTracks.length} {allTracks.length === 1 ? "track" : "tracks"}</span>
		<TrackTableSettings cols={cols} sort={view.sort} compact={view.compact} onCompactChange={(v) => view.compact = v} />
	</div>

	<div class="flex-1 min-h-0 h-full">
		<TrackTable tracks={filteredTracks} columns={cols} sort={view.sort} compact={view.compact} />
	</div>

</div>