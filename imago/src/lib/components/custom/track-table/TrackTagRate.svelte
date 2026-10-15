<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	import Input from "$shadcn/input/input.svelte";
	import Button from "$shadcn/button/button.svelte";

	import { tagStore } from "$ts/store/tagManager.svelte";
	import { reloadSingle } from "$ts/store/library.svelte";
	import { parseTags } from "$ts/util/parsers";
	import type { Track } from "$ts/util/types";

	let { track, mode } = $props<{ track: Track; mode: "tag" | "rate" }>();

	let query = $state("");
	let saving = $state(false);
	let ratingValue = $state("");
	let currentTags = $state<string[]>([]);

	$effect(() => {
		currentTags = parseTags(track.tags);
	});

	$effect(() => {
		ratingValue = track.rating != null ? String(Math.round(track.rating * 10) / 10) : "";
	});

	const suggestions = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const names = tagStore.namesFor("tag");
		const taken = new Set(currentTags.map((t) => t.toLowerCase()));
		return names
			.filter((n) => !taken.has(n.toLowerCase()) && (q === "" || n.toLowerCase().includes(q)))
			.slice(0, 8);
	});

	function guardKeys(e: KeyboardEvent) {
		if (e.key !== "Escape") e.stopPropagation();
	}

	async function saveRating() {
		const parsed = parseFloat(ratingValue);
		const clamped = isNaN(parsed) ? null : Math.min(10, Math.max(0, Math.round(parsed * 10) / 10));
		ratingValue = clamped === null ? "" : String(clamped);
		saving = true;
		try {
			await invoke("update_track_metadata", { uid: track.uid, update: { rating: clamped } });
			await reloadSingle(track.uid);
		} finally {
			saving = false;
		}
	}

	async function addTag(name: string) {
		const clean = name.trim();
		if (!clean) return;
		if (currentTags.some((t) => t.toLowerCase() === clean.toLowerCase())) {
			query = "";
			return;
		}
		saving = true;
		try {
			await tagStore.ensureTag(clean, "tag");
			const stored = tagStore.byName.get(clean.toLowerCase())?.name ?? clean;
			const next = [...currentTags, stored];
			await invoke("update_track_metadata", { uid: track.uid, update: { tags: JSON.stringify(next) } });
			currentTags = next;
			query = "";
			await reloadSingle(track.uid);
		} finally {
			saving = false;
		}
	}

	function onTagKeydown(e: KeyboardEvent) {
		guardKeys(e);
		if (e.key === "Enter") {
			e.preventDefault();
			addTag(query);
		}
	}

	function onRatingKeydown(e: KeyboardEvent) {
		guardKeys(e);
		if (e.key === "Enter") {
			e.preventDefault();
			saveRating();
		}
	}
</script>

{#if mode === "rate"}
	<div class="flex flex-col gap-2 p-2">
		<span class="text-xs text-muted-foreground">Rating (0-10)</span>
		<div class="flex items-center gap-2">
			<Input
				type="number"
				min="0"
				max="10"
				step="0.1"
				bind:value={ratingValue}
				onkeydown={onRatingKeydown}
				disabled={saving}
				class="h-8 text-sm"
				placeholder="0-10"
			/>
			<Button size="sm" onclick={saveRating} disabled={saving}>Save</Button>
		</div>
		{#if track.rating != null}
			<Button
				variant="ghost"
				size="sm"
				disabled={saving}
				onclick={() => { ratingValue = ""; saveRating(); }}
			>
				Clear rating
			</Button>
		{/if}
	</div>
{:else}
	<div class="flex flex-col gap-1 p-2">
		<Input
			bind:value={query}
			onkeydown={onTagKeydown}
			disabled={saving}
			class="h-8 text-sm"
			placeholder="Type a tag, press Enter"
		/>
		{#each suggestions as name}
			<button
				class="px-2 py-1.5 text-sm text-left rounded-sm hover:bg-muted/50 truncate"
				disabled={saving}
				onclick={() => addTag(name)}
			>
				{name}
			</button>
		{/each}
		{#if query.trim() !== "" && !suggestions.some((s) => s.toLowerCase() === query.trim().toLowerCase())}
			<button
				class="px-2 py-1.5 text-sm text-left rounded-sm hover:bg-muted/50 truncate"
				disabled={saving}
				onclick={() => addTag(query)}
			>
				Create "{query.trim()}"
			</button>
		{/if}
	</div>
{/if}
