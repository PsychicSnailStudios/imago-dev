<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";

	import { getAlbum, getTrack, library } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import type { Track, Album } from "$ts/util/types";
    import { parseArtistsToString } from "$ts/util/parsers";

	let { artistUid, artistName } = $props<{ artistUid: string; artistName: string }>();

	type Scrobble = {
		uid: string;
		track_uid: string;
		artist_uid: string;
		duration_played: number;
	};

	type TopTrack = { track: Track; plays: number };
	type TopAlbum = { album: Album; plays: number };

	type Tab = "tracks" | "albums";

	const MIN_DURATION_MS = 30_000;

	let activeTab = $state<Tab>("tracks");
	let topTracks = $state<TopTrack[]>([]);
	let topAlbums = $state<TopAlbum[]>([]);
	let loading = $state(true);

	async function load() {
		loading = true;
		try {
			const scrobbles = await invoke<Scrobble[]>("get_scrobbles");

			const relevant = scrobbles.filter(
				s => s.artist_uid === artistUid && s.duration_played >= MIN_DURATION_MS
			);

			const trackMap = new Map<string, number>();
			for (const s of relevant) {
				if (!s.track_uid) continue;
				trackMap.set(s.track_uid, (trackMap.get(s.track_uid) ?? 0) + 1);
			}

			topTracks = [...trackMap.entries()]
				.sort((a, b) => b[1] - a[1])
				.slice(0, 10)
				.flatMap(([uid, plays]) => {
					const track = getTrack(uid);
					if (!track) return [];
					return [{ track, plays }];
				});

			const albumMap = new Map<string, number>();
			for (const s of relevant) {
				const track = library.tracks.find(t => t.uid === s.track_uid);
				if (!track) continue;
				let albumUid = "";
				try {
					const albums = JSON.parse(track.albums ?? "[]");
					albumUid = albums[0]?.uid ?? "";
				} catch {}
				if (!albumUid) continue;
				albumMap.set(albumUid, (albumMap.get(albumUid) ?? 0) + 1);
			}

			topAlbums = [...albumMap.entries()]
				.sort((a, b) => b[1] - a[1])
				.slice(0, 10)
				.flatMap(([uid, plays]) => {
					const album = getAlbum(uid);
					if (!album) return [];
					return [{ album, plays }];
				});
		} catch (e) {
			console.error("ArtistTopTracks failed", e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		artistUid;
		load();
	});
</script>

<div class="flex flex-col gap-2">
	<div class="flex items-center justify-between">
		<div class="flex gap-1">
			<button
				onclick={() => activeTab = "tracks"}
				class="px-2.5 py-1 rounded text-xs font-medium transition-colors
					{activeTab === 'tracks' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}"
			>Top Tracks</button>
			<button
				onclick={() => activeTab = "albums"}
				class="px-2.5 py-1 rounded text-xs font-medium transition-colors
					{activeTab === 'albums' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}"
			>Top Albums</button>
		</div>
	</div>

	{#if loading}
		<div class="flex flex-col gap-1">
			{#each Array(5) as _}
				<div class="flex gap-2 p-1.5 items-center animate-pulse">
					<div class="rounded shrink-0 bg-muted" style="width:32px;height:32px;"></div>
					<div class="flex flex-col gap-1 flex-1 min-w-0">
						<div class="h-3 w-2/3 rounded bg-muted"></div>
						<div class="h-2 w-1/3 rounded bg-muted"></div>
					</div>
				</div>
			{/each}
		</div>

	{:else if activeTab === "tracks"}
		{#if topTracks.length === 0}
			<p class="text-xs text-muted-foreground py-3">No play history for this artist yet.</p>
		{:else}
			{@const maxPlays = topTracks[0].plays}
			<div class="flex flex-col gap-0.5">
				{#each topTracks as { track, plays }, i}
					<div class="relative flex items-center gap-2 p-1.5 rounded-md hover:bg-muted/40 transition-colors overflow-hidden">
						<div
							class="absolute inset-0 bg-primary/5 rounded-md"
							style="width: {Math.round((plays / maxPlays) * 100)}%;"
						></div>
						<span class="relative text-xs text-muted-foreground w-4 shrink-0 text-right tabular-nums">{i + 1}</span>
						<div class="relative shrink-0">
							<ArtworkDisplay entity={track} size={32} />
						</div>
						<div class="relative min-w-0 flex-1 grid">
							<button
								onclick={() => setSelection(track.uid)}
								class="text-sm truncate text-left hover:underline font-medium"
							>{track.title ?? "Unknown"}</button>
							<span class="text-xs text-muted-foreground truncate">{parseArtistsToString(track.artists)}</span>
						</div>
						<span class="relative text-xs text-muted-foreground tabular-nums shrink-0">
							{plays} {plays === 1 ? "play" : "plays"}
						</span>
					</div>
				{/each}
			</div>
		{/if}

	{:else}
		{#if topAlbums.length === 0}
			<p class="text-xs text-muted-foreground py-3">No play history for this artist yet.</p>
		{:else}
			{@const maxPlays = topAlbums[0].plays}
			<div class="flex flex-col gap-0.5">
				{#each topAlbums as { album, plays }, i}
					<div class="relative flex items-center gap-2 p-1.5 rounded-md hover:bg-muted/40 transition-colors overflow-hidden">
						<div
							class="absolute inset-0 bg-primary/5 rounded-md"
							style="width: {Math.round((plays / maxPlays) * 100)}%;"
						></div>
						<span class="relative text-xs text-muted-foreground w-4 shrink-0 text-right tabular-nums">{i + 1}</span>
						<div class="relative shrink-0">
							<ArtworkDisplay entity={album} size={32} />
						</div>
						<div class="relative min-w-0 flex-1 grid">
							<button
								onclick={() => setSelection(album.uid)}
								class="text-sm truncate text-left hover:underline font-medium"
							>{album.title}</button>
							<span class="text-xs text-muted-foreground truncate">{album.album_artist ?? ""}</span>
						</div>
						<span class="relative text-xs text-muted-foreground tabular-nums shrink-0">
							{plays} {plays === 1 ? "play" : "plays"}
						</span>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>
