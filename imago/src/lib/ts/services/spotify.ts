import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { listen } from "@tauri-apps/api/event";
import type { SpotifyPlaylistInfo, SpotifyPlaylistSummary } from "$ts/util/types";

export async function spotifyIsConnected(): Promise<boolean> {
	return invoke<boolean>("spotify_connection_status");
}

export async function connectSpotify(): Promise<void> {
	const [url] = await invoke<[string, string]>("spotify_get_auth_url");
	await open(url);
}

export async function disconnectSpotify(): Promise<void> {
	await invoke("spotify_disconnect_cmd");
}

export async function onSpotifyConnected(cb: () => void) {
	return listen("spotify:connected", cb);
}

export function extractSpotifyPlaylistId(input: string): string | null {
	input = input.trim();
	const urlMatch = input.match(/playlist\/([A-Za-z0-9]+)/);
	if (urlMatch) return urlMatch[1];
	if (/^[A-Za-z0-9]{22}$/.test(input)) return input;
	return null;
}

export async function getSpotifyPlaylists(): Promise<SpotifyPlaylistSummary[]> {
	return invoke<SpotifyPlaylistSummary[]>("spotify_get_playlists_cmd");
}

export async function getSpotifyPlaylistInfo(playlistId: string): Promise<SpotifyPlaylistInfo> {
	return invoke<SpotifyPlaylistInfo>("spotify_get_playlist_info_cmd", { playlistId });
}

export async function importSpotifyPlaylist(
	spotifyPlaylistId: string,
	playlistName: string,
	owner?: string,
): Promise<string> {
	return invoke<string>("spotify_import_playlist_cmd", {
		spotifyPlaylistId,
		playlistName,
		owner: owner ?? null,
	});
}

export async function spotifyEnrichTrack(uid: string): Promise<void> {
	await invoke("spotify_enrich_track_cmd", { uid });
}

export async function spotifyEnrichAlbum(uid: string): Promise<void> {
	await invoke("spotify_enrich_album_cmd", { uid });
}

export async function spotifyEnrichArtist(uid: string): Promise<void> {
	await invoke("spotify_enrich_artist_cmd", { uid });
}