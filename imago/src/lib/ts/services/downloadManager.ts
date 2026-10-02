import { invoke } from "@tauri-apps/api/core";
import { toast } from "svelte-sonner";

export async function downloadTrack(uid: string): Promise<string | null> {
	try {
		const path = await invoke<string>("download_track_cmd", { uid });
		toast.success("Track downloaded");
		return path;
	} catch (e) {
		toast.error(`Download failed: ${e}`);
		return null;
	}
}

export async function downloadAlbum(uid: string, subscribe = true): Promise<void> {
	const toastId = toast.loading("Downloading album...");
	try {
		await invoke("download_album_tracks_cmd", { uid, subscribe });
		toast.dismiss(toastId);
		toast.success("Album downloaded");
	} catch (e) {
		toast.dismiss(toastId);
		toast.error(`Album download failed: ${e}`);
	}
}

export async function downloadPlaylist(uid: string, subscribe = true): Promise<void> {
	const toastId = toast.loading("Downloading playlist...");
	try {
		await invoke("download_playlist_tracks_cmd", { uid, subscribe });
		toast.dismiss(toastId);
		toast.success("Playlist downloaded");
	} catch (e) {
		toast.dismiss(toastId);
		toast.error(`Playlist download failed: ${e}`);
	}
}

export async function unsubscribeOffline(uid: string): Promise<void> {
	try {
		await invoke("unsubscribe_offline_cmd", { uid });
	} catch (e) {
		toast.error(`Failed to unsubscribe: ${e}`);
	}
}

export async function syncOfflineSubscriptions(): Promise<void> {
	try {
		await invoke("sync_offline_subscriptions_cmd");
	} catch {}
}
