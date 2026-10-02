<script lang="ts">
	import { Search, Funnel, FunnelX, FunnelPlus, X } from "lucide-svelte";

	import * as InputGroup from "$shadcn/input-group/index.js";
   import { Button } from "$shadcn/button";

	let { search = $bindable(""), searchCount = $bindable(-1) } = $props<{ search: string; searchCount: number }>();
</script>

<div class="flex w-full max-w-sm items-center gap-2">
	<InputGroup.Root>
		<InputGroup.Input placeholder="Search…" bind:value={search} class="w-48" />
		<InputGroup.Addon>
		<Search />
		</InputGroup.Addon>
		{#if searchCount > 0 && search !== ""}
			<InputGroup.Addon align="inline-end">{searchCount} {searchCount === 1 ? "result" : "results"}</InputGroup.Addon>
		{/if}
		<InputGroup.Addon align="inline-end">
			<X class="cursor-pointer" onclick={() => (search = "")} />
		</InputGroup.Addon>
	</InputGroup.Root>
  <!-- <Button type="submit" variant="outline"><Funnel/></Button> -->
</div>
