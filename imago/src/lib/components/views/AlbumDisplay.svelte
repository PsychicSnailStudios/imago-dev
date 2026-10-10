<script lang="ts">
	import { Pencil } from "lucide-svelte";
	import { invoke } from "@tauri-apps/api/core";

	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";
	import Button from "$shadcn/button/button.svelte";

	import TrackTableSettings from "$lib/components/custom/track-table/TrackTableSettings.svelte";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import TrackTable from "$lib/components/custom/track-table/TrackTable.svelte";
	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import NavButtons from "$lib/components/custom/NavButtons.svelte";
	import TagList from "$lib/components/custom/tags/TagList.svelte";
	import DownloadButton from "$lib/components/custom/DownloadButton.svelte";

	import { selection, setSelection } from "$ts/store/session.svelte";
	import { getAlbum, getAlbums, getTrackArrayFromUID, onLibraryChange, onSingleChange } from "$ts/store/library.svelte";
	import { queueTracksByObject } from "$ts/audio/audioManager.svelte";
	import { openEditModal } from "$ts/ui/editModal.svelte";
	import { totalDuration } from "$ts/util/helpers";
	import { createPersistedViewState } from "$ts/store/session.svelte";
	import { artworkColorCache, fetchArtworkColor } from "$ts/library/artworkLoader";

	import type { Album, Track } from "$ts/util/types";
	import { parseTags } from "$ts/util/parsers";

	const view = createPersistedViewState("album", {
		sortField: "number",
		sortDir: "asc",
		colPreset: "album",
	});

	let album = $state<Album | null>(null);
	let tracks = $state<Track[]>([]);
	let artistAlbums = $state<Album[]>([]);
	let color = $state("rgb(30, 30, 30)");

	let genres = $derived(album?.genres ? parseTags(album.genres) : null);
	let tags = $derived(album?.tags ? parseTags(album.tags) : null);
	let albumArtistUid = $state<string | null>(null);

	$effect(() => {
		const name = album?.album_artist ?? null;
		if (!name) { albumArtistUid = null; return; }
		invoke<string | null>("get_artist_uid_by_name", { name }).then((uid) => { albumArtistUid = uid; });
	});

	async function loadAlbum() {
		const uid = selection.uid;
		if (!uid) { album = null; tracks = []; artistAlbums = []; return; }
		album = await getAlbum(uid);
		if (!album) { tracks = []; artistAlbums = []; return; }
		tracks = await getTrackArrayFromUID(album.uid, view.sort);
		await loadArtistAlbums();
	}

	async function loadArtistAlbums() {
		if (!album?.album_artist) { artistAlbums = []; return; }
		const artistName = album.album_artist.toLowerCase();
		const all = await getAlbums();
		artistAlbums = all.filter(a => (a.album_artist ?? "").toLowerCase() === artistName);
	}

	// Reload when selection changes
	$effect(() => {
		selection.uid;
		loadAlbum();
	});

	// Reload tracks when sort changes
	$effect(() => {
		view.sort;
		if (album) getTrackArrayFromUID(album.uid, view.sort).then(t => tracks = t);
	});

	// Color effect
	$effect(() => {
		const uid = selection.uid;
		if (!uid) return;
		const cached = artworkColorCache.get(`album:${uid}`);
		if (cached) { color = cached; return; }
		color = "var(--muted)";
		fetchArtworkColor(uid, "album").then(c => { color = c; });
	});

	// Event bus subscriptions
	$effect(() => {
		const unsubAlbums = onLibraryChange("albums:changed", loadAlbum);
		const unsubTracks = onLibraryChange("tracks:changed", async () => {
			if (album) tracks = await getTrackArrayFromUID(album.uid, view.sort);
		});
		const unsubSingle = onSingleChange((uid) => {
			if (album && uid === album.uid) loadAlbum();
		});
		return () => { unsubAlbums(); unsubTracks(); unsubSingle(); };
	});

	function trackIsGhost(t: Track): boolean {
		const hasLocal = t.path && t.path !== "" && t.path !== t.uid;
		const hasRemote = (t as any).remote_path && (t as any).remote_path.length > 0;
		return !hasLocal && !hasRemote;
	}

	let allGhosts = $derived(
		tracks.length > 0 && tracks.every(t => trackIsGhost(t))
	);
</script>

<div class="flex flex-col gap-4 p-4 border-2 h-full w-full overflow-hidden rounded-md" style="background: linear-gradient(180deg, {color} 0%, transparent 80%)">

	<ScrollArea class="min-h-0 min-w-0 h-full">
	<div class="flex flex-col gap-4 pb-4 pr-4">
		<NavButtons />
			
		{#if album}
		<div class="flex gap-4 items-end">
			<ArtworkDisplay entity={album} size={160} />

			<div class="flex flex-col gap-1">
				<span class="text-xs text-muted-foreground">{album.format ? album.format : "Album"}</span>
				<h2 class="text-2xl font-bold">{album.title}</h2>
				<div class="flex gap-3 text-sm text-muted-foreground flex-wrap">
					{#if album.album_artist}
						<span role="button" tabindex="0" onclick={() => { if (albumArtistUid) setSelection(albumArtistUid); }} onkeydown={(e) => { if (e.key === 'Enter' && albumArtistUid) setSelection(albumArtistUid); }} class="text-sm truncate cursor-pointer hover:underline">
							{album.album_artist}
						</span>
					{/if}
					{#if album.release_date}
						<span>|</span>
						<span>{album.release_date}</span>
					{/if}
					<span>|</span>
					<span>{tracks.length} {tracks.length === 1 ? "song" : "songs"}</span>
					{#if tracks.length > 0}
						<span>|</span>
						<span>{totalDuration(tracks)}</span>
					{/if}
				</div>

				<TagList uid={album.uid} tags={genres!} canEdit={false} />
				<TagList uid={album.uid} tags={tags!} canEdit={true} />
			</div>
		</div>

		<div class="flex gap-2 justify-between items-center flex-wrap p-2 rounded-md"
			  style="background: {color};">
			<div>
				<Button variant="default" disabled={allGhosts} onclick={() => queueTracksByObject(tracks, true, false, album.uid)}>{ tracks.length === 1 ? "Play" : "Play All"}</Button>
				{#if tracks.length > 1}
					<Button variant="outline" disabled={allGhosts} onclick={() => queueTracksByObject(tracks, true, true, album.uid)}>Shuffle</Button>
				{/if}
			</div>
			<div class="flex gap-1 items-center">
				<DownloadButton uid={album.uid} variant="ghost" />
				<Button variant="ghost" onclick={() => openEditModal({ type: "album", uid: album!.uid })}><Pencil /></Button>
				<TrackTableSettings cols={view.cols} sort={view.sort} compact={view.compact} onCompactChange={(v) => view.compact = v} />
			</div>
		</div>

		<TrackTable tracks={tracks} columns={view.cols} sort={view.sort} compact={view.compact} albumUid={album.uid} emulateType={album.emulate_type} />

		{#if album.label}
			<span class="text-muted-foreground text-sm">{album.label}</span>
		{/if}

		<div class="flex flex-col gap-2 w-full pt-4">
			<h4>More by {album.album_artist}</h4>
			<ScrollArea orientation="horizontal" class="min-h-0 min-w-0">
				<div class="grid gap-2 pb-4" style="grid-auto-columns: 150px; grid-auto-flow: column;">
					{#each artistAlbums as a}
						<AudioCard title={a.title} subTitle={a.album_artist ?? ""} entity={a} type="album" />
					{/each}
				</div>
			</ScrollArea>
		</div>
		{:else}
			<span class="text-muted-foreground text-sm">Loading...</span>
		{/if}

	</div>
	</ScrollArea>

</div>