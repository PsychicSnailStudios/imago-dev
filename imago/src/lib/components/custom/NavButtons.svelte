<script lang="ts">
	import { CircleArrowLeft, CircleArrowRight, CircleX } from "lucide-svelte";
   import Button, { type ButtonProps } from "$shadcn/button/button.svelte";
	
   import { canGoBack, canGoForward, clearSelection, moveSelection } from "$ts/store/session.svelte";

	let { class: className }: ButtonProps = $props();
</script>

<div class="{className} flex justify-between">
	<div class="flex gap-1">
		<Button
			variant="ghost"
			size="icon-sm"
			class="rounded-full"
			disabled={!canGoBack()}
			onclick={() => moveSelection(-1)}>
			<CircleArrowLeft class="w-4 h-4"/>
		</Button>
		<Button
			variant="ghost"
			size="icon-sm"
			class="rounded-full"
			disabled={!canGoForward()}
			onclick={() => moveSelection(1)}>
			<CircleArrowRight class="w-4 h-4"/>
		</Button>
	</div>
	<Button
		variant="ghost"
		size="icon-sm"
		class="rounded-full"
		onclick={() => clearSelection()}>
		<CircleX class="w-4 h-4"/>
	</Button>
</div>