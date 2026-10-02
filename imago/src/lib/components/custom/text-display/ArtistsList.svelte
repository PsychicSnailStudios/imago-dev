<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { setSelection } from "$ts/store/session.svelte";
	import { parseArtists } from "$ts/util/parsers";

	let { artists } = $props<{ artists: string | null }>();

	let names = $derived(parseArtists(artists));

	async function goToArtist(name: string) {
		const uid = await invoke<string | null>("get_artist_uid_by_name", { name });
		if (uid) setSelection(uid);
	}
</script>

{#if names.length > 0}
	{#each names as name, i}
		<button onclick={() => goToArtist(name)} class="text-sm truncate cursor-pointer hover:underline">
			<p class="text-sm leading-relaxed">{name}{i < names.length - 1 ? ", " : ""}</p>
		</button>
	{/each}
{/if}
