<script lang="ts">
	import * as Dialog from "$shadcn/dialog/index.js";
	import { Button } from "$shadcn/button/index.js";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { invoke } from "@tauri-apps/api/core";
	import { toast } from "svelte-sonner";
	import { Loader2 } from "lucide-svelte";
	import { getAlbums, getArtists, reloadLibrary } from "$ts/store/library.svelte";
	import { parseTags } from "$ts/util/parsers";

	type Kind = "artists" | "albums";
	type Rec = Record<string, any>;

	let {
		open = $bindable(false),
		kind,
		uids,
		onmerged,
	}: { open: boolean; kind: Kind; uids: string[]; onmerged?: () => void } = $props();

	const SCALARS: Record<Kind, { key: string; label: string }[]> = {
		artists: [
			{ key: "name", label: "Name" },
			{ key: "about", label: "Biography" },
		],
		albums: [
			{ key: "title", label: "Title" },
			{ key: "album_artist", label: "Album artist" },
			{ key: "release_date", label: "Release date" },
			{ key: "format", label: "Format" },
			{ key: "label", label: "Label" },
			{ key: "credits", label: "Credits" },
			{ key: "rating", label: "Rating" },
			{ key: "emulate_type", label: "Display type" },
		],
	};

	const LISTS: Record<Kind, { key: string; label: string }[]> = {
		artists: [
			{ key: "aka", label: "Also known as" },
			{ key: "tags", label: "Tags" },
			{ key: "genres", label: "Genres" },
			{ key: "websites", label: "Websites" },
			{ key: "members", label: "Members" },
		],
		albums: [
			{ key: "artists", label: "Artists" },
			{ key: "tags", label: "Tags" },
			{ key: "genres", label: "Genres" },
		],
	};

	let records = $state<Rec[]>([]);
	let primaryUid = $state("");
	let choices = $state<Record<string, string>>({});
	let imageFrom = $state("");
	let merging = $state(false);

	$effect(() => {
		if (open) loadRecords();
	});

	async function loadRecords() {
		const all: Rec[] = kind === "artists" ? await getArtists() : await getAlbums();
		const wanted = new Set(uids);
		records = all.filter((r) => wanted.has(r.uid));
		primaryUid = records[0]?.uid ?? "";
		choices = {};
		imageFrom = "";
	}

	function recLabel(r: Rec): string {
		return kind === "artists" ? r.name : r.title;
	}

	function isEmpty(v: unknown): boolean {
		return v === null || v === undefined || (typeof v === "string" && v.trim() === "");
	}

	function normalise(v: unknown): string {
		return String(v).trim().toLowerCase();
	}

	const primary = $derived(records.find((r) => r.uid === primaryUid));

	type Conflict = { key: string; label: string; options: { uid: string; value: any }[] };

	const conflicts = $derived.by<Conflict[]>(() => {
		const out: Conflict[] = [];
		for (const f of SCALARS[kind]) {
			const options: { uid: string; value: any }[] = [];
			const seen = new Set<string>();
			for (const r of records) {
				const v = r[f.key];
				if (isEmpty(v)) continue;
				const n = normalise(v);
				if (!seen.has(n)) {
					seen.add(n);
					options.push({ uid: r.uid, value: v });
				}
			}
			if (options.length > 1) out.push({ key: f.key, label: f.label, options });
		}
		return out;
	});

	function chosenValue(key: string): any {
		const conflict = conflicts.find((c) => c.key === key);
		if (conflict) {
			const pickedUid = choices[key];
			const picked = conflict.options.find((o) => o.uid === pickedUid);
			if (picked) return picked.value;
			const fromPrimary = conflict.options.find((o) => o.uid === primaryUid);
			return (fromPrimary ?? conflict.options[0]).value;
		}
		for (const r of [primary, ...records]) {
			if (r && !isEmpty(r[key])) return r[key];
		}
		return null;
	}

	function isChosen(key: string, uid: string): boolean {
		const conflict = conflicts.find((c) => c.key === key);
		if (!conflict) return false;
		const picked = choices[key];
		if (picked && conflict.options.some((o) => o.uid === picked)) return picked === uid;
		const fallback = conflict.options.find((o) => o.uid === primaryUid) ?? conflict.options[0];
		return fallback.uid === uid;
	}

	function imageOf(r: Rec): string | null {
		const thumb = kind === "artists" ? r.profile_art_thumb : r.artwork_thumb;
		const path = kind === "artists" ? r.profile_art_path : r.artwork_path;
		return thumb || path || null;
	}

	const imageRecords = $derived(records.filter((r) => !!imageOf(r)));
	const imageConflict = $derived(imageRecords.length > 1);

	const effectiveImageFrom = $derived.by(() => {
		if (imageFrom && imageRecords.some((r) => r.uid === imageFrom)) return imageFrom;
		if (imageRecords.some((r) => r.uid === primaryUid)) return primaryUid;
		return imageRecords[0]?.uid ?? "";
	});

	function unionList(key: string): string[] {
		const seen = new Set<string>();
		const out: string[] = [];
		for (const r of records) {
			for (const item of parseTags(r[key])) {
				const n = normalise(item);
				if (n && !seen.has(n)) {
					seen.add(n);
					out.push(item);
				}
			}
		}
		return out;
	}

	function buildUpdate(): Record<string, any> {
		const update: Record<string, any> = {};
		for (const f of SCALARS[kind]) {
			const v = chosenValue(f.key);
			if (v !== null && v !== undefined) update[f.key] = v;
		}
		for (const f of LISTS[kind]) {
			let list = unionList(f.key);
			if (kind === "artists" && f.key === "aka") {
				const finalName = normalise(update.name ?? "");
				const seen = new Set(list.map(normalise));
				for (const r of records) {
					const n = normalise(r.name);
					if (n && n !== finalName && !seen.has(n)) {
						seen.add(n);
						list.push(r.name);
					}
				}
			}
			if (list.length > 0) update[f.key] = JSON.stringify(list);
		}
		return update;
	}

	async function confirm() {
		if (!primary || records.length < 2) return;
		merging = true;
		try {
			const dropUids = records.filter((r) => r.uid !== primaryUid).map((r) => r.uid);
			const command = kind === "artists" ? "merge_artists_cmd" : "merge_albums_cmd";
			await invoke(command, {
				keepUid: primaryUid,
				dropUids,
				update: buildUpdate(),
				imageFromUid: imageConflict || imageRecords.length === 1 ? effectiveImageFrom || null : null,
			});
			toast.success(`Merged ${records.length} ${kind} into "${recLabel(primary)}"`);
			await reloadLibrary("all");
			open = false;
			onmerged?.();
		} catch (e) {
			toast.error(`Merge failed: ${e}`);
		} finally {
			merging = false;
		}
	}

	function display(v: any): string {
		const s = String(v);
		return s.length > 60 ? s.slice(0, 57) + "…" : s;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-xl">
		<Dialog.Header>
			<Dialog.Title>Merge {kind}</Dialog.Title>
			<Dialog.Description>
				Pick the one to keep. Where the {records.length} {kind} disagree you choose which value wins; lists such as tags and genres are combined.
				{#if kind === "artists"}
					Tracks using the other names are renamed to the kept name.
				{:else}
					Tracks from every merged album move into the kept album.
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		<ScrollArea class="max-h-[60vh] pr-3">
			<div class="flex flex-col gap-4">
				<div class="flex flex-col gap-1">
					<span class="text-xs font-medium text-muted-foreground">Keep</span>
					<div class="flex flex-wrap gap-1">
						{#each records as r (r.uid)}
							<Button
								variant={r.uid === primaryUid ? "default" : "outline"}
								size="sm"
								onclick={() => (primaryUid = r.uid)}
							>
								{display(recLabel(r))}
							</Button>
						{/each}
					</div>
				</div>

				{#each conflicts as c (c.key)}
					<div class="flex flex-col gap-1">
						<span class="text-xs font-medium text-muted-foreground">{c.label}</span>
						<div class="flex flex-wrap gap-1">
							{#each c.options as o (o.uid)}
								<Button
									variant={isChosen(c.key, o.uid) ? "default" : "outline"}
									size="sm"
									class="max-w-full"
									onclick={() => (choices = { ...choices, [c.key]: o.uid })}
								>
									<span class="truncate">{display(o.value)}</span>
								</Button>
							{/each}
						</div>
					</div>
				{/each}

				{#if imageConflict}
					<div class="flex flex-col gap-1">
						<span class="text-xs font-medium text-muted-foreground">Image</span>
						<div class="flex flex-wrap gap-2">
							{#each imageRecords as r (r.uid)}
								<button
									type="button"
									class="rounded border-2 p-0.5 {effectiveImageFrom === r.uid ? 'border-primary' : 'border-transparent'}"
									onclick={() => (imageFrom = r.uid)}
								>
									<img src={imageOf(r) ?? ""} alt={recLabel(r)} class="size-16 rounded object-cover" />
								</button>
							{/each}
						</div>
					</div>
				{/if}

				{#if conflicts.length === 0 && !imageConflict}
					<p class="text-sm text-muted-foreground">No conflicting values, everything will be combined automatically.</p>
				{/if}
			</div>
		</ScrollArea>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => (open = false)} disabled={merging}>Cancel</Button>
			<Button onclick={confirm} disabled={merging || records.length < 2}>
				{#if merging}<Loader2 class="w-4 h-4 mr-1 animate-spin" />{/if}
				Merge
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
