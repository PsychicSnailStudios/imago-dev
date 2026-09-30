import { invoke } from "@tauri-apps/api/core";
import { player } from "$ts/audio/audioPlayer.svelte";
import { offlineMode } from "$ts/store/session.svelte";
import { parseArtists, parseAlbumEntries } from "$ts/util/parsers";
import type { Track } from "$ts/util/types";

let activeScrobbleUid: string | null = null;
let activeTrackUid: string | null = null;
let activeTrackStartTime: number = 0;
let didSeek = false;
let didPause = false;

export async function scrobbleStart(
	track: Track,
	reason: string = "trackdone",
	playingLocal: boolean = true,
): Promise<void> {
	didSeek = false;
	didPause = false;

	if (activeScrobbleUid) {
		await scrobbleEnd(0, "trackdone", false);
	}

	activeTrackUid = track.uid;
	activeTrackStartTime = Math.floor(Date.now() / 1000);

	try {
		const artistName = track.album_artist ?? parseArtists(track.artists)[0] ?? null;
		const artistUid = artistName
			? (await invoke<string | null>("get_artist_uid_by_name", { name: artistName })) ?? ""
			: "";
		const firstAlbum = parseAlbumEntries(track.albums)[0] ?? null;

		activeScrobbleUid = await invoke<string>("log_scrobble", {
			trackUid: track.uid,
			artistUid,
			reasonStart: reason,
			shuffle: player.shuffleType > 0,
			offline: offlineMode.offline,
			playingLocal,
			trackName: track.title ?? null,
			trackArtist: artistName,
			trackAlbum: firstAlbum?.name || null,
			albumUid: firstAlbum?.uid || null,
			sourceUid: track.play_source_uid || null,
		});
	} catch (e) {
		console.error("scrobbleStart failed", e);
		activeScrobbleUid = null;
	}

	try {
		await invoke("update_now_playing", { uid: track.uid });
	} catch (e) {
		console.error("update_now_playing failed", e);
	}
}

export async function scrobbleEnd(
	durationPlayed: number,
	reason: string = "trackdone",
	skipped: boolean = false,
): Promise<void> {
	const uid = activeScrobbleUid;
	const trackUid = activeTrackUid;

	activeScrobbleUid = null;
	activeTrackUid = null;

	if (!uid) return;

	try {
		await invoke("update_scrobble", {
			uid,
			durationPlayed: Math.round(durationPlayed * 1000),
			didSeek,
			didPause,
			reasonEnd: reason,
			skipped,
		});
	} catch (e) {
		console.error("scrobbleEnd failed", e);
	}

	if (trackUid && durationPlayed > 30) {
		try {
			await invoke("scrobble_track", { uid: trackUid });
		} catch (e) {
			console.error("scrobble_track failed", e);
		}
	}
}

export function scrobbleMarkSeeked(): void {
	didSeek = true;
}

export function scrobbleMarkPaused(): void {
	didPause = true;
}