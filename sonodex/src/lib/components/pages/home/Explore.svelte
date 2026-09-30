<script lang="ts">
	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import DefultPlaylistArt from "$lib/components/pages/playlist/DefultPlaylistArt.svelte";

	import { getTrackArrayFromUID } from "$ts/store/library.svelte";
	import { explorePlaylists } from "$ts/config/explore.svelte";
</script>

<div class="flex flex-col gap-3">

	<div class="flex items-center justify-between gap-2 flex-wrap shrink-0">
		<h2 class="text-lg font-semibold tracking-tight">Explore</h2>
	</div>

	<div class="flex gap-2 overflow-x-auto pb-1">
		{#each explorePlaylists as playlist (playlist.uid)}
			<div class="w-36 shrink-0">
				<AudioCard title={playlist.title} subTitle={playlist.owner} entity={playlist} type="playlist">
					<DefultPlaylistArt tracks={getTrackArrayFromUID(playlist.uid)} />
				</AudioCard>
			</div>
		{/each}
	</div>
</div>