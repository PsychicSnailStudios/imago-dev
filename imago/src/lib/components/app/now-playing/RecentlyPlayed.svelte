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

<Tabs.Content value="recent">
	<ScrollArea class="min-h-0 min-w-0 h-[400px]">
		<div>
			{#each recentTracks as track}
				<QueueTrackItem track={track} />	
			{/each}
		</div>
	</ScrollArea>
</Tabs.Content>
