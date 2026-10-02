import { invoke } from "@tauri-apps/api/core";
import { SortState } from "$ts/util/sortConfig.svelte";
import { parseUidType, parseAlbumEntries } from "$ts/util/parsers";
import { loadTags } from '$ts/store/tagManager.svelte';

import type { Track, Album, Artist, Playlist, Lyrics, Library } from "$ts/util/types";

// ─── Event Bus ────────────────────────────────────────────────────────────────

export type LibraryChangeEvent =
	| "tracks:changed"
	| "albums:changed"
	| "artists:changed"
	| "playlists:changed"
	| "lyrics:changed"
	| "libraries:changed";

export type SingleChangeEvent = { event: "single:changed"; uid: string };

type ChangeCallback = () => void;
type SingleChangeCallback = (uid: string) => void;

const _listeners = new Map<LibraryChangeEvent, Set<ChangeCallback>>();
const _singleListeners = new Set<SingleChangeCallback>();

export function onLibraryChange(event: LibraryChangeEvent, callback: ChangeCallback): () => void {
	let set = _listeners.get(event);
	if (!set) { set = new Set(); _listeners.set(event, set); }
	set.add(callback);
	return () => set!.delete(callback);
}

export function onSingleChange(callback: SingleChangeCallback): () => void {
	_singleListeners.add(callback);
	return () => _singleListeners.delete(callback);
}

export function emitLibraryChange(event: LibraryChangeEvent): void {
	_listeners.get(event)?.forEach(cb => cb());
}

export function emitSingleChange(uid: string): void {
	_singleListeners.forEach(cb => cb(uid));
}

// ─── Library loaded flag + registry state ────────────────────────────────────

let _loaded = $state(false);

export const library = {
	get loaded() { return _loaded; },
	set loaded(v) { _loaded = v; },
};

export const libraryStore = $state({
	libraries: [] as Library[],
	loaded: false,
});

export async function loadLibraryRegistry(): Promise<void> {
	libraryStore.libraries = await invoke<Library[]>("get_libraries");
	libraryStore.loaded = true;
}

export function getLibraryByUid(uid: string): Library | null {
	return libraryStore.libraries.find(l => l.uid === uid) ?? null;
}

// ─── Load ─────────────────────────────────────────────────────────────────────

export async function loadLibrary(): Promise<void> {
	await loadTags();
	library.loaded = true;

	if (!libraryStore.loaded) {
		await loadLibraryRegistry();
	}
}

// reloadLibrary kept for call-site compatibility; emits the relevant event so
// subscribers re-query. Artwork prefetch has moved to view-level on demand.
export async function reloadLibrary(
	type: "tracks" | "all" | "tags" | "albums" | "artists" | "playlists" | "lyrics"
): Promise<void> {
	switch (type) {
		case "tracks":    emitLibraryChange("tracks:changed");    break;
		case "albums":    emitLibraryChange("albums:changed");    break;
		case "artists":   emitLibraryChange("artists:changed");   break;
		case "playlists": emitLibraryChange("playlists:changed"); break;
		case "lyrics":    emitLibraryChange("lyrics:changed");    break;
		case "tags":      await loadTags();                       break;
		case "all":
			emitLibraryChange("tracks:changed");
			emitLibraryChange("albums:changed");
			emitLibraryChange("artists:changed");
			emitLibraryChange("playlists:changed");
			emitLibraryChange("lyrics:changed");
			emitLibraryChange("libraries:changed");
			break;
	}
}

export async function reloadSingle(uid: string): Promise<void> {
	emitSingleChange(uid);
}

// ─── Direct DB query functions ────────────────────────────────────────────────

export async function getTracks(): Promise<Track[]> {
	return invoke<Track[]>("get_tracks");
}

export async function getAlbums(): Promise<Album[]> {
	return invoke<Album[]>("get_albums");
}

export async function getArtists(): Promise<Artist[]> {
	return invoke<Artist[]>("get_artists");
}

export async function getPlaylists(): Promise<Playlist[]> {
	return invoke<Playlist[]>("get_playlists");
}

export async function getAllLyrics(): Promise<Lyrics[]> {
	return invoke<Lyrics[]>("get_all_lyrics");
}

export async function getTrack(trackUid: string): Promise<Track | null> {
	return invoke<Track | null>("get_track", { uid: trackUid });
}

export async function getAlbum(albumUid: string): Promise<Album | null> {
	return invoke<Album | null>("get_album", { uid: albumUid });
}

export async function getArtist(artistUid: string): Promise<Artist | null> {
	return invoke<Artist | null>("get_artist", { uid: artistUid });
}

export async function getPlaylist(playlistUid: string): Promise<Playlist | null> {
	return invoke<Playlist | null>("get_playlist", { uid: playlistUid });
}

export async function getLyrics(trackUid: string): Promise<Lyrics | null> {
	return invoke<Lyrics | null>("get_track_lyrics", { uid: trackUid });
}

export async function getArtistAlbums(artistUid: string, akaUids?: string[]): Promise<Album[]> {
	return invoke<Album[]>("get_artist_albums", { artistUid, akaUids: akaUids ?? [] });
}

// ─── Search (delegated to backend) ───────────────────────────────────────────

export async function searchTracks(query: string): Promise<Track[]> {
	if (query.trim().length < 2) return getTracks();
	return invoke<Track[]>("search_tracks", { query });
}

export async function searchAlbums(query: string): Promise<Album[]> {
	if (query.trim().length < 2) return getAlbums();
	return invoke<Album[]>("search_albums", { query });
}

export async function searchArtists(query: string): Promise<Artist[]> {
	if (query.trim().length < 2) return getArtists();
	return invoke<Artist[]>("search_artists", { query });
}

// ─── Track array helpers ──────────────────────────────────────────────────────

const parsedTracksCache = new Map<string, { raw: string; entries: Array<{ uid: string; track_number?: number; order?: number }> }>();

export async function getTrackArrayFromUID(uid: string, sort?: SortState): Promise<Track[]> {
	const type = parseUidType(uid);
	const isAlbum = type === "album";
	if (!isAlbum && type !== "playlist") return [];

	const container = isAlbum
		? await getAlbum(uid)
		: await getPlaylist(uid);
	if (!container?.tracks) return [];

	const raw = container.tracks;
	let cached = parsedTracksCache.get(uid);
	if (!cached || cached.raw !== raw) {
		cached = { raw, entries: JSON.parse(raw) };
		parsedTracksCache.set(uid, cached);
	}
	const entries = cached.entries;
	if (entries.length === 0) return [];

	const trackUids = entries.map(e => e.uid);
	const metaMap = new Map<string, number>();
	for (const entry of entries) {
		metaMap.set(entry.uid, isAlbum ? (entry.track_number ?? Infinity) : (entry.order ?? Infinity));
	}

	const tracks = await getTrackArray(trackUids);
	const activeSort = sort ?? new SortState("number", "asc");

	return sortTracks(
		tracks,
		activeSort,
		isAlbum ? metaMap : undefined,
		!isAlbum ? metaMap : undefined
	);
}

export async function getTrackArray(trackUids: string[]): Promise<Track[]> {
	return invoke<Track[]>("get_tracks_by_uids", { uids: trackUids });
}

function sortTracks(
	tracks: Track[],
	sort: SortState,
	trackNumberMap?: Map<string, number | null>,
	orderMap?: Map<string, number>
): Track[] {
	const dir = sort.direction === "desc" ? -1 : 1;
	const field = sort.field;

	function getValue(track: Track): string | number {
		switch (field) {
			case "number":
				return (trackNumberMap ? trackNumberMap.get(track.uid) : orderMap?.get(track.uid)) ?? Infinity;
			case "title":
				return track.title ?? "";
			case "artist":
				return track.album_artist ?? "";
			case "album":
				return parseAlbumEntries(track.albums)[0]?.name ?? "";
			case "year":
				return track.year ?? "";
			case "duration":
				return track.duration_ms ?? 0;
			case "rating":
				return track.rating ?? 0;
			case "label":
				return track.label ?? "";
			default:
				return (orderMap ? orderMap.get(track.uid) : trackNumberMap?.get(track.uid)) ?? Infinity;
		}
	}

	const indices = Array.from({ length: tracks.length }, (_, i) => i);
	indices.sort((ai, bi) => {
		const a = getValue(tracks[ai]);
		const b = getValue(tracks[bi]);
		let cmp: number;
		if (typeof a === "number" && typeof b === "number") {
			cmp = a - b;
		} else {
			cmp = String(a).localeCompare(String(b));
		}
		return cmp * dir;
	});
	return indices.map(i => tracks[i]);
}