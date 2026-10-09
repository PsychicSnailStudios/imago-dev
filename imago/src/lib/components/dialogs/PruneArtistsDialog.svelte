<script lang="ts">
	import * as Dialog from "$shadcn/dialog/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { Checkbox } from "$shadcn/checkbox/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { pruneDialogState, resolvePrune } from "$ts/ui/pruneDialog.svelte";

	let selected = $state<Record<string, boolean>>({});

	$effect(() => {
		if (pruneDialogState.open) {
			const next: Record<string, boolean> = {};
			for (const name of pruneDialogState.names) next[name] = true;
			selected = next;
		}
	});

	const chosen = $derived(pruneDialogState.names.filter((n) => selected[n]));
	const allChosen = $derived(chosen.length === pruneDialogState.names.length);

	function toggleAll(value: boolean) {
		const next: Record<string, boolean> = {};
		for (const name of pruneDialogState.names) next[name] = value;
		selected = next;
	}

	function handleOpenChange(v: boolean) {
		if (!v && pruneDialogState.open) resolvePrune(null);
	}
</script>

<Dialog.Root open={pruneDialogState.open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Remove unreferenced artists?</Dialog.Title>
			<Dialog.Description>
				These artists are no longer used by any track or album. Choose which ones to delete; unchecked artists are kept.
			</Dialog.Description>
		</Dialog.Header>

		<div class="flex items-center gap-2 text-xs text-muted-foreground">
			<Checkbox checked={allChosen} onCheckedChange={(v) => toggleAll(!!v)} />
			<span>{chosen.length} of {pruneDialogState.names.length} selected</span>
		</div>

		<ScrollArea class="max-h-64 pr-3">
			<div class="flex flex-col gap-1">
				{#each pruneDialogState.names as name (name)}
					<label class="flex items-center gap-2 rounded px-2 py-1.5 text-sm border bg-background cursor-pointer">
						<Checkbox
							checked={selected[name] ?? false}
							onCheckedChange={(v) => (selected = { ...selected, [name]: !!v })}
						/>
						<span class="truncate">{name}</span>
					</label>
				{/each}
			</div>
		</ScrollArea>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => resolvePrune(null)}>Cancel</Button>
			<Button variant="outline" onclick={() => resolvePrune([])}>Keep all</Button>
			<Button variant="destructive" disabled={chosen.length === 0} onclick={() => resolvePrune(chosen)}>
				Delete {chosen.length}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
