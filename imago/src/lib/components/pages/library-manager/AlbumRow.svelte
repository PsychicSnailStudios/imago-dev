<script lang="ts">
	import { Pencil, Trash, CloudDownload, Loader2 } from "lucide-svelte";
	import * as Tooltip from "$shadcn/tooltip/index.js";
	import { buttonVariants } from "$shadcn/button/index.js";
	import { Checkbox } from "$shadcn/checkbox/index.js";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import { openEditModal } from "$ts/ui/editModal.svelte";
	import { removeAlbum } from "$ts/library/libraryManager";
	import { setSelection } from "$ts/store/session.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { parseArtists } from "$ts/util/parsers";
	import type { Album } from "$ts/util/types";
    import { enrichAlbum } from "$ts/library/enrichment";
    import ArtistsList from "$lib/components/custom/text-display/ArtistsList.svelte";

	let {
		album,
		selected = false,
		onToggle,
		onShiftClick,
	}: {
		album: Album;
		selected?: boolean;
		onToggle?: () => void;
		onShiftClick?: () => void;
	} = $props();

	let enriching = $state(false);

	async function handleEnrich(e: MouseEvent) {
		e.stopPropagation();
		enriching = true;
		try {
			await enrichAlbum(album.uid);
		} finally {
			enriching = false;
		}
	}

	const trackCount = $derived.by(() => {
		try {
			return album.tracks ? JSON.parse(album.tracks as string).length : 0;
		} catch { return 0; }
	});

	function handleRowClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (target.closest("a, button")) return;
		if (e.shiftKey) {
			onShiftClick?.();
		} else {
			onToggle?.();
		}
	}

	async function goToAlbumArtist() {
		if (!album.album_artist) return;
		const uid = await invoke<string | null>("get_artist_uid_by_name", { name: album.album_artist });
		if (uid) setSelection(uid);
	}
</script>

<div class="flex gap-2 items-center">
	<Checkbox
		checked={selected}
		onCheckedChange={(v) => {
			if (v !== selected) onToggle?.();
		}}
	/>
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="flex gap-2 p-2 border-2 rounded-md justify-between items-center flex-1 cursor-pointer"
		class:border-primary={selected}
		onclick={handleRowClick}
		onmousedown={(e) => { if (e.shiftKey) e.preventDefault(); }}
		role="row"
		aria-selected={selected}
	>
		<div class="flex gap-2 items-center min-w-0 flex-1">
			<ArtworkDisplay entity={album} size={40} />
			<div class="min-w-0 grid">
				<button
					onclick={(e) => { e.stopPropagation(); setSelection(album.uid); }}
					class="text-sm truncate cursor-pointer hover:underline text-left"
				>
					{album.title}
				</button>
				<div class="text-xs text-muted-foreground truncate">
					{#if parseArtists(album.artists).length > 0}
						<ArtistsList artists={album.artists!} />
					{:else if album.album_artist}
						<button
							onclick={(e) => { e.stopPropagation(); goToAlbumArtist(); }}
							class="text-xs cursor-pointer hover:underline"
						>
							{album.album_artist}
						</button>
					{:else}
						—
					{/if}
					{#if album.release_date}{" · "}{album.release_date}{/if}
					{" · "}{trackCount} {trackCount === 1 ? "track" : "tracks"}
				</div>
			</div>
		</div>

		{#if !selected}
			<div class="flex gap-2 shrink-0">
				<Tooltip.Root>
					<Tooltip.Trigger
						class={buttonVariants({ variant: "destructive", size: "icon" })}
						onclick={(e: MouseEvent) => { e.stopPropagation(); removeAlbum(album.uid); }}
					>
						<Trash />
					</Tooltip.Trigger>
					<Tooltip.Content><p>Delete album record</p></Tooltip.Content>
				</Tooltip.Root>

				<Tooltip.Root>
					<Tooltip.Trigger
						class={buttonVariants({ variant: "outline", size: "icon" })}
						onclick={handleEnrich}
						disabled={enriching}
					>
						{#if enriching}
							<Loader2 class="animate-spin" />
						{:else}
							<CloudDownload />
						{/if}
					</Tooltip.Trigger>
					<Tooltip.Content><p>Fetch metadata via API</p></Tooltip.Content>
				</Tooltip.Root>

				<Tooltip.Root>
					<Tooltip.Trigger
						class={buttonVariants({ variant: "outline", size: "icon" })}
						onclick={(e: MouseEvent) => { e.stopPropagation(); openEditModal({ type: "album", uid: album.uid }); }}
					>
						<Pencil />
					</Tooltip.Trigger>
					<Tooltip.Content><p>Edit album</p></Tooltip.Content>
				</Tooltip.Root>
			</div>
		{/if}
	</div>
</div>