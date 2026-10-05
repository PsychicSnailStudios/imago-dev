<script lang="ts">

	// COMPONENTS
	import * as Tabs from "$shadcn/tabs/index.js";
	import ScrollArea from "$lib/components/custom/ScrollView.svelte";

	// CUSTOM COMPONENTS
	import QueueTrackItem from "$lib/components/app/now-playing/QueueTrackItem.svelte";

	// SCRIPTS
	import { getPlayedTracks } from "$ts/audio/audioManager.svelte";

	// VARIABLES
	let recentTracks = $derived(getPlayedTracks());

</script>

<Tabs.Content value="recent" class="relative flex-1 min-h-0">
	<ScrollArea class="absolute inset-0 min-h-0 min-w-0">
		{#each recentTracks as track}
			<QueueTrackItem track={track} />	
		{/each}
	</ScrollArea>
</Tabs.Content>