import { invoke } from "@tauri-apps/api/core";
import { getTracks, getPlaylist, getTrackArray, reloadLibrary } from "$ts/store/library.svelte";
import { parseAlbumEntries } from "$ts/util/parsers";
import type { Playlist, Track, TrackEntry } from "$ts/util/types";

const OWNER = "Imago";
const DAY_SECONDS = 24 * 60 * 60;
const WEEK_SECONDS = 7 * DAY_SECONDS;
const MIN_PLAY_MS = 30_000;
const ON_LOOP_LIMIT = 50;
const RECENT_LIMIT = 100;
const ON_LOOP_UID = "p-explore-on-loop";
const RECENT_UID = "p-explore-recently-added";

type ScrobbleRow = {
	track_uid: string;
	timestamp: number;
	duration_played: number;
};

let explorePlaylists = $state<Playlist[]>([]);

function nowSeconds(): number {
	return Math.floor(Date.now() / 1000);
}

function shuffle<T>(items: T[]): T[] {
	const out = [...items];
	for (let i = out.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[out[i], out[j]] = [out[j], out[i]];
	}
	return out;
}

function addedAt(track: Track): number {
	return track.date_added ?? 0;
}

function albumKey(track: Track): string {
	const first = parseAlbumEntries(track.albums as unknown as string | null)[0];
	if (first?.uid) return first.uid;
	if (first?.name) return `${String(track.album_artist ?? "")}|${first.name}`;
	return track.uid;
}

function pickVaried(tracks: Track[], limit: number): Track[] {
	const groups = new Map<string, Track[]>();
	for (const track of tracks) {
		const key = albumKey(track);
		const group = groups.get(key);
		if (group) group.push(track);
		else groups.set(key, [track]);
	}

	const ordered = [...groups.values()]
		.map((group) => ({
			latest: Math.max(...group.map(addedAt)),
			tracks: shuffle(group),
		}))
		.sort((a, b) => b.latest - a.latest);

	const picked: Track[] = [];
	let round = 0;
	while (picked.length < limit) {
		let added = false;
		for (const group of ordered) {
			if (round >= group.tracks.length) continue;
			picked.push(group.tracks[round]);
			added = true;
			if (picked.length >= limit) break;
		}
		if (!added) break;
		round++;
	}
	return picked;
}

async function buildOnLoop(): Promise<Track[]> {
	const scrobbles = await invoke<ScrobbleRow[]>("get_scrobbles");
	const cutoff = nowSeconds() - WEEK_SECONDS;
	const stats = new Map<string, { plays: number; ms: number; last: number }>();

	for (const scrobble of scrobbles) {
		if (scrobble.timestamp < cutoff || scrobble.duration_played < MIN_PLAY_MS) continue;
		const entry = stats.get(scrobble.track_uid) ?? { plays: 0, ms: 0, last: 0 };
		entry.plays += 1;
		entry.ms += scrobble.duration_played;
		entry.last = Math.max(entry.last, scrobble.timestamp);
		stats.set(scrobble.track_uid, entry);
	}

	const ranked = [...stats.entries()]
		.sort((a, b) => b[1].plays - a[1].plays || b[1].ms - a[1].ms || b[1].last - a[1].last)
		.map(([uid]) => uid)
		.slice(0, ON_LOOP_LIMIT * 3);

	if (ranked.length === 0) return [];

	const tracks = await getTrackArray(ranked);
	const byUid = new Map(tracks.map((track) => [track.uid, track]));
	return ranked
		.map((uid) => byUid.get(uid))
		.filter((track): track is Track => !!track)
		.slice(0, ON_LOOP_LIMIT);
}

async function buildRecentlyAdded(): Promise<Track[]> {
	const all = await getTracks();
	const sorted = all
		.filter((track) => addedAt(track) > 0)
		.sort((a, b) => addedAt(b) - addedAt(a));

	const dayCutoff = nowSeconds() - DAY_SECONDS;
	const lastDay = sorted.filter((track) => addedAt(track) >= dayCutoff);

	if (lastDay.length > RECENT_LIMIT) return pickVaried(lastDay, RECENT_LIMIT);
	return sorted.slice(0, RECENT_LIMIT);
}

async function upsertPlaylist(
	uid: string,
	title: string,
	description: string,
	tracks: Track[],
): Promise<Playlist | null> {
	const entries: TrackEntry[] = tracks.map((track, i) => ({
		uid: track.uid,
		name: track.title ?? "Unknown Title",
		order: i + 1,
	}));
	const tracksJson = JSON.stringify(entries);
	const existing = await getPlaylist(uid);

	if (existing) {
		await invoke("update_playlist_entry", {
			uid,
			update: { title, description, owner: OWNER, tracks: tracksJson },
		});
		return entries.length === 0 ? null : await getPlaylist(uid);
	}

	if (entries.length === 0) return null;

	await invoke("create_playlist_entry", {
		playlist: {
			uid,
			title,
			description,
			owner: OWNER,
			tracks: tracksJson,
			artwork_path: null,
			folder: null,
			version: 1,
		},
	});
	return await getPlaylist(uid);
}

async function setExplorePlaylists(): Promise<void> {
	try {
		const [onLoopTracks, recentTracks] = await Promise.all([buildOnLoop(), buildRecentlyAdded()]);

		const onLoop = await upsertPlaylist(
			ON_LOOP_UID,
			"On Loop",
			"Your 50 most played tracks over the last week",
			onLoopTracks,
		);
		const recent = await upsertPlaylist(
			RECENT_UID,
			"Recently Added",
			"The newest tracks in your library",
			recentTracks,
		);

		const playlists = [onLoop, recent].filter((playlist): playlist is Playlist => playlist !== null);
		explorePlaylists.splice(0, explorePlaylists.length, ...playlists);

		await reloadLibrary("playlists");
	} catch (e) {
		console.error("setExplorePlaylists failed", e);
	}
}

export { explorePlaylists, setExplorePlaylists };