<script lang="ts">
	// COMPONENTS
	import { FolderPlus, ListPlus, FileDown, LayoutGrid, List, ChevronRight, ChevronDown, Folder, FolderOpen, House } from "lucide-svelte";
	import * as ContextMenu from "$shadcn/context-menu/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { Button } from "$shadcn/button/index.js";

	// CUSTOM COMPONENTS
	import PlaylistSortBar from "$lib/components/pages/playlist/PlaylistSortBar.svelte";
	import PlaylistFolderCard from "$lib/components/pages/playlist/PlaylistFolderCard.svelte";
	import PlaylistGridCard from "$lib/components/pages/playlist/PlaylistGridCard.svelte";
	import PlaylistCompactRow from "$lib/components/pages/playlist/PlaylistCompactRow.svelte";
	import FolderContext from "$lib/components/context-menus/FolderContext.svelte";
	import CreateNewPlaylist from "$lib/components/dialogs/playlists/CreateNewPlaylist.svelte";
	import CreateNewFolder from "$lib/components/dialogs/playlists/CreateNewFolder.svelte";
	import ImportPlaylist from "$lib/components/dialogs/playlists/ImportPlaylist.svelte";
	import SearchBar from "$lib/components/custom/search/SearchBar.svelte";
	import MediaGrid from "$lib/layouts/MediaGrid.svelte";
	import PlaylistViewContext from "$lib/components/context-menus/PlaylistViewContext.svelte";

	// SCRIPTS
	import Fuse from "fuse.js";
	import { getPlaylists, onLibraryChange } from "$ts/store/library.svelte";
	import { dragState, endDrag, setHoveredPlaylist } from "$ts/store/drag.svelte";
	import { isDraggingFolderType } from "$ts/drag/dragdrop";
	import { startFolderDrag, onFolderDragOver, onFolderDragExit, onBreadcrumbDragOver, onBreadcrumbDragExit, nestFolder, moveFolderToRoot, resetFolderTimers, doFolderReorder, doCompactFolderReorder } from "$ts/drag/dragdrop_folders";
	import { startPlaylistDrag, dropOnPlaylist, movePlaylists, doPlaylistReorder } from "$ts/drag/dragdrop_playlists";
	import { addTracksToPlaylist } from "$ts/audio/playlistManager.svelte";
	import { folderSelection, navigateTo, breadcrumbs } from "$ts/ui/folderSelection.svelte";
	import { playlistOrder } from "$ts/store/playlistOrderStore.svelte";
	import { folderOf, folderLabel, getSortedFolders, getDirectPlaylists, getFolderArtworkUids, compactRows, compactFolderRows } from "$ts/ui/playlistFolderTree.svelte";
	import type { PlaylistSortField, CompactRow } from "$ts/ui/playlistFolderTree.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import type { Playlist } from "$ts/util/types";

	type GridDropTarget = {
		kind: "folder" | "playlist";
		index: number;
		side: "before" | "after";
	} | null;

	type CompactDropTarget = { folderPath: string | null; index: number; side: "before" | "after" } | null;
	type CompactFolderDrop = { index: number; side: "before" | "after" } | null;

	// VARIABLES
	const SORT_KEY = "sonodex:view:playlists";
	const _saved = loadSort();
	let sortField = $state<PlaylistSortField>(_saved.field);
	let sortDir   = $state<"asc" | "desc">(_saved.dir);
	let compact   = $state<boolean>(_saved.compact);

	let playlistHoverTimer: ReturnType<typeof setTimeout> | null = null;
	let playlistHoverUid: string | null = null;

	let search          = $state("");
	let expandedFolders = $state<Set<string>>(new Set());

	let importDialogOpen   = $state(false);
	let createDialogOpen   = $state(false);
	let createDialogFolder = $state<string | null>(null);
	let folderDialogOpen   = $state(false);
	let folderDialogParent = $state<string | null>(null);

	let draggingPlaylistUid = $state<string | null>(null);
	let draggingFolderPath  = $state<string | null>(null);

	let gridDropTarget   = $state<GridDropTarget>(null);
	let compactDrop      = $state<CompactDropTarget>(null);
	let compactFolderDrop = $state<CompactFolderDrop>(null);

	// Async playlists state
	let playlists = $state<Playlist[]>([]);

	async function loadPlaylists() {
		playlists = await getPlaylists();
	}

	$effect(() => { loadPlaylists(); });
	$effect(() => {
		const unsub = onLibraryChange("playlists:changed", loadPlaylists);
		return unsub;
	});

	let currentPath = $derived(folderSelection.currentPath);
	let crumbs      = $derived(breadcrumbs());

	const visibleChildFolders = $derived(
		search.trim() === "" ? getSortedFolders(playlists, currentPath, sortField, sortDir) : []
	);

	const compactRowsResult = $derived(
		search.trim() === "" ? compactRows(playlists, currentPath, sortField, sortDir, expandedFolders) : []
	);

	const playlistIndexInFolder = $derived.by(() => {
		const map = new Map<string, number>();
		const folderCounts = new Map<string, number>();
		for (const row of compactRowsResult) {
			if (row.kind !== "playlist") continue;
			const key = row.folderPath ?? "";
			const idx = folderCounts.get(key) ?? 0;
			map.set(row.playlist.uid + "|" + key, idx);
			folderCounts.set(key, idx + 1);
		}
		return map;
	});

	let fuseInstance: Fuse<Playlist> | null = null;
	let lastPlaylistsRef: Playlist[] | null = null;

	function getFuse() {
		if (fuseInstance && lastPlaylistsRef === playlists) return fuseInstance;
		lastPlaylistsRef = playlists;
		fuseInstance = new Fuse(playlists, {
			keys: [
				{ name: "title", weight: 0.5,  getFn: (t) => t.title ?? "" },
				{ name: "owner", weight: 0.25, getFn: (t) => t.owner ?? "" },
			],
			threshold: 0.35,
			ignoreLocation: true,
			includeScore: false,
			useExtendedSearch: false,
			minMatchCharLength: 2,
		});
		return fuseInstance;
	}

	const filteredDirect = $derived(
		search.trim() === ""
			? getDirectPlaylists(playlists, currentPath, sortField, sortDir)
			: search.trim().length < 2 ? playlists : getFuse().search(search).map((r) => r.item)
	);

	const allFolderPaths = $derived(
		[...new Set([
			...playlists.map((p) => folderOf(p)).filter((f): f is string => f !== null),
			...folderSelection.knownFolders,
		])].sort()
	);

	$effect(() => {
		try {
			localStorage.setItem(SORT_KEY, JSON.stringify({ sortField, sortDir, compact }));
		} catch {}
	});

	// FUNCTIONS
	function loadSort(): { field: PlaylistSortField; dir: "asc" | "desc"; compact: boolean } {
		try {
			const raw = localStorage.getItem(SORT_KEY);
			if (raw) {
				const s = JSON.parse(raw);
				return {
					field: s.sortField ?? "custom",
					dir: s.sortDir ?? "asc",
					compact: s.compact ?? false,
				};
			}
		} catch {}
		return { field: "custom", dir: "asc", compact: false };
	}

	function openCreatePlaylistIn(folder: string | null) {
		createDialogFolder = folder;
		createDialogOpen = true;
	}

	function openCreateFolderIn(parent: string | null) {
		folderDialogParent = parent;
		folderDialogOpen = true;
	}

	function toggleExpand(folderPath: string) {
		const next = new Set(expandedFolders);
		next.has(folderPath) ? next.delete(folderPath) : next.add(folderPath);
		expandedFolders = next;
	}

	function getSide(e: DragEvent): "before" | "after" {
		const el = e.currentTarget as HTMLElement;
		const rect = el.getBoundingClientRect();
		return (e.clientX - rect.left) < rect.width / 2 ? "before" : "after";
	}

	function clearGridDrop() { gridDropTarget = null; }

	function cancelPlaylistHover() {
		if (playlistHoverTimer !== null) clearTimeout(playlistHoverTimer);
		playlistHoverTimer = null;
		playlistHoverUid = null;
	}

	function schedulePlaylistHover(uid: string) {
		if (playlistHoverUid === uid) return;
		cancelPlaylistHover();
		playlistHoverUid = uid;
		playlistHoverTimer = setTimeout(() => {
			playlistHoverTimer = null;
			setHoveredPlaylist(uid);
			setSelection(uid, "playlist");
		}, 600);
	}

	function handleDragEnd() {
		draggingPlaylistUid = null;
		draggingFolderPath = null;
		gridDropTarget = null;
		compactDrop = null;
		compactFolderDrop = null;
		cancelPlaylistHover();
		resetFolderTimers();
		endDrag();
	}

	function handleFolderDragStart(e: DragEvent, folderPath: string) {
		draggingFolderPath = folderPath;
		startFolderDrag(e, folderPath);
	}

	function handlePlaylistDragStart(e: DragEvent, playlistUid: string) {
		draggingPlaylistUid = playlistUid;
		startPlaylistDrag(e, playlistUid);
	}

	function onGridFolderDragOver(e: DragEvent, fi: number, folderPath: string) {
		e.preventDefault();
		if (isDraggingFolderType(e)) {
			gridDropTarget = { kind: "folder", index: fi, side: getSide(e) };
		} else {
			gridDropTarget = null;
			onFolderDragOver(folderPath, navigateTo);
		}
	}

	async function onGridFolderDrop(e: DragEvent, fi: number, folderPath: string) {
		e.preventDefault();
		if (isDraggingFolderType(e) && draggingFolderPath) {
			if (gridDropTarget?.kind === "folder") {
				await doFolderReorder(draggingFolderPath, fi, gridDropTarget.side, playlists, currentPath, sortField, sortDir);
			}
		} else if (draggingPlaylistUid) {
			await movePlaylists([draggingPlaylistUid], folderPath);
		}
		clearGridDrop();
		handleDragEnd();
	}

	function onGridPlaylistDragOver(e: DragEvent, pi: number, playlistUid: string) {
		if (isDraggingFolderType(e)) return;
		e.preventDefault();
		if (draggingPlaylistUid && draggingPlaylistUid !== playlistUid) {
			gridDropTarget = { kind: "playlist", index: pi, side: getSide(e) };
		} else if (!draggingPlaylistUid) {
			gridDropTarget = null;
			schedulePlaylistHover(playlistUid);
		}
	}

	async function onGridPlaylistDrop(e: DragEvent, pi: number, playlistUid: string) {
		e.preventDefault();
		if (draggingPlaylistUid && gridDropTarget?.kind === "playlist") {
			const visiblePlaylists = filteredDirect;
			await doPlaylistReorder(draggingPlaylistUid, pi, gridDropTarget.side, playlists, sortField, sortDir, currentPath);
		} else {
			await dropOnPlaylist(e, playlistUid);
		}
		clearGridDrop();
		handleDragEnd();
	}

	function onGridEmptyDragOver(e: DragEvent) {
		e.preventDefault();
		clearGridDrop();
	}

	async function onGridEmptyDrop(e: DragEvent) {
		e.preventDefault();
		if (draggingPlaylistUid) {
			await movePlaylists([draggingPlaylistUid], currentPath ?? "");
		}
		handleDragEnd();
	}

	function handleCompactFolderDragOver(e: DragEvent, ri: number, folderPath: string) {
		e.preventDefault();
		if (isDraggingFolderType(e)) {
			compactFolderDrop = { index: ri, side: e.clientY < (e.currentTarget as HTMLElement).getBoundingClientRect().top + 20 ? "before" : "after" };
		} else {
			compactFolderDrop = null;
			onFolderDragOver(folderPath, navigateTo);
		}
	}

	async function handleCompactFolderDrop(e: DragEvent, ri: number, folderPath: string) {
		e.preventDefault();
		if (isDraggingFolderType(e) && draggingFolderPath && compactFolderDrop) {
			await doCompactFolderReorder(draggingFolderPath, ri, compactFolderDrop.side, playlists, currentPath, sortField, sortDir, expandedFolders);
		} else if (draggingPlaylistUid) {
			await movePlaylists([draggingPlaylistUid], folderPath);
		}
		compactFolderDrop = null;
		handleDragEnd();
	}

	function handleCompactPlaylistDragOver(e: DragEvent, pi: number, folderPath: string | null) {
		e.preventDefault();
		if (isDraggingFolderType(e)) return;
		compactDrop = { folderPath, index: pi, side: e.clientY < (e.currentTarget as HTMLElement).getBoundingClientRect().top + 20 ? "before" : "after" };
	}

	async function handleCompactPlaylistDrop(e: DragEvent, pi: number, folderPath: string | null) {
		e.preventDefault();
		if (draggingPlaylistUid && compactDrop) {
			const folderPlaylists = filteredDirect.filter(p => folderOf(p) === folderPath);
			await doPlaylistReorder(draggingPlaylistUid, pi, compactDrop.side, playlists, sortField, sortDir, folderPath);
		}
		compactDrop = null;
		handleDragEnd();
	}

	async function handleDropOnRoot(e: DragEvent) {
		e.preventDefault();
		if (draggingPlaylistUid) await movePlaylists([draggingPlaylistUid], "");
		else if (draggingFolderPath) await moveFolderToRoot(draggingFolderPath);
		handleDragEnd();
	}
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="flex flex-col gap-2 pt-4 border-2 h-full w-full overflow-hidden rounded-md">
		<div class="flex justify-between items-center gap-2 pr-4 pl-4 shrink-0">
			<h1 class="h1">Playlists</h1>
			<SearchBar bind:search searchCount={filteredDirect.length} />
		</div>

		<div class="flex gap-1 items-center pr-4 pl-4 shrink-0">
			<Button variant="ghost" size="icon" onclick={() => importDialogOpen = true}><FileDown /></Button>
			<Button variant="ghost" size="icon" onclick={() => openCreatePlaylistIn(currentPath)}><ListPlus /></Button>
			<Button variant="ghost" size="icon" onclick={() => openCreateFolderIn(currentPath)}><FolderPlus /></Button>
			<div class="flex-1"></div>
			<PlaylistSortBar bind:sortField bind:sortDir />
			<Button variant="ghost" size="icon" onclick={() => compact = !compact}>
				{#if compact}<LayoutGrid />{:else}<List />{/if}
			</Button>
		</div>

		{#if crumbs.length > 1}
			<div class="flex items-center gap-1 px-4 text-xs text-muted-foreground flex-wrap shrink-0">
				{#each crumbs as crumb, i}
					{#if i > 0}<ChevronRight class="size-3 shrink-0" />{/if}
					<button
						class="hover:text-foreground transition-colors"
						class:text-foreground={i === crumbs.length - 1}
						onclick={() => navigateTo(crumb.path)}
						ondragover={(e) => { e.preventDefault(); onBreadcrumbDragOver(crumb.path, navigateTo); }}
						ondragleave={onBreadcrumbDragExit}
					>
						{#if i === 0}<House class="size-3 inline mr-0.5" />{/if}
						{crumb.label}
					</button>
				{/each}
			</div>
		{/if}

		<ScrollArea class="min-h-0 min-w-0 mt-0 pt-0">

			{@const showRootDrop = currentPath !== null && dragState.active}
			<button
				class="w-full mx-4 px-3 rounded-md border border-dashed border-muted-foreground/30 text-xs text-muted-foreground flex items-center gap-2 transition-all duration-150 overflow-hidden"
				class:opacity-0={!showRootDrop}
				class:max-h-0={!showRootDrop}
				class:max-h-10={showRootDrop}
				class:py-2={showRootDrop}
				class:mb-4={showRootDrop}
				class:pointer-events-none={!showRootDrop}
				ondragover={(e) => { e.preventDefault(); onBreadcrumbDragOver(null, navigateTo); }}
				ondragleave={onBreadcrumbDragExit}
				ondrop={handleDropOnRoot}
			>
				<House class="size-3" /> Drag here to move to root
			</button>

			{#if !compact}

				<MediaGrid>

					{#each visibleChildFolders as folderPath, fi (folderPath)}
						{@const artPlaylists = getFolderArtworkUids(playlists, folderPath).map((uid) => playlists.find((p) => p.uid === uid)).filter((p): p is Playlist => !!p)}
						{@const isDraggingThis = draggingFolderPath === folderPath}
						{@const isReorderTarget = gridDropTarget?.kind === "folder" && gridDropTarget.index === fi}
						{@const isMoveTarget = dragState.hoveredFolderPath === folderPath && gridDropTarget === null}

						<ContextMenu.Root>
							<ContextMenu.Trigger class="w-full text-left">
								<div
									class="relative rounded-md transition-all"
									class:opacity-40={isDraggingThis}
								>
									{#if isReorderTarget && gridDropTarget?.side === "before" && !isDraggingThis}
										<div class="absolute -left-2 top-0 bottom-0 w-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
									{/if}
									{#if isReorderTarget && gridDropTarget?.side === "after" && !isDraggingThis}
										<div class="absolute -right-2 top-0 bottom-0 w-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
									{/if}
									<button
										class="flex flex-col gap-1 w-full text-left rounded-md bg-muted"
										class:ring-2={isMoveTarget}
										class:ring-primary={isMoveTarget}
										draggable="true"
										onclick={() => navigateTo(folderPath)}
										ondragstart={(e) => handleFolderDragStart(e, folderPath)}
										ondragend={handleDragEnd}
										ondragover={(e) => onGridFolderDragOver(e, fi, folderPath)}
										ondragleave={() => { clearGridDrop(); onFolderDragExit(folderPath); }}
										ondrop={(e) => onGridFolderDrop(e, fi, folderPath)}
									>
										<PlaylistFolderCard folderPath={folderLabel(folderPath)} {artPlaylists} />
									</button>
								</div>
							</ContextMenu.Trigger>
							<FolderContext path={folderPath} folderPaths={allFolderPaths} sortField={sortField} sortDir={sortDir} onCreatePlaylist={openCreatePlaylistIn} onCreateFolder={openCreateFolderIn} />
						</ContextMenu.Root>
					{/each}

					{#each filteredDirect as p, pi (p.uid)}
						<PlaylistGridCard
							playlist={p}
							isDraggingThis={draggingPlaylistUid === p.uid}
							isHovered={dragState.hoveredPlaylistUid === p.uid}
							isReorderBefore={gridDropTarget?.kind === "playlist" && gridDropTarget.index === pi && gridDropTarget.side === "before"}
							isReorderAfter={gridDropTarget?.kind === "playlist" && gridDropTarget.index === pi && gridDropTarget.side === "after"}
							{allFolderPaths}
							ondragstart={(e) => handlePlaylistDragStart(e, p.uid)}
							ondragend={handleDragEnd}
							ondragover={(e) => onGridPlaylistDragOver(e, pi, p.uid)}
							ondragleave={() => { clearGridDrop(); setHoveredPlaylist(null); cancelPlaylistHover(); }}
							ondrop={(e) => onGridPlaylistDrop(e, pi, p.uid)}
						/>
					{/each}

					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="min-h-16 mt-2 rounded-md transition-colors"
						class:border-2={dragState.active}
						class:border-dashed={dragState.active}
						class:border-muted-foreground={dragState.active}
						ondragover={onGridEmptyDragOver}
						ondragleave={() => clearGridDrop()}
						ondrop={onGridEmptyDrop}
					></div>
				</MediaGrid>

			{:else}
				<div class="flex flex-col pr-4 pl-4 pb-4">

					{#each compactRowsResult as row, ri}
						{#if row.kind === "folder"}
							{@const isExpanded = expandedFolders.has(row.path)}
							{@const folderHovered = dragState.hoveredFolderPath === row.path && compactFolderDrop === null}
							{@const isReorderTarget = compactFolderDrop?.index === ri}

							<ContextMenu.Root>
								<ContextMenu.Trigger class="w-full">
									<div class="relative">
										{#if isReorderTarget && compactFolderDrop?.side === "before" && draggingFolderPath !== row.path}
											<div class="absolute left-0 right-0 -top-px h-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
										{/if}
										{#if isReorderTarget && compactFolderDrop?.side === "after" && draggingFolderPath !== row.path}
											<div class="absolute left-0 right-0 -bottom-px h-0.5 bg-primary rounded-full z-10 pointer-events-none"></div>
										{/if}
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											draggable="true"
											class="flex items-center gap-1 py-1.5 rounded-md hover:bg-muted/50 transition-all cursor-grab"
											class:ring-1={folderHovered}
											class:ring-primary={folderHovered}
											class:opacity-40={draggingFolderPath === row.path}
											style="padding-left: {(row.depth + 1) * 16}px; padding-right: 8px;"
											ondragstart={(e) => handleFolderDragStart(e, row.path)}
											ondragend={handleDragEnd}
											ondragover={(e) => handleCompactFolderDragOver(e, ri, row.path)}
											ondragleave={(e) => {
												if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
													compactFolderDrop = null;
													onFolderDragExit(row.path);
												}
											}}
											ondrop={(e) => handleCompactFolderDrop(e, ri, row.path)}
										>
											<button
												class="flex items-center gap-2 flex-1 min-w-0 text-left"
												onclick={() => navigateTo(row.path)}
											>
												{#if isExpanded}
													<FolderOpen class="size-4 shrink-0 text-muted-foreground" />
												{:else}
													<Folder class="size-4 shrink-0 text-muted-foreground" />
												{/if}
												<span class="text-sm truncate">{folderLabel(row.path)}</span>
											</button>
											<button
												class="shrink-0 text-muted-foreground hover:text-foreground transition-colors"
												onclick={() => toggleExpand(row.path)}
												aria-label={isExpanded ? "Collapse" : "Expand"}
											>
												{#if isExpanded}
													<ChevronDown class="size-4" />
												{:else}
													<ChevronRight class="size-4" />
												{/if}
											</button>
										</div>
									</div>
								</ContextMenu.Trigger>
								<FolderContext path={row.path} folderPaths={allFolderPaths} sortField={sortField} sortDir={sortDir} onCreatePlaylist={openCreatePlaylistIn} onCreateFolder={openCreateFolderIn} />
							</ContextMenu.Root>

						{:else}
							{@const pi = playlistIndexInFolder.get(row.playlist.uid + "|" + (row.folderPath ?? "")) ?? 0}
							<PlaylistCompactRow
								playlist={row.playlist}
								indent={(row.depth + 1) * 16}
								isDraggingThis={draggingPlaylistUid === row.playlist.uid}
								isDropBefore={compactDrop?.folderPath === row.folderPath && compactDrop.index === pi && compactDrop.side === "before"}
								isDropAfter={compactDrop?.folderPath === row.folderPath && compactDrop.index === pi && compactDrop.side === "after"}
								{allFolderPaths}
								{draggingPlaylistUid}
								ondragstart={(e) => handlePlaylistDragStart(e, row.playlist.uid)}
								ondragend={handleDragEnd}
								ondragover={(e) => handleCompactPlaylistDragOver(e, pi, row.folderPath)}
								ondragleave={() => { compactDrop = null; }}
								ondrop={(e) => handleCompactPlaylistDrop(e, pi, row.folderPath)}
								onrowdragover={(e) => { e.preventDefault(); setHoveredPlaylist(row.playlist.uid); if (!draggingPlaylistUid) schedulePlaylistHover(row.playlist.uid); }}
								onrowdragleave={() => { setHoveredPlaylist(null); cancelPlaylistHover(); }}
								onrowdrop={(e) => dropOnPlaylist(e, row.playlist.uid)}
							/>
						{/if}
					{/each}

				</div>
			{/if}

		</ScrollArea>
	</ContextMenu.Trigger>
	<PlaylistViewContext path={currentPath} onCreatePlaylist={openCreatePlaylistIn} onCreateFolder={openCreateFolderIn} />
</ContextMenu.Root>

<CreateNewPlaylist bind:open={createDialogOpen} folder={createDialogFolder} />
<CreateNewFolder bind:open={folderDialogOpen} parent={folderDialogParent} />
<ImportPlaylist bind:open={importDialogOpen} />