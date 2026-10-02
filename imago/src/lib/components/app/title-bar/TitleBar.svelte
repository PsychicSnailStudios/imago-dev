<script lang="ts">

  // APP
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';

	// COMPONENTS
  import { CircleSmall, CircleDashed, Minus, X } from 'lucide-svelte';
  import WindowContext from '$lib/components/app/title-bar/WindowContext.svelte';
  
	// VARIABLES
  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);

	// APP FUNCTIONS
  onMount(async () => {
    isMaximized = await appWindow.isMaximized();

    await appWindow.onResized(async () => {
      isMaximized = await appWindow.isMaximized();
    });
  });

	// FUNCTIONS
  async function startDrag() {
    await appWindow.startDragging();
  }

  async function minimize() {
    await appWindow.minimize();
  }

  async function maximize() {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  }

  async function close() {
    await appWindow.close();
  }

</script>

<div
  class="flex h-9 w-full select-none items-center justify-between bg-background px-2"
  role="presentation"
  tabindex="-1"
  onmousedown={startDrag}>

  <WindowContext />
  <span></span>

  <div class="flex items-center gap-2" role="presentation" onmousedown={(e) => e.stopPropagation()}>
    
    <button
      onclick={minimize}
      class="flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
      aria-label="Minimize"
    >
      <Minus size={16} />
    </button>

    <button
      onclick={maximize}
      class="flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
      aria-label="Maximize"
    >
      {#if isMaximized}
      <CircleDashed size={12} />
      {:else}
      <CircleSmall size={16} />
      {/if}
    </button>

    <button
      onclick={close}
      class="flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
      aria-label="Close"
    >
      <X size={16} />
    </button>

  </div>

</div>