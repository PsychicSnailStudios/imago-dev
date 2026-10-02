<script lang="ts">
	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-3 bg-muted rounded-md">
		<div class="space-y-2">
			<h2 class="text-sm font-semibold">Online Metadata Enrichment</h2>
			<p class="text-xs text-muted-foreground">Fetch missing metadata from MusicBrainz and TheAudioDB. Primary API is tried first; falls back to the other if not found.</p>

			<div class="flex items-center justify-between gap-4">
				<label class="text-sm">Primary API</label>
				<select
					class="flex-1 border rounded px-3 py-2 text-sm bg-background"
					value={settings["enrich_primary_api"] ?? "musicbrainz"}
					onchange={(e) => saveSetting("enrich_primary_api", (e.target as HTMLSelectElement).value)}
				>
					<option value="musicbrainz">MusicBrainz</option>
					<option value="audiodb">TheAudioDB</option>
				</select>
			</div>

			<div class="space-y-1">
				<label class="text-sm font-medium">Last.fm API Key</label>
				<p class="text-xs text-muted-foreground">Required to connect your Last.fm account and use Last.fm for enrichment.</p>
				<input
					class="w-full border rounded px-3 py-2 text-sm bg-background"
					placeholder="Your Last.fm API key"
					value={settings["api_lastfm_key"] ?? ""}
					onchange={(e) => saveSetting("api_lastfm_key", (e.target as HTMLInputElement).value)}
				/>
			</div>

			<div class="space-y-1">
				<label class="text-sm font-medium">TheAudioDB API Key</label>
				<p class="text-xs text-muted-foreground">Leave blank to use the free tier.</p>
				<input
					class="w-full border rounded px-3 py-2 text-sm bg-background"
					placeholder="Pro API key (optional)"
					value={settings["api_audiodb_key"] ?? ""}
					onchange={(e) => saveSetting("api_audiodb_key", (e.target as HTMLInputElement).value)}
				/>
			</div>

			<div class="space-y-1">
				<label class="text-sm font-medium">Discogs API Token</label>
				<p class="text-xs text-muted-foreground">Required to use Discogs for metadata enrichment.</p>
				<input
					class="w-full border rounded px-3 py-2 text-sm bg-background"
					placeholder="Your Discogs token (optional)"
					value={settings["api_discogs_key"] ?? ""}
					onchange={(e) => saveSetting("api_discogs_key", (e.target as HTMLInputElement).value)}
				/>
			</div>
		</div>

		<div class="space-y-2">
			<h2 class="text-sm font-semibold">Enrichment Field Priority</h2>
			<p class="text-xs text-muted-foreground">"Local" keeps your existing value and only fills blanks. "API" overwrites with API data if available.</p>
			{#each [
				{ key: "enrich_priority_title", label: "Title" },
				{ key: "enrich_priority_artists", label: "Artists" },
				{ key: "enrich_priority_album_artist", label: "Album Artist" },
				{ key: "enrich_priority_album", label: "Album" },
				{ key: "enrich_priority_year", label: "Year" },
				{ key: "enrich_priority_genres", label: "Genres" },
				{ key: "enrich_priority_bpm", label: "BPM" },
				{ key: "enrich_priority_key", label: "Key" },
				{ key: "enrich_priority_artwork", label: "Artwork" },
			] as { key, label }}
				<div class="flex items-center justify-between gap-4">
					<label class="text-sm w-28">{label}</label>
					<select
						class="flex-1 border rounded px-3 py-2 text-sm bg-background"
						value={settings[key] ?? "local"}
						onchange={(e) => saveSetting(key, (e.target as HTMLSelectElement).value)}
					>
						<option value="local">Local (fill blanks only)</option>
						<option value="api">API (overwrite)</option>
					</select>
				</div>
			{/each}
		</div>
	</div>
</div>
