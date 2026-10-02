<script lang="ts">

	// APP
	import { onMount } from "svelte";
	
	// COMPONENTS
	import * as Dialog from "$shadcn/dialog/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";
	
	// SCRIPTS
	import { profileState, loadProfiles, createProfile, getProfileAvatar, } from "$ts/store/profiles.svelte";

	// VARIABLES
	let { open = $bindable(true) } = $props<{ open: boolean }>();
	let newName = $state("");
	let creating = $state(false);

	let avatarUrls = $state<Record<string, string>>({});

	// APP FUNCTIONS
	onMount(async () => {
		loadAvatars();
	});

	// FUNCTIONS
	async function loadAvatars() {
		for (const p of profileState.all) {
			const bytes = await getProfileAvatar(p.uid);
			if (bytes) {
				const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
				avatarUrls[p.uid] = URL.createObjectURL(blob);
			}
		}
	}

	async function handleCreate() {
		if (!newName.trim()) return;
		creating = true;
		try {
			await createProfile(newName.trim(), null, null);
			await loadAvatars();
			newName = "";
			open = false;
		} finally {
			creating = false;
		}
	}
</script>

<Dialog.Root bind:open={open}>
	<Dialog.Content class="max-w-sm w-full">
		<Dialog.Header>
			<Dialog.Title>New Profile</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3 py-2">
			<div class="space-y-1.5">
				<Label for="new-profile-name">Name</Label>
				<Input
					id="new-profile-name"
					bind:value={newName}
					placeholder="Enter a name"
					onkeydown={(e) => { if (e.key === "Enter") handleCreate(); }}
				/>
			</div>
		</div>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => open = false}>Cancel</Button>
			<Button onclick={handleCreate} disabled={creating || !newName.trim()}>
				{creating ? "Creating…" : "Create"}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>