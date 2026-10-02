import type { Track, TrackAlbumEntry, PlaylistTrackEntry, AudioCatagories, UserOptions, Artist } from "$ts/util/types";

export function parseUidType(uid: string): AudioCatagories {
	if (uid.startsWith("t-")) return "track";
	if (uid.startsWith("ar-")) return "artist";
	if (uid.startsWith("a-")) return "album";
	if (uid.startsWith("p-")) return "playlist";
	return "unknown";
}

export function parseArtists(artistsJson: string | null): string[] {
	if (!artistsJson) return [];
	try {
		const parsed = JSON.parse(artistsJson);
		if (!Array.isArray(parsed)) return [];
		return parsed;
	} catch {
		return [];
	}
}

export function parseArtistsToString(artistsJson: string | null): string {
	const artists = parseArtists(artistsJson);
	if (artists.length === 0) return "Unknown Artist";
	return artists.map(name => name || "Unknown Artist").join(", ");
}

export function parseAlbumEntries(albumsJson: string | null): TrackAlbumEntry[] {
	if (!albumsJson) return [];
	try {
		const parsed = JSON.parse(albumsJson);
		if (!Array.isArray(parsed)) return [];
		return parsed;
	} catch {
		return [];
	}
}

export function parseAlbumsToString(albumsJson: string | null): string {
	const albums = parseAlbumEntries(albumsJson);
	if (albums.length === 0) return "Unknown Albums";
	return albums.map(album => album.name || "Unknown Albums").join(", ");
}

export function parseTags(tags: string | null): string[] {
	if (!tags) return [];
	try {
		const parsed = JSON.parse(tags);
		if (!Array.isArray(parsed)) return [];
		return parsed;
	} catch {
		return [];
	}
}