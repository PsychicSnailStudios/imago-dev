<script lang="ts">
	import { Switch } from "$shadcn/switch/index.js";

	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();

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

		<div class="space-y-1">
			<label class="text-sm font-medium">Custom Filename Pattern</label>
			<p class="text-xs text-muted-foreground">Use tokens: {"{title}"} {"{artist}"} {"{album}"} {"{year}"}. Example: <code>{"{artist}"} - {"{year}"} - {"{title}"}</code></p>
			<input
				class="w-full border rounded px-3 py-2 text-sm bg-background"
				placeholder="{'{artist}'} - {'{album}'} - {'{year}'} - {'{title}'}"
				value={settings["filename_custom_pattern"] ?? ""}
				onchange={(e) => saveSetting("filename_custom_pattern", (e.target as HTMLInputElement).value)}
			/>
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
