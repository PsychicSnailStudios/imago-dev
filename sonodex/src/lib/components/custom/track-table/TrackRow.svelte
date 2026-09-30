<script lang="ts">

	// COMPONENTS
	import { Pause, Play } from "lucide-svelte";
	import Button from "$shadcn/button/button.svelte";

	// CUSTOM COMPONENTS
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import TrackTableEditButton from "$lib/components/custom/track-table/TrackRowEditButton.svelte";
   import TrackRating from "$lib/components/custom/text-display/TrackRating.svelte";
	import ArtistsList from "$lib/components/custom/text-display/ArtistsList.svelte";

	// SCRIPTS
	import { invoke } from "@tauri-apps/api/core";
	import { setSelection } from "$ts/store/session.svelte";
	import { trackSelection, selectTrack } from "$ts/store/trackSelection.svelte";
	import { startDrag, endDrag } from "$ts/store/drag.svelte";
	import { playTrackByUid, togglePlay } from "$ts/audio/audioManager.svelte";
	import { formatDuration } from "$ts/util/helpers";
	import { parseArtistsToString, parseAlbumEntries } from "$ts/util/parsers";
	import { player } from "$ts/audio/audioPlayer.svelte";
	import type { Track } from "$ts/util/types";

	// PROPS
	let {
		track,
		orderedUids,
		uidIndexMap,
		viewId,
		index,
		compact = false,
		gridTemplate,
		showNumber,
		showArtwork,
		showTitle,
		showArtist,
		showAlbum,
		showYear,
		showRating,
		showDuration,
		showLabel,
		showOptions,
		playlistUid = null,
		sourceUid = null,
	} = $props<{
		track: Track;
		orderedUids: string[];
		uidIndexMap: Map<string, number>;
		index: number;
    	viewId: string;    
		compact?: boolean;
		gridTemplate: string;
		showNumber: boolean;
		showArtwork: boolean;
		showTitle: boolean;
		showArtist: boolean;
		showAlbum: boolean;
		showYear: boolean;
		showRating: boolean;
		showDuration: boolean;
		showLabel: boolean;
		showOptions: boolean;
		playlistUid?: string | null;
		sourceUid?: string | null;
	}>();

	// VARIABLES
	const isGhosted = $derived(isGhostTrack());
	let isSelected = $derived(trackSelection.isSelected(track.uid, viewId));

	function isGhostTrack(): boolean {
		if (track.remote_path && track.remote_path.length > 0) return false;
		if (!track.path || track.path === "" || track.path === track.uid) return true;
		return false;
	}
	
	// FUNCTIONS
	function handleRowClick(e: MouseEvent) {
		if ((e.target as HTMLElement).closest("button")) return;
		selectTrack(track.uid, orderedUids, e, viewId);
	}

	function handleRowDblClick(e: MouseEvent) {
		if ((e.target as HTMLElement).closest("button")) return;
		playTrackByUid(track.uid, sourceUid);
	}

	function handleDragStart(e: DragEvent) {
		if (!e.dataTransfer) return;

		let uids: string[];
		if (trackSelection.isSelected(track.uid, viewId) && trackSelection.count > 1) {
			uids = orderedUids.filter((uid: string) => trackSelection.isSelected(uid, viewId));
		} else {
			uids = [track.uid];
		}

		startDrag({ type: "tracks", uids, sourcePlaylistUid: playlistUid, sourceUid });
		e.dataTransfer.effectAllowed = "move";
		e.dataTransfer.setData("text/plain", uids.join(","));
	}

	function handleDragEnd() {
		endDrag();
	}

	function getTrackNumber(): string {
		if (playlistUid) {
			const idx = uidIndexMap.get(track.uid)
			return idx != null ? (idx + 1).toString() : "#"
		}
		const num = firstAlbum?.track_number
		return num != null ? num.toString() : "#"
	}

	let firstAlbum = $derived(parseAlbumEntries(track.albums)[0] ?? null);

	async function goToAlbumArtist() {
		if (!track.album_artist) return;
		const uid = await invoke<string | null>("get_artist_uid_by_name", { name: track.album_artist });
		if (uid) setSelection(uid);
	}
</script>

<div
	role="row"
	class="grid items-center content-center px-3 border-b cursor-pointer select-none transition-colors {isSelected ? 'bg-primary/15 hover:bg-primary/20' : 'hover:bg-muted/50'} {isGhosted ? 'opacity-50 pointer-events-none cursor-not-allowed' : ''}"
	style="grid-template-columns: {gridTemplate}; height: {compact ? '28px' : '56px'};"
	onclick={handleRowClick}
	ondblclick={handleRowDblClick}
	onkeydown={(e) => { if (e.key === 'Enter') playTrackByUid(track.uid, sourceUid); }}
	draggable="true"
	ondragstart={handleDragStart}
	ondragend={handleDragEnd}
	tabindex="0"
>
	{#if showNumber && showArtwork}
		<span class=" cursor-auto text-sm pointer-events-none">{getTrackNumber()}</span>
	{/if}

	{#if showNumber && !showArtwork}
		<div class="group relative w-full h-full flex items-center justify-start">
			<span class="text-sm pointer-events-none group-hover:opacity-0">{getTrackNumber()}</span>
			<div class="cursor-pointer absolute top-0 -left-3 inset-0 w-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
				{#if player.isPlaying && player.track?.uid === track.uid}
					<Button variant="ghost" size="icon" onclick={() => togglePlay()}>
						<Pause />
					</Button>
				{:else if player.track?.uid === track.uid}
					<Button variant="ghost" size="icon" onclick={() => togglePlay()}>
						<Play />
					</Button>
				{:else}
					<Button variant="ghost" size="icon" onclick={() => playTrackByUid(track.uid, sourceUid)}>
						<Play />
					</Button>
				{/if}
			</div>
		</div>
	{/if}

	{#if !compact && showArtwork}
		<div class="group relative w-full">
			<ArtworkDisplay entity={track} size={30} />
			<div class="cursor-pointer absolute top-0 inset-0 -left-1 w-full h-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
				{#if player.isPlaying && player.track?.uid === track.uid}
					<Button variant="ghost" size="icon" onclick={() => togglePlay()}>
						<Pause />
					</Button>
				{:else if !player.isPlaying && player.track?.uid === track.uid}
					<Button variant="ghost" size="icon" onclick={() => togglePlay()}>
						<Play />
					</Button>
				{:else}
					<Button variant="ghost" size="icon" onclick={() => playTrackByUid(track.uid, sourceUid)}>
						<Play />
					</Button>
				{/if}
			</div>
		</div>
	{/if}

	{#if showTitle}
		{#if compact}
			<div class="min-w-0 flex items-center">
				<span role="button" tabindex="0" onclick={() => setSelection(track.uid)} onkeydown={(e) => { if (e.key === 'Enter') setSelection(track.uid); }} class="text-sm truncate cursor-pointer hover:underline">
					{track.title ?? "Unknown Title"}
				</span>
			</div>
		{:else}
			<div class="flex flex-col min-w-0">
				<div class="min-w-0 flex items-center">
					<span role="button" tabindex="0" onclick={() => setSelection(track.uid)} onkeydown={(e) => { if (e.key === 'Enter') setSelection(track.uid); }} class="text-sm truncate cursor-pointer hover:underline">
						{track.title ?? "Unknown Title"}
					</span>
				</div>
				<div class="min-w-0 flex items-center">
					<span role="button" tabindex="0" onclick={goToAlbumArtist} onkeydown={(e) => { if (e.key === 'Enter') goToAlbumArtist(); }} class="text-xs text-muted-foreground truncate cursor-pointer hover:underline">
						<ArtistsList artists={track.artists} />
					</span>
				</div>
			</div>
		{/if}
	{/if}

	{#if showArtist}
		<span role="button" tabindex="0" onclick={goToAlbumArtist} onkeydown={(e) => { if (e.key === 'Enter') goToAlbumArtist(); }} class="text-sm truncate cursor-pointer hover:underline">
			<ArtistsList artists={track.artists} />
		</span>
	{/if}

	{#if showAlbum && firstAlbum}
		<div class="min-w-0 flex items-center pr-4">
			<span role="button" tabindex="0" onclick={() => { if (firstAlbum?.uid) setSelection(firstAlbum.uid) }} onkeydown={(e) => { if (e.key === 'Enter' && firstAlbum?.uid) setSelection(firstAlbum.uid); }} class="text-sm truncate cursor-pointer hover:underline">
				{firstAlbum?.name ?? "Unknown Album"}
			</span>
		</div>
	{/if}

	{#if showYear}
		<span class="text-sm pointer-events-none">{track.year ?? "—"}</span>
	{/if}

	{#if showRating}
		<TrackRating uid={track.uid} rating={track.rating} tags={track.tags} />
	{/if}

	{#if showDuration}
		<span class="text-sm font-mono pointer-events-none">{formatDuration(track.duration_ms)}</span>
	{/if}

	{#if showLabel}
		<span class="text-sm truncate pointer-events-none">{track.label ?? "—"}</span>
	{/if}

	{#if showOptions}
			<TrackTableEditButton {track} {sourceUid} />
	{/if}
</div>