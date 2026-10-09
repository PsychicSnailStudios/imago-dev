<script lang="ts">

	// COMPONENTS
	import * as ContextMenu from "$shadcn/context-menu/index.js";
	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";

	import TrackPlaylistContent from "$lib/components/context-menus/AddTrackToPlaylist.svelte";

	// SCRIPTS
   import { setSelection } from "$ts/store/session.svelte";
	import { addTrackToQueue, removeFromQueue } from "$ts/audio/audioManager.svelte";
   import { openEditModal } from "$ts/ui/editModal.svelte";
   import { copySelectedNameToClipboard } from "$ts/store/trackSelection.svelte";

   import type { Track } from "$ts/util/types";

	// PROPS
	let { track, inQueue = false, displayIndex = null, sourceUid = null } = $props<{ track: Track; inQueue?: boolean; displayIndex?: number | null; sourceUid?: string | null }>();

	let content = $state<ReturnType<typeof TrackPlaylistContent> | null>(null);

</script>

<ContextMenu.Content>
	<ContextMenu.Group>
		<ContextMenu.Item onSelect={() => copySelectedNameToClipboard(track)}>Copy Track & Artist Name</ContextMenu.Item>
		<ContextMenu.Item
			onSelect={() => addTrackToQueue(track, sourceUid)}
			class={track.path.match(/^[a-z]+-[0-9a-f-]{36}$/) ? 'opacity-50 pointer-events-none' : ''}>
			Add to Queue
		</ContextMenu.Item>
		{#if inQueue}
			<ContextMenu.Item onSelect={() => removeFromQueue(displayIndex)}>Remove from Queue</ContextMenu.Item>
		{/if}
	</ContextMenu.Group>
	<ContextMenu.Separator />
	<ContextMenu.Group>
		<DropdownMenu.Sub>
			<DropdownMenu.SubTrigger>Add to playlist</DropdownMenu.SubTrigger>
			<DropdownMenu.SubContent class="w-64">
				<TrackPlaylistContent bind:this={content} {track} />
			</DropdownMenu.SubContent>
		</DropdownMenu.Sub>
	</ContextMenu.Group>
	<ContextMenu.Separator />
	<ContextMenu.Group>
		<ContextMenu.Item onSelect={() => setSelection(track.uid)}>Go to Track</ContextMenu.Item>
		<!-- <ContextMenu.Item onSelect={() => openEditModal({ type: "track", uid: track!.uid })}>Edit Metadata</ContextMenu.Item> -->
	</ContextMenu.Group>
</ContextMenu.Content>