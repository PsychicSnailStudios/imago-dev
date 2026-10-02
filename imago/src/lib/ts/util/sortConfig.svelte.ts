export type SortField = "title" | "album" | "year" | "rating" | "duration" | "label" | "artist" | "number"
export type SortDirection = "asc" | "desc"

export class SortState {
	field = $state<SortField | null>(null)
	direction = $state<SortDirection | null>(null)

	constructor(defaultField: SortField | null = null, defaultDirection: SortDirection = "asc") {
		this.field = defaultField
		this.direction = defaultField ? defaultDirection : null
	}

	cycle(f: SortField) {
		if (this.field !== f) {
			this.field = f
			this.direction = "asc"
		} else if (this.direction === "asc") {
			this.direction = "desc"
		} else {
			this.field = null
			this.direction = null
		}
	}

	set(f: SortField, d: SortDirection = "asc") {
		this.field = f
		this.direction = d
	}

	clear() {
		this.field = null
		this.direction = null
	}

	active(f: SortField) {
		return this.field === f
	}
}