<script lang="ts">
	import { toast } from "svelte-sonner";
	import {
		Loader2,
		Plus,
		Trash2,
		FolderOpen,
		RefreshCw,
		Upload,
		Download,
		Globe,
		HardDrive,
		Lock,
		Unlock,
		AlertCircle,
		CheckCircle2,
		MoreHorizontal,
		Link,
		DatabaseZap,
		ChevronDown,
		ChevronUp,
		FolderPlus,
	} from "lucide-svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { open } from "@tauri-apps/plugin-dialog";
	import { onMount } from "svelte";

	import { Button, buttonVariants } from "$shadcn/button/index.js";
	import * as Dialog from "$shadcn/dialog/index.js";
	import * as Tooltip from "$shadcn/tooltip/index.js";
	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";

	import { libraryStore, loadLibraryRegistry } from "$ts/store/library.svelte";
	import {
		createLibrary,
		deleteLibrary,
		syncLibrary,
		pushLibrary,
		exportLibrary,
		importLibrary,
		checkWritePermission,
		rebuildMerged,
		addPathToLibrary,
	} from "$ts/library/libraryRegistry.svelte";
	import { scanState } from "$ts/store/session.svelte";
	import { loadLibrary } from "$ts/store/library.svelte";

	// ─── Per-library state ────────────────────────────────────────────────────────

	type LibState = {
		expanded: boolean;
		paths: { id: number; path: string }[];
		addingPath: boolean;
		newPath: string;
		removingPath: string | null;
		loadingPaths: boolean;
	};

	let libStates = $state<Record<string, LibState>>({});
	let lastScannedLibUid = $state<string | null>(null);

	function ensureLibState(libUid: string) {
		if (!libStates[libUid]) {
			libStates[libUid] = {
				expanded: false,
				paths: [],
				addingPath: false,
				newPath: "",
				removingPath: null,
				loadingPaths: false,
			};
		}
	}

	async function loadPathsForLib(libUid: string) {
		ensureLibState(libUid);
		libStates[libUid].loadingPaths = true;
		try {
			libStates[libUid].paths = await invoke<{ id: number; path: string }[]>(
				"get_paths_for_library",
				{ libUid }
			);
		} catch {
			libStates[libUid].paths = [];
		} finally {
			libStates[libUid].loadingPaths = false;
		}
	}

	async function toggleExpanded(libUid: string) {
		ensureLibState(libUid);
		const s = libStates[libUid];
		s.expanded = !s.expanded;
		if (s.expanded && s.paths.length === 0 && !s.loadingPaths) {
			await loadPathsForLib(libUid);
		}
	}

	async function browsePath(libUid: string) {
		const selected = await open({ directory: true, multiple: false });
		if (selected) libStates[libUid].newPath = selected as string;
	}

	async function addPath(libUid: string) {
		const s = libStates[libUid];
		if (!s.newPath.trim()) return;
		s.addingPath = true;
		scanState.loading = true;
		scanState.status = "Scanning…";
		scanState.progress = 0;
		scanState.total = 0;
		try {
			lastScannedLibUid = libUid;
			await addPathToLibrary(s.newPath.trim(), libUid);
			s.newPath = "";
			await loadPathsForLib(libUid);
		} catch (e) {
			scanState.status = `Error: ${e}`;
			scanState.loading = false;
		} finally {
			s.addingPath = false;
		}
	}

	async function removePath(libUid: string, path: string) {
		libStates[libUid].removingPath = path;
		await invoke("remove_path", { path: path.trim() });
		await loadPathsForLib(libUid);
		await loadLibrary();
		scanState.status = `Removed ${path}`;
		libStates[libUid].removingPath = null;
	}

	async function rescanPath(libUid: string, path: string) {
		libStates[libUid].addingPath = true;
		scanState.loading = true;
		scanState.status = "Scanning…";
		scanState.progress = 0;
		scanState.total = 0;
		lastScannedLibUid = libUid;
		try {
			await invoke("add_path", { path, libUid });
		} catch (e) {
			scanState.status = `Error: ${e}`;
			scanState.loading = false;
		} finally {
			libStates[libUid].addingPath = false;
		}
	}

	// ─── New local library dialog ─────────────────────────────────────────────────

	let newLibOpen = $state(false);
	let newLibName = $state("");
	let creatingLib = $state(false);
	let newLibError = $state("");

	// ─── Import dialog ────────────────────────────────────────────────────────────

	let importOpen = $state(false);
	let importName = $state("");
	let importUrl = $state("");
	let importMetaUrl = $state("");
	let importToken = $state("");
	let importing = $state(false);
	let importError = $state("");

	// ─── Per-library action states ────────────────────────────────────────────────

	let syncingUid = $state<string | null>(null);
	let pushingUid = $state<string | null>(null);
	let checkingUid = $state<string | null>(null);
	let rebuildingMerged = $state(false);

	onMount(async () => {
		if (!libraryStore.loaded) await loadLibraryRegistry();

		// Auto-expand the default library on first load
		const def = libraryStore.libraries.find((l) => l.is_default);
		if (def) {
			ensureLibState(def.uid);
			libStates[def.uid].expanded = true;
			await loadPathsForLib(def.uid);
		}

		await listen("scan:progress", async (event: any) => {
			scanState.loading = true;
			scanState.progress = event.payload.scanned;
			scanState.total = event.payload.total;
			scanState.status = `Scanning… ${scanState.progress} / ${scanState.total}`;
		});

		await listen("scan:done", async () => {
			scanState.status = "Scan done.";
			scanState.loading = false;
			scanState.progress = 0;
			scanState.total = 0;
		});

		await listen("library:updated", async () => {
			await loadLibrary();
			if (lastScannedLibUid) {
				await loadPathsForLib(lastScannedLibUid);
				lastScannedLibUid = null;
			}
		});

		await listen("scan:error", (event: any) => {
			scanState.status = `Scan error: ${event.payload}`;
			scanState.loading = false;
		});
	});

	async function rescan() {
		scanState.loading = true;
		scanState.status = "Rescanning…";
		scanState.progress = 0;
		scanState.total = 0;
		await invoke("rescan");
	}

	async function handleCreateLibrary() {
		if (!newLibName.trim()) return;
		creatingLib = true;
		newLibError = "";
		try {
			const created = await createLibrary(newLibName.trim());
			await loadLibraryRegistry();
			newLibOpen = false;
			newLibName = "";
			ensureLibState(created.uid);
			libStates[created.uid].expanded = true;
		} catch (e) {
			newLibError = String(e);
		} finally {
			creatingLib = false;
		}
	}

	async function handleSync(libUid: string) {
		syncingUid = libUid;
		try {
			await syncLibrary(libUid);
			await loadLibraryRegistry();
		} catch (e) {
			toast.error("Could not reach remote library", {
				description: String(e),
			});
		} finally {
			syncingUid = null;
		}
	}

	async function handlePush(libUid: string) {
		pushingUid = libUid;
		try {
			await pushLibrary(libUid);
		} finally {
			pushingUid = null;
		}
	}

	async function handleCheckPermission(libUid: string) {
		checkingUid = libUid;
		try {
			await checkWritePermission(libUid);
			await loadLibraryRegistry();
		} finally {
			checkingUid = null;
		}
	}

	async function handleExport(libUid: string) {
		await exportLibrary(libUid);
	}

	async function handleDelete(libUid: string) {
		await deleteLibrary(libUid, false);
		await loadLibraryRegistry();
		await loadLibrary();
	}

	async function handleRebuildMerged() {
		rebuildingMerged = true;
		try {
			await rebuildMerged();
		} finally {
			rebuildingMerged = false;
		}
	}

	async function handleImport() {
		if (!importName.trim() || !importUrl.trim()) return;
		importing = true;
		importError = "";
		try {
			await importLibrary(
				importName.trim(),
				importUrl.trim(),
				importToken.trim() || undefined
			);
			await loadLibraryRegistry();
			importOpen = false;
			importName = "";
			importUrl = "";
			importMetaUrl = "";
			importToken = "";
			toast.success("Library imported successfully");
		} catch (e) {
			toast.error("Could not import library", {
				description: String(e),
			});
			importError = String(e);
		} finally {
			importing = false;
		}
	}

	const sortedLibraries = $derived(
		[...libraryStore.libraries].sort((a, b) => {
			if (a.is_default && !b.is_default) return 1;
			if (!a.is_default && b.is_default) return -1;
			return (a.sort_order ?? 0) - (b.sort_order ?? 0);
		})
	);

	$effect(() => {
		for (const lib of sortedLibraries) {
			ensureLibState(lib.uid);
		}
	});
</script>

<!-- New local library dialog -->
<Dialog.Root bind:open={newLibOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>New Local Library</Dialog.Title>
			<Dialog.Description>
				Create a new local library database. You can add music folders to it after creation.
			</Dialog.Description>
		</Dialog.Header>
		<div class="flex flex-col gap-3 py-2">
			<div class="flex flex-col gap-1">
				<label class="text-xs font-medium text-muted-foreground">Library name</label>
				<input
					bind:value={newLibName}
					placeholder="e.g. Work Music, Classical Collection…"
					class="border rounded px-3 py-2 text-sm bg-background"
					onkeydown={(e) => e.key === "Enter" && handleCreateLibrary()}
				/>
			</div>
			{#if newLibError}
				<p class="text-xs text-destructive flex items-center gap-1">
					<AlertCircle class="w-3 h-3" /> {newLibError}
				</p>
			{/if}
		</div>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => { newLibOpen = false; newLibName = ""; newLibError = ""; }}>Cancel</Button>
			<Button onclick={handleCreateLibrary} disabled={creatingLib || !newLibName.trim()}>
				{#if creatingLib}
					<Loader2 class="w-4 h-4 mr-1 animate-spin" />
				{:else}
					<DatabaseZap class="w-4 h-4 mr-1" />
				{/if}
				Create
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Import remote dialog -->
<Dialog.Root bind:open={importOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Import Remote Library</Dialog.Title>
			<Dialog.Description>
				Connect to a remotely hosted Sonodex library. The host must publish a <code>.db</code> file and a sidecar <code>.json</code> file.
			</Dialog.Description>
		</Dialog.Header>
		<div class="flex flex-col gap-3 py-2">
			<div class="flex flex-col gap-1">
				<label class="text-xs font-medium text-muted-foreground">Display name</label>
				<input bind:value={importName} placeholder="My Friend's Library" class="border rounded px-3 py-2 text-sm bg-background" />
			</div>
			<div class="flex flex-col gap-1">
				<label class="text-xs font-medium text-muted-foreground">Database URL (.db)</label>
				<input bind:value={importUrl} placeholder="https://example.com/library.db" class="border rounded px-3 py-2 text-sm bg-background" />
			</div>
			{#if importUrl.startsWith('http://') || importUrl.startsWith('https://')}
				<div class="flex flex-col gap-1">
					<label class="text-xs font-medium text-muted-foreground">Write token (optional)</label>
					<input bind:value={importToken} type="password" placeholder="Leave blank for read-only" class="border rounded px-3 py-2 text-sm bg-background" />
				</div>
			{/if}
			{#if importError}
				<p class="text-xs text-destructive flex items-center gap-1">
					<AlertCircle class="w-3 h-3" /> {importError}
				</p>
			{/if}
		</div>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (importOpen = false)}>Cancel</Button>
			<Button onclick={handleImport} disabled={importing || !importName.trim() || !importUrl.trim()}>
				{#if importing}<Loader2 class="w-4 h-4 mr-1 animate-spin" />{/if}
				Import
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<div class="flex flex-col gap-4">

	<!-- Header row -->
	<div class="flex flex-col gap-1">
		<h4 class="text-sm font-semibold pt-2">Connected Libraries</h4>
		<p class="text-xs text-muted-foreground mt-0.5">
			Libraries are merged in order, the local default always wins conflicts.
		</p>

		<div class="flex gap-2 pt-2">
			<Button variant="outline" size="sm" onclick={rescan} disabled={scanState.loading}>
				{#if scanState.loading}
					<Loader2 class="w-3.5 h-3.5 mr-1 animate-spin" />
				{:else}
					<RefreshCw class="w-3.5 h-3.5 mr-1" />
				{/if}
				Rescan All
			</Button>
			<Tooltip.Root>
				<Tooltip.Trigger
					class={buttonVariants({ variant: "outline", size: "sm" })}
					onclick={handleRebuildMerged}
					disabled={rebuildingMerged}
				>
					{#if rebuildingMerged}
						<Loader2 class="w-3.5 h-3.5 mr-1 animate-spin" />
					{:else}
						<RefreshCw class="w-3.5 h-3.5 mr-1" />
					{/if}
					Rebuild Cache
				</Tooltip.Trigger>
				<Tooltip.Content><p>Force a full rebuild of the merged read cache</p></Tooltip.Content>
			</Tooltip.Root>
			<Button variant="outline" size="sm" onclick={() => (newLibOpen = true)}>
				<DatabaseZap class="w-3.5 h-3.5 mr-1" /> New Library
			</Button>
			<Button size="sm" onclick={() => (importOpen = true)}>
				<Link class="w-3.5 h-3.5 mr-1" /> Import Remote
			</Button>
		</div>
	</div>

	<!-- Library list -->
	{#if !libraryStore.loaded}
		<p class="text-sm text-muted-foreground">Loading…</p>
	{:else if sortedLibraries.length === 0}
		<p class="text-sm text-muted-foreground">No libraries connected.</p>
	{:else}
		<div class="flex flex-col gap-2">
			{#each sortedLibraries as lib (lib.uid)}
				{@const s = libStates[lib.uid]}
				{#if s}
				<div class="border rounded-md overflow-hidden">

					<!-- Library header row -->
					<button
						class="w-full flex items-center gap-3 px-3 py-2.5 text-sm text-left hover:bg-muted/40 transition-colors"
						onclick={() => toggleExpanded(lib.uid)}
					>
						<!-- Chevron -->
						<span class="shrink-0 text-muted-foreground">
							{#if s.expanded}
								<ChevronUp class="w-4 h-4" />
							{:else}
								<ChevronDown class="w-4 h-4" />
							{/if}
						</span>

						<!-- Type icon -->
						<span class="shrink-0 text-muted-foreground">
							{#if lib.sync_url}
								<Globe class="w-4 h-4" />
							{:else}
								<HardDrive class="w-4 h-4" />
							{/if}
						</span>

						<!-- Name + path -->
						<span class="flex-1 min-w-0">
							<span class="flex items-center gap-2">
								<span class="font-medium truncate">{lib.name}</span>
								{#if lib.is_default}
									<span class="text-[10px] px-1.5 py-0.5 rounded font-medium bg-secondary text-secondary-foreground">
										default
									</span>
								{/if}
								{#if lib.has_write_permission}
									<Unlock class="w-3 h-3 text-muted-foreground" />
								{:else if lib.sync_url}
									<Lock class="w-3 h-3 text-muted-foreground" />
								{/if}
							</span>
							<span class="block text-xs text-muted-foreground truncate mt-0.5 font-mono">
								{lib.sync_url ?? lib.file_path}
							</span>
						</span>

						<!-- Action buttons — stop propagation so they don't toggle expand -->
						<span class="flex items-center gap-1 shrink-0" onclick={(e) => e.stopPropagation()} role="none">
							{#if lib.sync_url}
								<Tooltip.Root>
									<Tooltip.Trigger
										class={buttonVariants({ variant: "ghost", size: "sm" })}
										onclick={() => handleSync(lib.uid)}
										disabled={syncingUid === lib.uid}
									>
										{#if syncingUid === lib.uid}
											<Loader2 class="w-3.5 h-3.5 animate-spin" />
										{:else}
											<Download class="w-3.5 h-3.5" />
										{/if}
									</Tooltip.Trigger>
									<Tooltip.Content><p>Pull latest from remote</p></Tooltip.Content>
								</Tooltip.Root>

								{#if lib.has_write_permission}
									<Tooltip.Root>
										<Tooltip.Trigger
											class={buttonVariants({ variant: "ghost", size: "sm" })}
											onclick={() => handlePush(lib.uid)}
											disabled={pushingUid === lib.uid}
										>
											{#if pushingUid === lib.uid}
												<Loader2 class="w-3.5 h-3.5 animate-spin" />
											{:else}
												<Upload class="w-3.5 h-3.5" />
											{/if}
										</Tooltip.Trigger>
										<Tooltip.Content><p>Push local changes to remote</p></Tooltip.Content>
									</Tooltip.Root>
								{/if}
							{/if}

							<DropdownMenu.Root>
								<DropdownMenu.Trigger class={buttonVariants({ variant: "ghost", size: "sm" })}>
									<MoreHorizontal class="w-3.5 h-3.5" />
								</DropdownMenu.Trigger>
								<DropdownMenu.Content align="end">
									{#if lib.sync_url}
										<DropdownMenu.Item onclick={() => handleCheckPermission(lib.uid)}>
											{#if checkingUid === lib.uid}
												<Loader2 class="w-3.5 h-3.5 mr-2 animate-spin" />
											{:else}
												<CheckCircle2 class="w-3.5 h-3.5 mr-2" />
											{/if}
											Check Write Permission
										</DropdownMenu.Item>
									{/if}
									{#if !lib.sync_url}
									<DropdownMenu.Item onclick={() => handleExport(lib.uid)}>
										<Download class="w-3.5 h-3.5 mr-2" /> Export as File
									</DropdownMenu.Item>
									{/if}
									{#if !lib.is_default}
										<DropdownMenu.Separator />
										<DropdownMenu.Item
											class="text-destructive focus:text-destructive"
											onclick={() => handleDelete(lib.uid)}
										>
											<Trash2 class="w-3.5 h-3.5 mr-2" /> Remove Library
										</DropdownMenu.Item>
									{/if}
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</span>
					</button>

					<!-- Expanded: paths + add row -->
					{#if s.expanded}
						<div class="border-t bg-muted/30 px-4 py-3 flex flex-col gap-2">

							{#if s.loadingPaths}
								<div class="flex items-center gap-2 text-xs text-muted-foreground py-1">
									<Loader2 class="w-3 h-3 animate-spin" /> Loading paths…
								</div>
							{:else if s.paths.length === 0}
								<p class="text-xs text-muted-foreground py-1">No folders added to this library yet.</p>
							{:else}
								<div class="flex flex-col gap-1">
									{#each s.paths as p (p.id)}
										<div class="flex items-center gap-2 rounded px-2 py-1.5 text-xs bg-background border">
											<FolderOpen class="w-3.5 h-3.5 shrink-0 text-muted-foreground" />
											<span class="truncate flex-1 font-mono">{p.path}</span>
											<Tooltip.Root>
												<Tooltip.Trigger
													class={buttonVariants({ variant: "ghost", size: "sm" })}
													style="height:1.5rem;width:1.5rem;padding:0;"
													disabled={scanState.loading || s.addingPath}
													onclick={() => rescanPath(lib.uid, p.path)}
												>
													{#if s.addingPath}
														<Loader2 class="w-3 h-3 animate-spin" />
													{:else}
														<RefreshCw class="w-3 h-3" />
													{/if}
												</Tooltip.Trigger>
												<Tooltip.Content><p>Rescan this folder</p></Tooltip.Content>
											</Tooltip.Root>
											<Button
												variant="ghost"
												size="sm"
												class="h-6 w-6 p-0 text-muted-foreground hover:text-destructive shrink-0"
												disabled={scanState.loading || s.removingPath === p.path}
												onclick={() => removePath(lib.uid, p.path)}
											>
												{#if s.removingPath === p.path}
													<Loader2 class="w-3 h-3 animate-spin" />
												{:else}
													<Trash2 class="w-3 h-3" />
												{/if}
											</Button>
										</div>
									{/each}
								</div>
							{/if}

							<!-- Add path -->
							<div class="flex gap-2 pt-1">
								<input
									bind:value={s.newPath}
									placeholder="Folder path…"
									class="flex-1 border rounded px-3 py-1.5 text-xs bg-background"
									onkeydown={(e) => e.key === "Enter" && addPath(lib.uid)}
								/>
								<Button
									variant="outline"
									size="sm"
									class="h-8 text-xs"
									onclick={() => browsePath(lib.uid)}
								>
									<FolderOpen class="w-3.5 h-3.5 mr-1" /> Browse
								</Button>
								<Button
									size="sm"
									class="h-8 text-xs"
									disabled={scanState.loading || s.addingPath || !s.newPath.trim()}
									onclick={() => addPath(lib.uid)}
								>
									{#if s.addingPath}
										<Loader2 class="w-3.5 h-3.5 animate-spin" />
									{:else}
										<FolderPlus class="w-3.5 h-3.5 mr-1" />
									{/if}
									Add & Scan
								</Button>
							</div>

							{#if lib.last_synced}
								<p class="text-[11px] text-muted-foreground">
									Last synced {new Date(lib.last_synced * 1000).toLocaleString()}
								</p>
							{/if}
						</div>
					{/if}
				</div>
			
				{/if}
			{/each}
		</div>
	{/if}
</div>