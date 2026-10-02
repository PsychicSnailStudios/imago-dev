<script lang="ts">
	import { Trash, Trash2, Upload, Plus, Pencil, Check, X, ChevronDown } from "lucide-svelte";
	import { Button, buttonVariants } from "$shadcn/button/index.js";
	import { tagStore } from "$ts/store/tagManager.svelte";
	import { showWarning } from "$ts/ui/dialogManager.svelte";
	import type { Tag, TagKind } from "$ts/store/tagManager.svelte";

	type Tab = "tags" | "genres";
	let activeTab = $state<Tab>("tags");

	let newName = $state("");
	let editingUid = $state<string | null>(null);
	let editingName = $state("");
	let importing = $state(false);
	let importError = $state("");

	let tagsOpen = $state(true);
	let groupsOpen = $state(true);

	const kind = $derived<TagKind>(activeTab === "tags" ? "tag" : "genre");
	const items = $derived(activeTab === "tags" ? tagStore.tags : tagStore.genres);
	const visibleGroups = $derived(tagStore.groups.filter((g) => g.kind === kind));

	async function addItem() {
		const name = newName.trim();
		if (!name) return;
		if (activeTab === "tags") await tagStore.addTag(name);
		else await tagStore.addGenre(name);
		newName = "";
	}

	function startEdit(tag: Tag) {
		editingUid = tag.uid;
		editingName = tag.name;
	}

	async function commitEdit() {
		if (!editingUid || !editingName.trim()) { cancelEdit(); return; }
		await tagStore.renameTag(editingUid, editingName.trim());
		cancelEdit();
	}

	function cancelEdit() {
		editingUid = null;
		editingName = "";
	}

	async function deleteItem(tag: Tag) {
		const confirmed = await showWarning({
			title: `Delete "${tag.name}"?`,
			description: "This will remove the tag from all tracks and cannot be undone.",
		});
		if (!confirmed) return;
		await tagStore.deleteTag(tag.uid);
	}

	async function deleteGroup(uid: string, name: string) {
		const confirmed = await showWarning({
			title: `Delete group "${name}"?`,
			description: "The tags inside will not be deleted, only the group.",
		});
		if (!confirmed) return;
		await tagStore.deleteGroup(uid);
	}

	async function deleteAll() {
		if (items.length === 0) return;
		const confirmed = await showWarning({
			title: `Delete all ${activeTab}?`,
			description: `This will permanently delete all ${items.length} ${activeTab} and cannot be undone.`,
		});
		if (!confirmed) return;
		for (const item of items) {
			await tagStore.deleteTag(item.uid);
		}
	}

  async function deleteAllGroups() {
    if (visibleGroups.length === 0) return;
    const confirmed = await showWarning({
      title: `Delete all groups?`,
      description: `This will permanently delete all ${visibleGroups.length} groups. The tags inside will not be deleted.`,
    });
    if (!confirmed) return;
    for (const group of visibleGroups) {
      await tagStore.deleteGroup(group.uid);
    }
  }

	// ─── CSV Import ───────────────────────────────────────────────────────────────

	async function handleCsvImport(event: Event) {
		const input = event.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		input.value = "";
		importing = true;
		importError = "";
		try {
			const text = await file.text();
			await importFromCsv(text, kind);
		} catch (e: any) {
			importError = e?.message ?? String(e);
		} finally {
			importing = false;
		}
	}

	async function importFromCsv(csvText: string, kind: TagKind): Promise<void> {
		const lines = csvText.split(/\r?\n/).filter((l) => l.trim());
		if (lines.length === 0) return;

		const parseRow = (line: string): string[] =>
			line.split(",").map((c) => c.trim().replace(/^"|"$/g, "").trim());

		const headerRow = parseRow(lines[0]);
		const firstColumnEmpty = !headerRow[0];

		if (firstColumnEmpty) {
			const allNames = new Set<string>();
			for (let r = 1; r < lines.length; r++) {
				for (const cell of parseRow(lines[r])) {
					if (cell) allNames.add(cell);
				}
			}
			for (const name of allNames) {
				await tagStore.ensureTag(name, kind);
			}
			return;
		}

		const groups: Record<string, string[]> = {};
		for (const groupName of headerRow) {
			if (groupName) groups[groupName] = [];
		}

		for (let r = 1; r < lines.length; r++) {
			const cells = parseRow(lines[r]);
			for (let c = 0; c < headerRow.length; c++) {
				const groupName = headerRow[c];
				const value = cells[c];
				if (groupName && value) groups[groupName].push(value);
			}
		}

		const allTagNames = [...new Set(Object.values(groups).flat())];
		for (const name of allTagNames) {
			await tagStore.ensureTag(name, kind);
		}

		await tagStore.load();

		for (const [groupName, memberNames] of Object.entries(groups)) {
			const memberUids = memberNames
				.map((n) => tagStore.byName.get(n.toLowerCase())?.uid)
				.filter((uid): uid is string => !!uid);

			const existingGroup = tagStore.groups.find(
				(g) => g.name.toLowerCase() === groupName.toLowerCase() && g.kind === kind
			);

			if (existingGroup) {
				const merged = [...new Set([...existingGroup.member_uids, ...memberUids])];
				await tagStore.updateGroup(existingGroup.uid, { memberUids: merged });
			} else if (memberUids.length > 0) {
				await tagStore.createGroup(groupName, kind, memberUids);
			}
		}
	}
</script>

<div class="flex flex-col gap-3 pt-2">
	<!-- Tab switcher -->
	<div class="flex rounded-md border text-xs overflow-hidden w-fit">
		<button
			class="px-3 py-1.5 transition-colors"
			class:bg-primary={activeTab === "tags"}
			class:text-primary-foreground={activeTab === "tags"}
			class:text-muted-foreground={activeTab !== "tags"}
			onclick={() => (activeTab = "tags")}
		>
			Tags
		</button>
		<button
			class="px-3 py-1.5 transition-colors"
			class:bg-primary={activeTab === "genres"}
			class:text-primary-foreground={activeTab === "genres"}
			class:text-muted-foreground={activeTab !== "genres"}
			onclick={() => (activeTab = "genres")}
		>
			Genres
		</button>
	</div>

	<!-- Add row -->
	<div class="flex gap-2">
		<input
			bind:value={newName}
			placeholder="New {activeTab === 'tags' ? 'tag' : 'genre'}…"
			class="flex-1 border rounded px-3 py-1.5 text-sm bg-background"
			onkeydown={(e) => { if (e.key === "Enter") addItem(); }}
		/>
		<Button size="sm" onclick={addItem} disabled={!newName.trim()}>
			<Plus class="w-4 h-4 mr-1" /> Add
		</Button>
	</div>

	<!-- Tags accordion -->
	<div class="border rounded-md overflow-hidden">
		<button
			class="flex items-center justify-between w-full px-3 py-2 text-sm font-medium bg-muted/40 hover:bg-muted/60 transition-colors"
			onclick={() => (tagsOpen = !tagsOpen)}
		>
			<span>
				{activeTab === "tags" ? "Tags" : "Genres"}
				<span class="text-muted-foreground font-normal">({items.length})</span>
			</span>
			<div class="flex items-center gap-2" role="none" onclick={(e) => e.stopPropagation()}>
				<label class={buttonVariants({ variant: "outline", size: "sm" }) + " cursor-pointer h-6 text-xs px-2"}>
					<Upload class="w-3 h-3 mr-1" />
					{importing ? "Importing…" : "Import CSV"}
					<input
						type="file"
						accept=".csv"
						class="hidden"
						disabled={importing}
						onchange={handleCsvImport}
					/>
				</label>
				{#if items.length > 0}
					<!-- svelte-ignore node_invalid_placement_ssr -->
					<button
						class={buttonVariants({ variant: "destructive", size: "sm" }) + " h-6 text-xs px-2"}
						onclick={deleteAll}
					>
						<Trash2 class="w-3 h-3 mr-1" /> Delete All
					</button>
				{/if}
				<ChevronDown
					class="w-4 h-4 text-muted-foreground transition-transform duration-200"
					style="transform: rotate({tagsOpen ? '180deg' : '0deg'})"
				/>
			</div>
		</button>

		{#if tagsOpen}
			<div class="flex flex-col divide-y">
				{#if importError}
					<p class="text-xs text-destructive px-3 py-2">{importError}</p>
				{/if}
				{#each items as tag (tag.uid)}
					<div class="flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-muted/20">
						{#if tag.color}
							<span class="w-2.5 h-2.5 rounded-full shrink-0" style="background:{tag.color}"></span>
						{/if}
						{#if editingUid === tag.uid}
							<input
								class="flex-1 bg-transparent outline-none text-sm"
								bind:value={editingName}
								onkeydown={(e) => {
									if (e.key === "Enter") commitEdit();
									if (e.key === "Escape") cancelEdit();
								}}
								autofocus
							/>
							<button onclick={commitEdit} class="text-primary hover:opacity-70 p-0.5">
								<Check class="w-3.5 h-3.5" />
							</button>
							<button onclick={cancelEdit} class="text-muted-foreground hover:opacity-70 p-0.5">
								<X class="w-3.5 h-3.5" />
							</button>
						{:else}
							<span class="flex-1 truncate">{tag.name}</span>
							<button onclick={() => startEdit(tag)} class="text-muted-foreground hover:text-foreground p-0.5">
								<Pencil class="w-3.5 h-3.5" />
							</button>
							<button onclick={() => deleteItem(tag)} class="text-muted-foreground hover:text-destructive p-0.5">
								<Trash class="w-3.5 h-3.5" />
							</button>
						{/if}
					</div>
				{:else}
					<p class="text-sm text-muted-foreground px-3 py-2">No {activeTab} yet.</p>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Groups accordion -->
	<div class="border rounded-md overflow-hidden">
		<button
      class="flex items-center justify-between w-full px-3 py-2 text-sm font-medium bg-muted/40 hover:bg-muted/60 transition-colors"
      onclick={() => (groupsOpen = !groupsOpen)}
    >
      <span>
        Groups
        <span class="text-muted-foreground font-normal">({visibleGroups.length})</span>
      </span>
      <div class="flex items-center gap-2" role="none" onclick={(e) => e.stopPropagation()}>
        {#if visibleGroups.length > 0}
          <!-- svelte-ignore node_invalid_placement_ssr -->
          <button
            class={buttonVariants({ variant: "destructive", size: "sm" }) + " h-6 text-xs px-2"}
            onclick={deleteAllGroups}
          >
            <Trash2 class="w-3 h-3 mr-1" /> Delete All
          </button>
        {/if}
        <ChevronDown
          class="w-4 h-4 text-muted-foreground transition-transform duration-200"
          style="transform: rotate({groupsOpen ? '180deg' : '0deg'})"
        />
      </div>
    </button>

		{#if groupsOpen}
			<div class="flex flex-col divide-y">
				{#each visibleGroups as group (group.uid)}
					<div class="flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-muted/20">
						{#if group.color}
							<span class="w-2.5 h-2.5 rounded-full shrink-0" style="background:{group.color}"></span>
						{/if}
						<span class="flex-1 truncate">{group.name}</span>
						<span class="text-xs text-muted-foreground shrink-0">{group.member_uids.length} members</span>
						<button
							onclick={() => deleteGroup(group.uid, group.name)}
							class="text-muted-foreground hover:text-destructive p-0.5"
						>
							<Trash class="w-3.5 h-3.5" />
						</button>
					</div>
				{:else}
					<p class="text-sm text-muted-foreground px-3 py-2">No groups yet.</p>
				{/each}
			</div>
		{/if}
	</div>
</div>