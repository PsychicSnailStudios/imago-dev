<script lang="ts">

	import { invoke } from "@tauri-apps/api/core";
	import X from "@lucide/svelte/icons/x";
	import { Badge } from "$shadcn/badge/index.js";
    import { parseUidType } from "$ts/util/parsers";

	let { tags, uid, canEdit = false }: { tags: string[], uid: string, canEdit?: boolean } = $props();

	async function removeTag(tag: string) {
		let type = parseUidType(uid);

		const new_tags = tags.filter(t => t !== tag);
		tags = new_tags;
		
		await invoke("update_track_metadata", {
			uid,
			update: { tags: JSON.stringify(new_tags) },
		});
	}

</script>

<div class="flex flex-wrap gap-1">
	{#each tags as tag}
		<Badge variant="outline">
			<span class="text-xs">{tag}</span>
			{#if canEdit}
				<button onclick={() => removeTag(tag)}><X class="cursor-pointer w-4 h-4"/></button>
			{/if}
		</Badge>
	{/each}
</div>