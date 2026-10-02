<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	import * as Tabs from "$shadcn/tabs";
	import { Label } from "$shadcn/label";
	import { Input } from "$shadcn/input";
	import { Textarea } from "$shadcn/textarea";
	import { Button } from "$shadcn/button";
	import { Separator } from "$shadcn/separator";

	import ArtworkEditor from "./ArtworkEditor.svelte";

	import { closeEditModal } from "$ts/ui/editModal.svelte";
	import { getPlaylist, reloadLibrary, reloadSingle } from "$ts/store/library.svelte";
	import { deletePlaylist } from "$ts/audio/playlistManager.svelte";
	import { artworkCache, artworkBytesCache, artworkColorCache } from "$ts/library/artworkLoader";
	import type { Playlist } from "$ts/util/types";

	let { uid } = $props<{ uid: string }>();

	let playlistEntity = $state<Playlist | null>(null);
	let title = $state("");
	let description = $state("");
	let owner = $state("");
	let artworkPath = $state<string | null>(null);
	let saving = $state(false);

	onMount(async () => {
		const playlist = await getPlaylist(uid);
		if (!playlist) return;

		playlistEntity = playlist;
		title = playlist.title ?? "";
		description = playlist.description ?? "";
		owner = playlist.owner ?? "";
		artworkPath = playlist.artwork_path ?? null;
	});

	async function save() {
		saving = true;
		try {
			const update: Record<string, any> = {
				title: title || null,
				description: description || null,
				owner: owner || null,
				artwork_path: artworkPath,
			};

			await invoke("update_playlist_entry", { uid, update });

			const cacheKey = `playlist:${uid}`;
			artworkCache.delete(cacheKey);
			artworkBytesCache.delete(cacheKey);
			artworkColorCache.delete(cacheKey);

			await reloadSingle(uid);
			await reloadLibrary("playlists");
		} finally {
			saving = false;
			closeEditModal();
		}
	}

	async function deleteThisPlaylist() {
		deletePlaylist(uid);
		closeEditModal();
	}
</script>

<div class="space-y-3 mt-4">
	<ArtworkEditor
		entityType="playlist"
		entityUid={uid}
		entity={playlistEntity}
		onchange={(path) => { artworkPath = path; }}
	/>
	<div class="space-y-1.5 mt-8">
		<Label for="playlist-title">Title</Label>
		<Input id="playlist-title" bind:value={title} />
	</div>
	<div class="space-y-1.5">
		<Label for="playlist-desc">Description</Label>
		<Textarea id="playlist-desc" bind:value={description} rows={4} />
	</div>
	<Separator class="my-4" />
</div>

<div class="flex justify-end gap-2">
	<Button variant="destructive" onclick={deleteThisPlaylist}>Delete</Button>
	<Button variant="outline" onclick={closeEditModal}>Cancel</Button>
	<Button onclick={() => save()} disabled={saving}>{saving ? "Saving…" : "Save"}</Button>
</div>