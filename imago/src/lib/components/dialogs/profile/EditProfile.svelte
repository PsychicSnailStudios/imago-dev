<script lang="ts">

	// APP
	import { onMount } from "svelte";
	
	// COMPONENTS
	import { User, Eye, EyeOff, Copy, Check } from "lucide-svelte";

	import * as Dialog from "$shadcn/dialog/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";
	import { Separator } from "$shadcn/separator/index.js";
	
	// SCRIPTS
	import {
		profileState,
		loadProfiles,
		updateProfile,
		getProfileAvatar,
		profileHasPassword,
		setProfilePassword,
		removeProfilePassword,
	} from "$ts/store/profiles.svelte";

	// VARIABLES
	let { open = $bindable(true) } = $props<{ open: boolean }>();
	let saving = $state(false);

	let editName = $state("");
	let editAvatarBytes = $state<number[] | null>(null);
	let editAvatarPreview = $state<string | null>(null);
	let avatarUrls = $state<Record<string, string>>({});

	// PASSWORD
	let hasPassword = $state(false);
	let showPasswordSection = $state(false);
	let newPassword = $state("");
	let confirmPassword = $state("");
	let showNewPw = $state(false);
	let showConfirmPw = $state(false);
	let passwordError = $state("");
	let savingPassword = $state(false);

	// RECOVERY KEY
	let recoveryKey = $state<string | null>(null);
	let copiedKey = $state(false);

	// APP FUNCTIONS
	onMount(async () => {
		await loadProfiles();
		loadAvatars();
		editName = profileState.active?.name ?? "";
		editAvatarBytes = null;
		editAvatarPreview = avatarUrls[profileState.active?.uid ?? ""] ?? null;
		if (profileState.active) {
			hasPassword = await profileHasPassword(profileState.active.uid);
		}
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

	function handleAvatarFileChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		editAvatarPreview = URL.createObjectURL(file);
		const reader = new FileReader();
		reader.onload = () => {
			const arrayBuffer = reader.result as ArrayBuffer;
			editAvatarBytes = Array.from(new Uint8Array(arrayBuffer));
		};
		reader.readAsArrayBuffer(file);
	}

	async function handleSaveEdit() {
		if (!profileState.active || !editName.trim()) return;
		saving = true;
		try {
			await updateProfile(profileState.active.uid, editName.trim(), editAvatarBytes);
			await loadProfiles();
			await loadAvatars();
			open = false;
		} finally {
			saving = false;
		}
	}

	async function handleSetPassword() {
		if (!profileState.active) return;
		passwordError = "";
		if (newPassword.length < 4) {
			passwordError = "Password must be at least 4 characters.";
			return;
		}
		if (newPassword !== confirmPassword) {
			passwordError = "Passwords do not match.";
			return;
		}
		savingPassword = true;
		try {
			const key = await setProfilePassword(profileState.active.uid, newPassword);
			hasPassword = true;
			recoveryKey = key;
			newPassword = "";
			confirmPassword = "";
			showPasswordSection = false;
		} catch (e) {
			passwordError = String(e);
		} finally {
			savingPassword = false;
		}
	}

	async function handleRemovePassword() {
		if (!profileState.active) return;
		await removeProfilePassword(profileState.active.uid);
		hasPassword = false;
		recoveryKey = null;
		showPasswordSection = false;
	}

	async function copyRecoveryKey() {
		if (!recoveryKey) return;
		await navigator.clipboard.writeText(recoveryKey);
		copiedKey = true;
		setTimeout(() => (copiedKey = false), 2000);
	}

	function openPasswordSection() {
		showPasswordSection = true;
		newPassword = "";
		confirmPassword = "";
		passwordError = "";
	}
</script>

<Dialog.Root bind:open={open}>
	<Dialog.Content class="max-w-sm w-full">
		<Dialog.Header>
			<Dialog.Title>Edit Profile</Dialog.Title>
		</Dialog.Header>

		<div class="space-y-4 py-2">
			<div class="flex flex-col items-center gap-3">
				<div class="w-20 h-20 rounded-full bg-muted overflow-hidden flex items-center justify-center flex-shrink-0">
					{#if editAvatarPreview}
						<img src={editAvatarPreview} alt="" class="w-full h-full object-cover" />
					{:else}
						<User class="w-10 h-10 text-muted-foreground" />
					{/if}
				</div>
				<Label
					class="cursor-pointer text-sm text-muted-foreground hover:text-foreground transition-colors"
					for="avatar-upload"
				>
					Change photo
				</Label>
				<input
					id="avatar-upload"
					type="file"
					accept="image/*"
					class="hidden"
					onchange={handleAvatarFileChange}
				/>
			</div>

			<div class="space-y-1.5">
				<Label for="edit-profile-name">Name</Label>
				<Input
					id="edit-profile-name"
					bind:value={editName}
					placeholder="Enter a name"
					onkeydown={(e) => { if (e.key === "Enter") handleSaveEdit(); }}
				/>
			</div>

			<Separator />

			<!-- PASSWORD SECTION -->
			<div class="space-y-2">
				<p class="text-sm font-medium">Password</p>

				{#if recoveryKey}
					<div class="rounded-md border border-yellow-500 bg-yellow-500/10 p-3 space-y-2">
						<p class="text-xs text-yellow-600 dark:text-yellow-400 font-medium">Save your recovery key — it won't be shown again.</p>
						<div class="flex items-center gap-2">
							<code class="flex-1 text-xs font-mono tracking-widest select-all">{recoveryKey}</code>
							<Button variant="ghost" size="icon" class="h-7 w-7 flex-shrink-0" onclick={copyRecoveryKey}>
								{#if copiedKey}
									<Check class="w-3.5 h-3.5 text-green-500" />
								{:else}
									<Copy class="w-3.5 h-3.5" />
								{/if}
							</Button>
						</div>
					</div>
				{/if}

				{#if !showPasswordSection}
					<div class="flex gap-2">
						<Button variant="outline" size="sm" onclick={openPasswordSection}>
							{hasPassword ? "Change password" : "Set password"}
						</Button>
						{#if hasPassword}
							<Button variant="ghost" size="sm" class="text-destructive hover:text-destructive" onclick={handleRemovePassword}>
								Remove
							</Button>
						{/if}
					</div>
				{:else}
					<div class="space-y-2">
						<div class="space-y-1">
							<Label for="new-pw">New password</Label>
							<div class="relative">
								<Input
									id="new-pw"
									type={showNewPw ? "text" : "password"}
									bind:value={newPassword}
									placeholder="Enter password"
									class="pr-9"
								/>
								<button
									type="button"
									class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
									onclick={() => (showNewPw = !showNewPw)}
								>
									{#if showNewPw}
										<EyeOff class="w-4 h-4" />
									{:else}
										<Eye class="w-4 h-4" />
									{/if}
								</button>
							</div>
						</div>

						<div class="space-y-1">
							<Label for="confirm-pw">Confirm password</Label>
							<div class="relative">
								<Input
									id="confirm-pw"
									type={showConfirmPw ? "text" : "password"}
									bind:value={confirmPassword}
									placeholder="Confirm password"
									class="pr-9"
									onkeydown={(e) => { if (e.key === "Enter") handleSetPassword(); }}
								/>
								<button
									type="button"
									class="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
									onclick={() => (showConfirmPw = !showConfirmPw)}
								>
									{#if showConfirmPw}
										<EyeOff class="w-4 h-4" />
									{:else}
										<Eye class="w-4 h-4" />
									{/if}
								</button>
							</div>
						</div>

						{#if passwordError}
							<p class="text-xs text-destructive">{passwordError}</p>
						{/if}

						<div class="flex gap-2">
							<Button size="sm" onclick={handleSetPassword} disabled={savingPassword}>
								{savingPassword ? "Saving…" : "Save password"}
							</Button>
							<Button variant="ghost" size="sm" onclick={() => (showPasswordSection = false)}>
								Cancel
							</Button>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => open = false}>Cancel</Button>
			<Button onclick={handleSaveEdit} disabled={saving || !editName.trim()}>
				{saving ? "Saving…" : "Save"}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>