import { invoke } from "@tauri-apps/api/core";
import { emitLibraryChange } from "$ts/store/library.svelte";
import { dragState, setHoveredFolder, setDragActive, setDragPayload, endDrag } from "$ts/store/drag.svelte";
import { isDraggingFolderType } from "$ts/drag/dragdrop";
import { registerFolder, removeFolder, renameFolderPaths } from "$ts/ui/folderSelection.svelte";
import { addTracksToPlaylist } from "$ts/audio/playlistManager.svelte";
import { movePlaylists } from "$ts/drag/dragdrop_playlists";
import { getSortedFolders, getDirectPlaylists, compactFolderRows } from "$ts/ui/playlistFolderTree.svelte";
import { playlistOrder } from "$ts/store/playlistOrderStore.svelte";
import type { Playlist } from "$ts/util/types";
import type { PlaylistSortField } from "$ts/ui/playlistFolderTree.svelte";

let _folderHoverTimer: ReturnType<typeof setTimeout> | null = null;
let _folderHoverKey: string | null = null;

let _breadcrumbHoverTimer: ReturnType<typeof setTimeout> | null = null;
let _breadcrumbHoverKey: string = "";

export function resetFolderTimers() {
	_clearFolderTimer();
	_clearBreadcrumbTimer();
}

function _clearFolderTimer() {
	if (_folderHoverTimer !== null) {
		clearTimeout(_folderHoverTimer);
		_folderHoverTimer = null;
	}
	_folderHoverKey = null;
}

function _clearBreadcrumbTimer() {
	if (_breadcrumbHoverTimer !== null) {
		clearTimeout(_breadcrumbHoverTimer);
		_breadcrumbHoverTimer = null;
	}
	_breadcrumbHoverKey = "";
}

export function onFolderDragOver(
	path: string,
	onNavigate: (path: string | null) => void
) {
	if (!dragState.active) return;
	setHoveredFolder(path);

	if (_folderHoverKey === path) return;
	_clearFolderTimer();
	_folderHoverKey = path;
	_folderHoverTimer = setTimeout(() => {
		onNavigate(path);
		_folderHoverKey = null;
		_folderHoverTimer = null;
	}, 700);
}

export function onFolderDragExit(path: string) {
	if (dragState.hoveredFolderPath === path) setHoveredFolder(null);
	if (_folderHoverKey === path) _clearFolderTimer();
}

export function onBreadcrumbDragOver(
	path: string | null,
	onNavigate: (path: string | null) => void
) {
	if (!dragState.active) return;
	const key = path ?? "__root__";

	if (_breadcrumbHoverKey === key) return;
	_clearBreadcrumbTimer();
	_breadcrumbHoverKey = key;
	_breadcrumbHoverTimer = setTimeout(() => {
		onNavigate(path);
		_breadcrumbHoverKey = "";
		_breadcrumbHoverTimer = null;
	}, 700);
}

export function onBreadcrumbDragExit() {
	_clearBreadcrumbTimer();
}

export function startFolderDrag(e: DragEvent, path: string) {
	if (e.dataTransfer) {
		e.dataTransfer.setData("text/plain", path);
		e.dataTransfer.setData("application/x-sonodex-folder", "1");
		e.dataTransfer.effectAllowed = "move";
	}
	setDragActive(true);
	setDragPayload({ type: "tracks", uids: [path], sourcePlaylistUid: null });
}

export async function nestFolder(draggedPath: string, targetPath: string) {
	if (draggedPath === targetPath) return;
	if (targetPath.startsWith(draggedPath + "/")) return;
	const folderName = draggedPath.split("/").at(-1)!;
	const newPath = `${targetPath}/${folderName}`;
	await invoke("rename_playlist_folder", { oldPath: draggedPath, newPath });
	removeFolder(draggedPath);
	registerFolder(newPath);
	emitLibraryChange("playlists:changed");
}

export async function moveFolderToRoot(draggedPath: string) {
	const folderName = draggedPath.split("/").at(-1)!;
	if (draggedPath === folderName) return;
	await invoke("rename_playlist_folder", { oldPath: draggedPath, newPath: folderName });
	removeFolder(draggedPath);
	registerFolder(folderName);
	emitLibraryChange("playlists:changed");
}

export async function renameFolder(oldPath: string, newName: string) {
	const parentPath = oldPath.includes("/")
		? oldPath.split("/").slice(0, -1).join("/")
		: null;
	const newPath = parentPath !== null ? `${parentPath}/${newName}` : newName;
	if (newPath === oldPath) return;
	await invoke("rename_playlist_folder", { oldPath, newPath });
	renameFolderPaths(oldPath, newPath);
	playlistOrder.renameFolderPaths(oldPath, newPath);
	emitLibraryChange("playlists:changed");
}

export async function deleteFolder(
	folderPath: string,
	playlists: Playlist[],
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc"
) {
	const parentPath = folderPath.includes("/")
		? folderPath.split("/").slice(0, -1).join("/")
		: null;

	// move all direct playlists to parent
	const direct = getDirectPlaylists(playlists, folderPath, sortField, sortDir);
	if (direct.length > 0) {
		await movePlaylists(direct.map((p) => p.uid), parentPath);
	}

	// move all immediate subfolders to parent
	const subfolders = getSortedFolders(playlists, folderPath, sortField, sortDir);
	for (const sub of subfolders) {
		const subName = sub.split("/").at(-1)!;
		const newPath = parentPath !== null ? `${parentPath}/${subName}` : subName;
		await invoke("rename_playlist_folder", { oldPath: sub, newPath });
		removeFolder(sub);
		registerFolder(newPath);
	}

	removeFolder(folderPath);
	emitLibraryChange("playlists:changed");
}

async function _deleteFolderRecursive(folderPath: string) {
	const current: any[] = await invoke("get_playlists");
	const direct = current.filter((p: any) => (p.folder ?? "") === folderPath);
	for (const p of direct) {
		await invoke("delete_playlist_entry", { uid: p.uid });
	}
	const subfolders = [...new Set(
		current
			.map((p: any) => p.folder ?? "")
			.filter((f: string) => f.startsWith(folderPath + "/"))
			.map((f: string) => f.slice(folderPath.length + 1).split("/")[0])
			.map((name: string) => `${folderPath}/${name}`)
	)];
	for (const sub of subfolders) {
		await _deleteFolderRecursive(sub);
	}
	removeFolder(folderPath);
}

export async function deleteFolderAndContents(folderPath: string) {
	await _deleteFolderRecursive(folderPath);
	emitLibraryChange("playlists:changed");
}

export async function dropOnFolder(
	e: DragEvent,
	targetPath: string,
	getDirectPlaylistsFn: (folder: string) => { uid: string }[]
) {
	e.preventDefault();
	e.stopPropagation();
	onFolderDragExit(targetPath);
	const raw = e.dataTransfer?.getData("text/plain");
	if (!raw) { endDrag(); return; }

	if (isDraggingFolderType(e)) {
		await nestFolder(raw.trim(), targetPath);
	} else {
		const uids = raw.split(",").map((u) => u.trim()).filter(Boolean);
		if (uids.every((u) => u.startsWith("p-"))) {
			await movePlaylists(uids, targetPath);
		} else {
			const first = getDirectPlaylistsFn(targetPath)[0];
			if (first) await addTracksToPlaylist(first.uid, uids);
		}
	}
	endDrag();
}

export async function dropOnRoot(e: DragEvent) {
	e.preventDefault();
	const raw = e.dataTransfer?.getData("text/plain");
	if (!raw) { endDrag(); return; }

	if (isDraggingFolderType(e)) {
		await moveFolderToRoot(raw.trim());
	} else {
		const uids = raw.split(",").map((u) => u.trim()).filter(Boolean);
		if (uids.every((u) => u.startsWith("p-"))) {
			await movePlaylists(uids, null);
		}
	}
	endDrag();
}

export async function doFolderReorder(
	draggedPath: string,
	targetIndex: number,
	side: "before" | "after",
	playlists: Playlist[],
	currentPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc"
) {
	const current = getSortedFolders(playlists, currentPath, sortField, sortDir);
	const fromIndex = current.indexOf(draggedPath);
	if (fromIndex === -1) {
		await nestFolder(draggedPath, currentPath ?? draggedPath.split("/").slice(0, -1).join("/"));
		const updated = getSortedFolders(playlists, currentPath, sortField, sortDir);
		const newFrom = updated.indexOf(draggedPath);
		if (newFrom === -1) return;
		const insertAt = side === "before" ? targetIndex : targetIndex + 1;
		const without = updated.filter((_, i) => i !== newFrom);
		const adjusted = newFrom < insertAt ? insertAt - 1 : insertAt;
		without.splice(Math.max(0, Math.min(adjusted, without.length)), 0, draggedPath);
		playlistOrder.setFolders(currentPath, without);
		return;
	}
	const insertAt = side === "before" ? targetIndex : targetIndex + 1;
	const without = current.filter((_, i) => i !== fromIndex);
	const adjusted = fromIndex < insertAt ? insertAt - 1 : insertAt;
	without.splice(Math.max(0, Math.min(adjusted, without.length)), 0, draggedPath);
	playlistOrder.setFolders(currentPath, without);
}

export async function doCompactFolderReorder(
	draggedPath: string,
	targetRi: number,
	side: "before" | "after",
	playlists: Playlist[],
	currentPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc",
	expandedFolders: Set<string>
) {
	const rows = compactFolderRows(playlists, currentPath, sortField, sortDir, expandedFolders);
	const targetRow = rows[targetRi];
	if (!targetRow) return;

	const targetParent = targetRow.path.includes("/")
		? targetRow.path.split("/").slice(0, -1).join("/")
		: null;

	const draggedParent = draggedPath.includes("/")
		? draggedPath.split("/").slice(0, -1).join("/")
		: null;

	const draggedName = draggedPath.split("/").at(-1)!;
	const newDraggedPath = targetParent !== null
		? `${targetParent}/${draggedName}`
		: draggedName;

	if (draggedParent !== targetParent) {
		if (targetParent !== null) {
			await nestFolder(draggedPath, targetParent);
		} else {
			await moveFolderToRoot(draggedPath);
		}
	}

	const current = getSortedFolders(playlists, targetParent, sortField, sortDir);
	const fromIndex = current.indexOf(newDraggedPath);
	const toIndex = current.indexOf(targetRow.path);
	if (fromIndex === -1 || toIndex === -1) return;

	const insertAt = side === "before" ? toIndex : toIndex + 1;
	const without = current.filter((_, i) => i !== fromIndex);
	const adjusted = fromIndex < insertAt ? insertAt - 1 : insertAt;
	without.splice(Math.max(0, Math.min(adjusted, without.length)), 0, newDraggedPath);
	playlistOrder.setFolders(targetParent, without);
}