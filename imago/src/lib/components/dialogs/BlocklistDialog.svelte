<script lang="ts">
	import { ShieldOff, Layers, Square } from "lucide-svelte";
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import { Checkbox } from "$shadcn/checkbox/index.js";
	import { blocklistDialogState, resolveBlocklistDialog } from "$ts/library/libraryRegistry.svelte";

	let remember = $state(false);

	function handleSingle() {
		resolveBlocklistDialog({ cascade: false, remember });
		remember = false;
	}

	function handleCascade() {
		resolveBlocklistDialog({ cascade: true, remember });
		remember = false;
	}

	function handleCancel() {
		resolveBlocklistDialog(null);
		remember = false;
	}

	const entityLabel = $derived(
		blocklistDialogState.entityType === "tracks"
			? "track"
			: blocklistDialogState.entityType === "albums"
				? "album"
				: "artist"
	);
</script>

<AlertDialog.Root bind:open={blocklistDialogState.open}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title class="flex items-center gap-2">
				<ShieldOff class="w-4 h-4 text-muted-foreground" />
				Hide "{blocklistDialogState.entityName}"?
			</AlertDialog.Title>
			<AlertDialog.Description>
				This {entityLabel} is from a read-only library and cannot be permanently deleted.
				Choose how to hide it.
			</AlertDialog.Description>
		</AlertDialog.Header>

		<div class="flex flex-col gap-2 py-2">
			<!-- Single item option -->
			<button
				class="flex items-start gap-3 rounded-md border px-4 py-3 text-left text-sm transition-colors hover:bg-accent"
				onclick={handleSingle}
			>
				<Square class="mt-0.5 w-4 h-4 shrink-0 text-muted-foreground" />
				<div>
					<p class="font-medium">This item only</p>
					<p class="text-xs text-muted-foreground">Hide just this {entityLabel}</p>
				</div>
			</button>

			<!-- Cascade option — only shown when there are related items -->
			{#if blocklistDialogState.hasRelated}
				<button
					class="flex items-start gap-3 rounded-md border px-4 py-3 text-left text-sm transition-colors hover:bg-accent"
					onclick={handleCascade}
				>
					<Layers class="mt-0.5 w-4 h-4 shrink-0 text-muted-foreground" />
					<div>
						<p class="font-medium">This item and all related content</p>
						<p class="text-xs text-muted-foreground">
							Also hide the {entityLabel === "track" ? "albums and artists" : entityLabel === "album" ? "artists" : "albums"} linked to this {entityLabel}
						</p>
					</div>
				</button>
			{/if}
		</div>

		<!-- Remember preference -->
		<label class="flex items-center gap-2 text-sm text-muted-foreground cursor-pointer select-none">
			<Checkbox bind:checked={remember} />
			Remember my choice for this library
		</label>

		<AlertDialog.Footer>
			<AlertDialog.Cancel onclick={handleCancel}>Cancel</AlertDialog.Cancel>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
