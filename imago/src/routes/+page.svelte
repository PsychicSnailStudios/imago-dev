<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { setExplorePlaylists } from "$ts/config/explore.svelte";

	import * as Resizable from "$shadcn/resizable/index.js";

	import PlayControls from "$lib/components/app/PlayControls.svelte";
	import NowPlaying from "$lib/components/app/NowPlaying.svelte";
	import AppNavigation from "$lib/components/app/AppNavigation.svelte";

	import HomeView from "$lib/components/pages/Home.svelte";
	import SearchView from "$lib/components/pages/LibrarySearch.svelte";
	import PlaylistsView from "$lib/components/pages/PlaylistsLibrary.svelte";
	import TracksView from "$lib/components/pages/TrackLibrary.svelte";
	import AlbumsView from "$lib/components/pages/AlbumLibrary.svelte";
	import ArtistsView from "$lib/components/pages/ArtistLibrary.svelte";
	import Settings from "$lib/components/pages/Settings.svelte";
	import LibraryManager from "$lib/components/pages/LibraryManager.svelte";

	import AlbumDisplay from "$lib/components/views/AlbumDisplay.svelte";
	import ArtistDisplay from "$lib/components/views/ArtistDisplay.svelte";
	import PlaylistDisplay from "$lib/components/views/PlaylistDisplay.svelte";
	import TrackDisplay from "$lib/components/views/TrackDisplay.svelte";

	import ProfileSetup from "$lib/components/dialogs/profile/ProfileSetup.svelte";
	import UpdateDialog from "$lib/components/dialogs/UpdateDialog.svelte";

	import { loadLibrary, reloadLibrary } from "$ts/store/library.svelte";
	import { selection, scanState, activeView, loadSessionState, saveSessionState } from "$ts/store/session.svelte";
	import { dragState } from "$ts/store/drag.svelte";
	import { togglePlay, skipBack, skipNext, loadPlayerState, savePlayerState } from "$ts/audio/audioManager.svelte";
	import { scrobbleSignal } from "$ts/audio/scrobbler.svelte";
	import { resetScrobbleCache } from "$ts/audio/scrobbleCache";
	import { checkForUpdate } from "$lib/updater.svelte";
	import { loadProfiles, profileState } from "$ts/store/profiles.svelte";

	let needsSetup = $state(false);
	let setupChecked = $state(false);

	let update = $state<import("@tauri-apps/plugin-updater").Update | null>(null);
	let showUpdateDialog = $state(false);

	let libraryRefreshTimer: ReturnType<typeof setTimeout> | null = null;

	let containerWidth = $state(0);
	let minViewWidth = $derived(containerWidth ? (375 / containerWidth) * 100 : 15);
	let minSidebarWidth = $derived(containerWidth ? (230 / containerWidth) * 100 : 15);
	let maxSidebarWidth = $derived(containerWidth ? (480 / containerWidth) * 100 : 40);
	let defaultSidebarWidth = $derived(containerWidth ? (300 / containerWidth) * 100 : 40);

	onMount(() => {
		window.addEventListener("beforeunload", handleUnload);
		return () => window.removeEventListener("beforeunload", handleUnload);
	});

	onMount(async () => {
		needsSetup = await invoke<boolean>("needs_profile_setup");
		setupChecked = true;

		const found = await checkForUpdate(true);
		if (found) {
			update = found;
			showUpdateDialog = true;
		}

		if (!needsSetup) {
			await loadProfiles();
			const uid = profileState.active?.uid;
			loadPlayerState();
			if (uid) loadSessionState(uid);
			await loadLibrary();
			setExplorePlaylists();
		}

		await listen("profile:ready", async () => {
			needsSetup = false;
			await loadProfiles();
			const uid = profileState.active?.uid;
			loadPlayerState();
			if (uid) loadSessionState(uid);
			await loadLibrary();
			setExplorePlaylists();
		});

		await listen("scan:progress", (event: any) => {
			scanState.loading = true;
			scanState.progress = event.payload.scanned;
			scanState.total = event.payload.total;
			scanState.status = `Scanning… ${scanState.progress} / ${scanState.total}`;
		});

		await listen("scan:done", () => {
			scanState.status = "Scan done.";
			scanState.loading = false;
			scanState.progress = 0;
			scanState.total = 0;
		});

		await listen("scan:error", (event: any) => {
			scanState.status = `Scan error: ${event.payload}`;
			scanState.loading = false;
		});

		await listen("enrich:progress", (event: any) => {
			scanState.enriching = true;
			scanState.enrichStage = event.payload.stage ?? "";
			scanState.enrichDone = event.payload.done;
			scanState.enrichTotal = event.payload.total;
			scanState.enrichErrors = event.payload.errors;
		});

		await listen("enrich:done", (event: any) => {
			scanState.enriching = false;
			scanState.enrichErrors = event.payload.errors;
			scanState.status = `Enrichment done. ${event.payload.total - event.payload.errors} updated, ${event.payload.errors} not found.`;
		});

		await listen("scrobbles:relinked", () => {
			resetScrobbleCache();
			scrobbleSignal.version++;
		});

		await listen("library:updated", () => {
			if (libraryRefreshTimer) clearTimeout(libraryRefreshTimer);
			libraryRefreshTimer = setTimeout(() => {
				libraryRefreshTimer = null;
				reloadLibrary("all");
				reloadLibrary("tags");
			}, 500);
		});

		window.addEventListener("keydown", keydown);
	});

	function handleUnload() {
		const uid = profileState.active?.uid;
		if (uid) saveSessionState(uid);
		savePlayerState();
	}

	function onSetupComplete() {
		needsSetup = false;
		loadLibrary();
	}

	function keydown(e: KeyboardEvent) {
		switch (e.key) {
			case "F5": location.reload(); break;
			case "MediaPlayPause": togglePlay(); break;
			case "MediaTrackNext": skipNext(); break;
			case "MediaTrackPrevious": skipBack(); break;
		}
	}

	function handleWindowDragOver(e: DragEvent) {
		if (dragState.active) e.preventDefault();
	}
</script>

<svelte:window ondragover={handleWindowDragOver} />

{#if setupChecked && needsSetup}
	<ProfileSetup onComplete={onSetupComplete} />
{/if}

<UpdateDialog bind:open={showUpdateDialog} {update} />

<div bind:clientWidth={containerWidth} class="h-full w-full overflow-hidden">
<Resizable.PaneGroup direction="horizontal" class="app-wrapper grid gap-0.5 pl-2 pr-2 pb-2 overflow-hidden">

	<Resizable.Pane defaultSize={defaultSidebarWidth} minSize={minSidebarWidth} maxSize={maxSidebarWidth} class="grid grid-rows-[minmax(0,1fr)] min-h-0">
		<div class="app-sidebar gap-1 max-[{maxSidebarWidth}px]">
			<div class="shrink-0">
				{#if scanState.loading && scanState.total > 0}
					<div class="space-y-1 px-2 py-1">
						<div class="w-full bg-muted rounded-full h-2">
							<div
								class="bg-primary h-2 rounded-full transition-all"
								style="width: {Math.round((scanState.progress / scanState.total) * 100)}%"
							></div>
						</div>
						<p class="text-xs text-muted-foreground">{scanState.progress} / {scanState.total} files</p>
					</div>
				{/if}
				{#if scanState.enriching && scanState.enrichTotal > 0}
					<div class="space-y-1 px-2 py-1">
						<div class="w-full bg-muted rounded-full h-2">
							<div
								class="bg-primary h-2 rounded-full transition-all"
								style="width: {Math.round((scanState.enrichDone / scanState.enrichTotal) * 100)}%"
							></div>
						</div>
						<p class="text-xs text-muted-foreground">Enriching {scanState.enrichStage} {scanState.enrichDone} / {scanState.enrichTotal}</p>
					</div>
				{/if}
			</div>

			<AppNavigation />

			<NowPlaying />
		</div>
	</Resizable.Pane>

	<Resizable.Handle class="opacity-0" />

	<Resizable.Pane class="app-body h-full w-full overflow-hidden grid gap-1">
		<div class="app-body h-full w-full overflow-hidden grid gap-1">
			<Resizable.PaneGroup direction="horizontal" class="app-views h-full w-full overflow-hidden flex gap-0.5">
				<Resizable.Pane minSize={minViewWidth}>
					{#if activeView.id === "home"}
						<HomeView />
					{:else if activeView.id === "search"}
						<SearchView />
					{:else if activeView.id === "tracks"}
						<TracksView />
					{:else if activeView.id === "albums"}
						<AlbumsView />
					{:else if activeView.id === "artists"}
						<ArtistsView />
					{:else if activeView.id === "playlists"}
						<PlaylistsView />
					{:else if activeView.id === "settings"}
						<Settings />
					{:else if activeView.id === "manage"}
						<LibraryManager />
					{/if}
				</Resizable.Pane>

				{#if selection.type !== "none"}
					<Resizable.Handle class="opacity-0" />

					<Resizable.Pane defaultSize={50} minSize={minViewWidth}>
						{#if selection.type === "album"}
							<AlbumDisplay />
						{/if}
						{#if selection.type === "track"}
							<TrackDisplay />
						{/if}
						{#if selection.type === "playlist"}
							<PlaylistDisplay />
						{/if}
						{#if selection.type === "artist"}
							<ArtistDisplay />
						{/if}
					</Resizable.Pane>
				{/if}
			</Resizable.PaneGroup>

			<PlayControls />
		</div>
	</Resizable.Pane>

</Resizable.PaneGroup>
</div>

<style>
	.app-wrapper {
		grid-template-columns: minmax(18rem, 25rem) minmax(50%, 85%);
		height: 100%;
	}

	.app-sidebar {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 10rem;
		max-width: 30rem;
	}

	.app-body {
		grid-template-rows: 1fr auto;
		flex-direction: column;
	}
</style>