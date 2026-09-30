import { toast } from "svelte-sonner";
import { getTrack, getTrackArray, getTrackArrayFromUID } from "$ts/store/library.svelte";
import { profileState } from "$ts/store/profiles.svelte";
import { offlineMode } from "$ts/store/session.svelte";
import { scrobbleStart, scrobbleEnd, scrobbleMarkPaused, scrobbleMarkSeeked } from "$ts/audio/scrobbler.svelte";
import type { Track } from "$ts/util/types";

import {
	player, currentlyPlaying,
	queuedTracks, playedTracks,
	getQueueIndex, setQueueIndex,
	getLastQueuedSet, setLastQueuedSet,
	setQueuedTracks,
} from "$ts/audio/audioPlayer.svelte";
import { buildAudioGraph, audioCtx, applyEqToGraph } from "$ts/audio/audioGraph.svelte";
import { isLocalPath, pathToSrc, isGhostTrack, markGhost, triggerOfflineMode, checkOnline, parseTrackData } from "$ts/audio/audioHelper";
import { spacedShuffle, smartShuffle } from "$ts/audio/audioShuffles";

let audio: HTMLAudioElement | null = null;

function playerKey() { return `imago:player:${profileState.active?.uid ?? "default"}`; }
function queueKey() { return `imago:queue:${profileState.active?.uid ?? "default"}`; }

// ─── Audio element ────────────────────────────────────────────────────────────

function bindEvents(el: HTMLAudioElement) {
	el.addEventListener("timeupdate", () => {
		player.currentTime = el.currentTime;
	});
	el.addEventListener("durationchange", () => {
		player.duration = el.duration;
	});
	el.addEventListener("ended", () => {
		onTrackEnd();
	});
	el.addEventListener("play", () => {
		player.isPlaying = true;
		if (audioCtx && audioCtx.state === "suspended") {
			audioCtx.resume();
		}
	});
	el.addEventListener("pause", () => {
		player.isPlaying = false;
	});
}

function startAudio(track: Track, play: boolean = true, srcOverride?: string) {
	if (audio) {
		audio.pause();
		audio = null;
	}

	player.track = track;
	currentlyPlaying.uid = track.uid;
	currentlyPlaying.track = track;

	const el = new Audio();
	el.crossOrigin = "anonymous";
	el.src = srcOverride ?? pathToSrc(track.path);
	el.volume = player.volume;
	el.muted = player.muted;
	bindEvents(el);
	audio = el;

	buildAudioGraph(el);

	if (!play) return;

	if (audioCtx && audioCtx.state === "suspended") {
		audioCtx.resume();
	}
	el.play();
}

// ─── Core playback logic ──────────────────────────────────────────────────────

async function tryPlayWithSrc(track: Track, src: string): Promise<boolean> {
	return new Promise((resolve) => {
		if (audio) {
			audio.pause();
			audio = null;
		}

		player.track = track;
		currentlyPlaying.uid = track.uid;
		currentlyPlaying.track = track;

		const el = new Audio();
		el.crossOrigin = "anonymous";
		el.src = src;
		el.volume = player.volume;
		el.muted = player.muted;
		bindEvents(el);
		audio = el;
		buildAudioGraph(el);

		if (audioCtx && audioCtx.state === "suspended") {
			audioCtx.resume();
		}

		const onCanPlay = () => {
			cleanup();
			el.play().catch(() => resolve(false));
			resolve(true);
		};

		const onError = () => {
			cleanup();
			resolve(false);
		};

		const cleanup = () => {
			el.removeEventListener("canplay", onCanPlay);
			el.removeEventListener("error", onError);
		};

		el.addEventListener("canplay", onCanPlay);
		el.addEventListener("error", onError);

		el.load();
	});
}

async function playTrack(track: Track) {
	if (currentlyPlaying.track) {
		playedTracks.unshift(currentlyPlaying.track);
	}

	if (isGhostTrack(track)) {
		toast.warning("Cannot play track — no file available.");
		scrobbleStart(track, "trackdone", false);
		onTrackEnd(true);
		return;
	}

	const hasLocal = isLocalPath(track.path);
	const hasRemote = !!(track.remote_path && track.remote_path.length > 0);

	const localBr = (parseTrackData(track.track_data).bitrate ?? null) ?? track.bitrate ?? 0;
	const remoteBr = (parseTrackData(track.remote_data).bitrate ?? null) ?? 0;

	const preferLocal = hasLocal && (!hasRemote || localBr >= remoteBr);

	if (preferLocal) {
		const ok = await tryPlayWithSrc(track, pathToSrc(track.path));
		if (ok) {
			scrobbleStart(track, "trackdone", true);
			return;
		}

		await markGhost(track, true);

		if (!hasRemote) {
			toast.warning("Cannot play track — local file missing.");
			onTrackEnd(true);
			return;
		}
	}

	if (hasRemote) {
		if (offlineMode.offline) {
			const ok = await tryPlayWithSrc(track, pathToSrc(track.remote_path!));
			if (ok) {
				offlineMode.offline = false;
				scrobbleStart(track, "trackdone", false);
				return;
			} else {
				onTrackEnd(true);
				return;
			}
		}

		const ok = await tryPlayWithSrc(track, pathToSrc(track.remote_path!));
		if (ok) {
			scrobbleStart(track, "trackdone", false);
			return;
		}

		const online = await checkOnline();
		if (!online) {
			triggerOfflineMode();
			if (hasLocal) {
				const localOk = await tryPlayWithSrc(track, pathToSrc(track.path));
				if (localOk) {
					scrobbleStart(track, "trackdone", true);
					return;
				}
			}
		} else {
			await markGhost(track, true);
			toast.warning("Cannot play track — stream unavailable.");
		}

		onTrackEnd(true);
		return;
	}

	toast.warning("Cannot play track — no file available.");
	onTrackEnd(true);
}

function onTrackEnd(skipGhost = false) {
	if (!skipGhost) scrobbleEnd(audio!.currentTime, "trackdone", false);

	if (player.loopType === 1) {
		if (audio) {
			scrobbleStart(player.track!, "trackdone", !!(player.track && isLocalPath(player.track.path)));
			audio.currentTime = 0;
			audio.play();
		}
		return;
	}

	const nextIndex = getQueueIndex() + 1;

	if (nextIndex < queuedTracks.length) {
		setQueueIndex(nextIndex);
		playTrack(queuedTracks[nextIndex]);
		return;
	}

	const lqs = getLastQueuedSet();
	if (player.loopType === 2 && lqs.length > 0) {
		let requeued: Track[];
		switch (player.shuffleType) {
			case 1: requeued = spacedShuffle(lqs); break;
			case 2: requeued = smartShuffle(lqs); break;
			default: requeued = lqs;
		}

		setQueuedTracks(requeued);
		setQueueIndex(0);
		playTrack(queuedTracks[0]);
		return;
	}

	player.isPlaying = false;
	player.currentTime = 0;
}

function applyShuffleToList(tracks: Track[]): Track[] {
	switch (player.shuffleType) {
		case 1: return spacedShuffle(tracks);
		case 2: return smartShuffle(tracks);
		default: return tracks;
	}
}

// ─── State persistence ────────────────────────────────────────────────────────

export function loadPlayerState() {
	try {
		const raw = localStorage.getItem(playerKey());
		if (!raw) return;
		const saved = JSON.parse(raw);
		player.volume = saved.volume ?? 1;
		player.muted = saved.muted ?? false;
		player.loopType = saved.loopType ?? 0;
		player.shuffleType = saved.shuffleType ?? 0;
		player.duration = saved.duration ?? 0;
		player.isPlaying = false;

		if (saved.track) {
			player.track = saved.track;
			startAudio(saved.track, false);

			if (audio && saved.currentTime) {
				const seek = () => {
					audio!.currentTime = saved.currentTime;
					player.currentTime = saved.currentTime;
					audio!.removeEventListener("canplay", seek);
				};
				audio.addEventListener("canplay", seek);
			}
		}
	} catch {}
	try {
		const raw = localStorage.getItem(queueKey());
		if (!raw) return;
		const saved = JSON.parse(raw);
		setQueuedTracks(saved.tracks ?? []);
		setQueueIndex(saved.index ?? -1);
		setLastQueuedSet(saved.lastSet ?? []);
	} catch {}
}

export function savePlayerState() {
	if (audio) scrobbleEnd(audio.currentTime, "appclose", false);

	try {
		localStorage.setItem(playerKey(), JSON.stringify({
			track: player.track,
			isPlaying: false,
			loopType: player.loopType,
			shuffleType: player.shuffleType,
			currentTime: player.currentTime,
			duration: player.duration,
			volume: player.volume,
			muted: player.muted,
		}));
	} catch {}
	try {
		localStorage.setItem(queueKey(), JSON.stringify({
			tracks: queuedTracks,
			index: getQueueIndex(),
			lastSet: getLastQueuedSet(),
		}));
	} catch {}
}

// ─── Public API ───────────────────────────────────────────────────────────────

function withSource(track: Track, sourceUid: string | null | undefined): Track {
	return { ...track, play_source_uid: sourceUid ?? null };
}

function withSourceAll(tracks: Track[], sourceUid: string | null | undefined): Track[] {
	return tracks.map(t => withSource(t, sourceUid));
}

export async function playTrackByUid(uid: string, sourceUid: string | null = null) {
	const track = await getTrack(uid);
	if (!track) return;
	playTrack(withSource(track, sourceUid));
}

export function playTrackByObject(track: Track, sourceUid: string | null = null) {
	playTrack(withSource(track, sourceUid));
}

export function clearQueue() {
	setQueuedTracks([]);
	setQueueIndex(-1);
	setLastQueuedSet([]);
}

export function getQueuedTracks(): Track[] {
	return queuedTracks.slice(getQueueIndex() + 1);
}

export function getQueuedTracksAll(): Track[] {
	return queuedTracks;
}

export function getPlayedTracks(): Track[] {
	return playedTracks;
}

export function addTrackToQueue(track: Track, sourceUid: string | null = null) {
	queuedTracks.push(withSource(track, sourceUid));
}

export function queueTracksByObject(tracksIn: Track[], play = false, shuffle = false, sourceUid: string | null = null) {
	if (play) clearQueue();
	if (shuffle && player.shuffleType === 0) player.shuffleType = 1;

	const tracks = withSourceAll(tracksIn, sourceUid);
	setLastQueuedSet(tracks);
	const ordered = applyShuffleToList(tracks);
	ordered.forEach(t => queuedTracks.push(t));

	if (play) startPlayingQueue();
}

export async function queueTracksByUid(uids: string[], play = false, shuffle = false, sourceUid: string | null = null) {
	if (play) clearQueue();
	if (shuffle && player.shuffleType === 0) player.shuffleType = 1;

	const tracks: Track[] = withSourceAll(await getTrackArray(uids), sourceUid);

	setLastQueuedSet(tracks);
	const ordered = applyShuffleToList(tracks);
	ordered.forEach(t => queuedTracks.push(t));

	if (play) startPlayingQueue();
}

export async function queueTracksFromUid(uid: string, play = false, shuffle = false) {
	if (play) clearQueue();
	if (shuffle && player.shuffleType === 0) player.shuffleType = 1;

	const tracks: Track[] = withSourceAll(await getTrackArrayFromUID(uid), uid);

	setLastQueuedSet(tracks);
	const ordered = applyShuffleToList(tracks);
	ordered.forEach(t => queuedTracks.push(t));

	if (play) startPlayingQueue();
}

function startPlayingQueue() {
	if (queuedTracks.length === 0) return;
	setQueueIndex(0);
	playTrack(queuedTracks[0]);
}

export function reorderQueue(fromDisplayIndices: number[], toDisplayIndex: number) {
	const offset = getQueueIndex() + 1;
	const absIndices = fromDisplayIndices.map(i => i + offset);
	const absTo = toDisplayIndex + offset;

	if (
		absIndices.some(i => i < offset || i >= queuedTracks.length) ||
		absTo < offset || absTo >= queuedTracks.length
	) return;

	const movingSet = new Set(absIndices);
	const removed = absIndices.map(i => queuedTracks[i]);
	const without = queuedTracks.filter((_, i) => !movingSet.has(i));

	const anchorTrack = queuedTracks[absTo];
	const anchorIndexInWithout = without.indexOf(anchorTrack);

	without.splice(anchorIndexInWithout, 0, ...removed);
	setQueuedTracks(without);
}

export function removeFromQueue(displayIndex: number) {
	const abs = displayIndex + getQueueIndex() + 1;
	if (abs < getQueueIndex() + 1 || abs >= queuedTracks.length) return;
	queuedTracks.splice(abs, 1);
}

export function insertIntoQueue(tracksIn: Track[], afterDisplayIndex?: number, sourceUid: string | null = null) {
	const tracks = tracksIn.map(t => t.play_source_uid !== undefined ? t : withSource(t, sourceUid));
	const offset = getQueueIndex() + 1;
	if (afterDisplayIndex === undefined) {
		tracks.forEach(t => queuedTracks.push(t));
	} else {
		const absInsert = afterDisplayIndex + offset + 1;
		const clampedInsert = Math.min(absInsert, queuedTracks.length);
		queuedTracks.splice(clampedInsert, 0, ...tracks);
	}
}

export function seek(seconds: number) {
	scrobbleMarkSeeked();
	if (!audio || !isFinite(seconds)) return;
	audio.currentTime = seconds;
}

export function setVolume(vol: number) {
	player.volume = vol;
	if (audio) audio.volume = vol;
}

export function toggleMute() {
	player.muted = !player.muted;
	if (audio) audio.muted = player.muted;
}

export function skipBack() {
	scrobbleEnd(audio!.currentTime, "backbtn", true);
	const el = audio;
	if (!el) return;

	if (el.currentTime >= 2) {
		el.currentTime = 0;
		return;
	}

	if (getQueueIndex() < 1) return;

	setQueueIndex(getQueueIndex() - 1);
	playTrack(queuedTracks[getQueueIndex()]);
}

export function skipNext() {
	scrobbleEnd(audio!.currentTime, "fwdbtn", true);
	let nextIndex = getQueueIndex() + 1;
	if (nextIndex >= queuedTracks.length) {
		if (player.loopType === 2) {
			nextIndex = 0;
		} else {
			return;
		}
	}

	setQueueIndex(nextIndex);
	playTrack(queuedTracks[getQueueIndex()]);
}

export function toggleLoop() {
	player.loopType = (player.loopType + 1) % 3;
}

export function toggleShuffle() {
	player.shuffleType = (player.shuffleType + 1) % 3;
}

export function togglePlay() {
	if (!audio) return;
	if (audioCtx && audioCtx.state === "suspended") {
		audioCtx.resume();
	}
	if (player.isPlaying) {
		scrobbleMarkPaused();
		audio.pause();
	} else {
		audio.play();
	}
}