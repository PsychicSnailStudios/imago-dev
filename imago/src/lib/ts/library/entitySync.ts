import { invoke } from "@tauri-apps/api/core";
import { showPrunePicker } from "$ts/ui/pruneDialog.svelte";
import type {TrackAlbumEntry} from "$ts/util/types";

// ─── Internal helpers ─────────────────────────────────────────────────────────

function parseJsonArray(val: string | null | undefined): any[] {
	if (!val) return [];
	try {
		const parsed = JSON.parse(val);
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

// ─── Rename propagation ───────────────────────────────────────────────────────

/**
 * Rewrite every occurrence of oldName in artists/album_artist across all
 * tracks and albums. Call this after an artist record's name changes.
 */
export async function renameArtistInLibrary(
	oldName: string,
	newName: string
): Promise<void> {
	if (!oldName || !newName || oldName.toLowerCase() === newName.toLowerCase()) return;
	await invoke("rename_artist_in_library_cmd", { oldName, newName });
}

/**
 * Rewrite the name field inside every track's albums JSON entries that
 * belong to the renamed album. When albumUid is given, entries are matched
 * by uid; otherwise by name and album artist. Call after an album record's
 * title changes.
 */
export async function renameAlbumInTracks(
	oldName: string,
	newName: string,
	albumArtist: string,
	albumUid: string | null = null
): Promise<void> {
	if (!oldName || !newName || oldName.toLowerCase() === newName.toLowerCase()) return;
	await invoke("rename_album_in_tracks_cmd", {
		oldName,
		newName,
		albumArtist: albumArtist ?? "",
		albumUid,
	});
}

// ─── AKA merge ────────────────────────────────────────────────────────────────

/**
 * For each AKA string, check if another artist record exists with that as
 * their name. If so, register a uid remap (old → surviving), then delete
 * the duplicate. Track/album name strings are left as-is since they still
 * point to the surviving artist's name via the remap.
 *
 * Returns the list of artist names that were merged so the caller can inform
 * the user if desired.
 */
export async function mergeArtistAkas(
	survivingUid: string,
	akaNames: string[]
): Promise<string[]> {
	if (akaNames.length === 0) return [];

	const allArtists = await invoke<any[]>("get_artists");
	const merged: string[] = [];

	for (const akaName of akaNames) {
		const akaLower = akaName.toLowerCase();
		const duplicate = allArtists.find(
			(a) => a.uid !== survivingUid && (a.name ?? "").toLowerCase() === akaLower
		);
		if (!duplicate) continue;

		// Register remap so any stored uid references resolve to the survivor
		await invoke("add_uid_remap", {
			oldUid: duplicate.uid,
			newUid: survivingUid,
			entityType: "artist",
		});

		// Delete the duplicate record
		await invoke("delete_artist_entry", { uid: duplicate.uid });
		merged.push(akaName);
	}

	return merged;
}

// ─── Artist sync ──────────────────────────────────────────────────────────────

export async function syncArtists(artistNames: string[]): Promise<void> {
	if (artistNames.length === 0) return;

	const allArtists = await invoke<any[]>("get_artists");
	const existingNames = new Set(allArtists.map((a) => (a.name ?? "").toLowerCase()));
	const seen = new Set<string>();

	for (const name of artistNames) {
		const lower = name.toLowerCase();
		if (existingNames.has(lower) || seen.has(lower)) continue;
		seen.add(lower);

		await invoke("create_artist_entry", {
			artist: {
				uid: `ar-${crypto.randomUUID()}`,
				name,
				aka: null,
				about: null,
				tags: JSON.stringify([]),
				genres: JSON.stringify([]),
				websites: null,
				members: null,
				profile_art_blob: null,
				profile_art_path: null,
				banner_art_blob: null,
				banner_art_path: null,
			},
		});
	}
}

/**
 * Delete artist records for any name in removedNames that is no longer
 * referenced by any track or album. Shows a confirmation dialog first
 * listing the artists that would be removed — the user can cancel to keep them.
 *
 * Returns true if the user confirmed (or there was nothing to prune),
 * false if the user cancelled.
 */
export async function pruneArtists(removedNames: string[]): Promise<boolean> {
	if (removedNames.length === 0) return true;

	const [allArtists, allTracks, allAlbums] = await Promise.all([
		invoke<any[]>("get_artists"),
		invoke<any[]>("get_tracks"),
		invoke<any[]>("get_albums"),
	]);

	const usedNames = new Set<string>();
	for (const t of allTracks) {
		parseJsonArray(t.artists).forEach((n: string) => usedNames.add(n.toLowerCase()));
		if (t.album_artist) usedNames.add((t.album_artist as string).toLowerCase());
	}
	for (const a of allAlbums) {
		parseJsonArray(a.artists).forEach((n: string) => usedNames.add(n.toLowerCase()));
		if (a.album_artist) usedNames.add((a.album_artist as string).toLowerCase());
	}

	const nameToUid = new Map(allArtists.map((a) => [a.name.toLowerCase(), a.uid as string]));

	const toPrune = removedNames.filter((name) => {
		const lower = name.toLowerCase();
		return !usedNames.has(lower) && nameToUid.has(lower);
	});

	if (toPrune.length === 0) return true;

	const chosen = await showPrunePicker(toPrune);
	if (chosen === null) return false;
	if (chosen.length === 0) return true;

	const uids = chosen
		.map((name) => nameToUid.get(name.toLowerCase()))
		.filter((uid): uid is string => !!uid);
	if (uids.length > 0) {
		await invoke("delete_artists_cmd", { uids });
	}

	return true;
}

// ─── Album sync ───────────────────────────────────────────────────────────────

export async function syncAlbums(
	cleanedAlbums: TrackAlbumEntry[],
	trackUid: string,
	trackTitle: string,
	trackAlbumArtist: string
): Promise<TrackAlbumEntry[]> {
	const allAlbums = await invoke<any[]>("get_albums");
	const finalAlbumEntries: TrackAlbumEntry[] = [];

	for (const entry of cleanedAlbums) {
		const nameLower = entry.name.toLowerCase();
		const artistLower = trackAlbumArtist.toLowerCase();

		const match = allAlbums.find((a) => {
			const titleMatch = (a.title ?? "").toLowerCase() === nameLower;
			const artistMatch = (a.album_artist ?? "").toLowerCase() === artistLower;
			return titleMatch && artistMatch;
		});

		if (match) {
			const existingTracks = parseJsonArray(match.tracks);
			const alreadyIn = existingTracks.some((t: any) => t.uid === trackUid);

			if (alreadyIn) {
				const updated = existingTracks.map((t: any) =>
					t.uid === trackUid ? { ...t, track_number: entry.track_number ?? null } : t
				);
				await invoke("update_album_entry", { uid: match.uid, update: { tracks: JSON.stringify(updated) } });
			} else {
				existingTracks.push({ uid: trackUid, name: trackTitle, track_number: entry.track_number ?? null });
				await invoke("update_album_entry", { uid: match.uid, update: { tracks: JSON.stringify(existingTracks) } });
			}

			finalAlbumEntries.push({ uid: match.uid, name: entry.name, track_number: entry.track_number, disc: entry.disc });
		} else {
			const newAlbum = {
				uid: `a-${crypto.randomUUID()}`,
				title: entry.name,
				album_artist: trackAlbumArtist || null,
				tracks: JSON.stringify([{ uid: trackUid, name: trackTitle, track_number: entry.track_number ?? null }]),
				artists: null,
				format: null,
				rating: null,
				release_date: null,
				tags: JSON.stringify([]),
				genres: JSON.stringify([]),
				credits: null,
				label: null,
				artwork_blob: null,
				artwork_path: null,
			};

			await invoke("create_album_entry", { album: newAlbum });

			const refreshed = await invoke<any[]>("get_albums");
			const created = refreshed.find((a) =>
				(a.title ?? "").toLowerCase() === entry.name.toLowerCase() &&
				(a.album_artist ?? "").toLowerCase() === trackAlbumArtist.toLowerCase()
			);

			finalAlbumEntries.push({
				uid: created?.uid ?? newAlbum.uid,
				name: entry.name,
				track_number: entry.track_number,
				disc: entry.disc,
			});
		}
	}

	return finalAlbumEntries;
}

/**
 * Remove this track from albums it was previously on.
 * If an album ends up with zero tracks after removal, delete the album record entirely.
 */
export async function removeTrackFromOldAlbums(
	trackUid: string,
	removedAlbumUids: string[]
): Promise<void> {
	for (const albumUid of removedAlbumUids) {
		const album = await invoke<any | null>("get_album", { uid: albumUid });
		if (!album) continue;

		const remaining = parseJsonArray(album.tracks).filter((t: any) => t.uid !== trackUid);

		if (remaining.length === 0) {
			await invoke("delete_album_entry", { uid: albumUid });
		} else {
			await invoke("update_album_entry", { uid: albumUid, update: { tracks: JSON.stringify(remaining) } });
		}
	}
}

// ─── Stub track creation ──────────────────────────────────────────────────────

export async function createStubTrack(
	title: string,
	artist: string,
	album: string,
	trackNumber: number | null,
	durationMs: number | null,
	year: string | null
): Promise<string> {
	const uid = `t-${crypto.randomUUID()}`;

	const albumArtist = artist;
	const albumEntry = album
		? JSON.stringify([{ uid: "", name: album, track_number: trackNumber ?? null }])
		: null;

	await invoke("add_track", {
		track: {
			uid,
			path: "",
			last_modified: 0,
			title,
			artists: JSON.stringify([artist]),
			album_artist: albumArtist,
			albums: albumEntry,
			genres: null,
			year,
			rating: null,
			tags: "[]",
			duration_ms: durationMs,
			bpm: null,
			key: null,
			credits: null,
			label: null,
			artwork_blob: null,
			artwork_path: null,
		},
	});

	if (album) {
		const albumEntries: TrackAlbumEntry[] = [{ uid: "", name: album, track_number: trackNumber ?? null, disc: null }];
		const synced = await syncAlbums(albumEntries, uid, title, albumArtist);
		await invoke("update_track_metadata", {
			uid,
			update: { albums: JSON.stringify(synced) },
		});
	}

	await syncArtists([artist]);

	return uid;
}

/**
 * Remove an album uid from the `albums` JSON field on every track that
 * references it. Call this before or after deleting the album record itself.
 */
export async function removeAlbumFromLinkedTracks(albumUid: string): Promise<void> {
	const allTracks = await invoke<any[]>("get_tracks");

	for (const track of allTracks) {
		const albumEntries: any[] = parseJsonArray(track.albums);
		const filtered = albumEntries.filter((e: any) => e.uid !== albumUid);
		if (filtered.length === albumEntries.length) continue;

		await invoke("update_track_metadata", {
			uid: track.uid,
			update: { albums: JSON.stringify(filtered) },
		});
	}
}