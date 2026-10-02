<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	import { Star } from "lucide-svelte";

	import * as Tabs from "$shadcn/tabs";
	import * as Select from "$shadcn/select/index.js";
	import Toggle from "$shadcn/toggle/toggle.svelte";
	import { Label } from "$shadcn/label";
	import { Input } from "$shadcn/input";
	import { Textarea } from "$shadcn/textarea";
	import { Button } from "$shadcn/button";
	import { Separator } from "$shadcn/separator";

	import ArtworkEditor from "$lib/components/dialogs/edit-metadata/ArtworkEditor.svelte";
	import TagSelector from "$lib/components/custom/tags/TagSelector.svelte";

	import { closeEditModal } from "$ts/ui/editModal.svelte";
	import { getAlbum, reloadLibrary } from "$ts/store/library.svelte";
	import { syncArtists, pruneArtists, renameArtistInLibrary, renameAlbumInTracks } from "$ts/library/entitySync";
	import { warnEmptyFields } from "$ts/ui/dialogManager.svelte";
	import { tagStore } from "$ts/store/tagManager.svelte";
	import { parseArtists } from "$ts/util/parsers";

	let { uid } = $props<{ uid: string }>();

	let title = $state("");
	let originalTitle = $state("");
	let albumArtist = $state("");
	let artists = $state("");
	let releaseDate = $state("");
	let format = $state("");
	let type = $state("None");
	let genreList = $state<string[]>([]);
	let label = $state("");
	let rating = $state<string>("");
	let credits = $state("");
	let tagList = $state<string[]>([]);
	let artworkPath = $state<string | null>(null);
	let saving = $state(false);

	let isFavoritePressed = $derived(tagList.includes("favorite"));

	let originalArtists = $state<string[]>([]);
	let originalAlbumArtist = $state("");

	const albumTypes = [
		{ value: "LP", label: "LP" },
		{ value: "EP", label: "EP" },
		{ value: "Single", label: "Single" },
		{ value: "Reissue", label: "Reissue" },
		{ value: "Soundtrack", label: "Soundtrack" }
	];
	const emulateTypes = [
		{ value: "None", label: "None (Digital)" },
		{ value: "Vinyl", label: "Vinyl" },
		{ value: "Cassette", label: "Cassette" },
		{ value: "Disc", label: "Disc" }
	];

	onMount(async () => {
		const album = await getAlbum(uid);
		if (!album) return;

		title = album.title ?? "";
		originalTitle = title;
		albumArtist = album.album_artist ?? "";
		originalAlbumArtist = albumArtist;
		releaseDate = album.release_date ?? "";
		format = album.format ?? "";
		label = album.label ?? "";
		credits = album.credits ?? "";
		rating = album.rating != null ? String(album.rating) : "";
		artworkPath = album.artwork_path ?? null;
		type = album.emulate_type ?? "None";

		const artistArr = parseArtists(album.artists);
		artists = artistArr.join(", ");
		originalArtists = artistArr;

		try { genreList = album.genres ? JSON.parse(album.genres) : []; } catch { genreList = []; }
		try { tagList = album.tags ? JSON.parse(album.tags) : []; } catch { tagList = []; }
	});

	async function save() {
		const hasEmpty = !title || !albumArtist;
		const proceed = await warnEmptyFields(hasEmpty);
		if (!proceed) return;

		saving = true;
		try {
			const artistArr = artists.split(",").map((s) => s.trim()).filter(Boolean);
			const allNewNames = [...artistArr, ...(albumArtist ? [albumArtist] : [])];

			if (originalTitle && title && originalTitle.toLowerCase() !== title.toLowerCase()) {
				await renameAlbumInTracks(originalTitle, title, albumArtist);
			}

			if (originalAlbumArtist && albumArtist && originalAlbumArtist.toLowerCase() !== albumArtist.toLowerCase()) {
				await renameArtistInLibrary(originalAlbumArtist, albumArtist);
			}
			const renamedArtists = originalArtists.filter((orig) => {
				return !artistArr.map((x) => x.toLowerCase()).includes(orig.toLowerCase());
			});
			for (const oldName of renamedArtists) {
				const matchingNew = artistArr.find(
					(n) => !originalArtists.map((x) => x.toLowerCase()).includes(n.toLowerCase())
				);
				if (matchingNew) await renameArtistInLibrary(oldName, matchingNew);
			}

			await syncArtists(allNewNames);

			const removedArtists = [
				...originalArtists.filter((n) => !artistArr.map((x) => x.toLowerCase()).includes(n.toLowerCase())),
				...(originalAlbumArtist && originalAlbumArtist !== albumArtist ? [originalAlbumArtist] : []),
			];
			await pruneArtists(removedArtists);

			const update: Record<string, any> = {
				title: title || null,
				album_artist: albumArtist || null,
				artists: artistArr.length ? JSON.stringify(artistArr) : null,
				release_date: releaseDate || null,
				format: format || null,
				genres: genreList.length ? JSON.stringify(genreList) : null,
				label: label || null,
				rating: rating ? Number(rating) : null,
				credits: credits || null,
				tags: tagList.length ? JSON.stringify(tagList) : null,
				artwork_path: artworkPath,
				emulate_type: type,
			};

			await invoke("update_album_entry", { uid, update });
			await reloadLibrary("albums");
			await reloadLibrary("artists");
			await reloadLibrary("tracks");
			tagStore.load();
		} finally {
			saving = false;
			closeEditModal();
		}
	}

	function onFavoritePressed() {
		if (tagList.includes("favorite")) {
			tagList = tagList.filter((t) => t !== "favorite");
		} else {
			tagList = [...tagList, "favorite"];
		}
	}
</script>

<Tabs.Root value="details">
	<Tabs.List class="w-full">
		<Tabs.Trigger value="details" class="flex-1">Details</Tabs.Trigger>
		<Tabs.Trigger value="credits" class="flex-1">Credits</Tabs.Trigger>
		<Tabs.Trigger value="artwork" class="flex-1">Artwork</Tabs.Trigger>
	</Tabs.List>

	<Tabs.Content value="details">
		<div class="flex-1 min-h-0 pr-1 space-y-3 mt-4">
			<div class="space-y-1.5">
				<Label for="album-title">Title</Label>
				<Input id="album-title" bind:value={title} />
			</div>

			<div class="grid grid-cols-2 gap-3 pt-2">
				<div class="space-y-1.5">
					<Label for="album-release">Release Date</Label>
					<Input id="album-release" bind:value={releaseDate} />
				</div>
				<div class="space-y-1.5">
					<Label for="album-rating">Rating</Label>
					<div class="flex gap-1 items-end">
						<Input id="album-rating" type="number" min="0" max="10" step="0.1" bind:value={rating} class="text-sm m-0" placeholder="1-10" />
						<Toggle pressed={isFavoritePressed} onPressedChange={onFavoritePressed} size="sm" class="data-[state=on]:bg-transparent data-[state=on]:*:[svg]:fill-yellow-500 data-[state=on]:*:[svg]:stroke-yellow-500">
							<Star />
						</Toggle>
					</div>
				</div>
			</div>
			<div class="grid grid-cols-2 gap-3 pt-2">
				<div class="space-y-1.5">
					<Label for="album-format">Format</Label>
					<Select.Root type="single" name="album-format" bind:value={format}>
						<Select.Trigger class="text-sm w-full">{format || "Select..."}</Select.Trigger>
						<Select.Content>
							<Select.Group>
								<Select.Label>Format</Select.Label>
								{#each albumTypes as t (t.value)}
									<Select.Item value={t.value} label={t.label}>{t.label}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</div>
				<div class="space-y-1.5">
					<Label for="album-type">Emulate Type</Label>
					<Select.Root type="single" name="album-type" bind:value={type}>
						<Select.Trigger class="text-sm w-full">{type || "Select..."}</Select.Trigger>
						<Select.Content>
							<Select.Group>
								<Select.Label>Emulate Type</Select.Label>
								{#each emulateTypes as t (t.value)}
									<Select.Item value={t.value} label={t.label}>{t.label}</Select.Item>
								{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</div>
			</div>

			<div class="space-y-1.5 pt-2">
				<Label>Genres</Label>
				<TagSelector bind:value={genreList} isGenre={true} placeholder="Add genre…" />
			</div>
			<div class="space-y-1.5 pt-2">
				<Label>Tags</Label>
				<TagSelector bind:value={tagList} placeholder="Add tag…" />
			</div>
		</div>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="credits" class="space-y-3 mt-4">
		<div class="space-y-1.5">
			<Label for="album-artist">Album Artist</Label>
			<Input id="album-artist" bind:value={albumArtist} />
		</div>
		<div class="space-y-1.5 pt-2">
			<Label for="album-artists">Artists <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
			<Input id="album-artists" bind:value={artists} />
		</div>
		<div class="space-y-1.5 pt-2">
			<Label for="album-label">Label</Label>
			<Input id="album-label" bind:value={label} />
		</div>
		<div class="space-y-1.5 pt-2">
			<Label for="album-credits">Credits</Label>
			<Textarea id="album-credits" bind:value={credits} rows={4} />
		</div>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="artwork" class="mt-4">
		<ArtworkEditor entityType="album" entityUid={uid} onchange={(path) => { artworkPath = path; }} />
		<Separator class="my-4" />
	</Tabs.Content>
</Tabs.Root>

<div class="flex justify-end gap-2">
	<Button variant="outline" onclick={closeEditModal}>Cancel</Button>
	<Button onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button>
</div>