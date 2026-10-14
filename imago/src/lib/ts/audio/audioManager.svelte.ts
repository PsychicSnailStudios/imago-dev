import { toast } from "svelte-sonner";
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { setMetadata, setPlaybackInfo, mediaControls, PlaybackStatus, RepeatMode } from 'tauri-plugin-media-api';

import { getTrack, getTrackArray, getTrackArrayFromUID } from "$ts/store/library.svelte";
import { profileState } from "$ts/store/profiles.svelte";
import { offlineMode } from "$ts/store/session.svelte";
import { scrobbleStart, scrobbleEnd, scrobbleMarkPaused, scrobbleMarkSeeked } from "$ts/audio/scrobbler.svelte";
import { parseAlbumEntries } from "$ts/util/parsers";
import type { Track } from "$ts/util/types";

import {
	player, currentlyPlaying,
	queuedTracks, playedTracks,
	getQueueIndex, setQueueIndex,
	getLastQueuedSet, setLastQueuedSet,
	setQueuedTracks,
} from "$ts/audio/audioPlayer.svelte";
import { buildAudioGraph, audioCtx, applyEqToGraph } from "$ts/audio/audioGraph.svelte";
import { isLocalPath, pathToSrc, isGhostTrack, markGhost, triggerOfflineMode, checkOnline, parseTrackData, parseExtraPaths } from "$ts/audio/audioHelper";
import { spacedShuffle, smartShuffle } from "$ts/audio/audioShuffles";

let audio: HTMLAudioElement | null = null;
let playToken = 0;
let cancelPending: (() => void) | null = null;

function playerKey() { return `imago:player:${profileState.active?.uid ?? "default"}`; }
function queueKey() { return `imago:queue:${profileState.active?.uid ?? "default"}`; }

const mediaReady: Promise<boolean> = mediaControls
	.initialize('imago-dev', 'Imago')
	.then(() => true)
	.catch(() => false);

let mediaChain: Promise<unknown> = mediaReady;

function runMedia(task: () => Promise<void>) {
	mediaChain = mediaChain
		.then(async () => {
			if (await mediaReady) await task();
		})
		.catch(() => {});
}

function artistText(track: Track): string {
	if (!track.artists) return "Unknown Artist";
	try {
		const parsed = JSON.parse(track.artists);
		if (Array.isArray(parsed)) {
			const names = parsed
				.map((a) => (typeof a === "string" ? a : a?.name))
				.filter((n): n is string => !!n);
			return names.length > 0 ? names.join(", ") : "Unknown Artist";
		}
	} catch {}
	return track.artists;
}

function UpdateTitle() {
	let title = "Imago";

	if (!player.isPlaying) {
		title = `${currentlyPlaying.track?.title}`;
	}

	getCurrentWindow().setTitle(title).catch(() => {});
}

async function pushMediaInfo(status?: PlaybackStatus) {
	await setPlaybackInfo({
		status: status ?? (player.isPlaying ? PlaybackStatus.Playing : PlaybackStatus.Paused),
		position: audio?.currentTime ?? player.currentTime ?? 0,
		shuffle: player.shuffleType !== 0,
		repeatMode: player.loopType === 1 ? RepeatMode.Track : player.loopType === 2 ? RepeatMode.List : RepeatMode.None,
		playbackRate: 1.0,
	});
}

function syncMediaInfo(status?: PlaybackStatus) {
	runMedia(() => pushMediaInfo(status));
}

function syncMediaMetadata(track: Track) {
	runMedia(async () => {
		const seconds = track.duration_ms ? track.duration_ms / 1000 : 0;
		await setMetadata({
			title: track.title ?? "Unknown Title",
			artist: artistText(track),
			album: parseAlbumEntries(track.albums)[0]?.name ?? undefined,
			albumArtist: track.album_artist ?? undefined,
			duration: seconds > 0 ? seconds : undefined,
		});
		await pushMediaInfo(PlaybackStatus.Paused);
	});
}

let unlistenCallbacks: (() => void)[] = [];

export async function setupMediaListeners() {
  const unlistenPlay = await listen('plugin:media:play', () => {
    togglePlay();
  });
  
  const unlistenPause = await listen('plugin:media:pause', () => {
    togglePlay();
  });

  const unlistenNext = await listen('plugin:media:next', () => {
    skipNext();
  });

  const unlistenPrev = await listen('plugin:media:previous', () => {
    skipBack();
  });

  unlistenCallbacks.push(unlistenPlay, unlistenPause, unlistenNext, unlistenPrev);
}

function releaseAudio() {
	cancelPending?.();
	cancelPending = null;
	if (!audio) return;
	const old = audio;
	audio = null;
	old.pause();
	old.removeAttribute("src");
	old.load();
}

function bindEvents(el: HTMLAudioElement) {
	el.addEventListener("timeupdate", () => {
		if (audio !== el) return;
		player.currentTime = el.currentTime;
	});
	el.addEventListener("durationchange", () => {
		if (audio !== el) return;
		player.duration = el.duration;
	});
	el.addEventListener("ended", () => {
		if (audio !== el) return;
		onTrackEnd();
	});
	el.addEventListener("play", () => {
		if (audio !== el) return;
		player.isPlaying = true;
		if (audioCtx && audioCtx.state === "suspended") {
			audioCtx.resume();
		}
		syncMediaInfo(PlaybackStatus.Playing);
	});
	el.addEventListener("pause", () => {
		if (audio !== el) return;
		player.isPlaying = false;
		if (!el.ended) syncMediaInfo(PlaybackStatus.Paused);
	});
	el.addEventListener("seeked", () => {
		if (audio !== el) return;
		syncMediaInfo();
	});
}

function startAudio(track: Track, play: boolean = true, srcOverride?: string) {
	releaseAudio();

	player.track = track;
	currentlyPlaying.uid = track.uid;
	currentlyPlaying.track = track;
	syncMediaMetadata(track);

	const el = new Audio();
	el.crossOrigin = "anonymous";
	el.src = srcOverride ?? pathToSrc(track.path);
	el.volume = player.volume;
	el.muted = player.muted;
	audio = el;
	bindEvents(el);

	buildAudioGraph(el);

	if (!play) return;

	if (audioCtx && audioCtx.state === "suspended") {
		audioCtx.resume();
	}
	el.play().catch(() => {});
}

async function tryPlayWithSrc(track: Track, src: string): Promise<boolean> {
	releaseAudio();

	player.track = track;
	currentlyPlaying.uid = track.uid;
	currentlyPlaying.track = track;
	syncMediaMetadata(track);

	return new Promise((resolve) => {
		const el = new Audio();
		el.crossOrigin = "anonymous";
		el.src = src;
		el.volume = player.volume;
		el.muted = player.muted;
		audio = el;
		bindEvents(el);
		buildAudioGraph(el);

		if (audioCtx && audioCtx.state === "suspended") {
			audioCtx.resume();
		}

		const cleanup = () => {
			el.removeEventListener("canplay", onCanPlay);
			el.removeEventListener("error", onError);
		};

		const onCanPlay = () => {
			cleanup();
			cancelPending = null;
			el.play().then(() => resolve(true)).catch(() => resolve(false));
		};

		const onError = () => {
			cleanup();
			cancelPending = null;
			resolve(false);
		};

		cancelPending = () => {
			cleanup();
			resolve(false);
		};

		el.addEventListener("canplay", onCanPlay);
		el.addEventListener("error", onError);

		el.load();
	});
}

async function tryExtraPaths(track: Track, token: number, localOnly = false): Promise<boolean> {
	for (const path of parseExtraPaths(track.extra_paths)) {
		if (!isLocalPath(path)) continue;
		const ok = await tryPlayWithSrc(track, pathToSrc(path));
		if (token !== playToken) return false;
		if (ok) {
			scrobbleStart(track, "trackdone", true);
			return true;
		}
	}

	if (localOnly || offlineMode.offline) return false;

	for (const path of parseExtraPaths(track.extra_remote_paths)) {
		const ok = await tryPlayWithSrc(track, pathToSrc(path));
		if (token !== playToken) return false;
		if (ok) {
			scrobbleStart(track, "trackdone", false);
			return true;
		}
	}

	return false;
}

async function playTrack(track: Track) {
	const token = ++playToken;

	if (currentlyPlaying.track) {
		playedTracks.unshift(currentlyPlaying.track);
	}

	UpdateTitle();

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
		if (token !== playToken) return;
		if (ok) {
			scrobbleStart(track, "trackdone", true);
			return;
		}

		if (!hasRemote) {
			if (await tryExtraPaths(track, token)) return;
			if (token !== playToken) return;
		}

		await markGhost(track, true);
		if (token !== playToken) return;

		if (!hasRemote) {
			toast.warning("Cannot play track — local file missing.");
			onTrackEnd(true);
			return;
		}
	}

	if (hasRemote) {
		if (offlineMode.offline) {
			const ok = await tryPlayWithSrc(track, pathToSrc(track.remote_path!));
			if (token !== playToken) return;
			if (ok) {
				offlineMode.offline = false;
				scrobbleStart(track, "trackdone", false);
				return;
			} else {
				if (await tryExtraPaths(track, token, true)) return;
				if (token !== playToken) return;
				onTrackEnd(true);
				return;
			}
		}

		const ok = await tryPlayWithSrc(track, pathToSrc(track.remote_path!));
		if (token !== playToken) return;
		if (ok) {
			scrobbleStart(track, "trackdone", false);
			return;
		}

		const online = await checkOnline();
		if (token !== playToken) return;
		if (!online) {
			triggerOfflineMode();
			if (hasLocal) {
				const localOk = await tryPlayWithSrc(track, pathToSrc(track.path));
				if (token !== playToken) return;
				if (localOk) {
					scrobbleStart(track, "trackdone", true);
					return;
				}
			}
			if (await tryExtraPaths(track, token, true)) return;
			if (token !== playToken) return;
		} else {
			if (await tryExtraPaths(track, token)) return;
			if (token !== playToken) return;
			await markGhost(track, true);
			if (token !== playToken) return;
			toast.warning("Cannot play track — stream unavailable.");
		}

		onTrackEnd(true);
		return;
	}

	toast.warning("Cannot play track — no file available.");
	onTrackEnd(true);
}

function onTrackEnd(skipGhost = false) {
	if (!skipGhost && audio) scrobbleEnd(audio.currentTime, "trackdone", false);

	if (player.loopType === 1 && !skipGhost) {
		if (audio) {
			scrobbleStart(player.track!, "trackdone", !!(player.track && isLocalPath(player.track.path)));
			audio.currentTime = 0;
			audio.play().catch(() => {});
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
	syncMediaInfo(PlaybackStatus.Stopped);
}

function applyShuffleToList(tracks: Track[]): Track[] {
	switch (player.shuffleType) {
		case 1: return spacedShuffle(tracks);
		case 2: return smartShuffle(tracks);
		default: return tracks;
	}
}

export function loadPlayerState() {
	try {
		const raw = localStorage.getItem(playerKey());
		if (raw) {
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

				const el = audio;
				if (el && saved.currentTime) {
					const seekToSaved = () => {
						el.currentTime = saved.currentTime;
						player.currentTime = saved.currentTime;
						el.removeEventListener("canplay", seekToSaved);
					};
					el.addEventListener("canplay", seekToSaved);
				}
			}
		}
	} catch {}
	try {
		const raw = localStorage.getItem(queueKey());
		if (raw) {
			const saved = JSON.parse(raw);
			setQueuedTracks(saved.tracks ?? []);
			setQueueIndex(saved.index ?? -1);
			setLastQueuedSet(saved.lastSet ?? []);
		}
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
	let insertAt = without.indexOf(anchorTrack);
	if (insertAt === -1) {
		insertAt = absTo - absIndices.filter(i => i < absTo).length;
	}

	without.splice(insertAt, 0, ...removed);
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
	if (!audio || !isFinite(seconds)) return;
	scrobbleMarkSeeked();
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
	const el = audio;
	if (!el) return;

	if (el.currentTime >= 2) {
		el.currentTime = 0;
		return;
	}

	if (getQueueIndex() < 1) return;

	scrobbleEnd(el.currentTime, "backbtn", true);
	setQueueIndex(getQueueIndex() - 1);
	playTrack(queuedTracks[getQueueIndex()]);
}

export function skipNext() {
	if (queuedTracks.length === 0) return;

	let nextIndex = getQueueIndex() + 1;
	if (nextIndex >= queuedTracks.length) {
		if (player.loopType === 2) {
			nextIndex = 0;
		} else {
			return;
		}
	}

	if (audio) scrobbleEnd(audio.currentTime, "fwdbtn", true);
	setQueueIndex(nextIndex);
	playTrack(queuedTracks[getQueueIndex()]);
}

export function toggleLoop() {
	player.loopType = (player.loopType + 1) % 3;
	syncMediaInfo();
}

export function toggleShuffle() {
	player.shuffleType = (player.shuffleType + 1) % 3;
	syncMediaInfo();
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
		audio.play().catch(() => {});
	}

	UpdateTitle();
}