<script lang="ts">
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import { Input } from "$shadcn/input/index.js";
	
	import { navigateTo, registerFolder } from "$ts/ui/folderSelection.svelte";

	let { open = $bindable(false), parent = null } = $props<{ open: boolean; parent?: string | null }>();

	let folderNameInput = $state("");

	async function handleCreateFolder() {
		if (!folderNameInput.trim()) return;
		const newPath = parent ? `${parent}/${folderNameInput.trim()}` : folderNameInput.trim();
		registerFolder(newPath);
		folderNameInput = "";
		open = false;
		navigateTo(newPath);
	}
</script>

<AlertDialog.Root bind:open={open}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>New Folder</AlertDialog.Title>
			<AlertDialog.Description>
				<Input placeholder="Folder name" bind:value={folderNameInput} class="w-full mt-2" />
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action onclick={handleCreateFolder}>Create</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>