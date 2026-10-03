<script lang="ts">
	import { convertFileSrc } from "@tauri-apps/api/core";

	import { Music4, User, DiscAlbum } from "lucide-svelte";

	import type { AudioCatagories, Track, Album, Artist, Playlist } from "$ts/util/types";
	import { artworkUrl, artworkVersion, type ArtworkQuality } from "$ts/library/artworkLoader";
	import { parseUidType } from "$ts/util/parsers";

	type ArtworkEntity = Track | Album | Artist | Playlist;

	let { entity, size = null, previewPath = null, quality = "card" }: {
		entity: ArtworkEntity;
		size?: number | null;
		previewPath?: string | null;
		quality?: ArtworkQuality;
	} = $props();

	const uid = $derived(entity?.uid ?? "");
	const type = $derived(uid ? parseUidType(uid) : ("unknown" as AudioCatagories));

	const artworkThumb = $derived.by(() => {
		if (type === "artist") return (entity as Artist).profile_art_thumb ?? null;
		return (entity as Track | Album | Playlist).artwork_thumb ?? null;
	});

	const artworkPath = $derived.by(() => {
		if (type === "track") return null;
		if (type === "artist") return (entity as Artist).profile_art_path ?? null;
		return (entity as Album | Playlist).artwork_path ?? null;
	});

	const src = $derived.by(() => {
		if (previewPath) return convertFileSrc(previewPath);
		if (artworkPath) return convertFileSrc(artworkPath);
		if (!uid || type === "unknown") return null;
		return artworkUrl(type, uid, quality, artworkVersion(artworkThumb));
	});

	let loadedSrc = $state<string | null>(null);
	let failedSrc = $state<string | null>(null);

	const loaded = $derived(src !== null && loadedSrc === src);
	const failed = $derived(src !== null && failedSrc === src);
</script>

<div
	style={size ? `width: ${size}px; height: ${size}px;` : ""}
	class="rounded-sm overflow-hidden relative bg-muted flex-shrink-0 w-full aspect-square">

	{#if artworkThumb && !loaded && !failed}
		<img
			src={artworkThumb}
			alt=""
			class="absolute inset-0 w-full h-full object-cover"
		/>
	{/if}

	{#if src && !failed}
		<img
			{src}
			alt=""
			loading="lazy"
			decoding="async"
			class="absolute inset-0 w-full h-full object-cover transition-opacity duration-300"
			class:opacity-0={!loaded}
			onload={() => (loadedSrc = src)}
			onerror={() => (failedSrc = src)}
		/>
	{/if}

	{#if failed || (!artworkThumb && !loaded)}
		<div class="absolute inset-0 flex items-center justify-center">
			{#if type === "track"}
				<Music4 class="text-muted-foreground" />
			{:else if type === "album"}
				<DiscAlbum class="text-muted-foreground" />
			{:else if type === "artist"}
				<User class="text-muted-foreground" />
			{:else if type === "playlist"}
				<slot />
			{/if}
		</div>
	{/if}
</div>