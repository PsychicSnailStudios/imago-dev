import { invoke } from "@tauri-apps/api/core";
import { setSelection } from "$ts/store/session.svelte";
import { parseAlbumEntries } from "$ts/util/parsers";
import type { Track } from "$ts/util/types";

type ArtistRef = { name: string; uid: string | null };

function refFromValue(value: unknown): ArtistRef | null {
	if (typeof value === "string") return value ? { name: value, uid: null } : null;
	if (value && typeof value === "object") {
		const obj = value as { name?: unknown; uid?: unknown };
		const name = typeof obj.name === "string" ? obj.name : "";
		const uid = typeof obj.uid === "string" && obj.uid ? obj.uid : null;
		if (name || uid) return { name, uid };
	}
	return null;
}

function firstArtistRef(raw: string | null): ArtistRef | null {
	if (!raw) return null;
	try {
		const parsed = JSON.parse(raw);
		if (Array.isArray(parsed)) return refFromValue(parsed[0]);
		return refFromValue(parsed);
	} catch {
		return { name: raw, uid: null };
	}
}

export function goToTrackAlbum(track: Track, sourceUid: string | null = null) {
	const entries = parseAlbumEntries(track.albums);
	const match = sourceUid ? entries.find((e) => e.uid === sourceUid) : undefined;
	const entry = match ?? entries.find((e) => e.uid) ?? null;
	if (entry?.uid) setSelection(entry.uid);
}

export async function goToTrackArtist(track: Track) {
	const ref = firstArtistRef(track.artists) ?? firstArtistRef(track.album_artist);
	if (!ref) return;
	if (ref.uid) {
		setSelection(ref.uid);
		return;
	}
	if (!ref.name) return;
	const uid = await invoke<string | null>("get_artist_uid_by_name", { name: ref.name });
	if (uid) setSelection(uid);
}
