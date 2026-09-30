import { profileState } from "$ts/store/profiles.svelte";

export type HistorySort = "recent" | "alpha" | "frequent";

export type SearchHistoryEntry = {
	query: string;
	count: number;
	lastUsed: number;
};

const MAX_ENTRIES = 50;

let entries = $state<SearchHistoryEntry[]>([]);
let sortMode = $state<HistorySort>("recent");
let storageKey = "";

function keyFor(profileUid: string): string {
	return `imago:search-history:${profileUid}`;
}

function persist() {
	if (!storageKey) return;
	try {
		localStorage.setItem(storageKey, JSON.stringify({ sort: sortMode, entries }));
	} catch {}
}

function sorted(): SearchHistoryEntry[] {
	const list = [...entries];
	if (sortMode === "alpha") return list.sort((a, b) => a.query.localeCompare(b.query));
	if (sortMode === "frequent") return list.sort((a, b) => b.count - a.count || b.lastUsed - a.lastUsed);
	return list.sort((a, b) => b.lastUsed - a.lastUsed);
}

export const searchHistory = {
	get entries(): SearchHistoryEntry[] {
		return sorted();
	},

	get sort(): HistorySort {
		return sortMode;
	},

	set sort(mode: HistorySort) {
		sortMode = mode;
		persist();
	},

	load() {
		const key = keyFor(profileState.active?.uid ?? "default");
		if (key === storageKey) return;
		storageKey = key;
		entries = [];
		sortMode = "recent";
		try {
			const raw = localStorage.getItem(key);
			if (!raw) return;
			const saved = JSON.parse(raw);
			if (Array.isArray(saved.entries)) entries = saved.entries;
			if (["recent", "alpha", "frequent"].includes(saved.sort)) sortMode = saved.sort;
		} catch {}
	},

	add(query: string) {
		const q = query.trim();
		if (q.length < 2) return;
		const now = Date.now();
		const existing = entries.find((e) => e.query.toLowerCase() === q.toLowerCase());
		if (existing) {
			existing.count += 1;
			existing.lastUsed = now;
			existing.query = q;
		} else {
			entries.push({ query: q, count: 1, lastUsed: now });
		}
		if (entries.length > MAX_ENTRIES) {
			entries = [...entries].sort((a, b) => b.lastUsed - a.lastUsed).slice(0, MAX_ENTRIES);
		}
		persist();
	},

	remove(query: string) {
		entries = entries.filter((e) => e.query.toLowerCase() !== query.toLowerCase());
		persist();
	},

	clear() {
		entries = [];
		persist();
	},
};
