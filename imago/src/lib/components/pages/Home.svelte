<script lang="ts">

	// APP
	import { onMount } from "svelte";
	
	// COMPONENTS
	import { Pencil, User, Settings } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";
	import { Button } from "$shadcn/button/index.js";
	
	// CUSTOM COMPONENTS
	import TopTracks from "$lib/components/pages/home/Analytics.svelte";
   import Resume from "$lib/components/pages/home/Resume.svelte";
   import Explore from "$lib/components/pages/home/Explore.svelte";
	import AddProfile from "$lib/components/dialogs/profile/AddProfile.svelte";
	import EditProfile from "$lib/components/dialogs/profile/EditProfile.svelte";
	import SwichProfile from "$lib/components/dialogs/profile/SwichProfile.svelte";
	
	// SCRIPTS
	import { profileState, loadProfiles, getProfileAvatar, } from "$ts/store/profiles.svelte";
   import { setView } from "$ts/store/session.svelte";

	// VARIABLES
	let swichOpen = $state(false);
	let editOpen = $state(false);
	let newOpen = $state(false);

	let avatarUrls = $state<Record<string, string>>({});

	// APP FUNCTIONS
	onMount(async () => {
		// await loadProfiles();
		loadAvatars();
	});

	async function loadAvatars() {
		for (const p of profileState.all) {
			const bytes = await getProfileAvatar(p.uid);
			if (bytes) {
				const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
				avatarUrls[p.uid] = URL.createObjectURL(blob);
			}
		}
	}

</script>

<EditProfile bind:open={editOpen} />
<SwichProfile bind:open={swichOpen} bind:openAdd={newOpen} />
<AddProfile bind:open={newOpen} />

<div class="flex flex-col gap-4 p-2 border-2 rounded-md h-full w-full overflow-hidden">

	<div class="flex items-center justify-between">
		<div class="flex gap-2 items-center">
			<div class="w-8 h-8 rounded-full bg-muted overflow-hidden flex items-center justify-center shrink-0">
				{#if avatarUrls[profileState.active?.uid as string]}
					<img src={avatarUrls[profileState.active?.uid as string]} alt="" class="w-full h-full object-cover" />
				{:else}
					<User class="w-5 h-5 text-muted-foreground" />
				{/if}
			</div>
			<h1 class="h1 text-2xl text-primary">{profileState.active?.name}</h1>
		</div>

		<div class="flex items-center gap-2">
			<Button variant="outline" size="sm" onclick={() => swichOpen = true}>
				Swich Profile
			</Button>
			<Button variant="ghost" size="sm" onclick={() => editOpen = true}>
				<Pencil />
			</Button>
			<Button variant="ghost" size="sm" onclick={() => setView("settings")}>
				<Settings />
			</Button>
		</div>
	</div>

	<ScrollArea class="flex-1 min-h-0 min-w-0">
		<div class="flex flex-col gap-6 p-2 pr-3">
			<Resume />
			<Explore />
			<TopTracks />
		</div>
	</ScrollArea>

</div>