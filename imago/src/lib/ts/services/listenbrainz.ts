import { invoke } from "@tauri-apps/api/core";

export async function listenbrainzIsConnected(): Promise<boolean> {
	return invoke<boolean>("listenbrainz_connection_status");
}

export async function listenbrainzConnect(token: string): Promise<void> {
	await invoke("listenbrainz_connect_cmd", { token });
}

export async function listenbrainzDisconnect(): Promise<void> {
	await invoke("listenbrainz_disconnect_cmd");
}

export async function listenbrainzValidateToken(token: string): Promise<boolean> {
	return invoke<boolean>("listenbrainz_validate_token_cmd", { token });
}

export async function importSpotifyHistory(zipPath: string): Promise<{ imported: number; skipped: number }> {
	return invoke<{ imported: number; skipped: number }>("import_spotify_history_cmd", { zipPath });
}
