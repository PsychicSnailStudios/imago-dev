<script lang="ts">
	import * as ContextMenu from "$shadcn/context-menu/index.js";
	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import PlaylistContext from "$lib/components/context-menus/PlaylistContext.svelte";
   import DefultPlaylistArt from "$lib/components/pages/playlist/DefultPlaylistArt.svelte";
	
	import { getTrackArrayFromUID } from "$ts/store/library.svelte";
   import type { Playlist } from "$ts/util/types";
    import { get } from "svelte/store";

	type Props = {
		playlist: Playlist;
		isDraggingThis: boolean;
		isReorderBefore: boolean;
		isReorderAfter: boolean;
		isHovered: boolean;
		allFolderPaths: string[];
		ondragstart: (e: DragEvent) => void;
		ondragend: (e: DragEvent) => void;
		ondragover: (e: DragEvent) => void;
		ondragleave: (e: DragEvent) => void;
		ondrop: (e: DragEvent) => void;
	};

	let {
		playlist,
		isDraggingThis,
		isReorderBefore,
		isReorderAfter,
		isHovered,
		allFolderPaths,
		ondragstart,
		ondragend,
		ondragover,
		ondragleave,
		ondrop,
	}: Props = $props();
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="w-full">
		<div
			class="relative transition-all w-full rounded-md"
			class:opacity-40={isDraggingThis}
			class:ring-2={isHovered}
			class:ring-primary={isHovered}
		>
			{#if isReorderBefore && !isDraggingThis}
				<div class="absolute -left-2 top-0 bottom-0 w-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
			{/if}
			{#if isReorderAfter && !isDraggingThis}
				<div class="absolute -right-2 top-0 bottom-0 w-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
			{/if}
			<div
				draggable="true"
				class="cursor-grab"
				{ondragstart}
				{ondragend}
				{ondragover}
				{ondragleave}
				{ondrop}
				role="region"
				aria-label="Playlist"
			>
				<AudioCard title={playlist.title} subTitle={playlist.owner} entity={playlist} type="playlist">
					<DefultPlaylistArt tracks={getTrackArrayFromUID(playlist.uid)} />
				</AudioCard>
			</div>
		</div>
	</ContextMenu.Trigger>
	<PlaylistContext folderPaths={allFolderPaths} uid={playlist.uid} />
</ContextMenu.Root>