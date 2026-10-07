import { invoke } from "@tauri-apps/api/core";
import { resolveResource } from "@tauri-apps/api/path";
import { getTracks, getPlaylist, reloadLibrary } from "$ts/store/library.svelte";
import { parseAlbumEntries } from "$ts/util/parsers";
import { getScrobbles } from "$ts/audio/scrobbleCache";
import type { Playlist, Track, TrackEntry } from "$ts/util/types";
import {
	playlistDefinitions,
	type DateRange,
	type PlaylistDefinition,
	type PlaylistFilters,
	type Rank,
	type RankSpec,
	type Refresh,
} from "./explorePlaylistDefinitions";

const DAY_SECONDS = 24 * 60 * 60;
const DEFAULT_MIN_PLAY_MS = 30_000;
const REFRESH_KEY_PREFIX = "explore_last_refresh:";
const ARTWORK_DIR = "assets/artwork";

type ScrobbleRow = {
	track_uid: string;
	timestamp: number;
	duration_played: number;
};

type PlayStats = {
	plays: number;
	ms: number;
	last: number;
	stamps: number[];
};

type Window = { from: number; to: number };

type RankContext = {
	tracks: Track[];
	stats: Map<string, PlayStats>;
	windowDays: number;
	limit: number;
};

type Scorer = (track: Track) => number | null;
type Ranker = (ctx: RankContext) => Scorer;
type TrackFilter = (track: Track, filters: PlaylistFilters, now: number) => boolean;

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

function parseList(raw: unknown): unknown[] {
	if (Array.isArray(raw)) return raw;
	if (typeof raw !== "string" || raw === "") return [];
	try {
		const parsed = JSON.parse(raw);
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

function namesOf(raw: unknown): string[] {
	return parseList(raw)
		.map((item) => (typeof item === "string" ? item : ((item as { name?: string })?.name ?? "")))
		.filter(Boolean)
		.map((name) => name.toLowerCase());
}

function matchesAny(wanted: string[] | undefined, have: string[]): boolean {
	if (!wanted || wanted.length === 0) return true;
	const set = new Set(wanted.map((name) => name.toLowerCase()));
	return have.some((name) => set.has(name));
}

function resolveRange(range: DateRange | undefined, now: number): Window | null {
	if (!range) return null;
	const to = range.to ?? now;
	const from = range.from ?? (range.lastDays !== undefined ? to - range.lastDays * DAY_SECONDS : 0);
	return { from, to };
}

function inWindow(value: number, window: Window | null): boolean {
	if (!window) return true;
	return value >= window.from && value <= window.to;
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

const trackFilters: TrackFilter[] = [
	(track, filters) => matchesAny(filters.artists, namesOf(track.artists)),
	(track, filters) => matchesAny(filters.tags, namesOf(track.tags)),
	(track, filters) => matchesAny(filters.genres, namesOf(track.genres)),
	(track, filters) => filters.minRating === undefined || (track.rating ?? 0) >= filters.minRating,
	(track, filters, now) => inWindow(addedAt(track), resolveRange(filters.added, now)),
];

function statScorer(
	ctx: RankContext,
	score: (stat: PlayStats, ctx: RankContext) => number | null,
	direction: 1 | -1,
): Scorer {
	return (track) => {
		const stat = ctx.stats.get(track.uid);
		if (!stat) return null;
		const value = score(stat, ctx);
		return value === null ? null : value * direction;
	};
}

function averageGap(stat: PlayStats): number | null {
	if (stat.stamps.length < 2) return null;
	const sorted = [...stat.stamps].sort((a, b) => a - b);
	return (sorted[sorted.length - 1] - sorted[0]) / (sorted.length - 1);
}

const rankers: Record<Rank, Ranker> = {
	rate: (ctx) => statScorer(ctx, (stat, c) => stat.plays / c.windowDays, 1),
	totalTime: (ctx) => statScorer(ctx, (stat) => stat.ms, 1),
	consistency: (ctx) =>
		statScorer(ctx, (stat) => new Set(stat.stamps.map((t) => Math.floor(t / DAY_SECONDS))).size, 1),
	gap: (ctx) => statScorer(ctx, (stat) => averageGap(stat), -1),
	rating: () => (track) => ((track.rating ?? 0) > 0 ? (track.rating as number) : null),
	recentlyAdded: () => (track) => (addedAt(track) > 0 ? addedAt(track) : null),
};

function tieBreak(ctx: RankContext, a: Track, b: Track): number {
	const statA = ctx.stats.get(a.uid);
	const statB = ctx.stats.get(b.uid);
	return (
		(statB?.plays ?? 0) - (statA?.plays ?? 0) ||
		(statB?.ms ?? 0) - (statA?.ms ?? 0) ||
		(statB?.last ?? 0) - (statA?.last ?? 0) ||
		addedAt(b) - addedAt(a)
	);
}

function parseRank(spec: RankSpec): { rank: Rank; invert: boolean } {
	return spec.startsWith("-")
		? { rank: spec.slice(1) as Rank, invert: true }
		: { rank: spec as Rank, invert: false };
}

const zeroWhenUnplayed: Rank[] = ["rate", "totalTime", "consistency"];

function buildScorer(ctx: RankContext, spec: RankSpec): Scorer {
	const { rank, invert } = parseRank(spec);
	const scorer = rankers[rank](ctx);
	if (!invert) return scorer;
	return (track) => {
		const value = scorer(track);
		if (value === null) {
			return zeroWhenUnplayed.includes(rank) && !ctx.stats.has(track.uid) ? 0 : null;
		}
		return -value;
	};
}

function rankTracks(ctx: RankContext, ranks: RankSpec[]): Track[] {
	const scorers = ranks.map((spec) => buildScorer(ctx, spec));
	const scored: { track: Track; values: number[] }[] = [];
	for (const track of ctx.tracks) {
		const values: number[] = [];
		let valid = true;
		for (const scorer of scorers) {
			const value = scorer(track);
			if (value === null) {
				valid = false;
				break;
			}
			values.push(value);
		}
		if (valid) scored.push({ track, values });
	}
	scored.sort((a, b) => {
		for (let i = 0; i < a.values.length; i++) {
			const diff = b.values[i] - a.values[i];
			if (diff !== 0) return diff;
		}
		return tieBreak(ctx, a.track, b.track);
	});
	return scored.map((entry) => entry.track);
}

function recentlyAddedVaried(tracks: Track[], limit: number): Track[] {
	const sorted = tracks.filter((track) => addedAt(track) > 0).sort((a, b) => addedAt(b) - addedAt(a));
	const dayCutoff = nowSeconds() - DAY_SECONDS;
	const lastDay = sorted.filter((track) => addedAt(track) >= dayCutoff);
	if (lastDay.length > limit) return pickVaried(lastDay, limit);
	return sorted;
}

const scrobbleRanks: Rank[] = ["rate", "totalTime", "consistency", "gap"];

function buildStats(
	scrobbles: ScrobbleRow[],
	window: Window | null,
	minPlayMs: number,
): Map<string, PlayStats> {
	const stats = new Map<string, PlayStats>();
	for (const scrobble of scrobbles) {
		if (!inWindow(scrobble.timestamp, window) || scrobble.duration_played < minPlayMs) continue;
		const entry = stats.get(scrobble.track_uid) ?? { plays: 0, ms: 0, last: 0, stamps: [] };
		entry.plays += 1;
		entry.ms += scrobble.duration_played;
		entry.last = Math.max(entry.last, scrobble.timestamp);
		entry.stamps.push(scrobble.timestamp);
		stats.set(scrobble.track_uid, entry);
	}
	return stats;
}

function computeWindowDays(window: Window | null, stats: Map<string, PlayStats>, now: number): number {
	const end = window?.to ?? now;
	let earliest = Infinity;
	for (const stat of stats.values()) {
		for (const stamp of stat.stamps) earliest = Math.min(earliest, stamp);
	}
	const start = window && window.from > 0 ? window.from : isFinite(earliest) ? earliest : end;
	return Math.max(1, (end - start) / DAY_SECONDS);
}

async function buildTracks(
	def: PlaylistDefinition,
	allTracks: Track[],
	loadScrobbles: () => Promise<ScrobbleRow[]>,
	now: number,
): Promise<Track[]> {
	const filters = def.filters;
	const ranks = Array.isArray(def.rank) ? def.rank : [def.rank];
	const minPlayMs = filters.minPlayMs ?? DEFAULT_MIN_PLAY_MS;
	let tracks = allTracks.filter((track) => trackFilters.every((filter) => filter(track, filters, now)));
	let stats = new Map<string, PlayStats>();
	const window = resolveRange(filters.scrobbleRange, now);
	const unplayedWindow = resolveRange(filters.unplayedRange, now);

	if (ranks.some((spec) => scrobbleRanks.includes(parseRank(spec).rank)) || window || unplayedWindow) {
		const scrobbles = await loadScrobbles();
		stats = buildStats(scrobbles, window, minPlayMs);
		if (window) tracks = tracks.filter((track) => stats.has(track.uid));
		if (unplayedWindow) {
			const recent = buildStats(scrobbles, unplayedWindow, minPlayMs);
			tracks = tracks.filter((track) => !recent.has(track.uid));
		}
	}

	if (ranks.length === 1 && ranks[0] === "recentlyAdded") {
		return recentlyAddedVaried(tracks, def.maxTracks).slice(0, def.maxTracks);
	}

	const ctx: RankContext = {
		tracks,
		stats,
		windowDays: computeWindowDays(window, stats, now),
		limit: def.maxTracks,
	};
	return rankTracks(ctx, ranks).slice(0, def.maxTracks);
}

function isDue(refresh: Refresh, last: number, now: number): boolean {
	if (refresh === "open") return true;
	if (refresh === "never") return false;
	return now - last >= refresh.days * DAY_SECONDS;
}

async function loadRefreshTimes(): Promise<Map<string, number>> {
	const settings = await invoke<{ key: string; value: string }[]>("get_settings");
	const times = new Map<string, number>();
	for (const setting of settings) {
		if (!setting.key.startsWith(REFRESH_KEY_PREFIX)) continue;
		times.set(setting.key.slice(REFRESH_KEY_PREFIX.length), Number(setting.value) || 0);
	}
	return times;
}

async function saveRefreshTime(uid: string, time: number): Promise<void> {
	await invoke("save_setting", { key: REFRESH_KEY_PREFIX + uid, value: String(time) });
}

async function resolveArtwork(artwork: string | null): Promise<string | null> {
	if (!artwork) return null;
	if (/^([a-zA-Z]:[\\/]|[\\/]|https?:)/.test(artwork)) return artwork;
	try {
		const path = await resolveResource(`${ARTWORK_DIR}/${artwork}`);
		return path.replace(/^\\\\\?\\/, "");
	} catch (e) {
		console.error(`explore artwork ${artwork} could not be resolved`, e);
		return null;
	}
}

async function upsertPlaylist(
	def: PlaylistDefinition,
	tracks: Track[],
	existing: Playlist | null,
): Promise<Playlist | null> {
	const entries: TrackEntry[] = tracks.map((track, i) => ({
		uid: track.uid,
		name: track.title ?? "Unknown Title",
		order: i + 1,
	}));
	const tracksJson = JSON.stringify(entries);
	const artwork = await resolveArtwork(def.artwork);

	if (existing) {
		const update: Record<string, unknown> = {
			title: def.title,
			description: def.description,
			owner: def.owner,
			tracks: tracksJson,
		};
		if (artwork !== null) update.artwork_path = artwork;
		await invoke("update_playlist_entry", { uid: def.uid, update });
		return entries.length === 0 ? null : await getPlaylist(def.uid);
	}

	if (entries.length === 0) return null;

	await invoke("create_playlist_entry", {
		playlist: {
			uid: def.uid,
			title: def.title,
			description: def.description,
			owner: def.owner,
			tracks: tracksJson,
			artwork_path: artwork,
			folder: null,
			version: 1,
		},
	});
	return await getPlaylist(def.uid);
}

async function setExplorePlaylists(): Promise<void> {
	try {
		const now = nowSeconds();
		const [allTracks, refreshTimes] = await Promise.all([getTracks(), loadRefreshTimes()]);

		const loadScrobbles = (): Promise<ScrobbleRow[]> => getScrobbles();

		const playlists: Playlist[] = [];
		for (const def of playlistDefinitions) {
			try {
				const existing = (await getPlaylist(def.uid)) ?? null;
				if (existing && !isDue(def.refresh, refreshTimes.get(def.uid) ?? 0, now)) {
					playlists.push(existing);
					continue;
				}
				const tracks = await buildTracks(def, allTracks, loadScrobbles, now);
				const playlist = await upsertPlaylist(def, tracks, existing);
				await saveRefreshTime(def.uid, now);
				if (playlist) playlists.push(playlist);
			} catch (e) {
				console.error(`explore playlist ${def.uid} failed`, e);
			}
		}

		explorePlaylists.splice(0, explorePlaylists.length, ...playlists);
		await reloadLibrary("playlists");
	} catch (e) {
		console.error("setExplorePlaylists failed", e);
	}
}

export { explorePlaylists, setExplorePlaylists };