<script lang="ts">
	import { BadgePlus } from "lucide-svelte";

	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	import Button from "$shadcn/button/button.svelte";

	import TrackPlaylistContent from "$lib/components/context-menus/AddTrackToPlaylist.svelte";
	import type { Track } from "$ts/util/types";

	let { track } = $props<{ track: Track }>();

	let content = $state<ReturnType<typeof TrackPlaylistContent> | null>(null);
</script>

<DropdownMenu.Root onOpenChange={(open) => { if (open) content?.reset(); }}>
	<DropdownMenu.Trigger>
		{#snippet child({ props }: { props: Record<string, unknown> })}
			<Button {...props} variant="ghost" size="icon"><BadgePlus /></Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content class="w-64">
		<TrackPlaylistContent bind:this={content} {track} />
	</DropdownMenu.Content>
</DropdownMenu.Root>