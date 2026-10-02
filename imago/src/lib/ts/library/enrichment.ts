import { invoke } from "@tauri-apps/api/core";

export async function enrichTrack(uid: string): Promise<void> {
	await invoke("enrich_track", { uid });
}

export async function enrichTracks(uids: string[]): Promise<void> {
	for (const uid of uids) {
		await invoke("enrich_track", { uid });
	}
}

export async function enrichAlbum(uid: string): Promise<void> {
	await invoke("enrich_album", { uid });
}

export async function enrichAlbums(uids: string[]): Promise<void> {
	for (const uid of uids) {
		await invoke("enrich_album", { uid });
	}
}

export async function enrichArtist(uid: string): Promise<void> {
	await invoke("enrich_artist", { uid });
}

export async function enrichArtists(uids: string[]): Promise<void> {
	for (const uid of uids) {
		await invoke("enrich_artist", { uid });
	}
}

export async function fetchLyrics(uid: string): Promise<void> {
	await invoke("fetch_track_lyrics", { uid });
}

export async function enrichAllAlbums(): Promise<void> {
	await invoke("enrich_all_albums");
}

export async function enrichAllArtists(): Promise<void> {
	await invoke("enrich_all_artists");
}

export async function enrichAllTracks(): Promise<void> {
	await invoke("enrich_all");
}