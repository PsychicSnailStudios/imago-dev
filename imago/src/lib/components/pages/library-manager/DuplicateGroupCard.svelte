<script lang="ts">
	import { Trash2, Check, FileX, GitMerge } from "lucide-svelte";
	import * as Tooltip from "$shadcn/tooltip/index.js";
	import { Button, buttonVariants } from "$shadcn/button/index.js";
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import ArtistsList from "$lib/components/custom/text-display/ArtistsList.svelte";
	import { formatDuration } from "$ts/util/helpers";
	import { keepTrack, deleteTrackFile, mergeRemoteLocal } from "$ts/library/libraryManager";
	import type { DuplicateGroup } from "$ts/util/types";

	let { group, onresolved }: { group: DuplicateGroup; onresolved: () => void } = $props();

	const isRemoteLocalPair = $derived(
		group.tracks.length === 2 &&
		group.tracks.some(t => t.path && t.path.length > 0) &&
		group.tracks.some(t => (!t.path || t.path.length === 0) && t.remote_path && t.remote_path.length > 0)
	);

	async function handleKeep(uid: string) {
		await keepTrack(uid, group);
		onresolved();
	}

	async function handleDelete(uid: string, path: string) {
		await deleteTrackFile(uid, path);
		onresolved();
	}

	async function handleMerge() {
		await mergeRemoteLocal(group);
		onresolved();
	}
</script>

<div class="border-2 rounded-md overflow-hidden">
	<div class="px-3 py-2 bg-muted text-xs text-muted-foreground font-medium flex items-center justify-between">
		<span>{group.tracks.length} duplicates — {group.tracks[0]?.title ?? "Unknown"}</span>
		{#if isRemoteLocalPair}
			<Tooltip.Root>
				<Tooltip.Trigger
					class={buttonVariants({ variant: "outline", size: "sm" })}
					onclick={handleMerge}
				>
					<GitMerge class="w-3 h-3 mr-1" /> Merge
				</Tooltip.Trigger>
				<Tooltip.Content><p>Merge into one track with both local and remote paths</p></Tooltip.Content>
			</Tooltip.Root>
		{/if}
	</div>
	<div class="flex flex-col divide-y divide-border">
		{#each group.tracks as track}
			<div class="flex gap-2 p-2 items-center justify-between">
				<div class="flex gap-2 min-w-0 flex-1">
					<ArtworkDisplay entity={track} size={36} />
					<div class="min-w-0 grid">
						<span class="text-sm truncate">{track.title}</span>
						<div class="text-xs text-muted-foreground truncate">
							<ArtistsList artists={track.artists} />
							{#if track.duration_ms}
								{" · "}{formatDuration(track.duration_ms)}
							{/if}
						</div>
						<span class="text-xs text-muted-foreground truncate opacity-60">
							{track.path || track.remote_path || "No file path"}
						</span>
					</div>
				</div>

				<div class="flex gap-2 shrink-0">
					<Tooltip.Root>
						<Tooltip.Trigger
							class={buttonVariants({ variant: "outline", size: "icon" })}
							onclick={() => handleKeep(track.uid)}
						>
							<Check />
						</Tooltip.Trigger>
						<Tooltip.Content><p>Keep this, remove others from library</p></Tooltip.Content>
					</Tooltip.Root>

					{#if track.path}
						<Tooltip.Root>
							<Tooltip.Trigger
								class={buttonVariants({ variant: "destructive", size: "icon" })}
								onclick={() => handleDelete(track.uid, track.path)}
							>
								<Trash2 />
							</Tooltip.Trigger>
							<Tooltip.Content><p>Delete file from disk</p></Tooltip.Content>
						</Tooltip.Root>
					{:else}
						<Tooltip.Root>
							<Tooltip.Trigger
								class={buttonVariants({ variant: "destructive", size: "icon" })}
								onclick={() => handleKeep(group.tracks.find(t => t.uid !== track.uid)?.uid ?? group.tracks[0].uid)}
							>
								<FileX />
							</Tooltip.Trigger>
							<Tooltip.Content><p>Remove ghost track from library</p></Tooltip.Content>
						</Tooltip.Root>
					{/if}
				</div>
			</div>
		{/each}
	</div>
</div>