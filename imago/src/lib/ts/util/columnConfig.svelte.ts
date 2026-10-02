export type ColumnKey =
	| "number"
	| "artwork"
	| "title"
	| "artist"
	| "album"
	| "year"
	| "rating"
	| "duration"
	| "label"
	| "options"

export type ColumnState = {
	visible: Record<ColumnKey, boolean>
}

export const ALL_COLUMNS: ColumnKey[] = [
	"number",
	"artwork",
	"title",
	"artist",
	"album",
	"year",
	"rating",
	"duration",
	"label",
	"options",
]

export const COLUMN_LABELS: Record<ColumnKey, string> = {
	number: "#",
	artwork: "Artwork",
	title: "Title",
	artist: "Artist",
	album: "Album",
	year: "Year",
	rating: "Rating",
	duration: "Duration",
	label: "Label",
	options: "Options",
}

const PRESETS: Record<string, Partial<Record<ColumnKey, boolean>>> = {
	default: {
		artwork: true,
		title: true,
		duration: true,
		options: true,
	},
	library: {
		artwork: true,
		title: true,
		album: true,
		year: true,
		rating: true,
		duration: true,
		options: true,
	},
	album: {
		number: true,
		title: true,
		rating: true,
		duration: true,
		options: true,
	},
	playlist: {
		artwork: true,
		number: true,
		title: true,
		album: true,
		duration: true,
		options: true,
	},
	simple: {
		artwork: true,
		title: true,
	},
}

export function createColumnState(preset: keyof typeof PRESETS = "default"): ColumnState {
	const base = PRESETS[preset] ?? PRESETS.default
	const visible = $state(
		Object.fromEntries(
			ALL_COLUMNS.map((col) => [
				col,
				base[col] ?? false,
			])
		) as Record<ColumnKey, boolean>
	)
	return { visible }
}