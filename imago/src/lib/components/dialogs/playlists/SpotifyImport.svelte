<script lang="ts">
	import { Button } from "$shadcn/button/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import {
		spotifyIsConnected,
		getSpotifyPlaylists,
		importSpotifyPlaylist,
		connectSpotify,
		type SpotifyPlaylistSummary,
	} from "$ts/services/spotify";
	import { reloadLibrary } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import { onMount } from "svelte";
	import Fuse from "fuse.js";
	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";

	let { open = $bindable(false) }: { open: boolean } = $props();

	let connected = $state(false);
	let playlists = $state<SpotifyPlaylistSummary[]>([]);
	let loading = $state(false);
	let selected = $state<Set<string>>(new Set());
	let search = $state("");

	let importingIds = $state<Set<string>>(new Set());
	let bulkProgress = $state<{ done: number; total: number } | null>(null);
	let bulkError = $state<string | null>(null);

	const fuse = $derived(
		new Fuse(playlists, {
			keys: [{ name: "name", weight: 1, getFn: (p) => p.name ?? "" }],
			threshold: 0.35,
			ignoreLocation: true,
			includeScore: false,
			minMatchCharLength: 2,
		})
	);

	const filteredPlaylists = $derived(
		search.trim().length < 2
			? playlists
			: fuse.search(search).map((r) => r.item)
	);

	const allFilteredSelected = $derived(
		filteredPlaylists.length > 0 && filteredPlaylists.every((p) => selected.has(p.id))
	);

	onMount(async () => {
		connected = await spotifyIsConnected();
		if (connected) await loadPlaylists();
	});

	async function loadPlaylists() {
		loading = true;
		try {
			playlists = await getSpotifyPlaylists();
			// console.log("playlists loaded:", playlists.length, playlists);
		} catch (e) {
			console.error("Failed to load Spotify playlists:", e);
		} finally {
			loading = false;
		}
	}

	function toggleSelect(id: string) {
		const next = new Set(selected);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selected = next;
	}

	function toggleSelectAll() {
		if (allFilteredSelected) {
			const next = new Set(selected);
			filteredPlaylists.forEach((p) => next.delete(p.id));
			selected = next;
		} else {
			const next = new Set(selected);
			filteredPlaylists.forEach((p) => next.add(p.id));
			selected = next;
		}
	}

	async function handleImportOne(pl: SpotifyPlaylistSummary) {
		importingIds = new Set([...importingIds, pl.id]);
		try {
			const uid = await importSpotifyPlaylist(pl.id, pl.name, pl.owner);
			await reloadLibrary("playlists");
			await reloadLibrary("tracks");
			setSelection(uid);
			open = false;
		} catch (e) {
			console.error("Failed to import playlist:", e);
		} finally {
			const next = new Set(importingIds);
			next.delete(pl.id);
			importingIds = next;
		}
	}

	async function handleImportSelected() {
		const toImport = playlists.filter((p) => selected.has(p.id));
		if (toImport.length === 0) return;
		await runBulkImport(toImport);
	}

	async function handleImportAll() {
		await runBulkImport(playlists);
	}

	async function runBulkImport(items: SpotifyPlaylistSummary[]) {
		bulkError = null;
		bulkProgress = { done: 0, total: items.length };
		let lastUid: string | null = null;

		for (const pl of items) {
			importingIds = new Set([...importingIds, pl.id]);
			try {
				lastUid = await importSpotifyPlaylist(pl.id, pl.name, pl.owner);
			} catch (e) {
				console.error(`Failed to import ${pl.name}:`, e);
			} finally {
				const next = new Set(importingIds);
				next.delete(pl.id);
				importingIds = next;
			}
			bulkProgress = { done: bulkProgress.done + 1, total: items.length };
		}

		await reloadLibrary("playlists");
		await reloadLibrary("tracks");

		if (lastUid) setSelection(lastUid);
		bulkProgress = null;
		selected = new Set();
		open = false;
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content class="max-w-xl w-full h-[580px] flex flex-col overflow-hidden">
		<AlertDialog.Header>
			<AlertDialog.Title>Import Spotify Playlists</AlertDialog.Title>
			<AlertDialog.Description>
				Select playlists to import into your library.
			</AlertDialog.Description>
		</AlertDialog.Header>

		<div class="flex-1 flex flex-col min-h-0 overflow-hidden">
			{#if !connected}
				<div class="flex flex-col items-center justify-center gap-3 py-8 text-center flex-1">
					<p class="text-sm text-muted-foreground">Connect your Spotify account to import playlists.</p>
					<Button onclick={connectSpotify}>Connect Spotify</Button>
				</div>
			{:else if loading}
				<div class="flex items-center justify-center py-8 flex-1">
					<p class="text-sm text-muted-foreground">Loading playlists...</p>
				</div>
			{:else if playlists.length === 0}
				<div class="flex flex-col items-center justify-center gap-3 py-8 text-center flex-1">
					<p class="text-sm text-muted-foreground">No playlists found.</p>
					<Button variant="outline" onclick={loadPlaylists}>Refresh</Button>
				</div>
			{:else}
				<div class="flex flex-col gap-2 flex-1 min-h-0">
					<div class="flex items-center gap-2">
						<SearchBar bind:search searchCount={playlists.length} />
						<Button variant="ghost" class="text-xs shrink-0" onclick={toggleSelectAll}>
							{allFilteredSelected ? "Deselect all" : "Select all"}
						</Button>
					</div>

					{#if bulkProgress}
						<div class="flex flex-col gap-1">
							<div class="flex justify-between text-xs text-muted-foreground">
								<span>Importing {bulkProgress.done} / {bulkProgress.total}</span>
								<span>{Math.round((bulkProgress.done / bulkProgress.total) * 100)}%</span>
							</div>
							<div class="w-full bg-muted rounded-full h-1.5">
								<div
									class="bg-primary h-1.5 rounded-full transition-all duration-300"
									style="width: {(bulkProgress.done / bulkProgress.total) * 100}%"
								></div>
							</div>
						</div>
					{/if}

					<ScrollArea class="flex-1 min-h-0">
						<div class="flex flex-col gap-1 pr-3">
							{#each filteredPlaylists as pl, i (pl.id + i)}
								<div
									class="flex items-center gap-3 border rounded px-3 py-2 text-sm w-full cursor-pointer transition-colors {selected.has(pl.id) ? 'bg-accent border-primary' : 'hover:bg-muted'}"
									onclick={() => toggleSelect(pl.id)}
									role="button"
									tabindex="0"
									onkeydown={(e) => e.key === "Enter" && toggleSelect(pl.id)}
								>
									{#if pl.image_url}
										<img src={pl.image_url} alt={pl.name} class="w-9 h-9 rounded object-cover shrink-0" />
									{:else}
										<div class="w-9 h-9 rounded bg-muted-foreground/20 shrink-0 flex items-center justify-center">
											<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-muted-foreground"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>
										</div>
									{/if}
									<div class="flex flex-col min-w-0 flex-1">
										<span class="font-medium truncate">{pl.name}</span>
										<span class="text-xs text-muted-foreground">{pl.track_count} tracks · {pl.owner}</span>
									</div>
									<Button
										variant="outline"
										class="shrink-0 ml-2 text-xs h-7 px-2"
										onclick={(e) => { e.stopPropagation(); handleImportOne(pl); }}
										disabled={importingIds.has(pl.id) || !!bulkProgress}
									>
										{importingIds.has(pl.id) ? "Importing..." : "Import"}
									</Button>
								</div>
							{/each}
						</div>
					</ScrollArea>
				</div>
			{/if}
		</div>

		<AlertDialog.Footer class="flex items-center justify-between gap-2 flex-wrap">
			<div class="flex items-center gap-2">
				{#if selected.size > 0}
					<span class="text-xs text-muted-foreground">{selected.size} selected</span>
					<Button
						variant="default"
						onclick={handleImportSelected}
						disabled={!!bulkProgress}
					>
						Import selected
					</Button>
				{/if}
				{#if playlists.length > 0}
					<Button
						variant="outline"
						onclick={handleImportAll}
						disabled={!!bulkProgress}
					>
						Import all
					</Button>
				{/if}
			</div>
			<AlertDialog.Cancel disabled={!!bulkProgress}>Close</AlertDialog.Cancel>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>