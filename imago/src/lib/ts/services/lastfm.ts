// Last.fm connection helpers
// Import and call these from your audioManager or UI components.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { listen } from "@tauri-apps/api/event";

// ── Connection state ──────────────────────────────────────────────────────────

export async function lastfmIsConnected(): Promise<boolean> {
	return invoke<boolean>("lastfm_connection_status");
}

// ── Auth ──────────────────────────────────────────────────────────────────────

/**
 * Begin Last.fm OAuth. Opens the Last.fm auth page in the system browser.
 * After the user grants access, Last.fm redirects to sonodex://lastfm-callback?token=...
 * which the Rust deep-link handler picks up automatically.
 *
 * Listen for the "lastfm:connected" event to know when auth completes.
 */
export async function connectLastfm(): Promise<void> {
	const url = await invoke<string>("lastfm_get_auth_url");
	await open(url);
}

export async function disconnectLastfm(): Promise<void> {
	await invoke("lastfm_disconnect_cmd");
}

// ── Scrobbling ────────────────────────────────────────────────────────────────

/**
 * Send a "now playing" update. Call when a track starts playing.
 * Silently no-ops if Last.fm is not connected.
 */
export async function updateNowPlaying(trackUid: string): Promise<void> {
	try {
		await invoke("update_now_playing", { uid: trackUid });
	} catch (e) {
		console.warn("[lastfm] updateNowPlaying failed:", e);
	}
}

/**
 * Scrobble a track. Call after ≥30 seconds of playback or ≥50% completion.
 * Silently no-ops if Last.fm is not connected.
 */
export async function scrobbleTrack(trackUid: string): Promise<void> {
	try {
		await invoke("scrobble_track", { uid: trackUid });
	} catch (e) {
		console.warn("[lastfm] scrobble failed:", e);
	}
}

/**
 * Listen for the connected event emitted after a successful deep-link callback.
 * Returns an unlisten function — call it on component unmount.
 */
export async function onLastfmConnected(cb: () => void) {
	return listen("lastfm:connected", cb);
}
