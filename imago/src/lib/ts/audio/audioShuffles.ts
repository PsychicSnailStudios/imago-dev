import { parseAlbumEntries } from "$ts/util/parsers";
import type { Track } from "$ts/util/types";

function albumUid(t: Track): string | null {
	return parseAlbumEntries(t.albums)[0]?.uid ?? null;
}

function trackNum(t: Track): number | null {
	return parseAlbumEntries(t.albums)[0]?.track_number ?? null;
}

function albumArtist(t: Track): string | null {
	return t.album_artist?.trim() || null;
}

function bpm(t: Track): number | null {
	return (t.bpm != null && t.bpm > 0) ? t.bpm : null;
}

function key(t: Track): string | null {
	return t.key?.trim() || null;
}

function genres(t: Track): Set<string> {
	try {
		const parsed = typeof t.genres === "string" ? JSON.parse(t.genres) : t.genres;
		if (Array.isArray(parsed))
			return new Set(parsed.map((g: string) => g?.toLowerCase?.().trim()).filter(Boolean));
	} catch {}
	return new Set();
}

function tags(t: Track): Set<string> {
	try {
		const parsed = typeof t.tags === "string" ? JSON.parse(t.tags) : t.tags;
		if (Array.isArray(parsed))
			return new Set(parsed.map((g: string) => g?.toLowerCase?.().trim()).filter(Boolean));
	} catch {}
	return new Set();
}

function isFavorite(t: Track): boolean {
	return tags(t).has("favorite");
}

function nonFavTags(t: Track): Set<string> {
	const t2 = tags(t);
	t2.delete("favorite");
	return t2;
}

function setsOverlap(a: Set<string>, b: Set<string>): boolean {
	for (const v of a) if (b.has(v)) return true;
	return false;
}

// ─── Spaced Shuffle ───────────────────────────────────────────────────────────

function adjacencyScore(a: Track, b: Track): number {
	const aAlbum = albumUid(a);
	const bAlbum = albumUid(b);
	const aArtist = a.album_artist ?? null;
	const bArtist = b.album_artist ?? null;
	const aNum = trackNum(a);
	const bNum = trackNum(b);

	if (
		aAlbum && bAlbum && aAlbum === bAlbum &&
		aNum !== null && bNum !== null &&
		Math.abs(aNum - bNum) === 1
	) return 3;

	if (aAlbum && bAlbum && aAlbum === bAlbum) return 2;

	if (aArtist && bArtist && aArtist === bArtist) return 1;

	return 0;
}

function tryRelocate(
	arr: Track[],
	i: Track,
	score: (a: Track, b: Track) => number,
	startAt: number,
	endAt: number
): boolean {
	const current = score(arr[startAt - 1] ?? arr[0], i);
	if (current === 0) return false;

	let bestIdx = -1;
	let bestScore = current;

	for (let j = startAt + 1; j <= endAt; j++) {
		const candidate = arr[j];
		const prevTrack = arr[j - 1];
		const candidateScore = score(prevTrack, candidate);

		const newScoreAtI = score(arr[startAt - 1] ?? arr[0], candidate);
		const newScoreAtJ = score(prevTrack, i);

		if (newScoreAtI + newScoreAtJ < current + candidateScore) {
			if (newScoreAtI + newScoreAtJ < bestScore) {
				bestScore = newScoreAtI + newScoreAtJ;
				bestIdx = j;
			}
		}
	}

	if (bestIdx !== -1) {
		[arr[startAt], arr[bestIdx]] = [arr[bestIdx], arr[startAt]];
		return true;
	}
	return false;
}

export function spacedShuffle(tracks: Track[]): Track[] {
	const shuffled = [...tracks];
	for (let i = shuffled.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
	}

	if (shuffled.length < 4) return shuffled;

	const passes = Math.min(3, Math.floor(shuffled.length / 2));
	for (let pass = 0; pass < passes; pass++) {
		for (let i = 1; i < shuffled.length; i++) {
			const penalty = adjacencyScore(shuffled[i - 1], shuffled[i]);
			if (penalty > 0) {
				const lookAhead = Math.min(i + 20, shuffled.length - 1);
				tryRelocate(shuffled, shuffled[i], adjacencyScore, i, lookAhead);
			}
		}
	}

	return shuffled;
}

// ─── Smart Shuffle ────────────────────────────────────────────────────────────

function separationPenalty(a: Track, b: Track): number {
	let score = 0;

	const aAlbum = albumUid(a);
	const bAlbum = albumUid(b);
	const sameAlbum = aAlbum !== null && aAlbum === bAlbum;

	if (sameAlbum) {
		const aNum = trackNum(a);
		const bNum = trackNum(b);
		if (aNum !== null && bNum !== null && Math.abs(aNum - bNum) === 1)
			score += 40;
	}

	if (sameAlbum) score += 25;

	const aArtist = albumArtist(a);
	const bArtist = albumArtist(b);
	if (aArtist !== null && aArtist === bArtist) score += 15;

	if (isFavorite(a) && isFavorite(b)) score += 3;

	return score;
}

function cohesionBonus(a: Track, b: Track): number {
	let score = 0;

	const aBpm = bpm(a);
	const bBpm = bpm(b);
	if (aBpm !== null && bBpm !== null && Math.abs(aBpm - bBpm) <= 10)
		score += 20;

	if (setsOverlap(genres(a), genres(b))) score += 15;

	if (setsOverlap(nonFavTags(a), nonFavTags(b))) score += 12;

	const aKey = key(a);
	const bKey = key(b);
	if (aKey !== null && aKey === bKey) score += 10;

	return score;
}

function edgeScore(a: Track, b: Track): number {
	return separationPenalty(a, b) - cohesionBonus(a, b);
}

export function smartShuffle(tracks: Track[]): Track[] {
	if (tracks.length < 4) return [...tracks].sort(() => Math.random() - 0.5);

	const shuffled = [...tracks];
	for (let i = shuffled.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
	}

	const WINDOW = 30;
	const PASSES = 4;

	for (let pass = 0; pass < PASSES; pass++) {
		let improved = false;

		for (let i = 1; i < shuffled.length - 1; i++) {
			const currentEdgeScore =
				edgeScore(shuffled[i - 1], shuffled[i]) +
				edgeScore(shuffled[i], shuffled[i + 1]);

			const lo = Math.max(i + 1, 0);
			const hi = Math.min(i + WINDOW, shuffled.length - 1);

			for (let j = lo; j <= hi; j++) {
				const jPrev = shuffled[j - 1];
				const jNext = j + 1 < shuffled.length ? shuffled[j + 1] : null;

				const beforeSwap =
					currentEdgeScore +
					edgeScore(jPrev, shuffled[j]) +
					(jNext ? edgeScore(shuffled[j], jNext) : 0);

				const ti = shuffled[i];
				const tj = shuffled[j];

				const afterSwap =
					edgeScore(shuffled[i - 1], tj) +
					edgeScore(tj, shuffled[i + 1]) +
					edgeScore(jPrev === ti ? tj : jPrev, ti) +
					(jNext ? edgeScore(ti, jNext === ti ? tj : jNext) : 0);

				if (afterSwap < beforeSwap) {
					[shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
					improved = true;
					break;
				}
			}
		}

		if (!improved) break;
	}

	return shuffled;
}
