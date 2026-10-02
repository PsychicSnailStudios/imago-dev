import { invoke } from "@tauri-apps/api/core";
import { emitLibraryChange } from "$ts/store/library.svelte";
import { setHoveredPlaylist, setDragActive, setDragPayload, endDrag } from "$ts/store/drag.svelte";
import { addTracksToPlaylist } from "$ts/audio/playlistManager.svelte";
import { getDirectPlaylists } from "$ts/ui/playlistFolderTree.svelte";
import { playlistOrder } from "$ts/store/playlistOrderStore.svelte";
import type { Playlist } from "$ts/util/types";
import type { PlaylistSortField } from "$ts/ui/playlistFolderTree.svelte";

export function startPlaylistDrag(e: DragEvent, uid: string) {
	if (e.dataTransfer) {
		e.dataTransfer.setData("text/plain", uid);
		e.dataTransfer.effectAllowed = "move";
	}
	setDragActive(true);
	setDragPayload({ type: "tracks", uids: [uid], sourcePlaylistUid: null });
}

export async function dropOnPlaylist(e: DragEvent, playlistUid: string) {
	e.preventDefault();
	setHoveredPlaylist(null);
	const raw = e.dataTransfer?.getData("text/plain");
	if (!raw) return;
	const uids = raw.split(",").map((u) => u.trim()).filter(Boolean);
	if (uids.every((u) => u.startsWith("p-"))) {
		endDrag();
		return;
	}
	await addTracksToPlaylist(playlistUid, uids);
	endDrag();
}

export async function movePlaylists(playlistUids: string[], targetFolder: string | null) {
	for (const uid of playlistUids) {
		await invoke("update_playlist_entry", {
			uid,
			update: { folder: targetFolder ?? "" },
		});
	}
	emitLibraryChange("playlists:changed");
}

export async function doPlaylistReorder(
	draggedUid: string,
	targetIndex: number,
	side: "before" | "after",
	playlists: Playlist[],
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc",
	targetFolderPath: string | null
) {
	const current = getDirectPlaylists(playlists, targetFolderPath, sortField, sortDir);
	const fromIndex = current.findIndex((p) => p.uid === draggedUid);
	if (fromIndex === -1) {
		await movePlaylists([draggedUid], targetFolderPath);
		const updated = getDirectPlaylists(playlists, targetFolderPath, sortField, sortDir);
		const newFrom = updated.findIndex((p) => p.uid === draggedUid);
		if (newFrom === -1) return;
		const insertAt = side === "before" ? targetIndex : targetIndex + 1;
		const uids = updated.map((p) => p.uid);
		uids.splice(newFrom, 1);
		const adjusted = newFrom < insertAt ? insertAt - 1 : insertAt;
		uids.splice(Math.max(0, Math.min(adjusted, uids.length)), 0, draggedUid);
		playlistOrder.setPlaylists(targetFolderPath, uids);
		return;
	}
	const insertAt = side === "before" ? targetIndex : targetIndex + 1;
	const uids = current.map((p) => p.uid);
	uids.splice(fromIndex, 1);
	const adjusted = fromIndex < insertAt ? insertAt - 1 : insertAt;
	uids.splice(Math.max(0, Math.min(adjusted, uids.length)), 0, draggedUid);
	playlistOrder.setPlaylists(targetFolderPath, uids);
}