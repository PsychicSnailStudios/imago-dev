import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { emitLibraryChange, emitSingleChange } from "$ts/store/library.svelte";
import { showWarning } from "$ts/ui/dialogManager.svelte";
import type { DuplicateGroup } from "$ts/util/types";

// ─── Track Actions ───────────────────────────────────────────────────────────

export async function removeTrackFromLibrary(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Track?", description: "This action cannot be undone" });
	if (!confirmed) return;
	await invoke("remove_track_from_library", { uid });
	emitLibraryChange("tracks:changed");
}

export async function removeTracksFromLibrary(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Tracks?`, description: "This action cannot be undone" });
	if (!confirmed) return;
	for (const uid of uids) {
		await invoke("remove_track_from_library", { uid });
	}
	emitLibraryChange("tracks:changed");
}

export async function replaceTrackPath(uid: string): Promise<void> {
	const selected = await open({
		multiple: false,
		filters: [{ name: "Audio", extensions: ["mp3", "flac", "m4a", "aac", "wav", "aiff", "ogg"] }],
	});
	if (!selected) return;
	const newPath = typeof selected === "string" ? selected : selected[0];
	await invoke("replace_track_path", { uid, newPath });
	emitSingleChange(uid);
}

export async function openTrackInExplorer(path: string): Promise<void> {
	await invoke("open_in_explorer", { path });
}

// ─── Album Actions ───────────────────────────────────────────────────────────

async function removeAlbumFromLinkedTracks(albumUid: string): Promise<void> {
	const allTracks = await invoke<any[]>("get_tracks");
	for (const track of allTracks) {
		let entries: any[];
		try {
			entries = track.albums ? JSON.parse(track.albums) : [];
		} catch {
			entries = [];
		}
		const filtered = entries.filter((e: any) => e.uid !== albumUid);
		if (filtered.length === entries.length) continue;
		await invoke("update_track_metadata", {
			uid: track.uid,
			update: { albums: JSON.stringify(filtered) },
		});
	}
}

export async function removeAlbum(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Album?", description: "This action cannot be undone" });
	if (!confirmed) return;
	await removeAlbumFromLinkedTracks(uid);
	await invoke("delete_album_entry", { uid });
	emitLibraryChange("albums:changed");
}

export async function removeAlbums(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Albums?`, description: "This action cannot be undone" });
	if (!confirmed) return;
	for (const uid of uids) {
		await removeAlbumFromLinkedTracks(uid);
		await invoke("delete_album_entry", { uid });
	}
	emitLibraryChange("albums:changed");
}

// ─── Artist Actions ──────────────────────────────────────────────────────────

export async function removeArtist(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Artist?", description: "This action cannot be undone" });
	if (!confirmed) return;
	await invoke("delete_artist_entry", { uid });
	emitLibraryChange("artists:changed");
}

export async function removeArtists(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Artists?`, description: "This action cannot be undone" });
	if (!confirmed) return;
	for (const uid of uids) {
		await invoke("delete_artist_entry", { uid });
	}
	emitLibraryChange("artists:changed");
}

// ─── Duplicate Actions ───────────────────────────────────────────────────────

export async function getDuplicates(): Promise<DuplicateGroup[]> {
	return await invoke<DuplicateGroup[]>("get_duplicates");
}

export async function keepTrack(keepUid: string, group: DuplicateGroup): Promise<void> {
	const toRemove = group.tracks.filter((t) => t.uid !== keepUid);
	for (const track of toRemove) {
		await invoke("remove_track_from_library", { uid: track.uid });
	}
	emitLibraryChange("tracks:changed");
}

export async function deleteTrackFile(uid: string, path: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Track?", description: "This action cannot be undone" });
	if (!confirmed) return;
	await invoke("delete_track_file", { uid, path });
	emitLibraryChange("tracks:changed");
}

export async function mergeKeepFirst(group: DuplicateGroup): Promise<void> {
	if (group.tracks.length < 2) return;
	await keepTrack(group.tracks[0].uid, group);
}

export async function mergeRemoteLocal(group: DuplicateGroup): Promise<void> {
	const local = group.tracks.find((t) => t.path && t.path.length > 0);
	const remote = group.tracks.find((t) => (!t.path || t.path.length === 0) && t.remote_path && t.remote_path.length > 0);
	if (!local || !remote) return;
	await invoke("merge_remote_local_tracks", { keepUid: local.uid, dropUid: remote.uid });
	emitLibraryChange("tracks:changed");
}