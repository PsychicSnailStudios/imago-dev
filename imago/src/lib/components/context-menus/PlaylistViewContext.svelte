<script lang="ts">

	// COMPONENTS
	import * as ContextMenu from "$shadcn/context-menu/index.js";

	// PROPS
	let {
		path,
		onCreatePlaylist,
		onCreateFolder,
	} = $props<{
		path: string | null;
		onCreatePlaylist: (folder: string) => void;
		onCreateFolder: (parent: string) => void;
	}>();

</script>

<ContextMenu.Content>
	<ContextMenu.Group>
		<ContextMenu.Item onSelect={() => onCreatePlaylist(path)}>New Playlist</ContextMenu.Item>
		<ContextMenu.Item onSelect={() => onCreateFolder(path)}>New Folder</ContextMenu.Item>		
	</ContextMenu.Group>
</ContextMenu.Content>