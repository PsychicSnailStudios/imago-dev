<script lang="ts">
	import type { Update } from "@tauri-apps/plugin-updater";
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import { downloadAndInstall } from "$lib/updater.svelte";

	let {
		open = $bindable(false),
		update,
	}: { open?: boolean; update?: Update | null } = $props();

	let installing = $state(false);
	let progress = $state<{ downloaded: number; total: number | null }>({ downloaded: 0, total: null });

	async function handleUpdate() {
		if (!update) return;
		installing = true;
		await downloadAndInstall(update, (downloaded, total) => {
			progress = { downloaded, total };
		});
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Update Available</AlertDialog.Title>
			<AlertDialog.Description>
				{#if installing}
					{#if progress.total}
						Downloading… {Math.round((progress.downloaded / progress.total) * 100)}%
					{:else}
						Downloading…
					{/if}
				{:else}
					Version {update?.version} is available.
				{/if}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel onclick={() => { open = false; }}>Later</AlertDialog.Cancel>
			<AlertDialog.Action onclick={handleUpdate} disabled={installing}>
				{installing ? "Installing…" : "Update Now"}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>