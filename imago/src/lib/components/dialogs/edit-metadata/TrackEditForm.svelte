<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	import { X, Plus, ChevronUp, ChevronDown, Star, Disc, Hash } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs";
	import { Label } from "$shadcn/label";
	import { Input } from "$shadcn/input";
	import { Textarea } from "$shadcn/textarea";
	import { Button } from "$shadcn/button";
	import { Separator } from "$shadcn/separator";
	import { Toggle } from "$shadcn/toggle/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";

	import ArtworkEditor from "./ArtworkEditor.svelte";
	import IconInput from "$lib/components/custom/IconInput.svelte";

	import { closeEditModal } from "$ts/ui/editModal.svelte";
	import { getTrack, reloadLibrary } from "$ts/store/library.svelte";
	import {
		syncAlbums,
		removeTrackFromOldAlbums,
		syncArtists,
		pruneArtists,
		renameArtistInLibrary,
		renameAlbumInTracks,
	} from "$ts/library/entitySync";
	import { warnEmptyFields } from "$ts/ui/dialogManager.svelte";
	import { enrichTrack, fetchLyrics } from "$ts/library/enrichment";
	import TagSelector from "$lib/components/custom/tags/TagSelector.svelte";

	import type { TrackAlbumEntry } from "$ts/util/types";
	import { tagStore } from "$ts/store/tagManager.svelte";
	import { parseArtists, parseAlbumEntries } from "$ts/util/parsers";

	let { uid } = $props<{ uid: string }>();

	let title = $state("");
	let artists = $state("");
	let albumArtist = $state("");
	let albums = $state<TrackAlbumEntry[]>([]);
	let originalAlbumUids = $state<string[]>([]);
	let originalArtists = $state<string[]>([]);
	let originalAlbumArtist = $state("");
	let year = $state("");
	let genreList = $state<string[]>([]);
	let tagList = $state<string[]>([]);
	let bpm = $state<string>("");
	let key = $state("");
	let rating = $state<string>("");
	let label = $state("");
	let credits = $state("");
	let path = $state("");
	let artworkPath = $state<string | null>(null);
	let saving = $state(false);
	let writeToFile = $state(false);
	let hasLyrics = $state(false);

	let isFavoritePressed = $derived(tagList.includes("favorite"));

	onMount(async () => {
		const lyrics = await invoke("get_track_lyrics", { uid });
		const track = await getTrack(uid);
		if (!track) return;

		hasLyrics = lyrics != null;
		title = track.title ?? "";
		albumArtist = track.album_artist ?? "";
		originalAlbumArtist = albumArtist;
		year = track.year ?? "";
		bpm = track.bpm != null ? String(track.bpm) : "";
		key = track.key ?? "";
		rating = track.rating != null ? String(track.rating) : "";
		label = track.label ?? "";
		credits = track.credits ?? "";
		artworkPath = track.artwork_path ?? null;
		path = track.path ?? "";

		const artistArr = parseArtists(track.artists);
		artists = artistArr.join(", ");
		originalArtists = artistArr;

		albums = parseAlbumEntries(track.albums).map(a => ({
			uid: a.uid,
			name: a.name,
			track_number: a.track_number ?? null,
			disc: a.disc ?? null,
		}));
		originalAlbumUids = albums.map(a => a.uid).filter(Boolean);

		try { genreList = track.genres ? JSON.parse(track.genres) : []; } catch { genreList = []; }
		try { tagList = track.tags ? JSON.parse(track.tags) : []; } catch { tagList = []; }
	});

	function addAlbum() {
		albums = [...albums, { uid: "", name: "", track_number: null, disc: null }];
	}

	function removeAlbum(index: number) {
		albums = albums.filter((_, i) => i !== index);
	}

	function moveUp(index: number) {
		if (index === 0) return;
		const next = [...albums];
		[next[index - 1], next[index]] = [next[index], next[index - 1]];
		albums = next;
	}

	function moveDown(index: number) {
		if (index === albums.length - 1) return;
		const next = [...albums];
		[next[index], next[index + 1]] = [next[index + 1], next[index]];
		albums = next;
	}

	function getLyrics() {
		fetchLyrics(uid);
		reloadLibrary("tracks");
	}

	function onFavoritePressed() {
		if (tagList.includes("favorite")) {
			tagList = tagList.filter((t) => t !== "favorite");
		} else {
			tagList = [...tagList, "favorite"];
		}
	}

	async function save() {
		const hasEmpty = !title || !artists || !albumArtist;
		const proceed = await warnEmptyFields(hasEmpty);
		if (!proceed) return;

		saving = true;
		try {
			const artistArr = artists.split(",").map((s) => s.trim()).filter(Boolean);

			const cleanedAlbums = albums
				.filter((a) => a.name.trim())
				.map((a) => ({
					uid: a.uid ?? "",
					name: a.name.trim(),
					track_number: a.track_number != null && !isNaN(Number(a.track_number)) ? Number(a.track_number) : null,
					disc: a.disc != null && !isNaN(Number(a.disc)) ? Number(a.disc) : null,
				}));

			const renamedArtists = originalArtists.filter((orig) => {
				const newArr = artistArr.map((x) => x.toLowerCase());
				return !newArr.includes(orig.toLowerCase());
			});
			for (const oldName of renamedArtists) {
				const matchingNew = artistArr.find(
					(n) => !originalArtists.map((x) => x.toLowerCase()).includes(n.toLowerCase())
				);
				if (matchingNew) {
					await renameArtistInLibrary(oldName, matchingNew);
				}
			}

			if (originalAlbumArtist && albumArtist && originalAlbumArtist.toLowerCase() !== albumArtist.toLowerCase()) {
				await renameArtistInLibrary(originalAlbumArtist, albumArtist);
			}

			const finalAlbumEntries = await syncAlbums(cleanedAlbums, uid, title, albumArtist);

			const currentAlbumUids = new Set(finalAlbumEntries.map((e) => e.uid).filter(Boolean));
			const removedAlbumUids = originalAlbumUids.filter((u) => !currentAlbumUids.has(u));
			await removeTrackFromOldAlbums(uid, removedAlbumUids);

			await syncArtists([...artistArr, ...(albumArtist ? [albumArtist] : [])]);

			const removedArtists = [
				...originalArtists.filter((n) => !artistArr.map((x) => x.toLowerCase()).includes(n.toLowerCase())),
				...(originalAlbumArtist && originalAlbumArtist !== albumArtist ? [originalAlbumArtist] : []),
			];
			await pruneArtists(removedArtists);

			const update: Record<string, any> = {
				title: title || null,
				artists: artistArr.length ? JSON.stringify(artistArr) : null,
				album_artist: albumArtist || null,
				albums: finalAlbumEntries.length ? JSON.stringify(finalAlbumEntries) : null,
				year: year || null,
				genres: genreList.length > 0 ? JSON.stringify(genreList) : null,
				bpm: bpm ? Number(bpm) : null,
				key: key || null,
				rating: rating ? Number(rating) : null,
				label: label || null,
				credits: credits || null,
				tags: tagList.length > 0 ? JSON.stringify(tagList) : null,
				artwork_path: artworkPath,
			};

			if (writeToFile) {
				await invoke("write_track_tags", { uid, path, update });
			} else {
				await invoke("update_track_metadata", { uid, update });
			}

			await reloadLibrary("tracks");
			await reloadLibrary("albums");
			await reloadLibrary("artists");
			tagStore.load();
		} finally {
			saving = false;
			closeEditModal();
		}
	}
</script>

<Tabs.Root value="details">
	<Tabs.List class="w-full">
		<Tabs.Trigger value="details" class="flex-1">Details</Tabs.Trigger>
		<Tabs.Trigger value="credits" class="flex-1">Credits</Tabs.Trigger>
		<Tabs.Trigger value="artwork" class="flex-1">Artwork</Tabs.Trigger>
	</Tabs.List>

	<Tabs.Content value="details" class="space-y-3 mt-4">
		<ScrollArea class="min-h-0 min-w-0 h-[380px]">
			<div class="flex flex-col gap-4 pb-4 pr-4 pl-1 mb-20">
				<div class="space-y-1.5">
					<Label for="track-title">Title</Label>
					<Input id="track-title" bind:value={title} />
				</div>
	
				<div class="grid grid-cols-2 gap-3">
					<div class="space-y-1.5">
						<Label for="track-year">Year</Label>
						<Input id="track-year" bind:value={year} />
					</div>
					<div class="space-y-1.5">
						<Label for="track-rating">Rating <span class="text-muted-foreground text-xs">(0–10)</span></Label>
						<div class="flex gap-1 items-end">
							<Input
								id="album-rating"
								type="number"
								min="0"
								max="10"
								step="0.1"
								bind:value={rating}
								class="text-sm m-0"
								placeholder="1-10"
							/>
							<Toggle
								pressed={isFavoritePressed}
								onPressedChange={onFavoritePressed}
								size="sm"
								class="data-[state=on]:bg-transparent data-[state=on]:*:[svg]:fill-yellow-500 data-[state=on]:*:[svg]:stroke-yellow-500"
							>
								<Star />
							</Toggle>
						</div>
					</div>
				</div>
				<div class="grid grid-cols-2 gap-3">
					<div class="space-y-1.5">
						<Label for="track-bpm">BPM</Label>
						<Input id="track-bpm" type="number" min="40" max="300" bind:value={bpm} />
					</div>
					<div class="space-y-1.5">
						<Label for="track-key">Key</Label>
						<Input id="track-key" placeholder="e.g. Am" bind:value={key} />
					</div>
				</div>
	
				<div class="space-y-1.5">
					<div class="flex items-center justify-between">
						<Label>Featured On:</Label>
						<Button variant="ghost" size="sm" onclick={addAlbum} class="h-7 px-2 text-xs gap-1">
							<Plus class="size-3" />
							Add Album
						</Button>
					</div>
					<div class="space-y-2 border-2 border-muted-foreground/20 rounded-md p-2">
						{#each albums as album, i}
							<div class="flex gap-1 items-center">
								<div class="flex flex-col">
									<Button variant="ghost" size="icon" onclick={() => moveUp(i)} disabled={i === 0} class="size-6 text-muted-foreground">
										<ChevronUp class="size-3" />
									</Button>
									<Button variant="ghost" size="icon" onclick={() => moveDown(i)} disabled={i === albums.length - 1} class="size-6 text-muted-foreground">
										<ChevronDown class="size-3" />
									</Button>
								</div>
								<Input placeholder="Album name" bind:value={album.name} class="flex-1" />
								<IconInput bind:value={album.track_number} placeholder="Track #" type="number" Icon={Hash} classes="w-20" />
								<IconInput bind:value={album.disc} placeholder="Disc #" type="number" Icon={Disc} classes="w-18" />
								<Button variant="ghost" size="icon" onclick={() => removeAlbum(i)} class="size-8 shrink-0 text-muted-foreground hover:text-destructive">
									<X class="size-4" />
								</Button>
							</div>
						{/each}
						{#if albums.length === 0}
							<p class="text-xs text-muted-foreground">No albums — click Add to link one.</p>
						{/if}
					</div>
				</div>
	
				<div class="space-y-1.5">
					<Label for="track-genres">Genres <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
					<TagSelector bind:value={genreList} isGenre={true} placeholder="Add genre…" />
				</div>
				<div class="space-y-1.5">
					<Label for="track-tags">Tags <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
					<TagSelector bind:value={tagList} placeholder="Add tag…" />
				</div>
			</div>
		</ScrollArea>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="credits" class="space-y-3 mt-4">
		<ScrollArea class="min-h-0 min-w-0 h-[380px]">
			<div class="flex flex-col gap-4 pb-4 pr-4 pl-1 mb-6">
				<div class="space-y-1.5">
					<Label for="track-artists">Artists <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
					<Input id="track-artists" bind:value={artists} />
				</div>
				<div class="space-y-1.5">
					<Label for="track-album-artist">Album Artist</Label>
					<Input id="track-album-artist" bind:value={albumArtist} />
				</div>
				<div class="space-y-1.5">
					<Label for="track-label">Label</Label>
					<Input id="track-label" bind:value={label} />
				</div>
				<div class="space-y-1.5">
					<Label for="track-credits">Credits</Label>
					<Textarea id="track-credits" bind:value={credits} rows={3} />
				</div>
			</div>
		</ScrollArea>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="artwork" class="mt-4">
		<ArtworkEditor
			entityType="track"
			entityUid={uid}
			onchange={(p) => { artworkPath = p; }}
		/>
		<Separator class="my-4" />
	</Tabs.Content>
</Tabs.Root>

<div class="flex items-center justify-between">
	<label class="flex items-center gap-2 text-sm cursor-pointer select-none">
		<input type="checkbox" bind:checked={writeToFile} class="rounded" />
		Write tags to file
	</label>
	<div class="flex gap-2">
		<Button variant="outline" onclick={closeEditModal}>Cancel</Button>
		<Button onclick={save} disabled={saving}>
			{saving ? "Saving…" : "Save"}
		</Button>
	</div>
</div>