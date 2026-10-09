<script lang="ts">
	import { Ellipsis } from "lucide-svelte";

	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import Button from "$shadcn/button/button.svelte";

	import TrackPlaylistContent from "$lib/components/context-menus/AddTrackToPlaylist.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { addTrackToQueue, removeFromQueue } from "$ts/audio/audioManager.svelte";
   import { openEditModal } from "$ts/ui/editModal.svelte";
	import { copySelectedNameToClipboard } from "$ts/store/trackSelection.svelte";
	import type { Track } from "$ts/util/types";

	let { track, sourceUid = null } = $props<{ track: Track; sourceUid?: string | null }>()

	let content = $state<ReturnType<typeof TrackPlaylistContent> | null>(null);
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="ghost" size="icon-sm">
				<Ellipsis class="w-4 h-4"/>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>

	<DropdownMenu.Content>
		<DropdownMenu.Group>
			<DropdownMenu.Item onSelect={() => copySelectedNameToClipboard(track)}>Copy Track & Artist Name</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={() => addTrackToQueue(track, sourceUid)}>Add to Queue</DropdownMenu.Item>
		</DropdownMenu.Group>

		<DropdownMenu.Separator />

		<DropdownMenu.Group>
			<DropdownMenu.Sub>
				<DropdownMenu.SubTrigger>Add to playlist</DropdownMenu.SubTrigger>
				<DropdownMenu.SubContent class="w-64">
					<TrackPlaylistContent bind:this={content} {track} />
				</DropdownMenu.SubContent>
			</DropdownMenu.Sub>
		</DropdownMenu.Group>
		<DropdownMenu.Separator />
		<DropdownMenu.Group>
			<DropdownMenu.Item onSelect={() => setSelection(track.uid)}>Go to Track</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={() => openEditModal({ type: "track", uid: track!.uid })}>Edit Metadata</DropdownMenu.Item>
		</DropdownMenu.Group>
	</DropdownMenu.Content>
</DropdownMenu.Root>