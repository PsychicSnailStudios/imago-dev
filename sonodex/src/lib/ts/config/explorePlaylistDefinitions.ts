export type Refresh = "open" | "never" | { days: number };

export type DateRange = {
	lastDays?: number;
	from?: number;
	to?: number;
};

export type Rank =
	| "rate" // plays per day across the scrobble window
	| "totalTime"
	| "consistency" // number of distinct days with a qualifying play
	| "gap" // shortest average time between plays first, so it needs at least 2 plays
	| "rating" // highest rated first
	| "recentlyAdded"; // newest first, keeping your current album-varied picking when the last day has more than the limit

export type RankSpec = Rank | `-${Rank}`; // prefix with "-" to invert, e.g. "-totalTime" for least listened first

export type PlaylistFilters = { // filters, all of which must match (AND)
	artists?: string[];
	tags?: string[];
	genres?: string[];
	minRating?: number;
	added?: DateRange;
	scrobbleRange?: DateRange; // last x days, or from/to
	unplayedRange?: DateRange;
	minPlayMs?: number; // a play shorter than this doesn't count, replacing the hardcoded MIN_PLAY_MS
};

export type PlaylistDefinition = {
	uid: string;
	title: string;
	description: string;
	owner: string;
	artwork: string | null;
	maxTracks: number;
	refresh: Refresh;
	filters: PlaylistFilters;
	rank: RankSpec | RankSpec[];
};

type PlaylistInput = Pick<PlaylistDefinition, "uid" | "title" | "description" | "rank"> &
	Partial<Omit<PlaylistDefinition, "uid" | "title" | "description" | "rank">>;

export function definePlaylist(input: PlaylistInput): PlaylistDefinition {
	return {
		owner: "Imago",
		artwork: null,
		maxTracks: 50,
		refresh: "open",
		filters: {},
		...input,
	};
}

export const playlistDefinitions: PlaylistDefinition[] = [
	definePlaylist({
		uid: "p-explore-on-loop",
		artwork: "on-loop.jpg",
		title: "On Loop",
		description: "Your 50 most played tracks over the last week",
		rank: "rate",
		maxTracks: 50,
		refresh: "open",
		filters: {
			scrobbleRange: { lastDays: 7 },
			minPlayMs: 30_000,
		},
	}),
	definePlaylist({
		uid: "p-explore-recently-added",
		artwork: "new-tracks.jpg",
		title: "Recently Added",
		description: "The newest tracks in your library",
		rank: "recentlyAdded",
		maxTracks: 100,
		refresh: "open",
	}),
	definePlaylist({
		uid: "p-explore-past-favorites",
		artwork: "past-favs.jpg",
		title: "Past Favorites",
		description: "Your favorites you haven't heard in a while",
		maxTracks: 50,
		refresh: "open",
		rank: ["rating","rate"],
		filters: {
			unplayedRange: { lastDays: 180 },
			minPlayMs: 30_000,
		},
	}),
	definePlaylist({
		uid: "p-explore-top-tracks",
		artwork: "top-tracks.jpg",
		title: "Top Tracks",
		description: "Top tracks of all time",
		maxTracks: 50,
		refresh: "open",
		rank: ["rating","totalTime","consistency"],
		filters: {
			minPlayMs: 30_000,
		},
	}),
	definePlaylist({
		uid: "p-explore-forgotten-tracks",
		artwork: "forgotten-tracks.jpg",
		title: "Forgotten Gems",
		description: "Tracks that you havent heard that much",
		maxTracks: 50,
		refresh: "open",
		rank: ["-totalTime","rating"],
	}),
];