import { Disc, Disc3, Videotape } from "lucide-svelte";
import type { Track, TrackAlbumEntry } from "$ts/util/types";

export type DiscBreakEntry = {
	label: string;
	Icon: any;
};

function sideLabel(discNumber: number, unitName: string): string {
	const sidesPerUnit = 2;
	const unitIndex = Math.floor((discNumber - 1) / sidesPerUnit);
	const sideIndex = (discNumber - 1) % sidesPerUnit;
	const sideLetter = String.fromCharCode(65 + sideIndex);
	if (unitIndex === 0) {
		return `Side ${sideLetter}`;
	}
	return `${unitName} ${unitIndex + 1} Side ${sideLetter}`;
}

export function getDiscLabel(discNumber: number, emulateType: string | null | undefined): string {
	switch (emulateType) {
		case "Vinyl":
			return sideLabel(discNumber, "Record");
		case "Cassette":
			return sideLabel(discNumber, "Cassette");
		case "Disc":
		case "None":
		default:
			return `Disc ${discNumber}`;
	}
}

export function getDiscIcon(emulateType: string | null | undefined): any {
	switch (emulateType) {
		case "Vinyl":    return Disc3;
		case "Cassette": return Videotape;
		case "Disc":
		case "None":
		default:         return Disc;
	}
}

export function parseDiscNumber(albumsJson: string | null | undefined, albumUid: string): number | null {
	if (!albumsJson) return null;
	let albums: TrackAlbumEntry[];
	try {
		albums = JSON.parse(albumsJson);
		if (!Array.isArray(albums)) return null;
	} catch {
		return null;
	}
	const match = albums.find(e => e.uid === albumUid);
	if (!match) return null;
	const disc = match.disc;
	if (disc == null || isNaN(Number(disc))) return null;
	return Number(disc);
}

export function buildDiscBreaks(
	tracks: Track[],
	albumUid: string,
	emulateType: string | null | undefined
): Map<string, DiscBreakEntry> {
	const breaks = new Map<string, DiscBreakEntry>();
	const Icon = getDiscIcon(emulateType);

	const discNumbers = tracks
		.map(t => parseDiscNumber(t.albums, albumUid))
		.filter((d): d is number => d != null);

	const uniqueDiscs = new Set(discNumbers);
	if (uniqueDiscs.size <= 1) return breaks;

	let lastDisc: number | null = null;

	for (const track of tracks) {
		const disc = parseDiscNumber(track.albums, albumUid);
		if (disc == null) continue;
		if (disc !== lastDisc) {
			breaks.set(track.uid, { label: getDiscLabel(disc, emulateType), Icon });
			lastDisc = disc;
		}
	}

	return breaks;
}