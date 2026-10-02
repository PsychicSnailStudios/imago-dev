import { invoke } from "@tauri-apps/api/core";
import { getArtworkColor } from "$ts/util/helpers";
import type { AudioCatagories } from "$ts/util/types";

export const artworkCache = new Map<string, string | null>();
export const artworkBytesCache = new Map<string, number[]>();
export const artworkInflight = new Map<string, Promise<string | null>>();
export const artworkColorCache = new Map<string, string>();
export const colorInflight = new Map<string, Promise<string>>();

const commandMap: Record<string, string> = {
	track: "get_track_artwork",
	album: "get_album_artwork",
	artist: "get_artist_profile_art",
	playlist: "get_playlist_artwork",
};

// Prefetch artwork for a set of uids. Call this from view-level code when a
// collection view mounts (e.g. the Albums page), rather than at library load time.
export async function prefetchArtwork(uids: string[], type: AudioCatagories): Promise<void> {
	const command = commandMap[type];
	if (!command) return;

	const uncached = uids.filter(uid => !artworkCache.has(`${type}:${uid}`));
	if (uncached.length === 0) return;

	await Promise.all(
		uncached.map(async (uid) => {
			const cacheKey = `${type}:${uid}`;
			if (artworkInflight.has(cacheKey)) return artworkInflight.get(cacheKey);

			const promise = invoke<number[] | null>(command, { uid }).then((bytes) => {
				let url: string | null = null;
				if (bytes) {
					const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
					url = URL.createObjectURL(blob);
					artworkBytesCache.set(cacheKey, bytes);
				}
				artworkCache.set(cacheKey, url);
				artworkInflight.delete(cacheKey);
				return url;
			}).catch(() => {
				artworkCache.set(cacheKey, null);
				artworkInflight.delete(cacheKey);
				return null;
			});

			artworkInflight.set(cacheKey, promise);
			return promise;
		})
	);
}

export async function fetchArtwork(uid: string, type: AudioCatagories): Promise<string | null> {
	const cacheKey = `${type}:${uid}`;

	const cached = artworkCache.get(cacheKey);
	if (cached !== undefined) return cached;

	if (artworkInflight.has(cacheKey)) return artworkInflight.get(cacheKey)!;

	const command = commandMap[type];
	if (!command) return null;

	const promise = invoke<number[] | null>(command, { uid }).then((bytes) => {
		let url: string | null = null;
		if (bytes) {
			const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
			url = URL.createObjectURL(blob);
			artworkBytesCache.set(cacheKey, bytes);
		}
		artworkCache.set(cacheKey, url);
		artworkInflight.delete(cacheKey);
		return url;
	}).catch(() => {
		artworkCache.set(cacheKey, null);
		artworkInflight.delete(cacheKey);
		return null;
	});

	artworkInflight.set(cacheKey, promise);
	return promise;
}

export async function fetchArtworkColor(uid: string, type: AudioCatagories): Promise<string> {
	const cacheKey = `${type}:${uid}`;

	const cached = artworkColorCache.get(cacheKey);
	if (cached) return cached;

	if (colorInflight.has(cacheKey)) return colorInflight.get(cacheKey)!;

	const command = commandMap[type];

	const promise = (async () => {
		let bytes = artworkBytesCache.get(cacheKey) ?? null;

		if (!bytes) {
			bytes = await invoke<number[] | null>(command, { uid }).then(b => {
				if (b) {
					artworkBytesCache.set(cacheKey, b);
					if (!artworkCache.has(cacheKey)) {
						const blob = new Blob([new Uint8Array(b)], { type: "image/jpeg" });
						artworkCache.set(cacheKey, URL.createObjectURL(blob));
					}
				}
				return b;
			}).catch(() => null);
		}

		if (!bytes) return "var(--muted)";

		try {
			const color = await getArtworkColor(bytes, 0.3);
			artworkColorCache.set(cacheKey, color);
			return color;
		} catch {
			return "var(--muted)";
		}
	})().finally(() => colorInflight.delete(cacheKey));

	colorInflight.set(cacheKey, promise);
	return promise;
}