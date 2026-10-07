import { invoke } from "@tauri-apps/api/core";

export type ScrobbleRecord = {
	uid: string;
	timestamp: number;
	track_uid: string;
	artist_uid: string;
	duration_played: number;
	did_seek: boolean;
	did_pause: boolean;
	track_name: string | null;
	track_artist: string | null;
	track_album: string | null;
	shuffle: boolean | null;
	skipped: boolean | null;
	offline: boolean | null;
	playing_local: boolean | null;
	source_uid: string | null;
};

const OVERLAP_SECONDS = 6 * 3600;

let cache: ScrobbleRecord[] | null = null;
let loading: Promise<ScrobbleRecord[]> | null = null;
let refreshing: Promise<ScrobbleRecord[]> | null = null;

export function getScrobbles(): Promise<ScrobbleRecord[]> {
	if (cache) return Promise.resolve(cache);
	loading ??= invoke<ScrobbleRecord[]>("get_scrobbles")
		.then((rows) => {
			cache = rows;
			return rows;
		})
		.finally(() => {
			loading = null;
		});
	return loading;
}

export function refreshScrobbles(): Promise<ScrobbleRecord[]> {
	if (!cache) return getScrobbles();
	if (refreshing) return refreshing;
	const since = Math.max(0, (cache[0]?.timestamp ?? 0) - OVERLAP_SECONDS);
	refreshing = invoke<ScrobbleRecord[]>("get_scrobbles_since", { since })
		.then((fresh) => {
			const ids = new Set(fresh.map((s) => s.uid));
			const kept = (cache ?? []).filter((s) => !ids.has(s.uid));
			cache = fresh.concat(kept);
			return cache;
		})
		.finally(() => {
			refreshing = null;
		});
	return refreshing;
}

export function resetScrobbleCache(): void {
	cache = null;
}
