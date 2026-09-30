<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";

	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";
	import { Button } from "$shadcn/button/index.js";

	import { getAlbum, getArtist, getTrackArray } from "$ts/store/library.svelte";
	import { setSelection } from "$ts/store/session.svelte";
	import type { Track, Album, Artist } from "$ts/util/types";
    import { parseArtistsToString, parseAlbumEntries } from "$ts/util/parsers";

	async function goToAlbumArtist(name: string | null) {
		if (!name) return;
		const uid = await invoke<string | null>("get_artist_uid_by_name", { name });
		if (uid) setSelection(uid);
	}

	type Scrobble = {
		uid: string;
		timestamp: number;
		track_uid: string;
		artist_uid: string;
		duration_played: number;
		did_seek: boolean;
		did_pause: boolean;
		track_name: string | null;
		track_artist: string | null;
		track_album: string | null;
		shuffle: boolean | null;
		skipped: boolean | null;
		offline: boolean | null;
		playing_local: boolean | null;
		source_uid: string | null;
	};

	type TopTrack = { track: Track; plays: number; ms: number };
	type TopArtist = { artist: Artist; plays: number; ms: number };
	type TopAlbum = { album: Album; plays: number; ms: number };
	type ActivityBar = { label: string; count: number; ms: number };

	type Period =
		| "today"
		| "week"
		| "month"
		| "year"
		| "all"
		| { month: number; year: number }
		| { year: number };

	type Tab = "tracks" | "artists" | "albums";

	const MIN_DURATION_MS = 30_000;
	const TOP_N_OPTIONS = [10, 25, 50, 100];

	let allScrobbles = $state<Scrobble[]>([]);
	let loading = $state(true);

	let period = $state<Period>("week");
	let activeTab = $state<Tab>("tracks");
	let topN = $state(10);

	let customYear = $state(new Date().getFullYear());
	let customMonth = $state<number | null>(null);
	let showCustomPicker = $state(false);

	const YEARS = (() => {
		const now = new Date().getFullYear();
		const arr = [];
		for (let y = now; y >= 2000; y--) arr.push(y);
		return arr;
	})();

	const MONTHS = [
		"January","February","March","April","May","June",
		"July","August","September","October","November","December"
	];

	function periodLabel(p: Period): string {
		if (p === "today") return "Today";
		if (p === "week") return "Last 7 days";
		if (p === "month") return "Last 30 days";
		if (p === "year") return "Last year";
		if (p === "all") return "All time";
		if (typeof p === "object" && "month" in p) return `${MONTHS[p.month]} ${p.year}`;
		if (typeof p === "object") return `${p.year}`;
		return "";
	}

	function getRange(p: Period): { from: number; to: number } {
		const now = Date.now();
		const nowSec = Math.floor(now / 1000);
		if (p === "today") {
			const start = new Date(); start.setHours(0,0,0,0);
			return { from: Math.floor(start.getTime() / 1000), to: nowSec };
		}
		if (p === "week") return { from: nowSec - 7 * 86400, to: nowSec };
		if (p === "month") return { from: nowSec - 30 * 86400, to: nowSec };
		if (p === "year") return { from: nowSec - 365 * 86400, to: nowSec };
		if (p === "all") return { from: 0, to: nowSec };
		if (typeof p === "object" && "month" in p) {
			const start = new Date(p.year, p.month, 1);
			const end = new Date(p.year, p.month + 1, 1);
			return { from: Math.floor(start.getTime() / 1000), to: Math.floor(end.getTime() / 1000) };
		}
		if (typeof p === "object") {
			const start = new Date(p.year, 0, 1);
			const end = new Date(p.year + 1, 0, 1);
			return { from: Math.floor(start.getTime() / 1000), to: Math.floor(end.getTime() / 1000) };
		}
		return { from: 0, to: nowSec };
	}

	function filteredScrobbles(p: Period): Scrobble[] {
		const { from, to } = getRange(p);
		return allScrobbles.filter(s =>
			s.timestamp >= from &&
			s.timestamp <= to &&
			s.duration_played >= MIN_DURATION_MS
		);
	}

	function tally(scrobbles: Scrobble[], keyOf: (s: Scrobble) => string | null): Map<string, { plays: number; ms: number }> {
		const map = new Map<string, { plays: number; ms: number }>();
		for (const s of scrobbles) {
			const key = keyOf(s);
			if (!key) continue;
			const e = map.get(key) ?? { plays: 0, ms: 0 };
			e.plays++;
			e.ms += s.duration_played;
			map.set(key, e);
		}
		return map;
	}

	function rankTop(map: Map<string, { plays: number; ms: number }>, n: number): [string, { plays: number; ms: number }][] {
		return [...map.entries()]
			.sort((a, b) => b[1].plays - a[1].plays)
			.slice(0, n);
	}

	async function computeTopTracks(scrobbles: Scrobble[], n: number): Promise<TopTrack[]> {
		const ranked = rankTop(tally(scrobbles, s => s.track_uid || null), n);
		if (ranked.length === 0) return [];
		const tracks = await getTrackArray(ranked.map(([uid]) => uid));
		const trackMap = new Map(tracks.map(t => [t.uid, t]));
		return ranked.flatMap(([uid, { plays, ms }]) => {
			const track = trackMap.get(uid);
			if (!track) return [];
			return [{ track, plays, ms }];
		});
	}

	async function computeTopArtists(scrobbles: Scrobble[], n: number): Promise<TopArtist[]> {
		const ranked = rankTop(tally(scrobbles, s => s.artist_uid || null), n);
		if (ranked.length === 0) return [];
		const artists = await Promise.all(ranked.map(([uid]) => getArtist(uid)));
		return ranked.flatMap(([, { plays, ms }], i) => {
			const artist = artists[i];
			if (!artist) return [];
			return [{ artist, plays, ms }];
		});
	}

	async function computeTopAlbums(scrobbles: Scrobble[], n: number): Promise<TopAlbum[]> {
		const uniqueTrackUids = [...new Set(scrobbles.map(s => s.track_uid).filter(Boolean))];
		const tracks = uniqueTrackUids.length > 0 ? await getTrackArray(uniqueTrackUids) : [];
		const trackAlbumMap = new Map(tracks.map(t => [t.uid, parseAlbumEntries(t.albums)[0]?.uid ?? null]));

		const ranked = rankTop(tally(scrobbles, s => trackAlbumMap.get(s.track_uid) ?? null), n);
		if (ranked.length === 0) return [];
		const albums = await Promise.all(ranked.map(([uid]) => getAlbum(uid)));
		return ranked.flatMap(([, { plays, ms }], i) => {
			const album = albums[i];
			if (!album) return [];
			return [{ album, plays, ms }];
		});
	}

	function computeActivity(scrobbles: Scrobble[], p: Period): ActivityBar[] {
		if (p === "today") {
			const hours = Array.from({ length: 24 }, (_, i) => ({
				label: i === 0 ? "12a" : i < 12 ? `${i}a` : i === 12 ? "12p" : `${i - 12}p`,
				count: 0,
				ms: 0,
			}));
			for (const s of scrobbles) {
				const h = new Date(s.timestamp * 1000).getHours();
				hours[h].count++;
				hours[h].ms += s.duration_played;
			}
			return hours;
		}
		if (p === "week") {
			const days: ActivityBar[] = [];
			for (let i = 6; i >= 0; i--) {
				const d = new Date();
				d.setDate(d.getDate() - i);
				d.setHours(0, 0, 0, 0);
				days.push({ label: ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"][d.getDay()], count: 0, ms: 0 });
			}
			for (const s of scrobbles) {
				const d = new Date(s.timestamp * 1000);
				const now = new Date();
				const diff = Math.floor((now.setHours(0,0,0,0) - d.setHours(0,0,0,0)) / 86400000);
				const idx = 6 - diff;
				if (idx >= 0 && idx < 7) { days[idx].count++; days[idx].ms += s.duration_played; }
			}
			return days;
		}
		if (p === "month") {
			const weeks: ActivityBar[] = Array.from({ length: 4 }, (_, i) => ({
				label: `Wk ${i + 1}`,
				count: 0,
				ms: 0,
			}));
			const now = Math.floor(Date.now() / 1000);
			for (const s of scrobbles) {
				const daysAgo = Math.floor((now - s.timestamp) / 86400);
				const wk = Math.min(3, Math.floor(daysAgo / 7));
				const idx = 3 - wk;
				weeks[idx].count++;
				weeks[idx].ms += s.duration_played;
			}
			return weeks;
		}
		if (p === "year" || p === "all") {
			const monthMap = new Map<string, ActivityBar>();
			for (const s of scrobbles) {
				const d = new Date(s.timestamp * 1000);
				const key = `${d.getFullYear()}-${d.getMonth()}`;
				if (!monthMap.has(key)) {
					monthMap.set(key, {
						label: `${MONTHS[d.getMonth()].slice(0, 3)} ${d.getFullYear()}`,
						count: 0,
						ms: 0,
					});
				}
				const e = monthMap.get(key)!;
				e.count++;
				e.ms += s.duration_played;
			}
			return [...monthMap.entries()]
				.sort((a, b) => a[0].localeCompare(b[0]))
				.map(([, v]) => v);
		}
		if (typeof p === "object" && "month" in p) {
			const daysInMonth = new Date(p.year, p.month + 1, 0).getDate();
			const days: ActivityBar[] = Array.from({ length: daysInMonth }, (_, i) => ({
				label: String(i + 1),
				count: 0,
				ms: 0,
			}));
			for (const s of scrobbles) {
				const d = new Date(s.timestamp * 1000);
				if (d.getMonth() === p.month && d.getFullYear() === p.year) {
					const idx = d.getDate() - 1;
					days[idx].count++;
					days[idx].ms += s.duration_played;
				}
			}
			return days;
		}
		if (typeof p === "object") {
			const months: ActivityBar[] = Array.from({ length: 12 }, (_, i) => ({
				label: MONTHS[i].slice(0, 3),
				count: 0,
				ms: 0,
			}));
			for (const s of scrobbles) {
				const d = new Date(s.timestamp * 1000);
				if (d.getFullYear() === p.year) {
					months[d.getMonth()].count++;
					months[d.getMonth()].ms += s.duration_played;
				}
			}
			return months;
		}
		return [];
	}

	function fmtMs(ms: number): string {
		const h = Math.floor(ms / 3600000);
		const m = Math.floor((ms % 3600000) / 60000);
		if (h > 0) return `${h}h ${m}m`;
		return `${m}m`;
	}

	function applyCustom() {
		if (customMonth !== null) {
			period = { month: customMonth, year: customYear };
		} else {
			period = { year: customYear };
		}
		showCustomPicker = false;
	}

	let filtered = $derived(filteredScrobbles(period));
	let activity = $derived(computeActivity(filtered, period));
	let maxActivity = $derived(Math.max(1, ...activity.map(a => a.count)));
	let totalMs = $derived(filtered.reduce((a, s) => a + s.duration_played, 0));
	let totalPlays = $derived(filtered.length);

	let topTracks = $state<TopTrack[]>([]);
	let topArtists = $state<TopArtist[]>([]);
	let topAlbums = $state<TopAlbum[]>([]);
	let topLoading = $state(false);
	let topGeneration = 0;

	$effect(() => {
		const scrobbles = filtered;
		const n = topN;
		const generation = ++topGeneration;
		topLoading = true;

		Promise.all([
			computeTopTracks(scrobbles, n),
			computeTopArtists(scrobbles, n),
			computeTopAlbums(scrobbles, n),
		]).then(([tracks, artists, albums]) => {
			if (generation !== topGeneration) return;
			topTracks = tracks;
			topArtists = artists;
			topAlbums = albums;
			topLoading = false;
		}).catch(e => {
			console.error("Top lists failed to compute", e);
			if (generation === topGeneration) topLoading = false;
		});
	});

	onMount(async () => {
		try {
			allScrobbles = await invoke<Scrobble[]>("get_scrobbles");
		} catch (e) {
			console.error("Analytics failed to load", e);
		} finally {
			loading = false;
		}
	});
</script>

<div class="flex flex-col gap-3">

	<!-- Header -->
	<div class="flex items-center justify-between gap-2 flex-wrap shrink-0">
		<h2 class="text-lg font-semibold tracking-tight">Analytics</h2>

		<div class="flex items-center gap-1 flex-wrap">
			{#each (["today","week","month","year","all"] as const) as p}
				<button
					onclick={() => { period = p; showCustomPicker = false; }}
					class="px-2.5 py-1 rounded text-xs font-medium transition-colors
						{period === p ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
				>
					{periodLabel(p)}
				</button>
			{/each}
			<div class="relative">
				<button
					onclick={() => showCustomPicker = !showCustomPicker}
					class="px-2.5 py-1 rounded text-xs font-medium transition-colors
						{typeof period === 'object' ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
				>
					{typeof period === "object" ? periodLabel(period) : "Custom"}
				</button>
				{#if showCustomPicker}
					<div class="absolute right-0 top-8 z-50 bg-popover border rounded-md shadow-lg p-3 flex flex-col gap-2 min-w-48">
						<select
							class="text-xs border rounded px-2 py-1 bg-background"
							bind:value={customYear}
						>
							{#each YEARS as y}
								<option value={y}>{y}</option>
							{/each}
						</select>
						<select
							class="text-xs border rounded px-2 py-1 bg-background"
							bind:value={customMonth}
						>
							<option value={null}>Full year</option>
							{#each MONTHS as m, i}
								<option value={i}>{m}</option>
							{/each}
						</select>
						<Button class="h-7 text-xs" onclick={applyCustom}>Apply</Button>
					</div>
				{/if}
			</div>
		</div>
	</div>

		<div class="flex flex-col gap-4">

			<!-- Summary stats -->
			<div class="grid grid-cols-3 gap-2">
				<div class="bg-muted rounded-lg p-3 flex flex-col gap-0.5">
					<span class="text-xs text-muted-foreground">Plays</span>
					<span class="text-xl font-bold tabular-nums">{totalPlays.toLocaleString()}</span>
				</div>
				<div class="bg-muted rounded-lg p-3 flex flex-col gap-0.5">
					<span class="text-xs text-muted-foreground">Listening time</span>
					<span class="text-xl font-bold tabular-nums">{fmtMs(totalMs)}</span>
				</div>
				<div class="bg-muted rounded-lg p-3 flex flex-col gap-0.5">
					<span class="text-xs text-muted-foreground">Unique tracks</span>
					<span class="text-xl font-bold tabular-nums">{new Set(filtered.map(s => s.track_uid).filter(Boolean)).size}</span>
				</div>
			</div>

			<!-- Activity graph -->
			<div class="bg-muted rounded-lg p-3 flex flex-col gap-2">
				<span class="text-xs font-medium text-muted-foreground uppercase tracking-wider">Listening Activity</span>
				{#if activity.length === 0}
					<p class="text-xs text-muted-foreground text-center py-4">No data for this period.</p>
				{:else}
					<div class="flex items-end gap-px overflow-x-auto pb-1" style="height:80px;">
						{#each activity as bar}
							<div class="flex flex-col items-center gap-0.5 flex-1 min-w-0 group" style="min-width: {Math.max(8, Math.floor(280 / activity.length))}px;">
								<div class="relative w-full flex items-end" style="height:64px;">
									<div
										class="w-full rounded-t bg-primary/70 group-hover:bg-primary transition-colors"
										style="height: {Math.max(2, Math.round((bar.count / maxActivity) * 64))}px;"
										title="{bar.count} plays · {fmtMs(bar.ms)}"
									></div>
								</div>
								{#if activity.length <= 31}
									<span class="text-[9px] text-muted-foreground truncate w-full text-center leading-none">{bar.label}</span>
								{/if}
							</div>
						{/each}
					</div>
					{#if activity.length > 31}
						<div class="flex justify-between text-[9px] text-muted-foreground">
							<span>{activity[0].label}</span>
							<span>{activity[Math.floor(activity.length / 2)].label}</span>
							<span>{activity[activity.length - 1].label}</span>
						</div>
					{/if}
				{/if}
			</div>

			<!-- Tabs + top N -->
			<div class="flex items-center justify-between gap-2">
				<div class="flex gap-1">
					{#each (["tracks","artists","albums"] as const) as tab}
						<button
							onclick={() => activeTab = tab}
							class="px-3 py-1 rounded text-xs font-medium capitalize transition-colors
								{activeTab === tab ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
						>{tab}</button>
					{/each}
				</div>
				<div class="flex items-center gap-1">
					<span class="text-xs text-muted-foreground">Top</span>
					{#each TOP_N_OPTIONS as n}
						<button
							onclick={() => topN = n}
							class="px-2 py-0.5 rounded text-xs font-medium transition-colors
								{topN === n ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
						>{n}</button>
					{/each}
				</div>
			</div>

			<!-- Top Tracks -->
			{#if activeTab === "tracks"}
				{#if topLoading}
					<div class="flex flex-col gap-1">
						{#each Array(5) as _}
							<div class="flex gap-2 p-2 items-center animate-pulse">
								<div class="rounded shrink-0 bg-muted/60" style="width:36px;height:36px;"></div>
								<div class="flex flex-col gap-1 flex-1 min-w-0">
									<div class="h-3 w-3/4 rounded bg-muted/60"></div>
									<div class="h-2 w-1/2 rounded bg-muted/60"></div>
								</div>
							</div>
						{/each}
					</div>
				{:else if topTracks.length === 0}
					<p class="text-xs text-muted-foreground text-center py-8">No plays recorded for this period.</p>
				{:else}
					<div class="flex flex-col gap-0.5">
						{#each topTracks as { track, plays, ms }, i}
							{@const maxPlays = topTracks[0].plays}
							<div class="relative flex items-center gap-2 p-2 rounded-md hover:bg-muted/40 transition-colors overflow-hidden group">
								<div
									class="absolute inset-0 bg-primary/5 rounded-md"
									style="width: {Math.round((plays / maxPlays) * 100)}%;"
								></div>
								<span class="relative text-xs text-muted-foreground w-5 shrink-0 text-right tabular-nums">{i + 1}</span>
								<div class="relative shrink-0">
									<ArtworkDisplay entity={track} size={36} />
								</div>
								<div class="relative min-w-0 flex-1 grid">
									<button
										onclick={() => setSelection(track.uid)}
										class="text-sm truncate text-left hover:underline font-medium"
									>{track.title ?? "Unknown"}</button>
									<button
										onclick={() => goToAlbumArtist(track.album_artist)}
										class="text-xs text-muted-foreground truncate text-left hover:underline"
									>{parseArtistsToString(track.artists)}</button>
								</div>
								<div class="relative flex flex-col items-end gap-0.5 shrink-0">
									<span class="text-xs font-semibold tabular-nums">{plays} <span class="font-normal text-muted-foreground">{plays === 1 ? "play" : "plays"}</span></span>
									<span class="text-xs text-muted-foreground tabular-nums">{fmtMs(ms)}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{/if}

			<!-- Top Artists -->
			{#if activeTab === "artists"}
				{#if topLoading}
					<div class="flex flex-col gap-1">
						{#each Array(5) as _}
							<div class="flex gap-2 p-2 items-center animate-pulse">
								<div class="rounded-full shrink-0 bg-muted/60" style="width:36px;height:36px;"></div>
								<div class="flex flex-col gap-1 flex-1 min-w-0">
									<div class="h-3 w-1/2 rounded bg-muted/60"></div>
								</div>
							</div>
						{/each}
					</div>
				{:else if topArtists.length === 0}
					<p class="text-xs text-muted-foreground text-center py-8">No plays recorded for this period.</p>
				{:else}
					<div class="flex flex-col gap-0.5">
						{#each topArtists as { artist, plays, ms }, i}
							{@const maxPlays = topArtists[0].plays}
							<div class="relative flex items-center gap-2 p-2 rounded-md hover:bg-muted/40 transition-colors overflow-hidden">
								<div
									class="absolute inset-0 bg-primary/5 rounded-md"
									style="width: {Math.round((plays / maxPlays) * 100)}%;"
								></div>
								<span class="relative text-xs text-muted-foreground w-5 shrink-0 text-right tabular-nums">{i + 1}</span>
								<div class="relative shrink-0">
									<ArtworkDisplay entity={artist} size={36} />
								</div>
								<div class="relative min-w-0 flex-1">
									<button
										onclick={() => setSelection(artist.uid)}
										class="text-sm truncate text-left hover:underline font-medium block w-full"
									>{artist.name}</button>
								</div>
								<div class="relative flex flex-col items-end gap-0.5 shrink-0">
									<span class="text-xs font-semibold tabular-nums">{plays} <span class="font-normal text-muted-foreground">{plays === 1 ? "play" : "plays"}</span></span>
									<span class="text-xs text-muted-foreground tabular-nums">{fmtMs(ms)}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{/if}

			<!-- Top Albums -->
			{#if activeTab === "albums"}
				{#if topLoading}
					<div class="flex flex-col gap-1">
						{#each Array(5) as _}
							<div class="flex gap-2 p-2 items-center animate-pulse">
								<div class="rounded shrink-0 bg-muted/60" style="width:36px;height:36px;"></div>
								<div class="flex flex-col gap-1 flex-1 min-w-0">
									<div class="h-3 w-1/2 rounded bg-muted/60"></div>
									<div class="h-2 w-1/3 rounded bg-muted/60"></div>
								</div>
							</div>
						{/each}
					</div>
				{:else if topAlbums.length === 0}
					<p class="text-xs text-muted-foreground text-center py-8">No plays recorded for this period.</p>
				{:else}
					<div class="flex flex-col gap-0.5">
						{#each topAlbums as { album, plays, ms }, i}
							{@const maxPlays = topAlbums[0].plays}
							<div class="relative flex items-center gap-2 p-2 rounded-md hover:bg-muted/40 transition-colors overflow-hidden">
								<div
									class="absolute inset-0 bg-primary/5 rounded-md"
									style="width: {Math.round((plays / maxPlays) * 100)}%;"
								></div>
								<span class="relative text-xs text-muted-foreground w-5 shrink-0 text-right tabular-nums">{i + 1}</span>
								<div class="relative shrink-0">
									<ArtworkDisplay entity={album} size={36}/>
								</div>
								<div class="relative min-w-0 flex-1 grid">
									<button
										onclick={() => setSelection(album.uid)}
										class="text-sm truncate text-left hover:underline font-medium"
									>{album.title}</button>
									<span class="text-xs text-muted-foreground truncate">{album.album_artist ?? ""}</span>
								</div>
								<div class="relative flex flex-col items-end gap-0.5 shrink-0">
									<span class="text-xs font-semibold tabular-nums">{plays} <span class="font-normal text-muted-foreground">{plays === 1 ? "play" : "plays"}</span></span>
									<span class="text-xs text-muted-foreground tabular-nums">{fmtMs(ms)}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			{/if}

		</div>
</div>