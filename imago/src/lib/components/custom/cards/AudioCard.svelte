<script lang="ts">
	// COMPONENTS
	import Button from "$shadcn/button/button.svelte";

	// CUSTOM COMPONENTS
	import { Play } from "lucide-svelte";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";

	// SCRIPTS
	import { setSelection } from "$ts/store/session.svelte";
	import { playTrackByObject, queueTracksByObject } from "$ts/audio/audioManager.svelte";
	import { getTrackArrayFromUID } from "$ts/store/library.svelte";
	import type { AudioCatagories } from "$ts/util/types";

	// PROPS
	let { title, subTitle, entity, type } = $props<{ title: string; subTitle: string | null; entity: any; type: AudioCatagories }>();

	// FUNCTIONS
	async function play(e: MouseEvent) {
		e.stopPropagation();
		if (!entity) return;
		if (type === "track") {
			playTrackByObject(entity);
		} else {
			const tracks = await getTrackArrayFromUID(entity.uid);
			queueTracksByObject(tracks, true, false, entity.uid);
		}
	}
</script>

{#if entity}
<button
	class="w-full text-left flex flex-col gap-2 p-2 rounded-md bg-background border hover:border-primary transition-colors cursor-default justify-start group"
	style="content-visibility: auto; contain-intrinsic-height: auto 260px;"
	onclick={() => setSelection(entity.uid)}
>
	<div class="relative w-full">
		<ArtworkDisplay {entity}>
			<slot />
		</ArtworkDisplay>
		<div class="absolute inset-0 flex items-end justify-end p-1 opacity-0 group-hover:opacity-100 transition-opacity">
			<Button variant="secondary" size="icon" onclick={play}>
				<Play />
			</Button>
		</div>
	</div>

	<div class="min-w-0 grid">
		<p class="text-sm font-medium truncate">{title}</p>
		<p class="text-xs text-muted-foreground truncate">{subTitle}</p>
	</div>
</button>
{/if}