import { parseTags } from "$ts/util/parsers";
import type { Track, Album, Artist, Playlist } from "$ts/util/types";

export type SearchFilters = {
	tags: string[];
	genres: string[];
	yearMin: number | null;
	yearMax: number | null;
	minRating: number;
	formats: string[];
	libraries: string[];
};

export const FILE_FORMATS = ["MP3", "FLAC", "M4A", "AAC", "WAV", "AIFF", "OGG", "OPUS"];

export function createEmptyFilters(): SearchFilters {
	return {
		tags: [],
		genres: [],
		yearMin: null,
		yearMax: null,
		minRating: 0,
		formats: [],
		libraries: [],
	};
}

export function countActiveFilters(f: SearchFilters): number {
	let count = f.tags.length + f.genres.length + f.formats.length + f.libraries.length;
	if (f.yearMin !== null || f.yearMax !== null) count++;
	if (f.minRating > 0) count++;
	return count;
}

type Common = {
	tags: string | null;
	genres: string | null;
	source_lib_uid: string | null;
};

function parseYear(value: string | null | undefined): number | null {
	const match = value?.match(/\d{4}/);
	return match ? parseInt(match[0], 10) : null;
}

function matchesList(raw: string | null, wanted: string[]): boolean {
	if (wanted.length === 0) return true;
	const have = new Set(parseTags(raw).map((v) => v.toLowerCase()));
	return wanted.some((w) => have.has(w.toLowerCase()));
}

function matchesCommon(entity: Common, f: SearchFilters, defaultLibUid: string | null): boolean {
	if (!matchesList(entity.tags, f.tags)) return false;
	if (!matchesList(entity.genres, f.genres)) return false;
	if (f.libraries.length > 0 && !f.libraries.includes(entity.source_lib_uid ?? defaultLibUid ?? "")) return false;
	return true;
}

function matchesYear(year: number | null, f: SearchFilters): boolean {
	if (f.yearMin === null && f.yearMax === null) return true;
	if (year === null) return false;
	if (f.yearMin !== null && year < f.yearMin) return false;
	if (f.yearMax !== null && year > f.yearMax) return false;
	return true;
}

function matchesRating(rating: number | null, f: SearchFilters): boolean {
	if (f.minRating <= 0) return true;
	return (rating ?? 0) >= f.minRating;
}

export function filterTracks(tracks: Track[], f: SearchFilters, defaultLibUid: string | null): Track[] {
	if (countActiveFilters(f) === 0) return tracks;
	return tracks.filter((t) => {
		if (!matchesCommon(t, f, defaultLibUid)) return false;
		if (!matchesYear(parseYear(t.year), f)) return false;
		if (!matchesRating(t.rating, f)) return false;
		if (f.formats.length > 0 && !f.formats.includes((t.format ?? "").toUpperCase())) return false;
		return true;
	});
}

export function filterAlbums(albums: Album[], f: SearchFilters, defaultLibUid: string | null): Album[] {
	if (countActiveFilters(f) === 0) return albums;
	if (f.formats.length > 0) return [];
	return albums.filter((a) => {
		if (!matchesCommon(a, f, defaultLibUid)) return false;
		if (!matchesYear(parseYear(a.release_date), f)) return false;
		if (!matchesRating(a.rating, f)) return false;
		return true;
	});
}

export function filterArtists(artists: Artist[], f: SearchFilters, defaultLibUid: string | null): Artist[] {
	if (countActiveFilters(f) === 0) return artists;
	if (f.formats.length > 0 || f.yearMin !== null || f.yearMax !== null || f.minRating > 0) return [];
	return artists.filter((a) => matchesCommon(a, f, defaultLibUid));
}

export function filterPlaylists(playlists: Playlist[], f: SearchFilters): Playlist[] {
	return countActiveFilters(f) === 0 ? playlists : [];
}
