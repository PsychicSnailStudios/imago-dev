<script lang="ts">

	// APP
	import { onMount } from "svelte";

	// COMPONENTS
	import * as Dialog from "$shadcn/dialog/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { Label } from "$shadcn/label/index.js";

	// SCRIPTS
	import { createProfile } from "$ts/store/profiles.svelte";

	// PROPS
	let { onComplete }: { onComplete: () => void } = $props();

	// VARIABLES
	let name = $state("");
	let creating = $state(false);

	// APP FUNCTIONS
	onMount(async () => {
		try {
			const { hostname } = await import("@tauri-apps/plugin-os");
			const host = await hostname();
			if (host && !name) name = host;
		} catch {}
	});

	// FUNCTIONS
	async function handleCreate() {
		if (!name.trim()) return;
		creating = true;
		try {
			await createProfile(name.trim(), null, null);
			onComplete();
		} finally {
			creating = false;
		}
	}
</script>

<Dialog.Root open={true} onOpenChange={() => {}} onEscapeKeyDown={(e) => e.preventDefault()} onInteractOutside={(e) => e.preventDefault()}>
	<Dialog.Content class="max-w-sm w-full" showCloseButton={false}>
		<Dialog.Header>
			<Dialog.Title>Welcome to Sonodex</Dialog.Title>
			<Dialog.Description>Create your profile to get started.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-2">
			<div class="space-y-1.5">
				<Label for="profile-name">Your name</Label>
				<Input
					id="profile-name"
					bind:value={name}
					placeholder="Enter your name"
					onkeydown={(e) => { if (e.key === "Enter") handleCreate(); }}
				/>
			</div>
		</div>

		<Dialog.Footer>
			<Button onclick={handleCreate} disabled={creating || !name.trim()} class="w-full">
				{creating ? "Creating…" : "Get Started"}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>