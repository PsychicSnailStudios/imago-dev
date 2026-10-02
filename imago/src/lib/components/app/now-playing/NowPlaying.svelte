<script lang="ts">

	// APP
	
	// COMPONENTS
	import { Rows4, BadgePlus, Square } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";
	import { Button } from "$shadcn/button";

	// CUSTOM COMPONENTS
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
   import AddToPlaylist from "$lib/components/custom/TrackPlaylistEditButton.svelte";
	import ScrollingText from "$lib/components/custom/text-display/ScrollingText.svelte";
	import Queue from "$lib/components/app/now-playing/Queue.svelte";
	import RecentlyPlayed from "$lib/components/app/now-playing/RecentlyPlayed.svelte";
	import LyricsViewer from "$lib/components/custom/LyricsViewer.svelte";

	// SCRIPTS
	import { setSelection } from "$ts/store/session.svelte";
	import { currentlyPlaying, player } from "$ts/audio/audioPlayer.svelte";
	import { fetchArtworkColor } from "$ts/library/artworkLoader";
   import { parseArtistsToString } from "$ts/util/parsers";
	import { invoke } from "@tauri-apps/api/core";

	async function goToAlbumArtist() {
		const name = currentlyPlaying.track?.album_artist ?? null;
		if (!name) return;
		const uid = await invoke<string | null>("get_artist_uid_by_name", { name });
		if (uid) setSelection(uid);
	}

	// VARIABLES
	let showQueue = $state(false);

	let color = $state("var(--muted)");

	// APP FUNCTIONS
	$effect(() => {
		color = "var(--muted)";
		const track = player.track;
		if (!track?.uid) return;
		fetchArtworkColor(track.uid, "track").then(c => { color = c; });
	});

</script>

<div class="app-now-playing-wrapper flex flex-col gap-1 bg-muted rounded-md">
	{#if showQueue}
	<div class="app-queue p-2 rounded-md h-[450px]">
		<Tabs.Root value="queue">
			<Tabs.List>
				<Tabs.Trigger value="queue">Queue</Tabs.Trigger>
				<Tabs.Trigger value="recent">Recently Played</Tabs.Trigger>
				<Tabs.Trigger value="lyrics">Lyrics</Tabs.Trigger>
			</Tabs.List>
			<Queue />
			<RecentlyPlayed />
			<Tabs.Content value="lyrics">
				<ScrollArea class="min-h-0 min-w-0 h-[400px] pl-4 pr-2">
					<LyricsViewer uid={currentlyPlaying.track!.uid} />
				</ScrollArea>
			</Tabs.Content>
		</Tabs.Root>
	</div>
	{/if}

	<div class="app-now-playing bg-muted grid gap-3 p-2 rounded-md items-center" style="background: linear-gradient(90deg, {color} 0%, transparent 75%)">
		{#if currentlyPlaying.track !== null}
			<ArtworkDisplay entity={currentlyPlaying.track} size={64} />

			<div class="flex flex-col min-w-0">
				<ScrollingText
					text={currentlyPlaying.track?.title!}
					class="text-sm font-medium cursor-pointer hover:underline"
					onclick={() => setSelection(currentlyPlaying.track?.uid!)}
					
				/>
				<ScrollingText
					text={parseArtistsToString(currentlyPlaying.track?.artists ?? null)}
					class="text-xs text-muted-foreground cursor-pointer hover:underline"
					onclick={goToAlbumArtist}
					hoverOnly
				/>
			</div>

			<div class="flex flex-col">
				<Button variant="ghost" size="icon" onclick={() => showQueue = !showQueue}>
					{#if showQueue}
						<Square />
					{:else}
						<Rows4 />
					{/if}
				</Button>
				<AddToPlaylist track={currentlyPlaying.track!} />
			</div>
		{/if}
	</div>
</div>

<style>
.app-now-playing {
	grid-template-columns: auto 1fr auto;
	max-height: 85px;
	height: 85px;
}
</style>