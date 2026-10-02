<script lang="ts">
	// COMPONENTS
	import * as ContextMenu from "$shadcn/context-menu/index.js";

	// CUSTOM COMPONENTS
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import TrackContext from "$lib/components/context-menus/TrackContext.svelte";

	// SCRIPTS
	import { setSelection } from "$ts/store/session.svelte";
	import { startDrag, endDrag } from "$ts/store/drag.svelte";
	import { queueSelection, selectQueueItem } from "$ts/store/queueSelection.svelte";
	import ArtistsList from "$lib/components/custom/text-display/ArtistsList.svelte";
	import type { Track } from "$ts/util/types";

	// PROPS
	let {
		track,
		displayIndex = null,
		isNowPlaying = false,
		allUpcomingUids = [],
	} = $props<{
		track: Track;
		displayIndex?: number | null;
		isNowPlaying?: boolean;
		allUpcomingUids?: string[];
	}>();

	// VARIABLES
	let isSelected = $derived(
		!isNowPlaying && displayIndex !== null && queueSelection.isSelected(displayIndex)
	);

	// FUNCTIONS
	function handleClick(e: MouseEvent) {
		if (isNowPlaying || displayIndex === null) return;
		if ((e.target as HTMLElement).closest("button")) return;
		selectQueueItem(displayIndex, e, allUpcomingUids.length);
	}

	function handleDragStart(e: DragEvent) {
		if (!e.dataTransfer) return;
		e.dataTransfer.effectAllowed = "move";

		if (isNowPlaying) {
			e.dataTransfer.setData("text/plain", track.uid);
			startDrag({ type: "tracks", uids: [track.uid], sourcePlaylistUid: null });
			return;
		}

		let dragIndices: number[];
		let dragUids: string[];

		if (displayIndex !== null && queueSelection.isSelected(displayIndex) && queueSelection.count > 1) {
			dragIndices = [...queueSelection.selected].sort((a, b) => a - b);
			dragUids = dragIndices.map(i => allUpcomingUids[i]).filter(Boolean);
		} else {
			dragIndices = displayIndex !== null ? [displayIndex] : [];
			dragUids = [track.uid];
		}

		e.dataTransfer.setData("text/plain", dragUids.join(","));
		startDrag({
			type: "tracks",
			uids: dragUids,
			sourcePlaylistUid: null,
			sourceQueueIndices: dragIndices,
		});
	}

	function handleDragEnd() {
		endDrag();
	}
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class="flex gap-2 p-2 items-center cursor-grab active:cursor-grabbing rounded-md transition-colors {isSelected ? 'bg-primary/15 hover:bg-primary/20' : 'hover:bg-muted/50'}"
			draggable="true"
			role="row"
			tabindex="0"
			onclick={handleClick}
			ondragstart={handleDragStart}
			ondragend={handleDragEnd}
		>
			<ArtworkDisplay entity={track} size={36} />
			<div class="min-w-0 grid flex-1">
				<span
					role="button"
					tabindex="0"
					onclick={(e) => { e.stopPropagation(); setSelection(track.uid); }}
					onkeydown={(e) => { if (e.key === 'Enter') setSelection(track.uid); }}
					class="text-sm truncate cursor-pointer hover:underline"
				>
					{track.title}
				</span>
				<div class="text-xs text-muted-foreground truncate">
					<ArtistsList artists={track.artists} />
				</div>
			</div>
		</div>
	</ContextMenu.Trigger>
	<TrackContext track={track} inQueue={!isNowPlaying} displayIndex={displayIndex} />
</ContextMenu.Root>