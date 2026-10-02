<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	import * as Tabs from "$shadcn/tabs";
	import { Label } from "$shadcn/label";
	import { Input } from "$shadcn/input";
	import { Textarea } from "$shadcn/textarea";
	import { Button } from "$shadcn/button";
	import { Separator } from "$shadcn/separator";

	import ArtworkEditor from "$lib/components/dialogs/edit-metadata/ArtworkEditor.svelte";
	import TagSelector from "$lib/components/custom/tags/TagSelector.svelte";

	import { closeEditModal } from "$ts/ui/editModal.svelte";
	import { getArtist, reloadSingle, reloadLibrary } from "$ts/store/library.svelte";
	import { renameArtistInLibrary, mergeArtistAkas } from "$ts/library/entitySync";
	import { warnEmptyFields } from "$ts/ui/dialogManager.svelte";
	import { tagStore } from "$ts/store/tagManager.svelte";

	let { uid } = $props<{ uid: string }>();

	let name = $state("");
	let originalName = $state("");
	let aka = $state("");
	let about = $state("");
	let genreList = $state<string[]>([]);
	let tagList = $state<string[]>([]);
	let websites = $state("");
	let members = $state("");
	let profileArtPath = $state<string | null>(null);
	let saving = $state(false);

	function splitList(val: string) {
		return val.split(",").map((s) => s.trim()).filter(Boolean);
	}

	onMount(async () => {
		const artist = await getArtist(uid);
		if (!artist) return;

		name = artist.name ?? "";
		originalName = name;
		about = artist.about ?? "";
		profileArtPath = artist.profile_art_path ?? null;

		try { aka = (artist.aka ? JSON.parse(artist.aka) : []).join(", "); } catch { aka = ""; }
		try { genreList = artist.genres ? JSON.parse(artist.genres) : []; } catch { genreList = []; }
		try { tagList = artist.tags ? JSON.parse(artist.tags) : []; } catch { tagList = []; }
		try { websites = (artist.websites ? JSON.parse(artist.websites) : []).join(", "); } catch { websites = ""; }
		try { members = (artist.members ? JSON.parse(artist.members) : []).join(", "); } catch { members = ""; }
	});

	async function save() {
		const hasEmpty = !name;
		const proceed = await warnEmptyFields(hasEmpty);
		if (!proceed) return;

		saving = true;
		try {
			if (originalName && name && originalName.toLowerCase() !== name.toLowerCase()) {
				await renameArtistInLibrary(originalName, name);
			}

			const akaList = splitList(aka);
			if (akaList.length > 0) {
				await mergeArtistAkas(uid, akaList);
			}

			const update: Record<string, any> = {
				name: name || null,
				about: about || null,
				aka: aka ? JSON.stringify(akaList) : null,
				genres: genreList.length > 0 ? JSON.stringify(genreList) : null,
				tags: tagList.length > 0 ? JSON.stringify(tagList) : null,
				websites: websites ? JSON.stringify(splitList(websites)) : null,
				members: members ? JSON.stringify(splitList(members)) : null,
				profile_art_path: profileArtPath,
			};

			await invoke("update_artist_entry", { uid, update });
			await reloadLibrary("artists");
			await reloadLibrary("tracks");
			await reloadLibrary("albums");
			tagStore.load();
		} finally {
			saving = false;
			closeEditModal();
		}
	}
</script>

<Tabs.Root value="info">
	<Tabs.List class="w-full">
		<Tabs.Trigger value="info" class="flex-1">Info</Tabs.Trigger>
		<Tabs.Trigger value="details" class="flex-1">Details</Tabs.Trigger>
		<Tabs.Trigger value="artwork" class="flex-1">Artwork</Tabs.Trigger>
	</Tabs.List>

	<Tabs.Content value="info">
		<div class="flex-1 min-h-0 pr-1 space-y-3 mt-4">
			<div class="space-y-1.5">
				<Label for="artist-name">Name</Label>
				<Input id="artist-name" bind:value={name} />
			</div>
			<div class="space-y-1.5">
				<Label for="artist-aka">Also Known As <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
				<Input id="artist-aka" bind:value={aka} />
			</div>
			<div class="space-y-1.5">
				<Label for="artist-members">Members <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
				<Input id="artist-members" bind:value={members} />
			</div>
			<div class="space-y-1.5">
				<Label for="artist-genres">Genres <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
				<TagSelector bind:value={genreList} isGenre={true} placeholder="Add genre…" />
			</div>
			<div class="space-y-1.5">
				<Label for="artist-tags">Tags <span class="text-muted-foreground text-xs">(comma-separated)</span></Label>
				<TagSelector bind:value={tagList} placeholder="Add tag…" />
			</div>
		</div>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="details" class="space-y-3 mt-4">
		<div class="space-y-1.5">
			<Label for="artist-websites">Websites <span class="text-muted-foreground text-xs">(comma-separated URLs)</span></Label>
			<Input id="artist-websites" bind:value={websites} />
		</div>
		<div class="space-y-1.5">
			<Label for="artist-about">Biography</Label>
			<Textarea id="artist-about" bind:value={about} rows={5} />
		</div>
		<Separator class="my-4" />
	</Tabs.Content>

	<Tabs.Content value="artwork" class="mt-4">
		<p class="text-xs text-muted-foreground mb-3">Profile art</p>
		<ArtworkEditor entityType="artist" entityUid={uid} onchange={(path) => { profileArtPath = path; }} />
		<Separator class="my-4" />
	</Tabs.Content>
</Tabs.Root>

<div class="flex justify-end gap-2">
	<Button variant="outline" onclick={closeEditModal}>Cancel</Button>
	<Button onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button>
</div>