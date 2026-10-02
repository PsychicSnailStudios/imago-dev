<script lang="ts">
	import { Pencil, Trash, CloudDownload, Loader2 } from "lucide-svelte";
	import * as Tooltip from "$shadcn/tooltip/index.js";
	import { buttonVariants } from "$shadcn/button/index.js";
	import { Checkbox } from "$shadcn/checkbox/index.js";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import { openEditModal } from "$ts/ui/editModal.svelte";
	import { removeArtist } from "$ts/library/libraryManager";
	import { setSelection } from "$ts/store/session.svelte";
	import { library } from "$ts/store/library.svelte";
	import type { Artist } from "$ts/util/types";
	import { enrichArtist } from "$ts/library/enrichment";

	let {
		artist,
		selected = false,
		onToggle,
		onShiftClick,
	}: {
		artist: Artist;
		selected?: boolean;
		onToggle?: () => void;
		onShiftClick?: () => void;
	} = $props();

	let enriching = $state(false);

	async function handleEnrich(e: MouseEvent) {
		e.stopPropagation();
		enriching = true;
		try {
			await enrichArtist(artist.uid);
		} finally {
			enriching = false;
		}
	}

	function handleRowClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (target.closest("a, button")) return;
		if (e.shiftKey) {
			onShiftClick?.();
		} else {
			onToggle?.();
		}
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
			<ArtworkDisplay entity={artist} size={40} />
			<div class="min-w-0 grid">
				<button
					onclick={(e) => { e.stopPropagation(); setSelection(artist.uid); }}
					class="text-sm truncate cursor-pointer hover:underline text-left"
				>
					{artist.name}
				</button>
			</div>
		</div>

		{#if !selected}
			<div class="flex gap-2 shrink-0">
				<Tooltip.Root>
					<Tooltip.Trigger
						class={buttonVariants({ variant: "destructive", size: "icon" })}
						onclick={(e: MouseEvent) => { e.stopPropagation(); removeArtist(artist.uid); }}
					>
						<Trash />
					</Tooltip.Trigger>
					<Tooltip.Content><p>Delete artist record</p></Tooltip.Content>
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
						onclick={(e: MouseEvent) => { e.stopPropagation(); openEditModal({ type: "artist", uid: artist.uid }); }}
					>
						<Pencil />
					</Tooltip.Trigger>
					<Tooltip.Content><p>Edit artist</p></Tooltip.Content>
				</Tooltip.Root>
			</div>
		{/if}
	</div>
</div>