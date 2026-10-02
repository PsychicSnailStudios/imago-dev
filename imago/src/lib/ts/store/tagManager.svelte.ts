// src/lib/tagManager.svelte.ts
//
// Frontend tag/genre dictionary store.
// Wraps the Tauri commands and provides reactive state for:
//   - the full tag/genre list (for autocomplete, management UI)
//   - tag groups
//   - helper lookups (by uid, by name)
//
// Usage:
//   import { tagStore, loadTags } from '$lib/tagManager.svelte';
//
//   await loadTags();
//   tagStore.tags      // Tag[]
//   tagStore.genres    // Tag[]
//   tagStore.groups    // TagGroup[]
//   tagStore.byUid     // Map<string, Tag>
//   tagStore.byName    // Map<string, Tag>  (lowercased key)
//   tagStore.groupsFor(tagUid) // TagGroup[] — all groups containing a tag

import { invoke } from '@tauri-apps/api/core';

// ─── Types (mirror Rust structs) ─────────────────────────────────────────────

export type TagKind = 'tag' | 'genre';

export interface Tag {
	uid: string;
	name: string;
	kind: TagKind;
	color: string | null;
	created_at: number;
}

export interface TagGroup {
	uid: string;
	name: string;
	kind: TagKind;
	color: string | null;
	member_uids: string[];
	created_at: number;
}

// ─── State ───────────────────────────────────────────────────────────────────

function createTagStore() {
	let allTags = $state<Tag[]>([]);
	let allGroups = $state<TagGroup[]>([]);
	let loaded = $state(false);

	// Derived slices
	const tags = $derived(allTags.filter((t) => t.kind === 'tag'));
	const genres = $derived(allTags.filter((t) => t.kind === 'genre'));

	// Fast lookups
	const byUid = $derived(new Map(allTags.map((t) => [t.uid, t])));
	const byName = $derived(new Map(allTags.map((t) => [t.name.toLowerCase(), t])));

	// Group membership index: tag_uid → TagGroup[]
	const groupIndex = $derived(() => {
		const idx = new Map<string, TagGroup[]>();
		for (const g of allGroups) {
			for (const uid of g.member_uids) {
				if (!idx.has(uid)) idx.set(uid, []);
				idx.get(uid)!.push(g);
			}
		}
		return idx;
	});

	return {
		// Reactive state
		get tags() { return tags; },
		get genres() { return genres; },
		get groups() { return allGroups; },
		get byUid() { return byUid; },
		get byName() { return byName; },
		get loaded() { return loaded; },

		/** All groups that contain the given tag uid. */
		groupsFor(tagUid: string): TagGroup[] {
			return groupIndex().get(tagUid) ?? [];
		},

		/** Autocomplete list for a given kind. Returns tag names sorted A-Z. */
		namesFor(kind: TagKind): string[] {
			return allTags
				.filter((t) => t.kind === kind)
				.map((t) => t.name)
				.sort((a, b) => a.localeCompare(b));
		},

		// ── Load ──────────────────────────────────────────────────────────────

		async load() {
			const [tags, groups] = await Promise.all([
				invoke<Tag[]>('get_all_tags_cmd'),
				invoke<TagGroup[]>('get_tag_groups_cmd'),
			]);
			allTags = tags;
			allGroups = groups;
			loaded = true;
		},

		// ── Tag CRUD ──────────────────────────────────────────────────────────

		async addTag(name: string, color?: string): Promise<Tag> {
			const uid = await invoke<string>('add_tag_cmd', { name, color: color ?? null });
			await this.load();
			return this.byUid.get(uid)!;
		},

		async addGenre(name: string, color?: string): Promise<Tag> {
			const uid = await invoke<string>('add_genre_cmd', { name, color: color ?? null });
			await this.load();
			return this.byUid.get(uid)!;
		},

		async renameTag(uid: string, newName: string): Promise<void> {
			await invoke('rename_tag_cmd', { uid, newName });
			await this.load();
		},

		async deleteTag(uid: string): Promise<void> {
			await invoke('delete_tag_cmd', { uid });
			await this.load();
		},

		async updateTagColor(uid: string, color: string | null): Promise<void> {
			await invoke('update_tag_color_cmd', { uid, color });
			await this.load();
		},

		// ── Group CRUD ────────────────────────────────────────────────────────

		async createGroup(
			name: string,
			kind: TagKind,
			memberUids: string[],
			color?: string
		): Promise<TagGroup> {
			const group = await invoke<TagGroup>('create_tag_group_cmd', {
				name,
				kind,
				color: color ?? null,
				memberUids,
			});
			await this.load();
			return group;
		},

		async updateGroup(
			uid: string,
			patch: { name?: string; color?: string | null; memberUids?: string[] }
		): Promise<void> {
			await invoke('update_tag_group_cmd', {
				uid,
				name: patch.name ?? null,
				color: 'color' in patch ? patch.color : undefined,
				memberUids: patch.memberUids ?? null,
			});
			await this.load();
		},

		async deleteGroup(uid: string): Promise<void> {
			await invoke('delete_tag_group_cmd', { uid });
			await this.load();
		},

		// ── Ensure (called from scanner/enrichment shims) ─────────────────────

		/** Ensure a tag exists — used when saving track metadata. Returns uid. */
		async ensureTag(name: string, kind: TagKind): Promise<string> {
			const existing = this.byName.get(name.toLowerCase());
			if (existing) return existing.uid;
			if (kind === 'genre') return this.addGenre(name).then((t) => t.uid);
			return this.addTag(name).then((t) => t.uid);
		},
	};
}

export const tagStore = createTagStore();

/** Shorthand — load tags+groups. Call once on app init (after profile:ready). */
export async function loadTags() {
	await tagStore.load();
}
