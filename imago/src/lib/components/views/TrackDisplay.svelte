<script lang="ts">
	import { revealItemInDir } from "@tauri-apps/plugin-opener";

	import { Pencil } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs/index.js";
	import ScrollArea from "$lib/components/custom/ScrollView.svelte";
	import { Button } from "$shadcn/button/index.js";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import NavButtons from "$lib/components/custom/NavButtons.svelte";
	import TrackPlaylistEditButton from "$lib/components/custom/TrackPlaylistEditButton.svelte";
	import TrackRating from "$lib/components/custom/text-display/TrackRating.svelte";
	import TagList from "$lib/components/custom/tags/TagList.svelte";
	import ArtistsList from "$lib/components/custom/text-display/ArtistsList.svelte";
	import LyricsViewer from "$lib/components/app/now-playing/LyricsViewer.svelte";
	import DownloadButton from "$lib/components/custom/DownloadButton.svelte";

	import { getTrack, getAlbum, onLibraryChange, onSingleChange } from "$ts/store/library.svelte";
	import { currentTrackTab, selection, setSelection } from "$ts/store/session.svelte";
	import { openEditModal } from "$ts/ui/editModal.svelte";
	import { formatDuration } from "$ts/util/helpers";
	import { playTrackByObject } from "$ts/audio/audioManager.svelte";
	import { fetchArtworkColor } from "$ts/library/artworkLoader";
	import { parseTags, parseArtists, parseAlbumEntries } from "$ts/util/parsers";
	import { invoke } from "@tauri-apps/api/core";

	import type { Track, Album } from "$ts/util/types";

	let track = $state<Track | null>(null);
	let featuredOnAlbums = $state<Album[]>([]);
	let color = $state("rgb(30, 30, 30)");

	let genres = $derived(track?.genres ? parseTags(track.genres) : null);
	let tags = $derived(track?.tags ? parseTags(track.tags) : null);

	let featuredArtistNames = $derived.by(() => {
		if (!track?.artists) return [];
		const main = (track.album_artist ?? "").toLowerCase();
		return parseArtists(track.artists).filter(n => n.toLowerCase() !== main);
	});

	let featuredArtistEntries = $state<{ name: string; uid: string | null }[]>([]);
	let albumArtistUid = $state<string | null>(null);

	$effect(() => {
		const names = featuredArtistNames;
		Promise.all(
			names.map(async (name) => ({
				name,
				uid: await invoke<string | null>("get_artist_uid_by_name", { name }),
			}))
		).then((entries) => { featuredArtistEntries = entries; });
	});

	$effect(() => {
		const name = track?.album_artist ?? null;
		if (!name) { albumArtistUid = null; return; }
		invoke<string | null>("get_artist_uid_by_name", { name }).then((uid) => { albumArtistUid = uid; });
	});

	let isNotGhost = $derived(() => {
		const hasLocal = track?.path && track.path !== "" && track.path !== track.uid;
		const hasRemote = (track as any)?.remote_path && (track as any).remote_path.length > 0;
		return !!(hasLocal || hasRemote);
	});

	async function loadTrack() {
		const uid = selection.uid;
		if (!uid) { track = null; featuredOnAlbums = []; return; }
		track = await getTrack(uid);
		if (!track) { featuredOnAlbums = []; return; }
		await loadFeaturedAlbums();
	}

	async function loadFeaturedAlbums() {
		if (!track?.albums) { featuredOnAlbums = []; return; }
		const results = await Promise.all(parseAlbumEntries(track.albums).map(e => getAlbum(e.uid)));
		featuredOnAlbums = results.filter((a): a is Album => a !== null);
	}

	// Reload when selection changes
	$effect(() => {
		selection.uid;
		loadTrack();
	});

	// Color effect
	$effect(() => {
		const uid = selection.uid;
		if (!uid) return;
		color = "var(--muted)";
		fetchArtworkColor(uid, "track").then(c => { color = c; });
	});

	// Event bus subscriptions
	$effect(() => {
		const unsubTracks = onLibraryChange("tracks:changed", loadTrack);
		const unsubAlbums = onLibraryChange("albums:changed", loadFeaturedAlbums);
		const unsubSingle = onSingleChange((uid) => {
			if (track && uid === track.uid) loadTrack();
		});
		return () => { unsubTracks(); unsubAlbums(); unsubSingle(); };
	});
</script>

<div class="flex flex-col gap-4 p-4 border-2 h-full w-full overflow-hidden rounded-md">
		<NavButtons />
	
		{#if track}
			<div class="flex gap-4 items-center">
				<ArtworkDisplay entity={track} size={160} />

				<div class="flex flex-col gap-1">
					<h2 class="text-2xl font-bold">{track.title ?? "Unknown Title"}</h2>
					<div class="flex gap-2 text-sm text-muted-foreground flex-wrap">
						<ArtistsList artists={track.artists!} />
						<span>|</span>
						<div>
							{#if track.albums}
							{@const albumList = parseAlbumEntries(track.albums)}
							{#each albumList as album, i}
								<button onclick={() => setSelection(album.uid)} class="text-sm truncate cursor-pointer hover:underline">
									{album.name}{i < albumList.length - 1 ? "," : ""}
								</button>
							{/each}
							{/if}
						</div>
						{#if track.year}
							<span>|</span>
							<span>{track.year}</span>
						{/if}
						<span>|</span>
						<span>{formatDuration(track.duration_ms)}</span>
						<span>|</span>
						<span>{(track as any).format ?? "Unknown Format"}</span>
						<span>|</span>
						<span>{(track as any).bitrate ?? "0"}</span><span>kbps</span>
					</div>
					<div>
						<TrackRating uid={track.uid} rating={track.rating} tags={track.tags} />
					</div>
				</div>
			</div>

			<div class="flex gap-2 justify-between items-center flex-wrap p-2 rounded-md"
				  style="background: {color};">
				<Button variant="default" disabled={!isNotGhost()} onclick={() => playTrackByObject(track)}>Play</Button>
				<div>
					<TrackPlaylistEditButton track={track} />
					<Button variant="ghost" size="icon" onclick={() => openEditModal({ type: "track", uid: track!.uid })}><Pencil /></Button>
					<DownloadButton uid={track.uid} variant="ghost" />
				</div>
			</div>

			<Tabs.Root bind:value={currentTrackTab.id} class="flex flex-col min-h-0 flex-1">
				<Tabs.List>
					<Tabs.Trigger value="lyrics">Lyrics</Tabs.Trigger>
					<Tabs.Trigger value="tags">Tags</Tabs.Trigger>
					<Tabs.Trigger value="credits">Credits</Tabs.Trigger>
					<Tabs.Trigger value="explore">Featured On</Tabs.Trigger>
					<Tabs.Trigger value="linked">Other Versions</Tabs.Trigger>
					<Tabs.Trigger value="paths">File Paths</Tabs.Trigger>
				</Tabs.List>

				<Tabs.Content value="lyrics" class="flex-1 overflow-y-auto mt-2">
					<ScrollArea class="min-h-0 min-w-0 h-full pl-4">
						<LyricsViewer uid={track.uid} />
					</ScrollArea>
				</Tabs.Content>

				<Tabs.Content value="tags" class="flex-1 overflow-y-auto mt-2">
					{#if tags}
						<TagList uid={track.uid} tags={genres!} canEdit={false} />
						<TagList uid={track.uid} tags={tags} canEdit={true} />
					{:else}
						<p class="text-muted-foreground text-sm">Track has no tags.</p>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="credits" class="flex-1 overflow-y-auto mt-2">
					<h4 class="text-foreground text-lg">Main Artist</h4>

					{#if track.album_artist}
						<button
							onclick={() => { if (albumArtistUid) setSelection(albumArtistUid); }}
							class="flex items-center gap-2 flex-row cursor-pointer p-2 rounded-md bg-muted/50 hover:bg-muted">
							<ArtworkDisplay entity={{ uid: albumArtistUid ?? "" }} size={40} />
							{track.album_artist}
						</button>
					{:else}
						<p class="text-muted-foreground text-sm">No album artist available.</p>
					{/if}

					<h4 class="text-foreground text-lg mt-4">Featured Artists</h4>
					{#if featuredArtistEntries.length > 0}
						{#each featuredArtistEntries as { name: artistName, uid: featArtistUID }}
							<button
								onclick={() => { if (featArtistUID) setSelection(featArtistUID); }}
								class="flex items-center gap-2 flex-row cursor-pointer p-2 mb-2 rounded-md bg-muted/50 hover:bg-muted">
								<ArtworkDisplay entity={{ uid: featArtistUID ?? "" }} size={40} />
								{artistName}
							</button>
						{/each}
					{:else}
						<p class="text-muted-foreground text-sm">No featured artists.</p>
					{/if}

					<h4 class="text-foreground text-lg mt-4">Credits</h4>
					{#if track.credits}
						<pre class="text-sm whitespace-pre-wrap font-sans">{track.credits}</pre>
					{:else}
						<p class="text-muted-foreground text-sm">No credits available.</p>
					{/if}

					{#if track.label}
						<h4 class="text-foreground text-lg mt-4">Label</h4>
						<pre class="text-sm whitespace-pre-wrap font-sans">{track.label}</pre>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="explore" class="flex-1 overflow-y-auto mt-2">
					{#if featuredOnAlbums.length > 0}
						<div class="flex flex-col gap-2 mt-2">
							{#each featuredOnAlbums as album}
								<button
									onclick={() => setSelection(album.uid)}
									class="flex items-center gap-3 cursor-pointer p-2 rounded-md bg-muted/50 hover:bg-muted text-left">
									<ArtworkDisplay entity={album} size={48} />
									<div class="flex flex-col">
										<span class="text-sm font-medium">{album.title}</span>
										<span class="text-xs text-muted-foreground">{album.album_artist}</span>
									</div>
								</button>
							{/each}
						</div>
					{:else}
						<p class="text-muted-foreground text-sm">No albums found.</p>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="linked" class="flex-1 overflow-y-auto mt-2">
					{#if featuredOnAlbums.length > 0}
						<div class="flex flex-col gap-2 mt-2">
							{#each featuredOnAlbums as album}
								<button
									onclick={() => setSelection(album.uid)}
									class="flex items-center gap-3 cursor-pointer p-2 rounded-md bg-muted/50 hover:bg-muted text-left">
									<ArtworkDisplay entity={album} size={48} />
									<div class="flex flex-col">
										<span class="text-sm font-medium">{album.title}</span>
										<span class="text-xs text-muted-foreground">{album.album_artist}</span>
									</div>
								</button>
							{/each}
						</div>
					{:else}
						<p class="text-muted-foreground text-sm">No albums found.</p>
					{/if}
				</Tabs.Content>

				<Tabs.Content value="paths" class="flex-1 overflow-y-auto mt-2">
					<div class="flex flex-col gap-2 mt-2">
						<span class="text-muted-foreground text-sm">
							<b>Local Path:</b>
							{#if track.path && track.path !== track.uid}
								<button
									onclick={() => revealItemInDir(track!.path)}
									class="cursor-pointer text-left hover:underline">
									{track.path}
								</button>
							{/if}
						</span>
						<span class="text-muted-foreground text-sm"><b>Remote Path:</b> {(track as any).remote_path}</span>
					</div>
				</Tabs.Content>
			</Tabs.Root>
		{:else}
			<span class="text-muted-foreground text-sm">Loading...</span>
		{/if}
</div>