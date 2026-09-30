<script lang="ts">
	// COMPONENTS
	import * as Tabs from "$shadcn/tabs/index.js";
	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";

	// CUSTOM COMPONENTS
	import QueueTrackItem from "$lib/components/app/now-playing/QueueTrackItem.svelte";

	// SCRIPTS
	import {
		clearQueue,
		getQueuedTracks,
		reorderQueue,
		insertIntoQueue,
		removeFromQueue,
	} from "$ts/audio/audioManager.svelte";
	import { player } from "$ts/audio/audioPlayer.svelte";
	import { dragState, endDrag } from "$ts/store/drag.svelte";
	import { clearQueueSelection, queueSelection } from "$ts/store/queueSelection.svelte";

	// VARIABLES
	let upcomingTracks = $derived(getQueuedTracks());
	let upcomingUids = $derived(upcomingTracks.map(t => t.uid));

	// A uid→track map built from whatever the drag payload carries.
	// The drag system sets dragState.payload.tracks when tracks are dragged from the library.
	// For queue reorders the tracks are already in upcomingTracks.
	function resolveUidsToTracks(uids: string[]) {
		// Build lookup from queue + drag payload
		const lookup = new Map(upcomingTracks.map(t => [t.uid, t]));
		const payloadTracks = dragState.payload?.tracks ?? [];
		for (const t of payloadTracks) lookup.set(t.uid, t);
		return uids.map(uid => lookup.get(uid)).filter((t): t is NonNullable<typeof t> => t !== undefined);
	}

	let dragOverIndex = $state<number | null>(null);
	let dragOverPosition = $state<"above" | "below">("below");
	let dragOverAppend = $state(false);

	// FUNCTIONS
	function formatTotalRemainingTime(): string {
		const queueMs = getQueuedTracks().reduce((acc, t) => acc + (t.duration_ms ?? 0), 0);
		const currentRemaining = (player.duration - player.currentTime) * 1000;
		const totalMs = queueMs + Math.max(0, currentRemaining);
		const totalSecs = Math.floor(totalMs / 1000);
		const h = Math.floor(totalSecs / 3600);
		const m = Math.floor((totalSecs % 3600) / 60);
		const s = totalSecs % 60;
		return `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === "Delete" && queueSelection.count > 0) {
			e.preventDefault();
			const indices = [...queueSelection.selected].sort((a, b) => b - a);
			indices.forEach(i => removeFromQueue(i));
			clearQueueSelection();
		}
	}

	function isQueueDrag(): boolean {
		return (dragState.payload?.sourceQueueIndices?.length ?? 0) > 0;
	}

	function handleRowDragOver(e: DragEvent, index: number) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
		dragOverAppend = false;
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		dragOverIndex = index;
		dragOverPosition = e.clientY < rect.top + rect.height / 2 ? "above" : "below";
	}

	function handleRowDragLeave(e: DragEvent) {
		const related = e.relatedTarget as HTMLElement | null;
		if (related && (e.currentTarget as HTMLElement).contains(related)) return;
		dragOverIndex = null;
	}

	function handleScrollAreaDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
		if (dragOverIndex === null) dragOverAppend = true;
	}

	function handleScrollAreaDragLeave(e: DragEvent) {
		const related = e.relatedTarget as HTMLElement | null;
		if (related && (e.currentTarget as HTMLElement).contains(related)) return;
		dragOverAppend = false;
	}

	function handleRowDrop(e: DragEvent, dropIndex: number) {
		e.preventDefault();
		e.stopPropagation();

		if (isQueueDrag()) {
			const fromIndices = dragState.payload!.sourceQueueIndices!;
			const toDisplayIndex = dragOverPosition === "above" ? dropIndex : dropIndex + 1;
			reorderQueue(fromIndices, toDisplayIndex);
		} else {
			const uids = getDropUids(e);
			const tracks = resolveUidsToTracks(uids);
			if (tracks.length > 0) {
				const afterIndex = dragOverPosition === "above" ? dropIndex - 1 : dropIndex;
				insertIntoQueue(tracks, afterIndex, dragState.payload?.sourceUid ?? null);
			}
		}

		dragOverIndex = null;
		dragOverAppend = false;
		clearQueueSelection();
		endDrag();
	}

	function handleScrollAreaDrop(e: DragEvent) {
		e.preventDefault();
		if (dragOverIndex !== null) return;

		if (isQueueDrag()) {
			const fromIndices = dragState.payload!.sourceQueueIndices!;
			reorderQueue(fromIndices, upcomingTracks.length - 1);
		} else {
			const uids = getDropUids(e);
			const tracks = resolveUidsToTracks(uids);
			if (tracks.length > 0) insertIntoQueue(tracks, undefined, dragState.payload?.sourceUid ?? null);
		}

		dragOverAppend = false;
		clearQueueSelection();
		endDrag();
	}

	function getDropUids(e: DragEvent): string[] {
		const raw = e.dataTransfer?.getData("text/plain") ?? "";
		return raw.split(",").map(u => u.trim()).filter(Boolean);
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<Tabs.Content value="queue" class="p-1 space-y-2">
	<p class="text-sm text-foreground">Now Playing</p>
	<div class="rounded-md border-2">
		{#if player.track}
			<QueueTrackItem track={player.track} isNowPlaying={true} allUpcomingUids={upcomingUids} />
		{/if}
	</div>

	{#if upcomingTracks.length > 0}
	<p class="text-sm text-foreground">Next up:</p>

	<div class="flex justify-between">
		<p class="text-xs text-muted-foreground">{formatTotalRemainingTime()} Remaining</p>
		<button class="text-xs text-muted-foreground cursor-pointer hover:underline" onclick={clearQueue}>Clear</button>
	</div>

	<ScrollArea
		class="min-h-0 min-w-0 h-[254px]"
		ondragover={handleScrollAreaDragOver}
		ondragleave={handleScrollAreaDragLeave}
		ondrop={handleScrollAreaDrop}
	>
		<div class="flex flex-col gap-0.5 pb-2">
			{#each upcomingTracks as track, i (track.uid)}
				<div
					class="relative"
					role="row"
					tabindex={i}
					ondragover={(e) => handleRowDragOver(e, i)}
					ondragleave={handleRowDragLeave}
					ondrop={(e) => handleRowDrop(e, i)}
				>
					{#if dragOverIndex === i && dragOverPosition === "above"}
						<div class="absolute top-0 left-0 right-0 h-0.5 bg-primary z-10 pointer-events-none"></div>
					{/if}

					<QueueTrackItem
						{track}
						displayIndex={i}
						isNowPlaying={false}
						allUpcomingUids={upcomingUids}
					/>

					{#if dragOverIndex === i && dragOverPosition === "below"}
						<div class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary z-10 pointer-events-none"></div>
					{/if}
				</div>
			{/each}

			{#if dragOverAppend}
				<div class="h-0.5 bg-primary mx-2 rounded pointer-events-none"></div>
			{/if}
		</div>
	</ScrollArea>
	{/if}
</Tabs.Content>