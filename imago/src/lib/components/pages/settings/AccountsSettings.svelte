<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$shadcn/button/index.js";
	import { open } from "@tauri-apps/plugin-dialog";
	import { connectLastfm, disconnectLastfm, lastfmIsConnected, onLastfmConnected } from "$ts/services/lastfm";
	import { connectSpotify, disconnectSpotify, spotifyIsConnected, onSpotifyConnected } from "$ts/services/spotify";
	import { listenbrainzIsConnected, listenbrainzConnect, listenbrainzDisconnect, listenbrainzValidateToken, importSpotifyHistory } from "$ts/services/listenbrainz";
	import { toast } from "svelte-sonner";
	import SpotifyImport from "$lib/components/dialogs/playlists/SpotifyImport.svelte";

	interface Props {
		settings: Record<string, string>;
		saveSetting: (key: string, value: string) => Promise<void>;
	}

	let { settings, saveSetting }: Props = $props();

	let lastfmConnected = $state(false);
	let spotifyConnected = $state(false);
	let lbConnected = $state(false);
	let spotifyImportOpen = $state(false);
	let lbTokenInput = $state("");
	let lbValidating = $state(false);
	let importingHistory = $state(false);

	onMount(() => {
		let cleanupLastfm: (() => void) | undefined;
		let cleanupSpotify: (() => void) | undefined;

		(async () => {
			lastfmConnected = await lastfmIsConnected();
			spotifyConnected = await spotifyIsConnected();
			lbConnected = await listenbrainzIsConnected();

			cleanupLastfm = await onLastfmConnected(async () => {
				lastfmConnected = true;
			});

			cleanupSpotify = await onSpotifyConnected(async () => {
				spotifyConnected = true;
				toast.success("Spotify connected");
			});
		})();

		return () => {
			cleanupLastfm?.();
			cleanupSpotify?.();
		};
	});

	async function handleLastfmConnect() {
		await connectLastfm();
	}

	async function handleLastfmDisconnect() {
		await disconnectLastfm();
		lastfmConnected = false;
	}

	async function handleSpotifyConnect() {
		await connectSpotify();
	}

	async function handleSpotifyDisconnect() {
		await disconnectSpotify();
		spotifyConnected = false;
	}

	async function handleLbConnect() {
		if (!lbTokenInput.trim()) {
			toast.error("Please enter your ListenBrainz token.");
			return;
		}
		lbValidating = true;
		try {
			const valid = await listenbrainzValidateToken(lbTokenInput.trim());
			if (!valid) {
				toast.error("Invalid ListenBrainz token.");
				return;
			}
			await listenbrainzConnect(lbTokenInput.trim());
			lbConnected = true;
			lbTokenInput = "";
			toast.success("ListenBrainz connected.");
		} catch (e) {
			toast.error("Failed to connect to ListenBrainz.");
		} finally {
			lbValidating = false;
		}
	}

	async function handleLbDisconnect() {
		await listenbrainzDisconnect();
		lbConnected = false;
	}

	async function handleImportSpotifyHistory() {
		const selected = await open({
			multiple: false,
			filters: [{ name: "Spotify Data ZIP", extensions: ["zip"] }],
		});
		if (!selected) return;

		importingHistory = true;
		const t = toast.loading("Importing Spotify streaming history…");
		try {
			const result = await importSpotifyHistory(selected as string);
			toast.dismiss(t);
			toast.success(`Imported ${result.imported} listens. ${result.skipped} skipped.`);
		} catch (e) {
			toast.dismiss(t);
			toast.error("Failed to import Spotify history.");
		} finally {
			importingHistory = false;
		}
	}
</script>

<SpotifyImport bind:open={spotifyImportOpen} />

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-3 bg-muted rounded-md">

		<!-- Last.fm -->
		<div class="flex items-center justify-between gap-4">
			<div class="flex flex-col gap-0.5">
				<p class="text-sm font-semibold">Last.fm</p>
				<p class="text-xs text-muted-foreground">
					{#if lastfmConnected}
						Connected — scrobbles will be sent automatically.
					{:else}
						Connect to enable scrobbling. Requires an API key in Metadata APIs below.
					{/if}
				</p>
			</div>
			{#if lastfmConnected}
				<Button variant="outline" onclick={handleLastfmDisconnect}>Disconnect</Button>
			{:else}
				<Button onclick={handleLastfmConnect}>Connect</Button>
			{/if}
		</div>

		{#if !lastfmConnected}
			<div class="space-y-1">
				<label class="text-sm font-medium">Last.fm Shared Secret</label>
				<p class="text-xs text-muted-foreground">Required alongside your API key to complete authentication.</p>
				<input
					class="w-full border rounded px-3 py-2 text-sm bg-background"
					placeholder="Your Last.fm shared secret"
					type="password"
					value={settings["api_lastfm_secret"] ?? ""}
					onchange={(e) => saveSetting("api_lastfm_secret", (e.target as HTMLInputElement).value)}
				/>
			</div>
		{/if}

		<div class="border-t border-border" />

		<!-- Spotify -->
		<div class="flex items-center justify-between gap-4">
			<div class="flex flex-col gap-0.5">
				<p class="text-sm font-semibold">Spotify</p>
				<p class="text-xs text-muted-foreground">
					{#if spotifyConnected}
						Connected — playlists can be imported and metadata enriched.
					{:else}
						Connect to import playlists and use Spotify for metadata enrichment.
					{/if}
				</p>
			</div>
		</div>

		<div class="flex items-center gap-2">
			{#if spotifyConnected}
				<Button variant="outline" onclick={handleSpotifyDisconnect}>Disconnect</Button>
				<Button variant="outline" onclick={() => spotifyImportOpen = true}>Import Playlists</Button>
			{:else}
				<Button onclick={handleSpotifyConnect}>Connect</Button>
			{/if}
			<Button variant="outline" onclick={handleImportSpotifyHistory} disabled={importingHistory}>
				{importingHistory ? "Importing…" : "Import Listening History"}
			</Button>
		</div>

		{#if !spotifyConnected}
			<div class="space-y-1">
				<label class="text-sm font-medium">Spotify Client ID</label>
				<p class="text-xs text-muted-foreground">
					Get this from your app at
					<a href="https://developer.spotify.com/dashboard" target="_blank" class="underline underline-offset-2">developer.spotify.com/dashboard</a>.
					Set <code>sonodex://spotify-callback</code> as a Redirect URI in your app settings.
				</p>
				<input
					class="w-full border rounded px-3 py-2 text-sm bg-background"
					placeholder="Your Spotify client ID"
					value={settings["spotify_client_id"] ?? ""}
					onchange={(e) => saveSetting("spotify_client_id", (e.target as HTMLInputElement).value)}
				/>
			</div>
		{/if}

		<div class="border-t border-border" />

		<!-- ListenBrainz -->
		<div class="flex items-center justify-between gap-4">
			<div class="flex flex-col gap-0.5">
				<p class="text-sm font-semibold">ListenBrainz</p>
				<p class="text-xs text-muted-foreground">
					{#if lbConnected}
						Connected — listens will be submitted automatically.
					{:else}
						Connect with your user token to submit listens to ListenBrainz.
					{/if}
				</p>
			</div>
			{#if lbConnected}
				<Button variant="outline" onclick={handleLbDisconnect}>Disconnect</Button>
			{/if}
		</div>

		{#if !lbConnected}
			<div class="space-y-1">
				<label class="text-sm font-medium">ListenBrainz User Token</label>
				<p class="text-xs text-muted-foreground">
					Find your token at
					<a href="https://listenbrainz.org/profile/" target="_blank" class="underline underline-offset-2">listenbrainz.org/profile</a>.
				</p>
				<div class="flex gap-2">
					<input
						class="flex-1 border rounded px-3 py-2 text-sm bg-background"
						placeholder="Your ListenBrainz user token"
						type="password"
						bind:value={lbTokenInput}
					/>
					<Button onclick={handleLbConnect} disabled={lbValidating}>
						{lbValidating ? "Validating…" : "Connect"}
					</Button>
				</div>
			</div>
		{/if}
	</div>
</div>
