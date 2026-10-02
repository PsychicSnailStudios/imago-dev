<script lang="ts">
	import * as Dialog from "$shadcn/dialog/index.js";
	import * as Popover from "$shadcn/popover/index.js";
	import * as Command from "$shadcn/command/index.js";
	import { Button } from "$shadcn/button/index.js";
	import ChevronsUpDownIcon from "@lucide/svelte/icons/chevrons-up-down";
	import CheckIcon from "@lucide/svelte/icons/check";
	import { cn } from "$lib/utils.js";
	import { tick } from "svelte";
	import { getAlbums, getTracks, onLibraryChange, reloadLibrary } from "$ts/store/library.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { parseTags, parseAlbumEntries } from "$ts/util/parsers";
	import type { Track, Album } from "$ts/util/types";

	let {
		open = $bindable(false),
		trackUids = [],
	}: { open: boolean; trackUids: string[] } = $props();

	let comboOpen = $state(false);
	let inputValue = $state("");
	let selectedAlbumUid = $state<string | null>(null);
	let triggerRef = $state<HTMLButtonElement>(null!);

	let allTracks = $state<Track[]>([]);
	let allAlbums = $state<Album[]>([]);

	async function load() {
		[allTracks, allAlbums] = await Promise.all([getTracks(), getAlbums()]);
	}

	// Load when dialog opens and stay fresh
	$effect(() => {
		if (open) load();
	});
	$effect(() => {
		const u1 = onLibraryChange("tracks:changed", load);
		const u2 = onLibraryChange("albums:changed", load);
		return () => { u1(); u2(); };
	});

	const selectedTracks = $derived(allTracks.filter(t => trackUids.includes(t.uid)));

	const inferredArtists = $derived.by<string[]>(() => {
		const set = new Set<string>();
		for (const t of selectedTracks) {
			parseTags(t.artists).forEach(a => set.add(a));
		}
		return [...set];
	});

	const filteredAlbums = $derived(
		allAlbums.filter(a => a.title.toLowerCase().includes(inputValue.toLowerCase()))
	);

	const exactMatch = $derived(
		allAlbums.find(a => a.title.toLowerCase() === inputValue.trim().toLowerCase())
	);

	const selectedAlbumTitle = $derived(
		selectedAlbumUid ? (allAlbums.find(a => a.uid === selectedAlbumUid)?.title ?? "") : ""
	);

	const displayLabel = $derived(selectedAlbumTitle || inputValue || "Search or create album…");
	const isPlaceholder = $derived(!selectedAlbumTitle && !inputValue);
	const actionLabel = $derived(selectedAlbumUid || exactMatch ? "Add to Album" : "Create & Add");
	const canConfirm = $derived(!!inputValue.trim() || !!selectedAlbumUid);

	function closeComboAndFocus() {
		comboOpen = false;
		tick().then(() => triggerRef?.focus());
	}

	function selectAlbum(uid: string, title: string) {
		selectedAlbumUid = uid;
		inputValue = title;
		closeComboAndFocus();
	}

	function clearSelection() {
		selectedAlbumUid = null;
		inputValue = "";
	}

	function handleOpenChange(v: boolean) {
		open = v;
		if (!v) clearSelection();
	}

	async function handleConfirm() {
		const name = (selectedAlbumTitle || inputValue).trim();
		if (!name) return;

		const targetUid = selectedAlbumUid ?? exactMatch?.uid ?? null;

		if (targetUid) {
			const album = allAlbums.find(a => a.uid === targetUid);
			if (!album) return;

			let existing: { uid: string; name: string; track_number: number | null }[] = [];
			try { existing = album.tracks ? JSON.parse(album.tracks as string) : []; } catch {}

			const nextOrder = existing.length + 1;
			const toAdd = selectedTracks
				.filter(t => !existing.some(e => e.uid === t.uid))
				.map((t, i) => ({ uid: t.uid, name: t.title ?? "", track_number: nextOrder + i }));

			await invoke("update_album_entry", {
				uid: targetUid,
				update: { tracks: JSON.stringify([...existing, ...toAdd]) },
			});

			for (const t of selectedTracks) {
				const albumEntries = parseAlbumEntries(t.albums);
				if (!albumEntries.some(e => e.uid === targetUid)) {
					albumEntries.push({ uid: targetUid, name, track_number: null, disc: null });
					await invoke("update_track_metadata", {
						uid: t.uid,
						update: { albums: JSON.stringify(albumEntries) },
					});
				}
			}
		} else {
			const newUid = "a-" + crypto.randomUUID();
			const trackEntries = selectedTracks.map((t, i) => ({
				uid: t.uid, name: t.title ?? "", track_number: i + 1,
			}));

			await invoke("create_album_entry", {
				album: {
					uid: newUid,
					title: name,
					artists: JSON.stringify(inferredArtists),
					album_artist: inferredArtists[0] ?? null,
					tracks: JSON.stringify(trackEntries),
					format: null, rating: null, release_date: null,
					tags: "[]", genres: "[]", credits: null, label: null, artwork_path: null,
				},
			});

			for (const t of selectedTracks) {
				const albumEntries = parseAlbumEntries(t.albums);
				albumEntries.push({ uid: newUid, name, track_number: null, disc: null });
				await invoke("update_track_metadata", {
					uid: t.uid,
					update: { albums: JSON.stringify(albumEntries) },
				});
			}
		}

		await reloadLibrary("albums");
		await reloadLibrary("tracks");
		clearSelection();
		open = false;
	}
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Add to Album</Dialog.Title>
			<Dialog.Description>
				Search for an existing album or type a new name to create one.
				{trackUids.length} track{trackUids.length === 1 ? "" : "s"} will be added.
			</Dialog.Description>
		</Dialog.Header>

		<div class="py-2">
			<Popover.Root bind:open={comboOpen}>
				<Popover.Trigger bind:ref={triggerRef}>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="outline"
							role="combobox"
							aria-expanded={comboOpen}
							class="w-full justify-between"
						>
							<span class={isPlaceholder ? "text-muted-foreground font-normal" : ""}>
								{displayLabel}
							</span>
							<ChevronsUpDownIcon class="ml-2 size-4 shrink-0 opacity-50" />
						</Button>
					{/snippet}
				</Popover.Trigger>
				<Popover.Content class="w-[--radix-popover-trigger-width] p-0" align="start">
					<Command.Root shouldFilter={false}>
						<Command.Input
							placeholder="Search or type album name…"
							bind:value={inputValue}
							oninput={() => { selectedAlbumUid = null; }}
						/>
						<Command.List>
							{#if filteredAlbums.length === 0 && !inputValue.trim()}
								<Command.Empty>Type to search or create an album.</Command.Empty>
							{:else if filteredAlbums.length === 0}
								<Command.Item value="__create__" onSelect={() => closeComboAndFocus()}>
									<span class="text-muted-foreground mr-2 text-xs">Create</span>
									"{inputValue.trim()}"
								</Command.Item>
							{:else}
								<Command.Group>
									{#each filteredAlbums as album (album.uid)}
										<Command.Item value={album.uid} onSelect={() => selectAlbum(album.uid, album.title)}>
											<CheckIcon class={cn("mr-2 size-4", selectedAlbumUid !== album.uid && "text-transparent")} />
											{album.title}
										</Command.Item>
									{/each}
									{#if inputValue.trim() && !exactMatch}
										<Command.Item value="__create__" onSelect={() => closeComboAndFocus()}>
											<span class="text-muted-foreground mr-2 text-xs">Create</span>
											"{inputValue.trim()}"
										</Command.Item>
									{/if}
								</Command.Group>
							{/if}
						</Command.List>
					</Command.Root>
				</Popover.Content>
			</Popover.Root>
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => handleOpenChange(false)}>Cancel</Button>
			<Button onclick={handleConfirm} disabled={!canConfirm}>{actionLabel}</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>