<script lang="ts">

	// COMPONENTS
	import * as Dialog from "$shadcn/dialog/index.js";

	// CUSTOM COMPONENTS
	import TrackEditForm from "$lib/components/dialogs/edit-metadata/TrackEditForm.svelte";
	import AlbumEditForm from "$lib/components/dialogs/edit-metadata/AlbumEditForm.svelte";
	import ArtistEditForm from "$lib/components/dialogs/edit-metadata/ArtistEditForm.svelte";
	import PlaylistEditForm from "$lib/components/dialogs/edit-metadata/PlaylistEditForm.svelte";

	// SCRIPTS
	import { editModal, closeEditModal } from "$ts/ui/editModal.svelte";

	// VARIABLES
	const titles: Record<string, string> = {
		track: "Edit Track",
		album: "Edit Album",
		artist: "Edit Artist",
		playlist: "Edit Playlist",
	};
</script>

<Dialog.Root open={editModal.open} onOpenChange={(v) => { if (!v) closeEditModal(); }}>
	<Dialog.Content class="max-w-lg w-full h-[600px] flex flex-col overflow-hidden">
		<Dialog.Header class="shrink-0">
			<Dialog.Title>
				{editModal.target ? titles[editModal.target.type] : "Edit"}
			</Dialog.Title>
		</Dialog.Header>

		<div class="flex-1 flex flex-col min-h-0 justify-between">
			{#if editModal.target}
				{#if editModal.target.type === "track"}
					<TrackEditForm uid={editModal.target.uid} />
				{:else if editModal.target.type === "album"}
					<AlbumEditForm uid={editModal.target.uid} />
				{:else if editModal.target.type === "artist"}
					<ArtistEditForm uid={editModal.target.uid} />
				{:else if editModal.target.type === "playlist"}
					<PlaylistEditForm uid={editModal.target.uid} />
				{/if}
			{/if}
		</div>
	</Dialog.Content>
</Dialog.Root>
