<script lang="ts">
		
	// COMPONENTS
   import { Loader2 } from "lucide-svelte";
	import { Button } from "$shadcn/button/index.js";
	import ScrollArea from "$shadcn/scroll-area/scroll-area.svelte";
	import * as Tabs from "$shadcn/tabs/index.js";

	// SCRIPTS
	import { getLyrics, library } from "$ts/store/library.svelte";
	import type { Lyrics } from "$ts/util/types";
   import { fetchLyrics } from "$ts/library/enrichment";

	// VARIABLES
	let { uid } = $props<{ uid: string }>();
	let lyrics: Lyrics | null = $state(null);
	let fetchingLyrics = $state(false);
	
	$effect(() => {
		lyrics = null;
		if (!uid) return;
		getLyrics(uid).then(result => {
			lyrics = result;
		});
	});

	async function doFetchLyrics() {
		if (!uid || fetchingLyrics) return;
		fetchingLyrics = true;
		try {
			await fetchLyrics(uid);
			lyrics = await getLyrics(uid);
		} finally {
			fetchingLyrics = false;
		}
	}
</script>

<Tabs.Content value="lyrics">
	<ScrollArea class="min-h-0 min-w-0 h-[400px] pl-4 pr-2">
		{#if fetchingLyrics}
			<div class="flex items-center gap-2 text-muted-foreground text-sm">
				<Loader2 class="animate-spin size-4" />
				<span>Fetching lyrics...</span>
			</div>
		{:else if lyrics === null}
			<Button variant="outline" onclick={doFetchLyrics}>Get Lyrics</Button>
		{:else if lyrics.instrumental}
			<p class="text-muted-foreground text-sm">This track is instrumental.</p>
		{:else if lyrics.plain}
			<pre class="text-sm whitespace-pre-wrap font-sans leading-relaxed">{lyrics.plain}</pre>
		{:else}
			<p class="text-muted-foreground text-sm">No lyrics available.</p>
		{/if}
	</ScrollArea>
</Tabs.Content>