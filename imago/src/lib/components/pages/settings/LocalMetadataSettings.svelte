<script lang="ts">
	import { Switch } from "$shadcn/switch/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";
	import { invoke } from "@tauri-apps/api/core";

	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();

	type PatternPreview = {
		matched: boolean;
		title: string | null;
		artist: string | null;
		album: string | null;
		year: string | null;
		track: number | null;
		disc: number | null;
	};

	let previewFilename = $state("Bicep - Isles - 2021 - 01 - Atlas.flac");
	let preview = $state<PatternPreview | null>(null);

	$effect(() => {
		const pattern = settings["filename_custom_pattern"] ?? "";
		const filename = previewFilename;
		if (!pattern.trim() || !filename.trim()) {
			preview = null;
			return;
		}
		invoke<PatternPreview>("preview_filename_pattern_cmd", { filename, pattern })
			.then((r) => (preview = r))
			.catch(() => (preview = null));
	});

	const PREVIEW_FIELDS: { key: keyof PatternPreview; label: string }[] = [
		{ key: "title", label: "Title" },
		{ key: "artist", label: "Artist" },
		{ key: "album", label: "Album" },
		{ key: "year", label: "Year" },
		{ key: "track", label: "Track" },
		{ key: "disc", label: "Disc" },
	];

	const PRIORITY_OPTIONS = [
		{ value: "tag", label: "File Tag" },
		{ value: "filename", label: "Filename (override)" },
		{ value: "folder", label: "Folder (override)" },
	];

	const SETTING_LABELS: Record<string, string> = {
		filename_priority_title: "Title",
		filename_priority_artist: "Artist",
		filename_priority_album: "Album",
		filename_priority_year: "Year",
	};
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-3 bg-muted rounded-md">
		<div class="space-y-2">
			<h2 class="text-sm font-semibold">Metadata Priority</h2>
			<p class="text-xs text-muted-foreground">File Tag uses embedded tags with filename as fallback. Filename override prefers the filename. Folder uses the folder structure as the override source.</p>
			{#each ["filename_priority_title", "filename_priority_artist", "filename_priority_album", "filename_priority_year"] as key}
				<div class="flex items-center justify-between gap-4">
					<label class="text-sm w-24">{SETTING_LABELS[key]}</label>
					<select
						class="flex-1 border rounded px-3 py-2 text-sm bg-background"
						value={settings[key] ?? "tag"}
						onchange={(e) => saveSetting(key, (e.target as HTMLSelectElement).value)}
					>
						{#each PRIORITY_OPTIONS as opt}
							<option value={opt.value}>{opt.label}</option>
						{/each}
					</select>
				</div>
			{/each}
		</div>

		<div class="space-y-2">
			<Label class="text-sm font-medium">Custom Filename Pattern</Label>
			<p class="text-xs text-muted-foreground">
				Tokens: {"{title}"} {"{artist}"} {"{album}"} {"{year}"} {"{track}"} {"{disc}"} {"{ignore}"} (skips that part).
				Example: <code>{"{track} - {artist} - {title}"}</code>. Text between tokens must match exactly; when the pattern does not match a file, the built-in patterns are used instead.
			</p>
			<Input
				placeholder={"{artist} - {album} - {year} - {title}"}
				value={settings["filename_custom_pattern"] ?? ""}
				onchange={(e) => saveSetting("filename_custom_pattern", (e.target as HTMLInputElement).value)}
			/>
			<Label class="text-xs text-muted-foreground">Test with a filename</Label>
			<Input bind:value={previewFilename} />
			{#if preview}
				{#if preview.matched}
					<div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
						{#each PREVIEW_FIELDS as f}
							<div class="flex justify-between gap-2">
								<span class="text-muted-foreground">{f.label}</span>
								<span class="truncate">{preview[f.key] ?? "—"}</span>
							</div>
						{/each}
					</div>
				{:else}
					<p class="text-xs text-destructive">Pattern does not match this filename.</p>
				{/if}
			{/if}
		</div>

		<div class="space-y-2">
			<h2 class="text-sm font-semibold">Artist Parsing</h2>
			<p class="text-xs text-muted-foreground">Attempt to split artist names containing "&" into separate artists when the result matches already-known artists.</p>
			<div class="flex items-center justify-between gap-4">
				<label class="text-sm">Try parse &amp;</label>
				<Switch
					checked={settings["scan_try_parse_ampersand"] === "true"}
					onCheckedChange={(checked) => saveSetting("scan_try_parse_ampersand", checked ? "true" : "false")}
				/>
			</div>
		</div>
	</div>
</div>
