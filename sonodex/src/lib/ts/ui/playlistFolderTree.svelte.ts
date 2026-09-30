import { folderSelection } from "$ts/ui/folderSelection.svelte";
import { playlistOrder, applyCustomOrder } from "$ts/store/playlistOrderStore.svelte";
import type { Playlist } from "$ts/util/types";

export type PlaylistSortField = "title" | "date_created" | "custom";

export type CompactRow =
	| { kind: "folder"; path: string; depth: number }
	| { kind: "playlist"; playlist: Playlist; depth: number; folderPath: string | null };

export function folderOf(p: Playlist): string | null {
	const f = (p as any).folder;
	return f && f !== "" ? f : null;
}

export function folderLabel(fullPath: string): string {
	return fullPath.split("/").at(-1) ?? fullPath;
}

function immediateChild(parentPath: string | null, folderPath: string): string | null {
	if (parentPath === null) return folderPath.split("/")[0];
	if (!folderPath.startsWith(parentPath + "/")) return null;
	return folderPath.slice(parentPath.length + 1).split("/")[0];
}

export function getChildFoldersRaw(
	playlists: Playlist[],
	parentPath: string | null
): string[] {
	const seen = new Set<string>();
	for (const p of playlists) {
		const pf = folderOf(p);
		if (!pf) continue;
		const child = immediateChild(parentPath, pf);
		if (child) seen.add(parentPath === null ? child : `${parentPath}/${child}`);
	}
	for (const known of folderSelection.knownFolders) {
		const child = immediateChild(parentPath, known);
		if (child) seen.add(parentPath === null ? child : `${parentPath}/${child}`);
	}
	return [...seen];
}

export function getSortedFolders(
	playlists: Playlist[],
	parentPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc"
): string[] {
	const raw = getChildFoldersRaw(playlists, parentPath);
	if (sortField === "custom") {
		return applyCustomOrder(raw, playlistOrder.getFolders(parentPath), (f) => f);
	}
	return [...raw].sort((a, b) => {
		const la = a.split("/").at(-1) ?? a;
		const lb = b.split("/").at(-1) ?? b;
		return sortDir === "asc" ? la.localeCompare(lb) : lb.localeCompare(la);
	});
}

export function getDirectPlaylists(
	playlists: Playlist[],
	folderPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc"
): Playlist[] {
	const direct = playlists.filter((p) => folderOf(p) === folderPath);
	if (sortField === "custom") {
		return applyCustomOrder(direct, playlistOrder.getPlaylists(folderPath), (p) => p.uid);
	}
	return [...direct].sort((a, b) => {
		if (sortField === "title") {
			return sortDir === "asc"
				? a.title.localeCompare(b.title)
				: b.title.localeCompare(a.title);
		}
		if (sortField === "date_created") {
			const ai = (a as any).id ?? 0;
			const bi = (b as any).id ?? 0;
			return sortDir === "asc" ? ai - bi : bi - ai;
		}
		return 0;
	});
}

export function getFolderArtworkUids(playlists: Playlist[], folderPath: string): string[] {
	return playlists
		.filter((p) => {
			const pf = folderOf(p);
			return pf === folderPath || (pf?.startsWith(folderPath + "/") ?? false);
		})
		.map((p) => p.uid)
		.slice(0, 4);
}

export function compactRows(
	playlists: Playlist[],
	parentPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc",
	expandedFolders: Set<string>,
	depth: number = 0
): CompactRow[] {
	const result: CompactRow[] = [];
	for (const fp of getSortedFolders(playlists, parentPath, sortField, sortDir)) {
		result.push({ kind: "folder", path: fp, depth });
		if (expandedFolders.has(fp)) {
			result.push(...compactRows(playlists, fp, sortField, sortDir, expandedFolders, depth + 1));
		}
	}
	for (const p of getDirectPlaylists(playlists, parentPath, sortField, sortDir)) {
		result.push({ kind: "playlist", playlist: p, depth, folderPath: parentPath });
	}
	return result;
}

export function compactFolderRows(
	playlists: Playlist[],
	parentPath: string | null,
	sortField: PlaylistSortField,
	sortDir: "asc" | "desc",
	expandedFolders: Set<string>,
	depth: number = 0
): { path: string; depth: number }[] {
	const result: { path: string; depth: number }[] = [];
	for (const fp of getSortedFolders(playlists, parentPath, sortField, sortDir)) {
		result.push({ path: fp, depth });
		if (expandedFolders.has(fp)) {
			result.push(...compactFolderRows(playlists, fp, sortField, sortDir, expandedFolders, depth + 1));
		}
	}
	return result;
}