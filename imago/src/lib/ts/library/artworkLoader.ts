import { convertFileSrc } from "@tauri-apps/api/core";
import type { AudioCatagories } from "$ts/util/types";

export type ArtworkQuality = "card" | "large";
export type ArtworkKind = Exclude<AudioCatagories, "unknown"> | "banner";

export const artworkCache = new Map<string, string | null>();
export const artworkBytesCache = new Map<string, number[]>();
export const artworkInflight = new Map<string, Promise<string | null>>();
export const artworkColorCache = new Map<string, string>();
export const colorInflight = new Map<string, Promise<string>>();

export function artworkVersion(thumb: string | null | undefined): string {
	if (!thumb) return "n";
	let hash = 5381;
	for (let i = 0; i < thumb.length; i++) {
		hash = ((hash << 5) + hash + thumb.charCodeAt(i)) | 0;
	}
	return (hash >>> 0).toString(36);
}

export function artworkUrl(
	type: ArtworkKind,
	uid: string,
	quality: ArtworkQuality = "card",
	version = "0"
): string {
	return `${convertFileSrc(`${type}/${uid}`, "artwork")}?s=${quality}&v=${version}`;
}

export async function prefetchArtwork(_uids: string[], _type: AudioCatagories): Promise<void> {}

export async function fetchArtwork(uid: string, type: AudioCatagories): Promise<string | null> {
	if (type === "unknown") return null;
	return artworkUrl(type, uid);
}

export function fetchArtworkColor(uid: string, type: AudioCatagories, version = "0"): Promise<string> {
	const fallback = "var(--muted)";
	if (type === "unknown") return Promise.resolve(fallback);

	const cacheKey = `${type}:${uid}`;

	const cached = artworkColorCache.get(cacheKey);
	if (cached) return Promise.resolve(cached);

	const running = colorInflight.get(cacheKey);
	if (running) return running;

	const promise = new Promise<string>((resolve) => {
		const img = new Image();
		img.crossOrigin = "anonymous";
		img.decoding = "async";

		img.onload = () => {
			try {
				const canvas = document.createElement("canvas");
				canvas.width = 10;
				canvas.height = 10;
				const ctx = canvas.getContext("2d");
				if (!ctx) return resolve(fallback);

				ctx.drawImage(img, 0, 0, 10, 10);
				const data = ctx.getImageData(0, 0, 10, 10).data;
				const pixels = data.length / 4;

				let r = 0;
				let g = 0;
				let b = 0;
				for (let i = 0; i < data.length; i += 4) {
					r += data[i];
					g += data[i + 1];
					b += data[i + 2];
				}

				const color = `rgba(${Math.round(r / pixels)}, ${Math.round(g / pixels)}, ${Math.round(b / pixels)}, 0.3)`;
				artworkColorCache.set(cacheKey, color);
				resolve(color);
			} catch {
				resolve(fallback);
			}
		};

		img.onerror = () => resolve(fallback);
		img.src = artworkUrl(type, uid, "card", version);
	}).finally(() => colorInflight.delete(cacheKey));

	colorInflight.set(cacheKey, promise);
	return promise;
}