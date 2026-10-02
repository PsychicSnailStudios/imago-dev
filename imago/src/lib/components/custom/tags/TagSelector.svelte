<script lang="ts">
	import { onMount, tick } from 'svelte';

	import X from '@lucide/svelte/icons/x';
	import { Badge } from "$shadcn/badge/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";

	import { tagStore } from '$ts/store/tagManager.svelte';
	import type { TagKind } from '$ts/store/tagManager.svelte';

	let {
		value = $bindable<string[]>([]),
		isGenre = false,
		placeholder = 'Add tag...',
		disabled = false,
	}: {
		value?: string[];
		isGenre?: boolean;
		placeholder?: string;
		disabled?: boolean;
	} = $props();

	const kind: TagKind = $derived(isGenre ? 'genre' : 'tag');

	let inputEl = $state<HTMLInputElement | null>(null);
	let containerEl = $state<HTMLDivElement | null>(null);
	let query = $state('');
	let open = $state(false);
	let highlighted = $state(0);

	const suggestions = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const names = tagStore.namesFor(kind);
		const filtered = q ? names.filter(n => n.toLowerCase().includes(q)) : names;
		return filtered.filter(n => !value.includes(n));
	});

	const showCreate = $derived.by(() => {
		const q = query.trim();
		if (!q) return false;
		const exact = tagStore.byName.get(q.toLowerCase());
		if (exact && value.includes(exact.name)) return false;
		if (suggestions.some(s => s.toLowerCase() === q.toLowerCase())) return false;
		return true;
	});

	const listItems = $derived.by(() => {
		const items: Array<{ type: 'existing'; name: string } | { type: 'create'; name: string }> = [
			...suggestions.map(name => ({ type: 'existing' as const, name })),
		];
		if (showCreate) items.push({ type: 'create' as const, name: query.trim() });
		return items;
	});

	$effect(() => {
		if (highlighted >= listItems.length) highlighted = Math.max(0, listItems.length - 1);
	});

	function openDropdown() {
		if (!disabled) open = true;
	}

	function closeDropdown() {
		open = false;
		query = '';
		highlighted = 0;
	}

	async function select(name: string, isNew: boolean) {
		if (isNew) {
			await tagStore.ensureTag(name, kind);
		}
		value = [...value, name];
		query = '';
		highlighted = 0;
		await tick();
		inputEl?.focus();
	}

	function remove(name: string) {
		value = value.filter(t => t !== name);
	}

	function onKeydown(e: KeyboardEvent) {
		if (!open && (e.key === 'ArrowDown' || e.key === 'Enter')) {
			openDropdown();
			return;
		}
		if (!open) return;

		if (e.key === 'ArrowDown') {
			e.preventDefault();
			highlighted = (highlighted + 1) % Math.max(1, listItems.length);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			highlighted = (highlighted - 1 + Math.max(1, listItems.length)) % Math.max(1, listItems.length);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const item = listItems[highlighted];
			if (item) select(item.name, item.type === 'create');
		} else if (e.key === 'Escape') {
			closeDropdown();
		} else if (e.key === 'Backspace' && query === '' && value.length > 0) {
			value = value.slice(0, -1);
		}
	}

	function onDocClick(e: MouseEvent) {
		if (containerEl && !containerEl.contains(e.target as Node)) {
			closeDropdown();
		}
	}

	onMount(() => {
		document.addEventListener('mousedown', onDocClick);
		return () => document.removeEventListener('mousedown', onDocClick);
	});
</script>

<div bind:this={containerEl} class="relative w-full" class:opacity-50={disabled} class:pointer-events-none={disabled}>
	<div
		class="flex flex-wrap items-center gap-1 min-h-[38px] px-2 py-1 bg-background border border-border rounded-md cursor-text transition-colors"
		class:ring-1={open}
		class:ring-ring={open}
		class:border-ring={open}
		role="button"
		tabindex="-1"
		onclick={() => { inputEl?.focus(); openDropdown(); }}
		onkeydown={() => {}}
	>
		{#each value as tag}
			<Badge variant="outline">
				<span class="text-xs">{tag}</span>
				{#if !disabled}
					<button onclick={(e) => { e.stopPropagation(); remove(tag); }}>
						<X class="cursor-pointer w-4 h-4" />
					</button>
				{/if}
			</Badge>
		{/each}

		<input
			bind:this={inputEl}
			bind:value={query}
			type="text"
			class="flex-1 min-w-[80px] border-none outline-none bg-transparent text-sm text-foreground placeholder:text-muted-foreground py-0.5"
			{placeholder}
			{disabled}
			autocomplete="off"
			onfocus={openDropdown}
			onkeydown={onKeydown}
		/>
	</div>

	{#if open && listItems.length > 0}
	<div
		class="absolute top-[calc(100%+4px)] left-0 right-0 z-50 bg-popover border border-border rounded-md p-1 max-h-[220px] overflow-y-auto shadow-md"
		role="listbox"
	>
		{#each listItems as item, i}
			<button
				type="button"
				class="flex items-center gap-1.5 w-full px-2.5 py-1.5 rounded-sm text-left text-sm text-foreground transition-colors"
				class:bg-accent={i === highlighted}
				class:text-accent-foreground={i === highlighted}
				role="option"
				aria-selected={i === highlighted}
				onmouseenter={() => highlighted = i}
				onmousedown={(e) => { e.preventDefault(); select(item.name, item.type === 'create'); }}
			>
				{#if item.type === 'create'}
					<span class="text-[11px] font-medium px-1.5 py-px rounded-full bg-secondary text-secondary-foreground whitespace-nowrap">
						Create
					</span>
				{/if}
				<span class:italic={item.type === 'create'}>{item.name}</span>
			</button>
		{/each}
	</div>
	{/if}
</div>