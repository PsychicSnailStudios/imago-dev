<script lang="ts">

	// APP
	import { onMount } from "svelte";
	
	// COMPONENTS
	import { User, Lock, Eye, EyeOff } from "lucide-svelte";

	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import * as Dialog from "$shadcn/dialog/index.js";
	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";
	import { Button } from "$shadcn/button/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";
	
	// SCRIPTS
	import {
		profileState,
		loadProfiles,
		switchProfile,
		deleteProfile,
		getProfileAvatar,
		profileHasPassword,
		verifyProfilePassword,
		verifyRecoveryKey,
	} from "$ts/store/profiles.svelte";
	import { saveSessionState } from "$ts/store/session.svelte";
	import { savePlayerState } from "$ts/audio/audioManager.svelte";
	
	// VARIABLES
	let { open = $bindable(true), openAdd = $bindable(true) } = $props<{ open: boolean, openAdd: boolean }>();

	let avatarUrls = $state<Record<string, string>>({});
	let passwordProtected = $state<Record<string, boolean>>({});

	// PASSWORD PROMPT STATE
	let promptUid = $state<string | null>(null);
	let promptPassword = $state("");
	let promptShowPw = $state(false);
	let promptError = $state("");
	let promptVerifying = $state(false);
	let showRecovery = $state(false);
	let recoveryInput = $state("");
	let recoveryError = $state("");
	let pendingDeleteUid = $state<string | null>(null);
	let deletePassword = $state("");
	let deleteShowPw = $state(false);
	let deleteError = $state("");
	let deleteVerifying = $state(false);

	// APP FUNCTIONS
	onMount(async () => {
		await loadProfiles();
		await loadAvatars();
		await loadPasswordFlags();
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

	async function loadPasswordFlags() {
		for (const p of profileState.all) {
			if (p.uid !== profileState.active?.uid) {
				passwordProtected[p.uid] = await profileHasPassword(p.uid);
			}
		}
	}

	async function handleSwitch(uid: string) {
		if (uid === profileState.active?.uid) return;
		if (passwordProtected[uid]) {
			promptUid = uid;
			promptPassword = "";
			promptError = "";
			promptShowPw = false;
			showRecovery = false;
			recoveryInput = "";
			recoveryError = "";
			return;
		}
		await doSwitch(uid);
	}

	async function doSwitch(uid: string) {
		const activeUid = profileState.active?.uid;
		if (activeUid) {
			saveSessionState(activeUid);
			savePlayerState();
		}
		await switchProfile(uid);
	}

	async function handlePasswordSubmit() {
		if (!promptUid) return;
		promptVerifying = true;
		promptError = "";
		try {
			const ok = await verifyProfilePassword(promptUid, promptPassword);
			if (ok) {
				const uid = promptUid;
				promptUid = null;
				await doSwitch(uid);
			} else {
				promptError = "Incorrect password.";
			}
		} catch {
			promptError = "Verification failed.";
		} finally {
			promptVerifying = false;
		}
	}

	async function handleRecoverySubmit() {
		if (!promptUid) return;
		promptVerifying = true;
		recoveryError = "";
		try {
			const ok = await verifyRecoveryKey(promptUid, recoveryInput.trim());
			if (ok) {
				const uid = promptUid;
				promptUid = null;
				await doSwitch(uid);
			} else {
				recoveryError = "Invalid recovery key.";
			}
		} catch {
			recoveryError = "Verification failed.";
		} finally {
			promptVerifying = false;
		}
	}

	async function handleDelete(uid: string) {
		if (passwordProtected[uid]) {
			pendingDeleteUid = uid;
			deletePassword = "";
			deleteError = "";
			deleteShowPw = false;
			return;
		}
		await doDelete(uid);
	}

	async function doDelete(uid: string) {
		await deleteProfile(uid);
		await loadAvatars();
		await loadPasswordFlags();
	}

	async function handleDeletePasswordSubmit() {
		if (!pendingDeleteUid) return;
		deleteVerifying = true;
		deleteError = "";
		try {
			const ok = await verifyProfilePassword(pendingDeleteUid, deletePassword);
			if (ok) {
				const uid = pendingDeleteUid;
				pendingDeleteUid = null;
				await doDelete(uid);
			} else {
				deleteError = "Incorrect password.";
			}
		} catch {
			deleteError = "Verification failed.";
		} finally {
			deleteVerifying = false;
		}
	}

	function cancelPrompt() {
		promptUid = null;
		promptPassword = "";
		promptError = "";
		showRecovery = false;
		recoveryInput = "";
		recoveryError = "";
	}

</script>

<Dialog.Root bind:open={open}>
	<Dialog.Content class="max-w-sm w-full">
		<Dialog.Header>
			<Dialog.Title>Switch Profile</Dialog.Title>
		</Dialog.Header>
		
		<div class="flex flex-col gap-3 h-[350px]">
			<ScrollArea class="min-h-0 min-w-0 h-full">
				<div class="flex flex-col gap-2 pr-4">
					{#each profileState.all as profile}
						<div class="flex items-center gap-3 p-2 rounded-md border {profile.uid === profileState.active?.uid ? 'border-primary bg-muted' : 'hover:bg-muted/50'}">

							<div class="w-10 h-10 rounded-full bg-muted overflow-hidden flex items-center justify-center flex-shrink-0">
								{#if avatarUrls[profile.uid]}
									<img src={avatarUrls[profile.uid]} alt="" class="w-full h-full object-cover" />
								{:else}
									<User class="w-5 h-5 text-muted-foreground" />
								{/if}
							</div>

							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-1.5">
									<p class="text-sm font-medium truncate">{profile.name}</p>
									{#if passwordProtected[profile.uid]}
										<Lock class="w-3 h-3 text-muted-foreground flex-shrink-0" />
									{/if}
								</div>
								{#if profile.uid === profileState.active?.uid}
									<p class="text-xs text-muted-foreground">Active</p>
								{/if}
							</div>

							<div class="flex gap-1 flex-shrink-0">
								{#if profile.uid !== profileState.active?.uid}
									<Button variant="outline" size="sm" onclick={() => handleSwitch(profile.uid)}>
										Switch
									</Button>
									<AlertDialog.Root>
									<AlertDialog.Trigger>
										<Button variant="destructive" size="sm">
											Delete
										</Button>
									</AlertDialog.Trigger>
									<AlertDialog.Content>
										<AlertDialog.Header>
											<AlertDialog.Title>Delete profile?</AlertDialog.Title>
											<AlertDialog.Description>
												This will permanently delete {profile.name}'s profile and all their library data.
											</AlertDialog.Description>
										</AlertDialog.Header>
										<AlertDialog.Footer>
											<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
											<AlertDialog.Action onclick={() => handleDelete(profile.uid)}>Delete</AlertDialog.Action>
										</AlertDialog.Footer>
									</AlertDialog.Content>
								</AlertDialog.Root>
								{/if}
							</div>

						</div>
					{/each}
				</div>
			</ScrollArea>
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => openAdd = true}>Add Profile</Button>
			<Button variant="outline" onclick={() => open = false}>Close</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- PASSWORD PROMPT DIALOG -->
<Dialog.Root open={promptUid !== null} onOpenChange={(v) => { if (!v) cancelPrompt(); }}>
	<Dialog.Content class="max-w-xs w-full">
		<Dialog.Header>
			<Dialog.Title>{showRecovery ? "Recovery Key" : "Enter Password"}</Dialog.Title>
			<Dialog.Description>
				{#if showRecovery}
					Enter your recovery key to switch profiles.
				{:else}
					This profile is password protected.
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-3 py-1">
			{#if !showRecovery}
				<div class="space-y-1">
					<Label for="switch-pw">Password</Label>
					<div class="relative">
						<Input
							id="switch-pw"
							type={promptShowPw ? "text" : "password"}
							bind:value={promptPassword}
							placeholder="Enter password"
							class="pr-9"
							onkeydown={(e) => { if (e.key === "Enter") handlePasswordSubmit(); }}
						/>
						<button
							type="button"
							class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
							onclick={() => (promptShowPw = !promptShowPw)}
						>
							{#if promptShowPw}
								<EyeOff class="w-4 h-4" />
							{:else}
								<Eye class="w-4 h-4" />
							{/if}
						</button>
					</div>
					{#if promptError}
						<p class="text-xs text-destructive">{promptError}</p>
					{/if}
				</div>
				<button
					type="button"
					class="text-xs text-muted-foreground hover:text-foreground transition-colors"
					onclick={() => { showRecovery = true; recoveryError = ""; recoveryInput = ""; }}
				>
					Forgot password?
				</button>
			{:else}
				<div class="space-y-1">
					<Label for="recovery-key">Recovery key</Label>
					<Input
						id="recovery-key"
						bind:value={recoveryInput}
						placeholder="XXXX-XXXX-XXXX-XXXX-XXXX-XXXX"
						class="font-mono text-sm tracking-widest"
						onkeydown={(e) => { if (e.key === "Enter") handleRecoverySubmit(); }}
					/>
					{#if recoveryError}
						<p class="text-xs text-destructive">{recoveryError}</p>
					{/if}
				</div>
				<button
					type="button"
					class="text-xs text-muted-foreground hover:text-foreground transition-colors"
					onclick={() => { showRecovery = false; promptError = ""; promptPassword = ""; }}
				>
					Back to password
				</button>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={cancelPrompt}>Cancel</Button>
			{#if !showRecovery}
				<Button onclick={handlePasswordSubmit} disabled={promptVerifying || !promptPassword}>
					{promptVerifying ? "Checking…" : "Unlock"}
				</Button>
			{:else}
				<Button onclick={handleRecoverySubmit} disabled={promptVerifying || !recoveryInput}>
					{promptVerifying ? "Checking…" : "Unlock"}
				</Button>
			{/if}
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- DELETE PASSWORD PROMPT -->
<Dialog.Root open={pendingDeleteUid !== null} onOpenChange={(v) => { if (!v) { pendingDeleteUid = null; deletePassword = ""; deleteError = ""; } }}>
	<Dialog.Content class="max-w-xs w-full">
		<Dialog.Header>
			<Dialog.Title>Confirm Delete</Dialog.Title>
			<Dialog.Description>
				Enter the profile password to delete it.
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-3 py-1">
			<div class="space-y-1">
				<Label for="delete-pw">Password</Label>
				<div class="relative">
					<Input
						id="delete-pw"
						type={deleteShowPw ? "text" : "password"}
						bind:value={deletePassword}
						placeholder="Enter password"
						class="pr-9"
						onkeydown={(e) => { if (e.key === "Enter") handleDeletePasswordSubmit(); }}
					/>
					<button
						type="button"
						class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
						onclick={() => (deleteShowPw = !deleteShowPw)}
					>
						{#if deleteShowPw}
							<EyeOff class="w-4 h-4" />
						{:else}
							<Eye class="w-4 h-4" />
						{/if}
					</button>
				</div>
				{#if deleteError}
					<p class="text-xs text-destructive">{deleteError}</p>
				{/if}
			</div>
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => { pendingDeleteUid = null; deletePassword = ""; deleteError = ""; }}>Cancel</Button>
			<Button variant="destructive" onclick={handleDeletePasswordSubmit} disabled={deleteVerifying || !deletePassword}>
				{deleteVerifying ? "Checking…" : "Delete"}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>