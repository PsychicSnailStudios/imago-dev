<script lang="ts">
	import { Folder } from "lucide-svelte";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import DefultPlaylistArt from "$lib/components/pages/playlist/DefultPlaylistArt.svelte";
	import { getTrackArrayFromUID } from "$ts/store/library.svelte";
	import type { Track, Playlist } from "$ts/util/types";

	let { folderPath, artPlaylists } = $props<{ folderPath: string; artPlaylists: Playlist[] }>();

	let tracksByUid = $state<Map<string, Track[]>>(new Map());

	const shown = $derived(artPlaylists.slice(0, 4));

	$effect(() => {
		for (const p of shown) {
			if (!tracksByUid.has(p.uid)) {
				getTrackArrayFromUID(p.uid).then(tracks => {
					tracksByUid = new Map(tracksByUid).set(p.uid, tracks);
				});
			}
		}
	});
</script>

<div class="flex flex-col gap-1 p-2">
	<div class="w-full aspect-square rounded-md overflow-hidden">
		{#if shown.length > 0}
			<div class="grid grid-cols-2 w-full h-full gap-2 p-2">
				{#each shown as p (p.uid)}
					<ArtworkDisplay entity={p}>
						<DefultPlaylistArt tracks={tracksByUid.get(p.uid) ?? []} />
					</ArtworkDisplay>
				{/each}
			</div>
		{:else}
			<div class="w-full h-full flex items-center justify-center">
				<Folder class="size-12 text-muted-foreground/40" />
			</div>
		{/if}
	</div>
	<div class="px-1">
		<p class="text-sm font-medium truncate">{folderPath}</p>
		<p class="text-xs text-muted-foreground">Folder</p>
	</div>
</div>