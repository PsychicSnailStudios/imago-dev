<script lang="ts">
	
	// APP
	import { invoke } from "@tauri-apps/api/core";

	// COMPONENTS
	import { ListMusic } from "lucide-svelte";

	// SCRIPTS
	import { getArtworkColor } from '$ts/util/helpers';
   import type { Track } from "$ts/util/types";

	// PROPS
	let { tracks } = $props<{ tracks: Track[] | Promise<Track[]> }>();

	// VARIABLES
	let colorA = $state("var(--muted)")
	let colorB = $state("var(--muted)")

	// APP FUNCTIONS
	$effect(() => {
		colorA = "var(--muted)"
		colorB = "var(--muted)"

		if (!tracks) return;

		let cancelled = false;

		Promise.resolve(tracks).then((resolved) => {
			if (cancelled || !resolved || resolved.length === 0) return;

			const uidA = resolved[0]?.uid;
			const uidB = resolved[resolved.length - 1]?.uid;

			if (!uidA || !uidB) return;

			invoke("get_track_artwork", { uid: uidA }).then((trackBytes) => {
				if (!cancelled && trackBytes) getArtworkColor(trackBytes as number[], 0.9).then((c) => colorA = c);
			});
			invoke("get_track_artwork", { uid: uidB }).then((trackBytes) => {
				if (!cancelled && trackBytes) getArtworkColor(trackBytes as number[], 0.9).then((c) => colorB = c);
			});
		});

		return () => { cancelled = true; };
	});

</script>

<div class="absolute inset-0 flex items-center justify-center"
	  style="background: linear-gradient(180deg, {colorA}, {colorB})">
	<ListMusic class="app-icon-adaptive" style="mix-blend-mode: difference;" />
</div>