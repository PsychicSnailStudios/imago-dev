import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";
import { offlineMode } from "$ts/store/session.svelte";
import type { Track } from "$ts/util/types";

// ─── Path classification ──────────────────────────────────────────────────────

export function isRemotePath(path: string): boolean {
	return (
		path.startsWith("http://") ||
		path.startsWith("https://") ||
		path.startsWith("\\\\") ||
		path.startsWith("//")
	);
}

export function isLocalPath(path: string): boolean {
	return path.length > 0 && !isRemotePath(path);
}

export function pathToSrc(path: string): string {
	if (path.startsWith("http://") || path.startsWith("https://")) {
		return path;
	}
	return convertFileSrc(path);
}

// ─── Track data helpers ───────────────────────────────────────────────────────

export interface TrackData {
	bitrate?: number | null;
	format?: string | null;
	is_ghost?: boolean;
}

export interface RemoteData {
	bitrate?: number | null;
	format?: string | null;
	is_ghost?: boolean;
}

export function parseTrackData(json: string | null): TrackData {
	try { return json ? JSON.parse(json) : {}; } catch { return {}; }
}

export function isGhostTrack(track: Track): boolean {
	const data = parseTrackData(track.track_data);
	if (data.is_ghost === true) return true;
	if (track.remote_path && track.remote_path.length > 0) return false;
	if (!track.path || track.path === "" || track.path === track.uid) return true;
	return false;
}

export async function markGhost(track: Track, isGhost: boolean) {
	try {
		const data = parseTrackData(track.track_data);
		data.is_ghost = isGhost;
		track.track_data = JSON.stringify(data);
		let uid = track.uid;
		await invoke("update_track_metadata", {
			uid,
			update: { track_data: track.track_data },
		});
	} catch {}
}

// ─── Offline mode ─────────────────────────────────────────────────────────────

let offlineModeToastShown = false;

export function triggerOfflineMode() {
	offlineMode.offline = true;
	if (offlineModeToastShown) return;
	offlineModeToastShown = true;

	toast.warning("No internet connection — going into offline mode.", {
		duration: 10000,
		action: {
			label: "Stay online",
			onClick: () => {
				offlineMode.offline = false;
				offlineModeToastShown = false;
			},
		},
		onDismiss: () => {
			offlineModeToastShown = false;
		},
	});
}

export async function checkOnline(): Promise<boolean> {
	try {
		const res = await fetch("https://www.gstatic.com/generate_204", {
			method: "HEAD",
			cache: "no-store",
			signal: AbortSignal.timeout(3000),
		});
		return res.ok || res.status === 204;
	} catch {
		return false;
	}
}

// ─── Settings cache ───────────────────────────────────────────────────────────

let cachedAutoOffline: boolean | null = null;

export async function getAutoOfflineSetting(): Promise<boolean> {
	if (cachedAutoOffline !== null) return cachedAutoOffline;
	try {
		const settings = await invoke<Array<{ key: string; value: string }>>("get_settings");
		cachedAutoOffline = settings.find(s => s.key === "offline_mode_auto")?.value === "true";
		return cachedAutoOffline;
	} catch {
		return false;
	}
}
