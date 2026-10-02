<script lang="ts">
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Button } from "$shadcn/button/index.js";

	import { renameFolder } from "$ts/drag/dragdrop_folders";
	import { folderLabel } from "$ts/ui/playlistFolderTree.svelte";

	let { open = $bindable(false), path, existingPaths = [] } = $props<{
		open: boolean;
		path: string;
		existingPaths?: string[];
	}>();

	let nameInput = $state("");
	let error = $state("");
	let saving = $state(false);

	$effect(() => {
		if (open) {
			nameInput = folderLabel(path);
			error = "";
		}
	});

	async function handleRename() {
		const name = nameInput.trim();
		if (!name) {
			error = "Enter a folder name";
			return;
		}
		if (name.includes("/")) {
			error = "Folder names can't contain /";
			return;
		}

		const parent = path.includes("/") ? path.split("/").slice(0, -1).join("/") : null;
		const newPath = parent ? `${parent}/${name}` : name;

		if (newPath === path) {
			open = false;
			return;
		}
		if (existingPaths.some((p: string) => p !== path && p.toLowerCase() === newPath.toLowerCase())) {
			error = "A folder with that name already exists";
			return;
		}

		saving = true;
		try {
			await renameFolder(path, name);
			open = false;
		} catch {
			error = "Couldn't rename folder";
		} finally {
			saving = false;
		}
	}
</script>

<AlertDialog.Root bind:open={open}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Rename Folder</AlertDialog.Title>
		</AlertDialog.Header>
		<div class="space-y-2">
			<Input
				placeholder="Folder name"
				bind:value={nameInput}
				class="w-full"
				onkeydown={(e) => { if (e.key === "Enter") handleRename(); }}
			/>
			{#if error}
				<p class="text-sm text-destructive">{error}</p>
			{/if}
		</div>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
			<Button onclick={handleRename} disabled={saving}>Rename</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
