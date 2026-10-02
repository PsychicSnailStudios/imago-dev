import { getTracks } from "$ts/store/library.svelte";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { Track } from "$ts/util/types";

type SelectionContext = "library" | "playlist";

let selectedUids = $state<Set<string>>(new Set());
let lastClickedUid = $state<string | null>(null);
let context = $state<SelectionContext>("library");
let contextUid = $state<string | null>(null);

let ownerViewId = $state<string | null>(null)
let clipboardUids = $state<string[]>([]);

export const trackSelection = {
	get selected() { return selectedUids; },
	get lastClicked() { return lastClickedUid; },
	get context() { return context; },
	get contextUid() { return contextUid; },
	get count() { return selectedUids.size; },
	get clipboardUids() { return clipboardUids; },
	isSelected(uid: string, viewId: string): boolean {
		return ownerViewId === viewId && selectedUids.has(uid)
	}
};

export function generateViewId(): string {
  return crypto.randomUUID()
}

export function setTrackSelectionContext(ctx: SelectionContext, uid: string | null = null) {
	context = ctx;
	contextUid = uid;
	selectedUids = new Set();
	lastClickedUid = null;
}

export function clearTrackSelection(viewId?: string) {
	selectedUids = new Set();
	lastClickedUid = null;

	if (!viewId || ownerViewId === viewId) {
		selectedUids = new Set()
		ownerViewId = null
	}
}

export function selectTrack(uid: string, orderedUids: string[], event: MouseEvent, viewId: string) {
	if (ownerViewId !== viewId) {
		selectedUids = new Set()
		ownerViewId = viewId
	}
	
	if (event.shiftKey && lastClickedUid) {
		const a = orderedUids.indexOf(lastClickedUid);
		const b = orderedUids.indexOf(uid);
		if (a !== -1 && b !== -1) {
			const [lo, hi] = a < b ? [a, b] : [b, a];
			const range = orderedUids.slice(lo, hi + 1);
			if (event.ctrlKey || event.metaKey) {
				selectedUids = new Set([...selectedUids, ...range]);
			} else {
				selectedUids = new Set(range);
			}
			return;
		}
	}

	if (event.ctrlKey || event.metaKey) {
		const next = new Set(selectedUids);
		if (next.has(uid)) {
			next.delete(uid);
		} else {
			next.add(uid);
		}
		selectedUids = next;
	} else {
		selectedUids = new Set([uid]);
	}

	lastClickedUid = uid;
}

export async function getSelectedTracks(): Promise<Track[]> {
	const tracks = await getTracks();
	return tracks.filter((t) => selectedUids.has(t.uid));
}

export async function copySelectedToClipboard(orderedUids: string[]) {
	const allTracks = await getTracks();
	const trackMap = new Map(allTracks.map(t => [t.uid, t]));
	const tracks = orderedUids
		.filter((uid) => selectedUids.has(uid))
		.map((uid) => trackMap.get(uid))
		.filter(Boolean);

	if (tracks.length === 0) return;

	clipboardUids = tracks.map((t) => t!.uid);
	const trackNames = tracks.map((t) => (t!.title ?? "Unknown Title") + "; " + (t!.album_artist ?? "Unknown Artist")).join("\n");
	writeText(trackNames).catch(() => {});
}

export async function copySelectedNamesToClipboard(orderedUids: string[]) {
	const allTracks = await getTracks();
	const trackMap = new Map(allTracks.map(t => [t.uid, t]));
	const tracks = orderedUids
		.filter((uid) => selectedUids.has(uid))
		.map((uid) => trackMap.get(uid))
		.filter(Boolean);

	if (tracks.length === 0) return;

	const trackNames = tracks.map((t) => (t!.title ?? "Unknown Title") + "; " + (t!.album_artist ?? "Unknown Artist")).join("\n");

	writeText(trackNames).catch(() => {});
}

export async function copySelectedUIDsToClipboard(orderedUids: string[]) {
	const allTracks = await getTracks();
	const trackMap = new Map(allTracks.map(t => [t.uid, t]));
	const tracks = orderedUids
		.filter((uid) => selectedUids.has(uid))
		.map((uid) => trackMap.get(uid))
		.filter(Boolean);

	const uids = tracks.map((t) => t!.uid).join("\n");

	writeText(uids).catch(() => {});
}
export function copySelectedNameToClipboard(track: Track) {
	writeText((track.title ?? "Unknown Title") + "; " + (track.album_artist ?? "Unknown Artist")).catch(() => {});
}