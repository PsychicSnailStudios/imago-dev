<script lang="ts">
  import { onMount } from "svelte";
  import { saveWindowState, restoreStateCurrent, StateFlags } from "@tauri-apps/plugin-window-state";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  
  import './layout.css';
  import favicon from '$lib/assets/favicon.svg';

  import TitleBar from '$lib/components/app/title-bar/TitleBar.svelte';
  import EditModal from '$lib/components/dialogs/edit-metadata/EditModal.svelte';

	import * as Tooltip from "$shadcn/tooltip/index.js";
  import { Toaster } from "$shadcn/sonner/index.js";
  import { ModeWatcher } from "mode-watcher";

  import WarningDialog from "$lib/components/dialogs/WarnDialog.svelte";
	import { dialogState, confirmDialog, cancelDialog } from "$ts/ui/dialogManager.svelte";

  import circleLoader from '$lib/assets/circle-loader.json';
  import wavLoader from '$lib/assets/wav-loader.json';
  import loading from '$lib/assets/loading.json';

  let { children } = $props();
  let appLoading = $state(true);
  let loadingFading = $state(false);
  let isDark = $state(document.documentElement.classList.contains('dark') || document.documentElement.getAttribute('data-theme') === 'dark');

  onMount(async () => {
    await restoreStateCurrent(StateFlags.ALL);

    const appWindow = getCurrentWindow();
    await appWindow.onCloseRequested(async () => {
      await saveWindowState(StateFlags.ALL);
    });

    const minDelay = new Promise(res => setTimeout(res, 600));
    await minDelay;

    loadingFading = true;
    setTimeout(() => {
      appLoading = false;
    }, 500);
  })
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

{#if appLoading}
	<div
		class="fixed inset-0 z-[9999] flex flex-col items-center justify-center bg-background transition-opacity duration-500"
		style="opacity: {loadingFading ? 0 : 1}; pointer-events: {loadingFading ? 'none' : 'all'};"
	>
  <lottie-player
    src={loading}
    background="transparent"
    speed="1"
    style="width: 160px; height: 160px; filter: {isDark ? 'invert(1)' : 'none'};"
    loop
    autoplay
  ></lottie-player>
	</div>
{/if}

<div class="flex h-screen flex-col overflow-hidden main-app-bg">
	<ModeWatcher />
	<TitleBar />
	<Tooltip.Provider>
		<main class="h-screen overflow-hidden">
			{@render children()}
		</main>
	</Tooltip.Provider>
	<EditModal />
	<Toaster position="top-center" />
	<WarningDialog bind:open={dialogState.open} title={dialogState.title} description={dialogState.description} onconfirm={confirmDialog} oncancel={cancelDialog} />
</div>

<style>
	.main-app-bg {
		background: radial-gradient(circle, oklch(from var(--background) l c h / 99%) 0%, oklch(from var(--background) l c h / 96%) 100%);
		backdrop-filter: blur(100px);
	}

  :global(html[data-theme="dark"]) {
    --lottie-filter: invert(1);
  }
  :global(html[data-theme="light"]) {
    --lottie-filter: none;
  }
</style>
