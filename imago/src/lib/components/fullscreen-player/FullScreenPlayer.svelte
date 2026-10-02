<script lang="ts">

	// APP
   import { invoke } from "@tauri-apps/api/core";
	
	// COMPONENTS
	import { Rows4, BadgePlus, Square } from "lucide-svelte";
	import { Button } from "$shadcn/button";

	// CUSTOM COMPONENTS
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
   import AddToPlaylist from "$lib/components/custom/TrackPlaylistEditButton.svelte";
	import ScrollingText from "$lib/components/custom/text-display/ScrollingText.svelte";

	// SCRIPTS
	import { currentlyPlaying, player } from "$ts/audio/audioPlayer.svelte";
	import { getArtworkColor } from "$ts/util/helpers";

	// VARIABLES
	let color = $state("rgb(30, 30, 30)");

	// APP FUNCTIONS
	$effect(() => {
		const track = player.track;
		if (!track?.uid) return;

		invoke("get_track_artwork", { uid: track.uid }).then((bytes) => {
			if (bytes) getArtworkColor(bytes as number[], 0.3).then((c) => color = c);
		});
	});

</script>

<div class="app-now-playing-wrapper flex flex-col gap-1 bg-muted rounded-md">

	<ArtworkDisplay entity={currentlyPlaying.track} size={64} />

	<AddToPlaylist track={currentlyPlaying.track!} />

</div>
