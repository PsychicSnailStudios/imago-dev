export type DragPayload = {
	type: "tracks";
	uids: string[];
	sourcePlaylistUid: string | null;
	sourceUid?: string | null;
	sourceQueueIndices?: number[];
};

let active = $state(false);
let payload = $state<DragPayload | null>(null);
let hoveredPlaylistUid = $state<string | null>(null);
let hoveredFolderPath = $state<string | null>(null);

export const dragState = {
	get active() { return active; },
	get payload() { return payload; },
	get hoveredPlaylistUid() { return hoveredPlaylistUid; },
	get hoveredFolderPath() { return hoveredFolderPath; },
};

export function setDragActive(val: boolean) { active = val; }
export function setDragPayload(val: DragPayload | null) { payload = val; }
export function setHoveredPlaylist(uid: string | null) { hoveredPlaylistUid = uid; }
export function setHoveredFolder(path: string | null) { hoveredFolderPath = path; }

export function startDrag(p: DragPayload) {
	active = true;
	payload = p;
}

export function endDrag() {
	active = false;
	payload = null;
	hoveredPlaylistUid = null;
	hoveredFolderPath = null;
}