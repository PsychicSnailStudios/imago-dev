<script lang="ts">
	import * as ContextMenu from "$shadcn/context-menu/index.js";

	import { nestFolder, moveFolderToRoot, deleteFolder, deleteFolderAndContents } from "$ts/drag/dragdrop_folders";
	import { getPlaylists, onLibraryChange } from "$ts/store/library.svelte";
	import type { PlaylistSortField } from "$ts/ui/playlistFolderTree.svelte";
	import type { Playlist } from "$ts/util/types";
	import RenameFolder from "$lib/components/dialogs/playlists/RenameFolder.svelte";

	let renameOpen = $state(false);

	let {
		path,
		folderPaths,
		sortField,
		sortDir,
		onCreatePlaylist,
		onCreateFolder,
	} = $props<{
		path: string;
		folderPaths: string[];
		sortField: PlaylistSortField;
		sortDir: "asc" | "desc";
		onCreatePlaylist: (folder: string) => void;
		onCreateFolder: (parent: string) => void;
	}>();

	let playlists = $state<Playlist[]>([]);

	async function loadPlaylists() {
		playlists = await getPlaylists();
	}

	$effect(() => { loadPlaylists(); });
	$effect(() => {
		const unsub = onLibraryChange("playlists:changed", loadPlaylists);
		return unsub;
	});

	// exclude self and own children from move targets
	const moveTargets = $derived(
		folderPaths.filter((fp: string) => fp !== path && !fp.startsWith(path + "/"))
	);

	async function handleDelete() {
		await deleteFolder(path, playlists, sortField, sortDir);
	}

	async function handleDeleteAll() {
		await deleteFolderAndContents(path);
	}

	async function handleMoveTo(targetPath: string | null) {
		if (targetPath === null) {
			await moveFolderToRoot(path);
		} else {
			await nestFolder(path, targetPath);
		}
	}
</script>

<ContextMenu.Content>
	<ContextMenu.Item onclick={() => onCreatePlaylist(path)}>
		New playlist inside
	</ContextMenu.Item>
	<ContextMenu.Item onclick={() => onCreateFolder(path)}>
		New folder inside
	</ContextMenu.Item>
	<ContextMenu.Separator />
	<ContextMenu.Item onclick={() => { renameOpen = true; }}>
		Rename
	</ContextMenu.Item>
	<ContextMenu.Sub>
		<ContextMenu.SubTrigger>Move to folder</ContextMenu.SubTrigger>
		<ContextMenu.SubContent>
			<ContextMenu.Item onclick={() => handleMoveTo(null)}>
				Home (no folder)
			</ContextMenu.Item>
			{#if moveTargets.length > 0}
				<ContextMenu.Separator />
				{#each moveTargets as fp}
					<ContextMenu.Item onclick={() => handleMoveTo(fp)}>
						{fp}
					</ContextMenu.Item>
				{/each}
			{/if}
		</ContextMenu.SubContent>
	</ContextMenu.Sub>
	<ContextMenu.Separator />
	<ContextMenu.Item class="text-destructive" onclick={handleDelete}>
		Delete Folder
	</ContextMenu.Item>
	<ContextMenu.Item class="text-destructive" onclick={handleDeleteAll}>
		Delete Folder + Contents
	</ContextMenu.Item>
</ContextMenu.Content>

<RenameFolder bind:open={renameOpen} {path} existingPaths={folderPaths} />