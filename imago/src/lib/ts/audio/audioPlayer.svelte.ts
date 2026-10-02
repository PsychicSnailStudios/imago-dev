import type { Track } from "$ts/util/types";

export const queuedTracks = $state<Track[]>([]);
export const playedTracks = $state<Track[]>([]);

export const currentlyPlaying = $state({
	uid: "" as string,
	track: null as Track | null,
});

export const player = $state({
	track: null as Track | null,
	isPlaying: false,
	loopType: 0,
	shuffleType: 0,
	currentTime: 0,
	duration: 0,
	volume: 1,
	muted: false,
});

let _queueIndex = $state(-1);
let _lastQueuedSet: Track[] = [];

export function getQueueIndex(): number {
	return _queueIndex;
}

export function setQueueIndex(i: number) {
	_queueIndex = i;
}

export function getLastQueuedSet(): Track[] {
	return _lastQueuedSet;
}

export function setLastQueuedSet(tracks: Track[]) {
	_lastQueuedSet = tracks;
}

export function setQueuedTracks(tracks: Track[]) {
	queuedTracks.splice(0, queuedTracks.length, ...tracks);
}