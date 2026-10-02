<script lang="ts">
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import * as Tabs from "$shadcn/tabs/index.js";
	import { Input } from "$shadcn/input/index.js";

	import ImportPlaylist from "$lib/components/dialogs/playlists/ImportPlaylist.svelte";
	import SpotifyURLImport from "$lib/components/dialogs/playlists/SpotifyUrlImport.svelte";
	
	import { createPlaylist } from "$ts/audio/playlistManager.svelte";
	import { profileState } from "$ts/store/profiles.svelte";
   import { setSelection } from "$ts/store/session.svelte";

	let { open = $bindable(false), folder = null } = $props<{ open: boolean; folder?: string | null }>();

	let nameInput = $state("New Playlist");

	async function handleCreatePlaylist() {
		if (!nameInput.trim()) return;
		let uid = await createPlaylist(nameInput.trim(), profileState.active?.name ?? null, folder);
		nameInput = "";

		setSelection(uid);
		open = false;
	}
</script>

<AlertDialog.Root bind:open={open}>
	<AlertDialog.Content class="max-w-lg w-full h-[500px] flex flex-col overflow-hidden">
		<Tabs.Root value="new" class="h-full">
			<Tabs.List class="w-full">
				<Tabs.Trigger value="new" class="flex-1">New</Tabs.Trigger>
				<Tabs.Trigger value="import" class="flex-1">Import CSV</Tabs.Trigger>
				<Tabs.Trigger value="spotify" class="flex-1">Import Spotify</Tabs.Trigger>
				<!-- <Tabs.Trigger value="url" class="flex-1">URL</Tabs.Trigger> -->
			</Tabs.List>

			<Tabs.Content value="new">
				<div class="flex flex-col h-full justify-between">
					<AlertDialog.Header>
						<AlertDialog.Title class="mt-4">New Playlist</AlertDialog.Title>
						<AlertDialog.Description>
							<Input placeholder="Playlist name" bind:value={nameInput} class="w-full mt-2" />
						</AlertDialog.Description>
					</AlertDialog.Header>

					<AlertDialog.Footer>
						<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
						<AlertDialog.Action onclick={handleCreatePlaylist}>Create</AlertDialog.Action>
					</AlertDialog.Footer>
				</div>
			</Tabs.Content>
			
			<Tabs.Content value="import">
				<ImportPlaylist folder={folder} onClose={() => { open = false; }} />
			</Tabs.Content>

			<Tabs.Content value="spotify">
				<SpotifyURLImport folder={folder} onClose={() => { open = false; }} />
			</Tabs.Content>

			<Tabs.Content value="url">
				<AlertDialog.Header>
					<AlertDialog.Title class="mt-4">Import URL</AlertDialog.Title>
				</AlertDialog.Header>
			</Tabs.Content>
		</Tabs.Root>
	</AlertDialog.Content>
</AlertDialog.Root>