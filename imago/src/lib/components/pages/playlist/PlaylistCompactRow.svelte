<script lang="ts">
	import * as ContextMenu from "$shadcn/context-menu/index.js";
	import PlaylistRow from "$lib/components/pages/playlist/PlaylistRow.svelte";
	import PlaylistContext from "$lib/components/context-menus/PlaylistContext.svelte";
	import { dragState } from "$ts/store/drag.svelte";
	import type { Playlist } from "$ts/util/types";

	type Props = {
		playlist: Playlist;
		indent: number;
		isDraggingThis: boolean;
		isDropBefore: boolean;
		isDropAfter: boolean;
		allFolderPaths: string[];
		draggingPlaylistUid: string | null;
		ondragstart: (e: DragEvent) => void;
		ondragend: (e: DragEvent) => void;
		ondragover: (e: DragEvent) => void;
		ondragleave: (e: DragEvent) => void;
		ondrop: (e: DragEvent) => void;
		onrowdragover: (e: DragEvent) => void;
		onrowdragleave: (e: DragEvent) => void;
		onrowdrop: (e: DragEvent) => void;
	};

	let {
		playlist,
		indent,
		isDraggingThis,
		isDropBefore,
		isDropAfter,
		allFolderPaths,
		draggingPlaylistUid,
		ondragstart,
		ondragend,
		ondragover,
		ondragleave,
		ondrop,
		onrowdragover,
		onrowdragleave,
		onrowdrop,
	}: Props = $props();
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="w-full">
		<div
			class="relative"
			class:opacity-40={isDraggingThis}
		>
			{#if isDropBefore}
				<div class="absolute left-0 right-0 -top-px h-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
			{/if}
			{#if isDropAfter}
				<div class="absolute left-0 right-0 -bottom-px h-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
			{/if}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				draggable="true"
				{ondragstart}
				{ondragend}
				{ondragover}
				{ondragleave}
				{ondrop}
			>
				<PlaylistRow
					{playlist}
					{indent}
					highlighted={dragState.hoveredPlaylistUid === playlist.uid && draggingPlaylistUid === null}
					ondragover={onrowdragover}
					ondragleave={onrowdragleave}
					ondrop={onrowdrop}
				/>
			</div>
		</div>
	</ContextMenu.Trigger>
	<PlaylistContext folderPaths={allFolderPaths} uid={playlist.uid} />
</ContextMenu.Root>
