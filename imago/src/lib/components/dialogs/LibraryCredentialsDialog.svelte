<script lang="ts">
	import * as Dialog from "$shadcn/dialog/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";
	import { toast } from "svelte-sonner";
	import { Loader2 } from "lucide-svelte";
	import { updateLibrary } from "$ts/library/libraryRegistry.svelte";

	let {
		open = $bindable(false),
		libUid,
		libName = "",
		message = "",
		allowSkip = false,
		onsaved,
		onskip,
	}: {
		open: boolean;
		libUid: string;
		libName?: string;
		message?: string;
		allowSkip?: boolean;
		onsaved?: () => void;
		onskip?: () => void;
	} = $props();

	let username = $state("");
	let password = $state("");
	let saving = $state(false);

	$effect(() => {
		if (open) {
			username = "";
			password = "";
		}
	});

	async function save() {
		if (!username.trim() || !password) return;
		saving = true;
		try {
			await updateLibrary(libUid, { write_token: `${username.trim()}:${password}` });
			toast.success("Login saved");
			open = false;
			onsaved?.();
		} catch (e) {
			toast.error("Could not save login", { description: String(e) });
		} finally {
			saving = false;
		}
	}

	async function clearLogin() {
		saving = true;
		try {
			await updateLibrary(libUid, { write_token: "" });
			toast.success("Login removed");
			open = false;
		} catch (e) {
			toast.error("Could not remove login", { description: String(e) });
		} finally {
			saving = false;
		}
	}

	function skip() {
		open = false;
		onskip?.();
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Login for {libName || "this library"}</Dialog.Title>
			<Dialog.Description>
				{message || "Enter the username and password for the WebDAV server or remote library. They are stored with this library and used for scanning, playback, downloads and sync."}
			</Dialog.Description>
		</Dialog.Header>

		<div class="flex flex-col gap-3 py-2">
			<div class="flex flex-col gap-1">
				<Label class="text-xs text-muted-foreground">Username</Label>
				<Input bind:value={username} autocomplete="off" />
			</div>
			<div class="flex flex-col gap-1">
				<Label class="text-xs text-muted-foreground">Password</Label>
				<Input
					type="password"
					bind:value={password}
					autocomplete="off"
					onkeydown={(e) => {
						if (e.key === "Enter") save();
					}}
				/>
			</div>
		</div>

		<Dialog.Footer class="gap-2">
			<Button variant="ghost" onclick={clearLogin} disabled={saving}>Remove login</Button>
			{#if allowSkip}
				<Button variant="outline" onclick={skip} disabled={saving}>No login needed</Button>
			{:else}
				<Button variant="outline" onclick={() => (open = false)} disabled={saving}>Cancel</Button>
			{/if}
			<Button onclick={save} disabled={saving || !username.trim() || !password}>
				{#if saving}<Loader2 class="w-4 h-4 mr-1 animate-spin" />{/if}
				Save
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
