<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onMount } from "svelte";
	import { ScrollArea } from "$shadcn/scroll-area/index.js";
	import { scanState } from "$ts/store/session.svelte";
	import { loadEqSettings } from "$ts/store/eqStore.svelte";

	import * as Tabs from "$shadcn/tabs/index.js";

	import GeneralSettings from "$lib/components/pages/settings/GeneralSettings.svelte";
	import EQSettings from "$lib/components/pages/settings/EQSettings.svelte";
	import LocalMetadataSettings from "$lib/components/pages/settings/LocalMetadataSettings.svelte";
	import ScanSettings from "$lib/components/pages/settings/ScanSettings.svelte";
	import AccountsSettings from "$lib/components/pages/settings/AccountsSettings.svelte";
	import MetadataAPISettings from "$lib/components/pages/settings/MetadataAPISettings.svelte";
	import DownloadsSettings from "$lib/components/pages/settings/DownloadsSettings.svelte";

	let settings = $state<Record<string, string>>({});
	let activeTab = $state("general");

	const TABS = [
		{ id: "general", label: "General" },
		{ id: "eq", label: "EQ" },
		{ id: "metadata", label: "Local Metadata" },
		{ id: "scan", label: "Scan" },
		{ id: "accounts", label: "Accounts" },
		{ id: "apis", label: "Metadata APIs" },
		{ id: "downloads", label: "Downloads" },
	];

	onMount(() => {
		(async () => {
			await loadSettings();
			await loadEqSettings();

			await listen("enrich:progress", (event: any) => {
				scanState.enriching = true;
				scanState.enrichDone = event.payload.done;
				scanState.enrichTotal = event.payload.total;
				scanState.enrichErrors = event.payload.errors;
			});

			await listen("enrich:done", (event: any) => {
				scanState.enriching = false;
				scanState.enrichErrors = event.payload.errors;
				scanState.status = `Enrichment done. ${event.payload.total - event.payload.errors} updated, ${event.payload.errors} not found.`;
			});
		})();
	});

	async function loadSettings() {
		const raw: { key: string; value: string }[] = await invoke("get_settings");
		settings = Object.fromEntries(raw.map((s) => [s.key, s.value]));
	}

	async function saveSetting(key: string, value: string) {
		settings[key] = value;
		await invoke("save_setting", { key, value });
	}
</script>

<div class="flex flex-col gap-2 p-2 border-2 rounded-md h-full w-full overflow-hidden">
	<h2 class="h2">Settings</h2>

	<div class="flex gap-1 flex-wrap">
		{#each TABS as tab}
			<button
				class="px-3 py-1.5 text-sm rounded-md transition-colors {activeTab === tab.id ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				onclick={() => activeTab = tab.id}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<div class="border-b border-border" />

	<ScrollArea class="h-full w-full min-h-0 min-w-0">
		<div class="p-2 pr-4">
			{#if activeTab === "general"}
				<GeneralSettings />
			{:else if activeTab === "eq"}
				<EQSettings />
			{:else if activeTab === "metadata"}
				<LocalMetadataSettings {settings} {saveSetting} />
			{:else if activeTab === "scan"}
				<ScanSettings {settings} {saveSetting} />
			{:else if activeTab === "accounts"}
				<AccountsSettings {settings} {saveSetting} />
			{:else if activeTab === "apis"}
				<MetadataAPISettings {settings} {saveSetting} />
			{:else if activeTab === "downloads"}
				<DownloadsSettings {settings} {saveSetting} />
			{/if}
		</div>
	</ScrollArea>
</div>