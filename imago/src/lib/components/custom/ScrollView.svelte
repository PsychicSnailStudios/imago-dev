<script lang="ts">
	import type { HTMLAttributes } from "svelte/elements";

	let { class: className = "", el = $bindable(null), children, ...rest }: HTMLAttributes<HTMLDivElement> & {
		el?: HTMLDivElement | null;
	} = $props();
</script>

<div bind:this={el} class="scroll-view overflow-y-auto overflow-x-hidden {className}" {...rest}>
	{@render children?.()}
</div>

<style>
	.scroll-view::-webkit-scrollbar {
		width: 10px;
		height: 10px;
	}

	.scroll-view::-webkit-scrollbar-track {
		background: transparent;
	}

	.scroll-view::-webkit-scrollbar-corner {
		background: transparent;
	}

	.scroll-view::-webkit-scrollbar-thumb {
		background-color: var(--border);
		border-radius: 9999px;
		border: 1px solid transparent;
		background-clip: padding-box;
	}

	@supports not selector(::-webkit-scrollbar) {
		.scroll-view {
			scrollbar-width: thin;
			scrollbar-color: var(--border) transparent;
		}
	}
</style>
