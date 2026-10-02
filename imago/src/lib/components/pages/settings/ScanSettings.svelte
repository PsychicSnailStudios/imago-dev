<script lang="ts">
	import { Switch } from "$shadcn/switch/index.js";

	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-3 p-3 bg-muted rounded-md">
		<p class="text-xs text-muted-foreground">These run automatically when a path is scanned. Enrichment and lyrics fetch require an internet connection and will slow down scanning.</p>

		<div class="flex items-center justify-between gap-4">
			<div>
				<label class="text-sm font-medium">Auto-enrich tracks</label>
				<p class="text-xs text-muted-foreground">Fetch missing metadata from APIs during scan.</p>
			</div>
			<Switch
				checked={settings["auto_enrich_tracks"] === "true"}
				onCheckedChange={(checked) => saveSetting("auto_enrich_tracks", checked ? "true" : "false")}
			/>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<label class="text-sm font-medium">Auto-enrich albums</label>
				<p class="text-xs text-muted-foreground">Fetch album metadata after tracks are scanned.</p>
			</div>
			<Switch
				checked={settings["auto_enrich_albums"] === "true"}
				onCheckedChange={(checked) => saveSetting("auto_enrich_albums", checked ? "true" : "false")}
			/>
		</div>

		<div class="flex items-center justify-between gap-4">
			<div>
				<label class="text-sm font-medium">Auto-fetch lyrics</label>
				<p class="text-xs text-muted-foreground">Fetch lyrics from lrclib for each track during scan.</p>
			</div>
			<Switch
				checked={settings["auto_fetch_lyrics"] === "true"}
				onCheckedChange={(checked) => saveSetting("auto_fetch_lyrics", checked ? "true" : "false")}
			/>
		</div>
	</div>
</div>
