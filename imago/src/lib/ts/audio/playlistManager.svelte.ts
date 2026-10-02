import { getPlaylist, getTrack, reloadLibrary } from "$ts/store/library.svelte";
import { invoke } from "@tauri-apps/api/core";
import { showWarning } from "$ts/ui/dialogManager.svelte";

import type { Playlist, Track, TrackEntry } from "$ts/util/types";

export function parseTracks(raw: unknown): TrackEntry[] {
	if (!raw) return []
	if (typeof raw === "string") return JSON.parse(raw)
	if (Array.isArray(raw)) return raw
	return []
}

export async function addTrackToPlaylist(playlist: Playlist, track: Track) {
	const current = parseTracks(playlist.tracks)
	if (current.some((t) => t.uid === track.uid)) return

	const maxOrder = current.reduce((m, t) => Math.max(m, t.order ?? 0), 0)
	const updated = [...current, { uid: track.uid!, name: track.title ?? "Unknown Title", order: maxOrder + 1 }]

	await invoke("update_playlist_entry", {
		uid: String(playlist.uid),
		update: { tracks: JSON.stringify(updated) },
	})

	await reloadLibrary("playlists")
}

export async function addTracksToPlaylist(playlistUid: string, trackUids: string[]) {
	const playlist = await getPlaylist(playlistUid)
	if (!playlist) return

	const current = parseTracks(playlist.tracks)
	const existingUids = new Set(current.map((t) => t.uid))
	const toAdd = trackUids.filter((uid) => !existingUids.has(uid))
	if (toAdd.length === 0) return

	let maxOrder = current.reduce((m, t) => Math.max(m, t.order ?? 0), 0)

	const newEntries: TrackEntry[] = await Promise.all(toAdd.map(async (uid) => {
		const track = await getTrack(uid)
		maxOrder += 1
		return { uid, name: track?.title ?? "Unknown Title", order: maxOrder }
	}))

	await invoke("update_playlist_entry", {
		uid: playlistUid,
		update: { tracks: JSON.stringify([...current, ...newEntries]) },
	})

	await reloadLibrary("playlists")
}

export async function removeTrackFromPlaylist(playlist: Playlist, track: Track) {
	const current = parseTracks(playlist.tracks)
	const updated = current.filter((t) => t.uid !== track.uid)

	await invoke("update_playlist_entry", {
		uid: String(playlist.uid),
		update: { tracks: JSON.stringify(updated) },
	})

	await reloadLibrary("playlists")
}

export async function removeTracksFromPlaylist(playlistUid: string, trackUids: string[]) {
	const playlist = await getPlaylist(playlistUid)
	if (!playlist) return

	const toRemove = new Set(trackUids)
	const updated = parseTracks(playlist.tracks).filter((t) => !toRemove.has(t.uid))

	await invoke("update_playlist_entry", {
		uid: playlistUid,
		update: { tracks: JSON.stringify(updated) },
	})

	await reloadLibrary("playlists")
}

export async function reorderPlaylistTracks(playlistUid: string, orderedUids: string[]) {
	const playlist = await getPlaylist(playlistUid)
	if (!playlist) return

	const current = parseTracks(playlist.tracks)
	const byUid = new Map(current.map((t) => [t.uid, t]))

	const updated = orderedUids
		.map((uid, i) => {
			const entry = byUid.get(uid)
			if (!entry) return null
			return { ...entry, order: i + 1 }
		})
		.filter(Boolean)

	await invoke("update_playlist_entry", {
		uid: playlistUid,
		update: { tracks: JSON.stringify(updated) },
	})

	await reloadLibrary("playlists")
}

export async function createPlaylist(
	name: string,
	owner: string | null = null,
	folder: string | null = null,
	tracks: TrackEntry[] | null = null): Promise<string>
{
	let uid = "p-" + crypto.randomUUID();
	await invoke("create_playlist_entry", {
		playlist: {
			uid: uid,
			title: name.trim(),
			description: null,
			owner: owner,
			tracks: tracks ? JSON.stringify(tracks) : null,
			artwork_path: null,
			folder: folder,
			version: 1,
		},
	})

	await reloadLibrary("playlists")
	return uid;
}

export async function deletePlaylist(playlistUid: string) {

	const confirmed = await showWarning({ title: "Delete playlist?", description: "This action cannot be undone",});
	if (!confirmed) return false;

	await invoke("delete_playlist_entry", { uid: playlistUid })
	await reloadLibrary("playlists")
}