export type Refresh = "open" | "never" | { days: number };

export type DateRange = {
	lastDays?: number;
	from?: number;
	to?: number;
};

export type Rank =
	| "rate"
	| "totalTime"
	| "consistency"
	| "gap"
	| "rating"
	| "recentlyAdded";

export type PlaylistFilters = {
	artists?: string[];
	tags?: string[];
	genres?: string[];
	minRating?: number;
	added?: DateRange;
	scrobbleRange?: DateRange;
	unplayedRange?: DateRange;
	minPlayMs?: number;
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
	rank: Rank | Rank[];
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
		title: "Recently Added",
		description: "The newest tracks in your library",
		rank: "recentlyAdded",
		maxTracks: 100,
		refresh: "open",
	}),
	definePlaylist({
		uid: "p-explore-past-favorites",
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
		title: "Top Tracks",
		description: "Top tracks of all time",
		maxTracks: 50,
		refresh: "open",
		rank: ["rating","totalTime","consistency"],
		filters: {
			minPlayMs: 30_000,
		},
	}),
];