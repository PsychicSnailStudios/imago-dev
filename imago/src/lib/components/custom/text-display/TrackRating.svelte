<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	import { Star } from 'lucide-svelte';
	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import { Input } from "$shadcn/input";
	import { Toggle } from "$shadcn/toggle/index.js";
   import { reloadSingle } from "$ts/store/library.svelte";

	let { uid, rating, tags }: { uid: string; rating: number | null, tags: string[] } = $props();

	let value = $state(rating != null ? String(Math.round(rating * 10) / 10) : "-");
	let saving = $state(false);
	let localTags = $state<string[]>(
		Array.isArray(tags)
			? tags
			: (tags ? JSON.parse(tags as unknown as string) : [])
	);
	let isFavoritePressed = $derived(localTags.includes("favorite"));

	$effect(() => {
		localTags = Array.isArray(tags)
			? tags
			: (tags ? JSON.parse(tags as unknown as string) : []);
	});

	async function save() {
		const parsed = parseFloat(value);
		const clamped = isNaN(parsed) ? null : Math.min(10, Math.max(0, Math.round(parsed * 10) / 10));

		if (clamped !== null) {
			value = String(clamped);
		}

		saving = true;
		try {
			await invoke("update_track_metadata", {
				uid,
				update: { rating: clamped },
			});
		} finally {
			saving = false;
		}
	}

	async function onFavoritePressed() {
		const hasFavorite = localTags.includes("favorite");
		const new_tags = hasFavorite
			? localTags.filter((t) => t.trim() !== "favorite")
			: [...localTags, "favorite"];

		localTags = new_tags;

		if (!hasFavorite && (parseFloat(value) < 9.5 || value === "")) {
			value = "9.5";
			save();
		}

		saving = true;
		try {
			await invoke("update_track_metadata", {
				uid,
				update: { tags: JSON.stringify(new_tags) },
			});
		} finally {
			saving = false;
		}

		await reloadSingle(uid);
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === "Enter") {
			(e.currentTarget as HTMLInputElement).blur();
		}
	}
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
	{#if isFavoritePressed}
		<div class="*:[svg]:fill-sidebar-foreground flex align-middle justify-center">
			<Star class="w-4 h-4" />
		</div>
	{:else}
		{#if value === "rate"}
			<span class="text-sm font-mono uppercase p-1 border rounded-md">{value}</span>
		{:else}
			<span class="text-sm font-mono">{value}</span>
		{/if}

	{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content>
    <DropdownMenu.Group>
		<div class="flex gap-1 items-center">
			<Input
				type="number"
				min="0"
				max="10"
				step="0.1"
				bind:value
				onchange={save}
				onblur={save}
				{onkeydown}
				disabled={saving}
				class="h-7 text-sm text-center"
				placeholder="1-10"
			/>
			<Toggle
				pressed={isFavoritePressed}
				onPressedChange={onFavoritePressed}
				size="sm"
				class="data-[state=on]:bg-transparent data-[state=on]:*:[svg]:fill-yellow-500 data-[state=on]:*:[svg]:stroke-yellow-500"
			>
				<Star />
			</Toggle>
		</div>
    </DropdownMenu.Group>
  </DropdownMenu.Content>
</DropdownMenu.Root>

