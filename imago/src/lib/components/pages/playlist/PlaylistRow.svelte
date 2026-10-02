<script lang="ts">
	import { Play } from "lucide-svelte";
	import Button from "$shadcn/button/button.svelte";

	import { setSelection } from "$ts/store/session.svelte";
	import { queueTracksByObject } from "$ts/audio/audioManager.svelte";
	import { getTrackArrayFromUID } from "$ts/store/library.svelte";
	import type { Playlist } from "$ts/util/types";

	let {
		playlist,
		indent = 0,
		ondragover,
		ondragleave,
		ondrop,
		highlighted = false,
	} = $props<{
		playlist: Playlist;
		indent?: number;
		ondragover?: (e: DragEvent) => void;
		ondragleave?: (e: DragEvent) => void;
		ondrop?: (e: DragEvent) => void;
		highlighted?: boolean;
	}>();

	async function play(e: MouseEvent) {
		e.stopPropagation();
		const tracks = await getTrackArrayFromUID(playlist.uid);
		queueTracksByObject(tracks, true, false, playlist.uid);
	}
</script>

<div
	class="flex items-center gap-2 pr-2 rounded-md hover:bg-muted/50 group cursor-default"
	class:ring-1={highlighted}
	class:ring-primary={highlighted}
	style="padding-left: {indent+7}px"
	role="region"
	aria-label="Playlist"
	{ondragover}
	{ondragleave}
	{ondrop}
>
	{#if indent > 20}
		<div class="h-8 w-0.5 bg-muted"></div>
	{/if}
	
	<button
		class="flex items-center gap-2 flex-1 min-w-0 text-left h-8"
		onclick={() => setSelection(playlist.uid)}
	>
		<span class="text-sm truncate leading-tight">{playlist.title}</span>
	</button>

	<Button
		variant="ghost"
		size="icon"
		class="size-6 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
		onclick={play}
	>
		<Play class="size-3" />
	</Button>
</div>