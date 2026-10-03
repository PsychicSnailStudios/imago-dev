<script lang="ts">
	import { untrack } from "svelte";
	import { invoke } from "@tauri-apps/api/core";

	import { Pencil } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import { Button } from "$shadcn/button/index.js";
	import ScrollArea from "$lib/components/custom/ScrollView.svelte";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import AudioCard from "$lib/components/custom/cards/AudioCard.svelte";
	import NavButtons from "$lib/components/custom/NavButtons.svelte";
	import ArtistTopTracks from "$lib/components/pages/home/ArtistTopTracks.svelte";
	import TagList from "$lib/components/custom/tags/TagList.svelte";

	import { selection } from "$ts/store/session.svelte";
	import { getArtist, getArtistAlbums, getArtists, onLibraryChange, onSingleChange } from "$ts/store/library.svelte";
	import { openEditModal } from "$ts/ui/editModal.svelte";
	import { currentArtistTab } from "$ts/store/session.svelte";
	import type { Artist, Album } from "$ts/util/types";
	import { parseTags } from "$ts/util/parsers";

	let bannerUrl = $state<string | null>(null);
	let artist = $state<Artist | null>(null);
	let artistAlbums = $state<Album[]>([]);

	let genres = $derived(artist?.genres ? parseTags(artist.genres) : null);
	let tags = $derived(artist?.tags ? parseTags(artist.tags) : null);
	let akaList = $derived<string[]>(artist?.aka ? (JSON.parse(artist.aka) as string[]) : []);

	async function loadArtist() {
		const uid = selection.uid;
		if (!uid) { artist = null; artistAlbums = []; return; }
		artist = await getArtist(uid);
		if (!artist) { artistAlbums = []; return; }
		await loadArtistAlbums();
	}

	async function loadArtistAlbums() {
		if (!artist) { artistAlbums = []; return; }

		// Resolve aka UIDs by name-matching against all artists
		const akaNames = new Set(akaList.map(n => n.toLowerCase()));
		const akaUids: string[] = [];
		if (akaNames.size > 0) {
			const all = await getArtists();
			for (const a of all) {
				if (akaNames.has(a.name.toLowerCase())) akaUids.push(a.uid.toString());
			}
		}

		artistAlbums = await getArtistAlbums(artist.uid.toString(), akaUids);
	}

	// Reload when selection changes
	$effect(() => {
		selection.uid;
		loadArtist();
	});

	// Banner art effect
	$effect(() => {
		const uid = selection.uid;
		if (!uid) return;

		untrack(() => {
			if (bannerUrl) { URL.revokeObjectURL(bannerUrl); bannerUrl = null; }
		});

		if (!artist || artist.banner_art_path) return;

		invoke<number[] | null>("get_artist_banner_art", { uid }).then((bytes) => {
			if (bytes && bytes.length > 0) {
				untrack(() => { if (bannerUrl) URL.revokeObjectURL(bannerUrl); });
				const blob = new Blob([new Uint8Array(bytes)], { type: "image/jpeg" });
				bannerUrl = URL.createObjectURL(blob);
			}
		});
	});

	// Event bus subscriptions
	$effect(() => {
		const unsubArtists = onLibraryChange("artists:changed", loadArtist);
		const unsubAlbums  = onLibraryChange("albums:changed",  loadArtistAlbums);
		const unsubSingle  = onSingleChange((uid) => {
			if (artist && uid === artist.uid) loadArtist();
		});
		return () => { unsubArtists(); unsubAlbums(); unsubSingle(); };
	});
</script>

<div class="flex flex-col gap-4 border-2 h-full w-full overflow-hidden rounded-md">

	{#if artist}
		<div class="relative">
			<NavButtons class="absolute top-4 left-4"/>

			{#if artist.banner_art_path || bannerUrl}
				<img src={artist.banner_art_path ?? bannerUrl} alt="" class="w-full h-32 object-cover rounded-t-md" />
			{:else}
				<div class="w-full h-32 bg-muted rounded-t-md"></div>
			{/if}

			<div class="absolute bottom-0 translate-y-1/2 left-4">
				<ArtworkDisplay entity={artist} size={80} />
			</div>
		</div>

		<div class="flex flex-col gap-2 px-4 pt-10">
			<div class="flex items-center gap-3">
				<h2 class="text-2xl font-bold">{artist.name}</h2>
				<div class="flex gap-2">
					<Button variant="default">Play All</Button>
					<Button variant="outline">Shuffle</Button>
					<Button variant="ghost" onclick={() => openEditModal({ type: "artist", uid: artist!.uid })}><Pencil/></Button>
				</div>
			</div>
		</div>

		<Tabs.Root bind:value={currentArtistTab.id} class="flex flex-col min-h-0 flex-1 px-4">
			<Tabs.List>
				<Tabs.Trigger value="home">Home</Tabs.Trigger>
				<Tabs.Trigger value="discography">Discography</Tabs.Trigger>
				<Tabs.Trigger value="about">About</Tabs.Trigger>
			</Tabs.List>

			<Tabs.Content value="home" class="flex-1 overflow-y-auto">
				<h3 class="text-sm font-semibold mb-2 mt-2">TAGS & GENRES</h3>
				<TagList uid={artist.uid} tags={genres!} canEdit={false} />
				<TagList uid={artist.uid} tags={tags!} canEdit={false} />

				<h3 class="text-sm font-semibold mb-2 mt-2 pt-4">TOP SONGS</h3>
				<ArtistTopTracks artistUid={artist.uid} artistName={artist.name} />
			</Tabs.Content>

			<Tabs.Content value="discography" class="flex-1 overflow-y-auto">
				<div class="flex flex-col gap-2 h-full w-full">
					<ScrollArea class="min-h-0 min-w-0">
						<div class="grid gap-2 mt-2 pr-4" style="grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));">
							{#each artistAlbums as album}
								<AudioCard title={album.title} subTitle={album.album_artist ?? "Unknown Artist"} entity={album} type="album" />
							{/each}
						</div>
					</ScrollArea>
				</div>
			</Tabs.Content>

			<Tabs.Content value="about" class="flex-1 overflow-y-auto">
			{#if artist.aka}
				<h3 class="text-sm font-semibold mb-2 mt-2">AKA</h3>
				<div class="flex flex-row gap-1 flex-wrap">
					{#each akaList as aka, i}
						<p class="text-sm leading-relaxed">{aka}{i < akaList.length - 1 ? "," : ""}</p>
					{/each}
				</div>
			{/if}
				
				<h3 class="text-sm font-semibold mb-2 mt-2 pt-4">BIO</h3>
				{#if artist.about}
					<p class="text-sm leading-relaxed mt-2">{artist.about}</p>
				{:else}
					<p class="text-muted-foreground text-sm mt-2">No biography available.</p>
				{/if}
			</Tabs.Content>
		</Tabs.Root>
	{:else}
		<NavButtons />
		<span class="text-muted-foreground text-sm p-4">Loading...</span>
	{/if}

</div>