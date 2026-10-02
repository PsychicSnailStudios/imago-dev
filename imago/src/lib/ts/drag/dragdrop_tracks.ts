import { setDragActive, setDragPayload } from "$ts/store/drag.svelte";
import type { DragPayload } from "$ts/store/drag.svelte";

export function startTrackDrag(e: DragEvent, uids: string[], sourcePlaylistUid: string | null = null) {
	if (e.dataTransfer) {
		e.dataTransfer.setData("text/plain", uids.join(","));
		e.dataTransfer.effectAllowed = "move";
	}
	const p: DragPayload = { type: "tracks", uids, sourcePlaylistUid };
	setDragActive(true);
	setDragPayload(p);
}
