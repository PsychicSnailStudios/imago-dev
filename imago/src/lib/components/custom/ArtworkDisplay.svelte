<script lang="ts">
	import { untrack } from "svelte";
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";

	import { Music4, User, DiscAlbum } from "lucide-svelte";

	import type { AudioCatagories, Track, Album, Artist, Playlist } from "$ts/util/types";
	import { artworkCache, artworkInflight } from "$ts/library/artworkLoader";
	import { parseUidType } from "$ts/util/parsers";

	type ArtworkEntity = Track | Album | Artist | Playlist;

	let { entity, size = null, previewPath = null }: {
		entity: ArtworkEntity;
		size?: number | null;
		previewPath?: string | null;
	} = $props();

	const commandMap: Partial<Record<AudioCatagories, string>> = {
		track: "get_track_artwork",
		album: "get_album_artwork",
		artist: "get_artist_profile_art",
		playlist: "get_playlist_artwork",
	};

	const uid = $derived(entity?.uid ?? "");
	const type = $derived(uid ? parseUidType(uid) : "unknown" as AudioCatagories);

	const artworkThumb = $derived.by(() => {
		if (type === "artist") return (entity as Artist).profile_art_thumb ?? null;
		return (entity as Track | Album | Playlist).artwork_thumb ?? null;
	});

	const artworkPath = $derived.by(() => {
		if (type === "track") return null;
		if (type === "artist") return (entity as Artist).profile_art_path ?? null;
		return (entity as Album | Playlist).artwork_path ?? null;
	});

	let artworkUrl: string | null = $state(null);
	let loaded = $state(false);
	let el: HTMLDivElement;
	let fetchedKey = $state<string | null>(null);

	async function fetchAndCache(cacheKey: string, command: string, fetchUid: string): Promise<string | null> {
		if (artworkCache.has(cacheKey)) return artworkCache.get(cacheKey)!;
		if (artworkInflight.has(cacheKey)) return artworkInflight.get(cacheKey)!;

		const promise = invoke<number[] | null>(command, { uid: fetchUid }).then((bytes) => {
			let url: string | null = null;
			if (bytes) {
				const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
				url = URL.createObjectURL(blob);
			}
			artworkCache.set(cacheKey, url);
			artworkInflight.delete(cacheKey);
			return url;
		}).catch(() => {
			artworkCache.set(cacheKey, null);
			artworkInflight.delete(cacheKey);
			return null;
		});

		artworkInflight.set(cacheKey, promise);
		return promise;
	}

	$effect(() => {
		if (previewPath) {
			artworkUrl = convertFileSrc(previewPath);
			fetchedKey = null;
			loaded = false;
			return;
		}

		if (artworkPath) {
			artworkUrl = convertFileSrc(artworkPath);
			fetchedKey = null;
			loaded = false;
			return;
		}

		const currentUid = uid;
		const currentType = type;
		const currentKey = `${currentType}:${currentUid}`;

		if (untrack(() => fetchedKey) === currentKey && untrack(() => artworkUrl) !== null) {
			return;
		}

		loaded = false;

		if (artworkCache.has(currentKey)) {
			artworkUrl = artworkCache.get(currentKey) ?? null;
			fetchedKey = currentKey;
			return;
		}

		artworkUrl = null;

		let observer: IntersectionObserver | null = null;

		if (el) {
			observer = new IntersectionObserver(async ([entry]) => {
				if (entry.isIntersecting) {
					observer?.disconnect();
					const command = commandMap[currentType];
					if (!command) return;
					const url = await fetchAndCache(currentKey, command, currentUid);
					artworkUrl = url;
					fetchedKey = currentKey;
				}
			}, { rootMargin: "200px" });

			observer.observe(el);
		}

		return () => {
			observer?.disconnect();
		};
	});
</script>

<div bind:this={el}
	style={size ? `width: ${size}px; height: ${size}px;` : ""}
	class="rounded-sm overflow-hidden relative bg-muted flex-shrink-0 w-full aspect-square">

	{#if artworkThumb && !loaded}
		<img
			src={artworkThumb}
			alt=""
			class="absolute inset-0 w-full h-full object-cover"
			style="filter: blur(4px); transform: scale(1.1);"
		/>
	{/if}

	{#if artworkUrl}
		<img
			src={artworkUrl}
			alt=""
			class="absolute inset-0 w-full h-full object-cover transition-opacity duration-500 aspect-square"
			class:opacity-0={!loaded}
			onload={() => loaded = true}
		/>
	{:else if !artworkThumb}
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