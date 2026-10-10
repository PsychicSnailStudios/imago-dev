<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import { Button } from "$shadcn/button/index.js";
	import { Switch } from "$shadcn/switch/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";

	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();

	const PATH_STYLE_OPTIONS = [
		{ value: "{artist}/{album}", label: "Artist / Album" },
		{ value: "{artist}/{year} - {album}", label: "Artist / Year - Album" },
		{ value: "{year}/{artist}/{album}", label: "Year / Artist / Album" },
		{ value: "{artist}", label: "Artist only" },
	];

	const FILENAME_STYLE_OPTIONS = [
		{ value: "{track_number} - {title}", label: "01 - Title" },
		{ value: "{title}", label: "Title" },
		{ value: "{artist} - {title}", label: "Artist - Title" },
		{ value: "{track_number} - {artist} - {title}", label: "01 - Artist - Title" },
	];

	const SAMPLE: Record<string, string> = {
		artist: "Bicep",
		album: "Isles",
		year: "2021",
		track_number: "03",
		title: "Atlas",
	};

	function applySample(template: string): string {
		return template.replace(/\{(\w+)\}/g, (match, key) => SAMPLE[key] ?? match);
	}

	const folderStyle = $derived(settings["download_path_style"] ?? "{artist}/{album}");
	const filenameStyle = $derived(settings["download_filename_style"] ?? "{track_number} - {title}");
	const previewPath = $derived(`${applySample(folderStyle)}/${applySample(filenameStyle)}.mp3`);

	async function browseDownloadPath() {
		const selected = await open({ directory: true, multiple: false });
		if (selected) await saveSetting("download_path", selected as string);
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-3 bg-muted rounded-md">
		<p class="text-xs text-muted-foreground">Configure where downloaded tracks are saved and how they are named.</p>

		<div class="space-y-1">
			<label class="text-sm font-medium">Download Location</label>
			<p class="text-xs text-muted-foreground">Folder where offline tracks will be saved.</p>
			<div class="flex gap-2">
				<input
					class="flex-1 border rounded px-3 py-2 text-sm bg-background"
					placeholder="C:\Music\Downloads"
					value={settings["download_path"] ?? ""}
					onchange={(e) => saveSetting("download_path", (e.target as HTMLInputElement).value)}
				/>
				<Button variant="outline" onclick={browseDownloadPath}>Browse</Button>
			</div>
		</div>

		<div class="space-y-2">
			<Label class="text-sm font-medium">Folder Structure</Label>
			<p class="text-xs text-muted-foreground">
				Subfolders inside the download location. Tokens: {"{artist}"} {"{album}"} {"{year}"}. Use / to nest folders.
			</p>
			<Input
				placeholder={"{artist}/{album}"}
				value={settings["download_path_style"] ?? "{artist}/{album}"}
				onchange={(e) => saveSetting("download_path_style", (e.target as HTMLInputElement).value)}
			/>
			<div class="flex flex-wrap gap-1">
				{#each PATH_STYLE_OPTIONS as opt}
					<Button variant="outline" size="sm" onclick={() => saveSetting("download_path_style", opt.value)}>
						{opt.label}
					</Button>
				{/each}
			</div>
		</div>

		<div class="space-y-2">
			<Label class="text-sm font-medium">Filename Style</Label>
			<p class="text-xs text-muted-foreground">
				How downloaded files are named. Tokens: {"{title}"} {"{artist}"} {"{album}"} {"{year}"} {"{track_number}"}. The file extension is added automatically.
			</p>
			<Input
				placeholder={"{track_number} - {title}"}
				value={settings["download_filename_style"] ?? "{track_number} - {title}"}
				onchange={(e) => saveSetting("download_filename_style", (e.target as HTMLInputElement).value)}
			/>
			<div class="flex flex-wrap gap-1">
				{#each FILENAME_STYLE_OPTIONS as opt}
					<Button variant="outline" size="sm" onclick={() => saveSetting("download_filename_style", opt.value)}>
						{opt.label}
					</Button>
				{/each}
			</div>
			<p class="text-xs text-muted-foreground">
				Preview: <span class="text-foreground">{previewPath}</span>
			</p>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<label class="text-sm font-medium">Convert to MP3</label>
				<p class="text-xs text-muted-foreground">Re-encode downloaded tracks to MP3 320kbps using FFmpeg.</p>
			</div>
			<Switch
				checked={settings["download_convert_mp3"] === "true"}
				onCheckedChange={(checked) => saveSetting("download_convert_mp3", checked ? "true" : "false")}
			/>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<label class="text-sm font-medium">Auto offline mode</label>
				<p class="text-xs text-muted-foreground">Automatically switch to offline mode when a stream fails due to no internet connection.</p>
			</div>
			<Switch
				checked={settings["offline_mode_auto"] === "true"}
				onCheckedChange={(checked) => saveSetting("offline_mode_auto", checked ? "true" : "false")}
			/>
		</div>
	</div>
</div>
