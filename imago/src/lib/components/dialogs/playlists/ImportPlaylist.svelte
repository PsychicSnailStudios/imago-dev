<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	import * as AlertDialog from "$shadcn/alert-dialog/index.js";
	import { Input } from "$shadcn/input/index.js";
	import { Label } from "$shadcn/label/index.js";

	import { getTracks, loadLibrary, reloadLibrary } from "$ts/store/library.svelte";
	import { createStubTrack } from "$ts/library/entitySync";
	import { profileState } from "$ts/store/profiles.svelte";
	import { createPlaylist } from "$ts/audio/playlistManager.svelte";
	import { parseArtists, parseAlbumEntries } from "$ts/util/parsers";
	import type { Track, ParsedTrack, ImportState } from "$ts/util/types";

	let {
		open = $bindable(false),
		folder = null,
	}: {
		open: boolean;
		folder?: string | null;
	} = $props();

	let state = $state<ImportState>("idle");
	let playlistName = $state("");
	let fileInput = $state<HTMLInputElement | null>(null);
	let parsedTracks = $state<ParsedTrack[]>([]);
	let parseError = $state("");
	let importError = $state("");
	let matchedCount = $state(0);
	let createdCount = $state(0);
	let fileName = $state("");

	function reset() {
		state = "idle";
		playlistName = "";
		parsedTracks = [];
		parseError = "";
		importError = "";
		fileName = "";
		matchedCount = 0;
		createdCount = 0;
		if (fileInput) fileInput.value = "";
	}

	function handleOpenChange(v: boolean) {
		open = v;
		if (!v) reset();
	}

	function normalizeHeader(h: string): string {
		return h.toLowerCase().replace(/[^a-z0-9]/g, "");
	}

	function parseDurationToMs(val: string): number | null {
		if (!val) return null;
		val = val.trim();
		if (/^\d+$/.test(val)) return parseInt(val);
		const parts = val.split(":").map(Number);
		if (parts.length === 2) return (parts[0] * 60 + parts[1]) * 1000;
		if (parts.length === 3) return (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000;
		return null;
	}

	function parseCSV(text: string): string[][] {
		const rows: string[][] = [];
		let i = 0;
		while (i < text.length) {
			const row: string[] = [];
			while (i < text.length && text[i] !== "\n" && text[i] !== "\r") {
				if (text[i] === '"') {
					i++;
					let cell = "";
					while (i < text.length) {
						if (text[i] === '"' && text[i + 1] === '"') { cell += '"'; i += 2; }
						else if (text[i] === '"') { i++; break; }
						else { cell += text[i++]; }
					}
					row.push(cell);
					if (text[i] === ",") i++;
				} else {
					let cell = "";
					while (i < text.length && text[i] !== "," && text[i] !== "\n" && text[i] !== "\r") {
						cell += text[i++];
					}
					row.push(cell.trim());
					if (text[i] === ",") i++;
				}
			}
			if (text[i] === "\r") i++;
			if (text[i] === "\n") i++;
			if (row.length > 0 && !(row.length === 1 && row[0] === "")) rows.push(row);
		}
		return rows;
	}

	function detectAndParse(text: string): ParsedTrack[] {
		const rows = parseCSV(text);
		if (rows.length < 2) throw new Error("CSV has fewer than 2 rows");

		let dataStartIndex = 0;
		let headers: string[] = [];

		for (let i = 0; i < Math.min(5, rows.length); i++) {
			const norm = rows[i].map(normalizeHeader);
			const hasTitleCol = norm.some(h => ["title", "trackname", "name", "song", "songtitle"].includes(h));
			const hasArtistCol = norm.some(h => ["artist", "artistname", "artists", "performer"].includes(h));
			if (hasTitleCol || hasArtistCol) {
				headers = rows[i].map(normalizeHeader);
				dataStartIndex = i + 1;
				break;
			}
		}

		if (headers.length === 0) {
			const first = rows[0].map(normalizeHeader);
			const looksLikeHeader = first.every(h => /^[a-z]/.test(h) && !h.match(/^\d/));
			if (looksLikeHeader) { headers = first; dataStartIndex = 1; }
			else { headers = rows[0].map((_, idx) => String(idx)); dataStartIndex = 0; }
		}

		const col = (names: string[]): number => {
			for (const name of names) {
				const idx = headers.indexOf(normalizeHeader(name));
				if (idx !== -1) return idx;
			}
			return -1;
		};

		const titleIdx   = col(["Track Name", "Title", "Name", "Song", "Song Title"]);
		const artistIdx  = col(["Artist Name(s)", "Artist", "Artists", "Performer"]);
		const albumIdx   = col(["Album Name", "Album"]);
		const durationIdx = col(["Duration (ms)", "Duration", "Length"]);
		const yearIdx    = col(["Year", "Release Year", "Date"]);
		const trackNumIdx = col(["Track Number", "Track #", "Track"]);

		if (titleIdx === -1 && artistIdx === -1) {
			throw new Error("Could not detect title or artist columns. Make sure your CSV has a header row with columns like 'Title', 'Artist', 'Album'.");
		}

		const tracks: ParsedTrack[] = [];
		for (let i = dataStartIndex; i < rows.length; i++) {
			const row = rows[i];
			if (row.every(c => c === "")) continue;
			const get = (idx: number) => (idx !== -1 && idx < row.length ? row[idx].trim() : "");
			const title = get(titleIdx);
			const artist = get(artistIdx);
			if (!title && !artist) continue;
			const rawYear = get(yearIdx);
			let year: string | null = null;
			if (rawYear) { const m = rawYear.match(/\d{4}/); if (m) year = m[0]; }
			const rawTrackNum = get(trackNumIdx);
			const trackNum = rawTrackNum ? parseInt(rawTrackNum) || null : null;
			tracks.push({
				title: title || "Unknown Title",
				artist: artist || "Unknown Artist",
				album: get(albumIdx),
				duration_ms: parseDurationToMs(get(durationIdx)),
				year,
				track_number: trackNum,
			});
		}

		if (tracks.length === 0) throw new Error("No tracks found in CSV");
		return tracks;
	}

	async function handleFileChange(event: Event) {
		const input = event.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		fileName = file.name;
		if (!playlistName) playlistName = file.name.replace(/\.[^.]+$/, "");
		parseError = "";
		try {
			const text = await file.text();
			parsedTracks = detectAndParse(text);
			state = "parsed";
		} catch (e: any) {
			parseError = e.message ?? "Failed to parse CSV";
			state = "error";
		}
	}

	async function findExistingTrack(parsed: ParsedTrack, allTracks: Track[]): Promise<Track | null> {
		const titleLower = parsed.title.toLowerCase();
		const artistLower = parsed.artist.toLowerCase();

		for (const track of allTracks) {
			const trackTitle = (track.title ?? "").toLowerCase();
			if (trackTitle !== titleLower) continue;
			const artistMatch =
				parseArtists(track.artists).some(a => a.toLowerCase() === artistLower) ||
				(track.album_artist ?? "").toLowerCase() === artistLower;
			if (!artistMatch) continue;
			if (parsed.album) {
				if (parseAlbumEntries(track.albums).some(a => a.name.toLowerCase() === parsed.album.toLowerCase())) return track;
			} else {
				return track;
			}
		}

		if (parsed.album) {
			for (const track of allTracks) {
				const trackTitle = (track.title ?? "").toLowerCase();
				if (trackTitle !== titleLower) continue;
				const artistMatch =
					parseArtists(track.artists).some(a => a.toLowerCase() === artistLower) ||
					(track.album_artist ?? "").toLowerCase() === artistLower;
				if (artistMatch) return track;
			}
		}

		return null;
	}

	async function handleImportPlaylist() {
		if (!playlistName.trim()) return;
		state = "importing";
		importError = "";
		matchedCount = 0;
		createdCount = 0;

		try {
			// Fetch all tracks once up front for matching
			const allTracks = await getTracks();
			const playlistTrackEntries: { uid: string; name: string; order: number }[] = [];

			for (let i = 0; i < parsedTracks.length; i++) {
				const parsed = parsedTracks[i];
				let uid: string;
				const existing = await findExistingTrack(parsed, allTracks);
				if (existing) {
					uid = existing.uid;
					matchedCount++;
				} else {
					uid = await createStubTrack(
						parsed.title, parsed.artist, parsed.album,
						parsed.track_number, parsed.duration_ms, parsed.year
					);
					createdCount++;
				}
				playlistTrackEntries.push({ uid, name: parsed.title, order: i + 1 });
			}

			await createPlaylist(playlistName.trim(), profileState.active?.name ?? null, folder, playlistTrackEntries);

		} catch (e: any) {
			importError = e.message ?? String(e);
			state = "error";
			console.error("Failed to import playlist:", e);
			return;
		}

		await loadLibrary();
		state = "done";
	}
</script>

<AlertDialog.Root {open} onOpenChange={handleOpenChange}>
	<AlertDialog.Content>
		<div class="flex flex-col h-full justify-between">
			<div>
				<AlertDialog.Header>
					<AlertDialog.Title class="mt-4">Import CSV</AlertDialog.Title>
				</AlertDialog.Header>

				<div class="flex flex-col gap-4 py-2 mt-2">
					{#if state === "idle" || state === "error"}
						<div class="flex flex-col gap-2">
							<Label for="playlist-name">Playlist Name</Label>
							<Input id="playlist-name" bind:value={playlistName} placeholder="My Playlist" />
						</div>

						<div class="flex flex-col gap-2">
							<Label for="csv-file">CSV File</Label>
							<input
								id="csv-file"
								type="file"
								accept=".csv,.txt"
								class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
								bind:this={fileInput}
								onchange={handleFileChange}
							/>
							<p class="text-xs text-muted-foreground">
								Supports Spotify, iTunes/Apple Music exports, and generic CSVs with Title/Artist/Album columns.
							</p>
						</div>

						{#if parseError}
							<p class="text-sm text-destructive">{parseError}</p>
						{/if}
					{/if}

					{#if state === "parsed"}
						<div class="rounded-md border border-border bg-muted/40 px-4 py-3 flex flex-col gap-1">
							<p class="text-sm font-medium">Ready to import</p>
							<p class="text-sm text-muted-foreground">{parsedTracks.length} tracks found in <span class="font-mono text-xs">{fileName}</span></p>
						</div>
						<div class="flex flex-col gap-2">
							<Label for="playlist-name-confirm">Playlist Name</Label>
							<Input id="playlist-name-confirm" bind:value={playlistName} placeholder="My Playlist" />
						</div>
						<p class="text-xs text-muted-foreground">
							Tracks already in your library will be matched. Unrecognised tracks will be created as stubs — they won't play until the file is scanned into your library.
						</p>
					{/if}

					{#if state === "importing"}
						<div class="rounded-md border border-border bg-muted/40 px-4 py-3">
							<p class="text-sm text-muted-foreground">Importing tracks…</p>
						</div>
					{/if}

					{#if state === "done"}
						<div class="rounded-md border border-border bg-muted/40 px-4 py-3 flex flex-col gap-1">
							<p class="text-sm font-medium">Import complete</p>
							<p class="text-sm text-muted-foreground">{matchedCount} matched from library</p>
							<p class="text-sm text-muted-foreground">{createdCount} stub tracks created</p>
						</div>
					{/if}

					{#if importError}
						<p class="text-sm text-destructive">{importError}</p>
					{/if}
				</div>
			</div>

			<AlertDialog.Footer>
				{#if state === "done"}
					<AlertDialog.Action onclick={() => { reset(); open = false; }}>Done</AlertDialog.Action>
				{:else}
					<AlertDialog.Cancel onclick={() => { reset(); open = false; }}>Cancel</AlertDialog.Cancel>
					{#if state === "parsed"}
						<AlertDialog.Action onclick={handleImportPlaylist} disabled={!playlistName.trim()}>
							Import {parsedTracks.length} Tracks
						</AlertDialog.Action>
					{/if}
				{/if}
			</AlertDialog.Footer>
		</div>
	</AlertDialog.Content>
</AlertDialog.Root>