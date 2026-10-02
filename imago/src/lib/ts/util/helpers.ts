import type { Track, UserOptions } from "$ts/util/types";
import { convertFileSrc } from "@tauri-apps/api/core";

export function formatRating(rating: number | null): string {
	if (rating === null) return "—";
	return rating.toFixed(1);
}

export function formatDuration(ms: number | null): string {
	if (ms === null) return "—";
	const totalSeconds = Math.floor(ms / 1000);
	const minutes = Math.floor(totalSeconds / 60);
	const seconds = totalSeconds % 60;
	return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function totalDuration(tracks: Track[]): string {
	const total = tracks.reduce((sum, t) => sum + (t.duration_ms ?? 0), 0);
	return formatDuration(total);
}

export async function getArtworkColor(bytes: number[], opacity = 1): Promise<string> {
	return new Promise((resolve) => {
		const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" })
		const url = URL.createObjectURL(blob)
		const img = new Image()
		img.crossOrigin = "anonymous"
		img.src = url

		img.onload = () => {
			const canvas = document.createElement("canvas")
			canvas.width = 10
			canvas.height = 10
			const ctx = canvas.getContext("2d")!
			ctx.drawImage(img, 0, 0, 10, 10)

			const data = ctx.getImageData(0, 0, 10, 10).data
			let r = 0, g = 0, b = 0, count = 0

			for (let i = 0; i < data.length; i += 4) {
				r += data[i]
				g += data[i + 1]
				b += data[i + 2]
				count++
			}

			r = Math.floor(r / count)
			g = Math.floor(g / count)
			b = Math.floor(b / count)

			URL.revokeObjectURL(url)
			resolve(`rgba(${r}, ${g}, ${b}, ${opacity})`)
		}

		img.onerror = () => {
			URL.revokeObjectURL(url)
			resolve(`rgba(30, 30, 30, ${opacity})`)
		}
	})
}
export async function getArtworkColorFromPath(path: string, opacity = 1): Promise<string> {
	try {
		const url = convertFileSrc(path);
		const res = await fetch(url);
		const buf = await res.arrayBuffer();
		return getArtworkColor(Array.from(new Uint8Array(buf)), opacity);
	} catch {
		return `rgba(30, 30, 30, ${opacity})`;
	}
}

export function clamp(num: number, min: number, max: number): number {
  return num <= min ? min : num >= max ? max : num;
}

export function serializeUserOptions(opts: UserOptions): string {
	return JSON.stringify(opts);
}

export function isReadOnly(entity: { source_lib_uid: string | null; is_local_override: boolean | null }): boolean {
	if (!entity.source_lib_uid) return false;
	return !entity.is_local_override;
}