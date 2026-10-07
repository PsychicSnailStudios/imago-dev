<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import DefultPlaylistArt from "$lib/components/pages/playlist/DefultPlaylistArt.svelte";

	import { getAlbum, getArtist, getPlaylist, getTrack, getTrackArrayFromUID } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { scrobbleSignal } from "$ts/audio/scrobbler.svelte";
	import { parseAlbumEntries, parseUidType } from "$ts/util/parsers";
	import type { Album, Artist, Playlist } from "$ts/util/types";

	type Scrobble = {
		uid: string;
		timestamp: number;
		track_uid: string;
		artist_uid: string;
		duration_played: number;
		did_seek: boolean;
		did_pause: boolean;
		track_name: string | null;
		track_artist: string | null;
		track_album: string | null;
		shuffle: boolean | null;
		skipped: boolean | null;
		offline: boolean | null;
		playing_local: boolean | null;
		source_uid: string | null;
	};

	type ResumeItem = {
		uid: string;
		type: "album" | "playlist" | "artist";
		title: string;
		subtitle: string;
		entity: Album | Playlist | Artist;
	};

	const RESUME_LIMIT = 8;
	const SCROBBLE_SCAN_LIMIT = 300;
	const MIN_DURATION_MS = 20_000;

	let recentItems = $state<ResumeItem[]>([]);
	let loading = $state(true);

	async function refresh() {
		try {
			recentItems = await loadRecentItems();
		} catch (e) {
			console.error("Jump Back In failed to load", e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		scrobbleSignal.version;
		refresh();
	});

	async function resolveItem(uid: string): Promise<ResumeItem | null> {
		const type = parseUidType(uid);

		if (type === "album") {
			const album = await getAlbum(uid);
			if (!album) return null;
			return {
				uid,
				type,
				title: album.title,
				subtitle: album.album_artist ? String(album.album_artist) : "Album",
				entity: album,
			};
		}

		if (type === "playlist") {
			const playlist = await getPlaylist(uid);
			if (!playlist) return null;
			return {
				uid,
				type,
				title: playlist.title,
				subtitle: playlist.owner ?? "Playlist",
				entity: playlist,
			};
		}

		if (type === "artist") {
			const artist = await getArtist(uid);
			if (!artist) return null;
			return {
				uid,
				type,
				title: artist.name,
				subtitle: "Artist",
				entity: artist,
			};
		}

		return null;
	}

	async function loadRecentItems(): Promise<ResumeItem[]> {
		const scrobbles = await invoke<Scrobble[]>("get_recent_scrobbles", { limit: SCROBBLE_SCAN_LIMIT });
		const trackAlbumCache = new Map<string, string | null>();
		const seen = new Set<string>();
		const items: ResumeItem[] = [];

		for (const s of scrobbles.slice(0, SCROBBLE_SCAN_LIMIT)) {
			if (items.length >= RESUME_LIMIT) break;
			if (!s.track_uid || s.duration_played < MIN_DURATION_MS) continue;

			let uid: string | null = s.source_uid || null;

			if (!uid) {
				let albumUid = trackAlbumCache.get(s.track_uid);
				if (albumUid === undefined) {
					const track = await getTrack(s.track_uid);
					albumUid = parseAlbumEntries(track?.albums ?? null)[0]?.uid ?? null;
					trackAlbumCache.set(s.track_uid, albumUid);
				}
				uid = albumUid;
			}

			if (!uid || seen.has(uid)) continue;
			seen.add(uid);

			const item = await resolveItem(uid);
			if (item) items.push(item);
		}

		return items;
	}
</script>

<div class="flex flex-col gap-3">

	<div class="flex items-center justify-between gap-2 flex-wrap shrink-0">
		<h2 class="text-lg font-semibold tracking-tight">Jump Back In</h2>
	</div>

	{#if loading}
		<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
			{#each Array(RESUME_LIMIT) as _}
				<div class="flex items-center gap-3 p-2 rounded-md bg-muted/50 animate-pulse">
					<div class="w-12 h-12 rounded-sm bg-muted/60 shrink-0"></div>
					<div class="flex flex-col gap-1 flex-1 min-w-0">
						<div class="h-3 w-3/4 rounded bg-muted/60"></div>
						<div class="h-2 w-1/2 rounded bg-muted/60"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if recentItems.length === 0}
		<p class="text-xs text-muted-foreground text-center py-8">Nothing played recently yet.</p>
	{:else}
		<div class="app-music-grid grid gap-3 pr-4 pl-4 pb-4">
			{#each recentItems as item (item.uid)}
				<button
					onclick={() => setSelection(item.uid)}
					class="flex items-center gap-3 cursor-pointer p-2 rounded-md bg-muted/50 hover:bg-muted text-left">
					<ArtworkDisplay entity={item.entity} size={48}>
						{#if item.type === "playlist"}
							<DefultPlaylistArt tracks={getTrackArrayFromUID(item.uid)} />
						{/if}
					</ArtworkDisplay>
					<div class="flex flex-col min-w-0">
						<span class="text-sm font-medium truncate">{item.title}</span>
						<span class="text-xs text-muted-foreground truncate">{item.subtitle}</span>
					</div>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
.app-music-grid {
	grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
}
</style>