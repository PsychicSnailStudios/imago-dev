import { getPlaylists } from "$ts/store/library.svelte";

let currentPath = $state<string | null>(null);
let _knownFolders = $state<Set<string>>(new Set());
let _seeded = false;

async function _seedFromLibrary() {
	if (_seeded) return;
	_seeded = true;
	const playlists = await getPlaylists();
	for (const p of playlists) {
		const f = (p as any).folder;
		if (f && f !== "") _knownFolders = new Set([..._knownFolders, f]);
	}
}

export const folderSelection = {
	get currentPath() { return currentPath; },
	get knownFolders(): Set<string> {
		return _knownFolders;
	},
	async init() {
		await _seedFromLibrary();
	},
};

export function registerFolder(path: string) {
	_knownFolders = new Set([..._knownFolders, path]);
}

export function removeFolder(path: string) {
	const next = new Set(_knownFolders);
	for (const k of next) {
		if (k === path || k.startsWith(path + "/")) next.delete(k);
	}
	_knownFolders = next;
}

function remapPath(path: string, oldPath: string, newPath: string): string {
	if (path === oldPath) return newPath;
	if (path.startsWith(oldPath + "/")) return newPath + path.slice(oldPath.length);
	return path;
}

export function renameFolderPaths(oldPath: string, newPath: string) {
	_knownFolders = new Set([..._knownFolders].map((k) => remapPath(k, oldPath, newPath)));
	if (currentPath !== null) currentPath = remapPath(currentPath, oldPath, newPath);
}

export function navigateTo(path: string | null) {
	currentPath = path;
}

export function navigateInto(segment: string) {
	currentPath = currentPath ? `${currentPath}/${segment}` : segment;
}

export function breadcrumbs(): { label: string; path: string | null }[] {
	const crumbs: { label: string; path: string | null }[] = [
		{ label: "Home", path: null },
	];
	if (!currentPath) return crumbs;
	let acc = "";
	for (const part of currentPath.split("/")) {
		acc = acc ? `${acc}/${part}` : part;
		crumbs.push({ label: part, path: acc });
	}
	return crumbs;
}