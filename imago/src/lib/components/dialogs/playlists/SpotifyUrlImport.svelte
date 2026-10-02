<script lang="ts">
	import { Button } from "$shadcn/button/index.js";
	import { Input } from "$shadcn/input/index.js";
	import {
		spotifyIsConnected,
		getSpotifyPlaylistInfo,
		importSpotifyPlaylist,
		extractSpotifyPlaylistId,
		type SpotifyPlaylistInfo,
	} from "$ts/services/spotify";
	import { reloadLibrary } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { onMount } from "svelte";

	let { folder = null, onClose }: { folder?: string | null; onClose: () => void } = $props();

	let connected = $state(false);
	let urlInput = $state("");
	let info = $state<SpotifyPlaylistInfo | null>(null);
	let lookingUp = $state(false);
	let importing = $state(false);
	let error = $state<string | null>(null);

	onMount(async () => {
		connected = await spotifyIsConnected();
	});

	async function handleLookup() {
		error = null;
		info = null;
		const id = extractSpotifyPlaylistId(urlInput);
		if (!id) {
			error = "Invalid Spotify playlist URL or ID.";
			return;
		}
		lookingUp = true;
		try {
			info = await getSpotifyPlaylistInfo(id);
		} catch (e: any) {
			error = e?.toString() ?? "Failed to fetch playlist info.";
		} finally {
			lookingUp = false;
		}
	}

	async function handleImport() {
		if (!info) return;
		importing = true;
		error = null;
		try {
			const uid = await importSpotifyPlaylist(info.id, info.name, info.owner);
			await reloadLibrary("playlists");
			await reloadLibrary("tracks");
			setSelection(uid);
			onClose();
		} catch (e: any) {
			error = e?.toString() ?? "Import failed.";
		} finally {
			importing = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") handleLookup();
	}
</script>

{#if !connected}
	<div class="flex flex-col items-center justify-center gap-3 py-8 text-center">
		<p class="text-sm text-muted-foreground">Connect your Spotify account in Settings to import playlists.</p>
	</div>
{:else}
	<div class="flex flex-col gap-4 pt-4">
		<div class="flex flex-col gap-1">
			<p class="text-sm text-muted-foreground">Paste a Spotify playlist URL or ID.</p>
			<div class="flex gap-2">
				<Input
					placeholder="https://open.spotify.com/playlist/..."
					bind:value={urlInput}
					onkeydown={handleKeydown}
					class="flex-1"
				/>
				<Button onclick={handleLookup} disabled={lookingUp || !urlInput.trim()}>
					{lookingUp ? "Looking up..." : "Look up"}
				</Button>
			</div>
		</div>

		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{/if}

		{#if info}
			<div class="flex items-center gap-3 border rounded-md p-3">
				{#if info.image_url}
					<img src={info.image_url} alt={info.name} class="w-12 h-12 rounded object-cover shrink-0" />
				{/if}
				<div class="flex flex-col min-w-0 flex-1">
					<span class="font-medium truncate">{info.name}</span>
					<span class="text-xs text-muted-foreground">{info.track_count} tracks · {info.owner}</span>
					{#if info.description}
						<span class="text-xs text-muted-foreground truncate">{info.description}</span>
					{/if}
				</div>
				<Button onclick={handleImport} disabled={importing} class="shrink-0">
					{importing ? "Importing..." : "Import"}
				</Button>
			</div>
		{/if}
	</div>
{/if}
