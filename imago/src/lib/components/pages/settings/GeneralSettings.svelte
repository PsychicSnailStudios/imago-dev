<script lang="ts">
	import { setMode, mode } from "mode-watcher";
	import * as Select from "$shadcn/select/index.js";

	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
	import { Switch } from "$shadcn/switch/index.js";

	let themeOptions = [
		{ value: "system", label: "System" },
		{ value: "light", label: "Light" },
		{ value: "dark", label: "Dark" },
	];

	let launchOnStartup = $state(false);
	let startMinimized = $state(false);
	let closeToTray = $state(false);
	let loaded = $state(false);
 
	async function saveBool(key: string, value: boolean) {
		await invoke("save_setting", { key, value: value ? "true" : "false" });
	}
 
	onMount(async () => {
		const settings = await invoke<{ key: string; value: string }[]>("get_settings");
		const read = (key: string) => settings.find((s) => s.key === key)?.value === "true";
		startMinimized = read("start_minimized");
		closeToTray = read("close_to_tray");
		launchOnStartup = await isEnabled();
		loaded = true;
	});
 
	async function toggleLaunchOnStartup(value: boolean) {
		launchOnStartup = value;
		if (value) {
			await enable();
		} else {
			await disable();
		}
		await saveBool("launch_on_startup", value);
	}
 
	async function toggleStartMinimized(value: boolean) {
		startMinimized = value;
		await saveBool("start_minimized", value);
	}
 
	async function toggleCloseToTray(value: boolean) {
		closeToTray = value;
		await saveBool("close_to_tray", value);
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-2 p-3 bg-muted rounded-md">
		<label class="text-sm">Theme</label>
		<Select.Root
			type="single"
			value={mode.current}
			onValueChange={(value) => setMode(value as "system" | "light" | "dark")}
		>
			<Select.Trigger class="w-[180px]">{mode.current}</Select.Trigger>
			<Select.Content>
				{#each themeOptions as opt}
					<Select.Item value={opt.value}>{opt.label}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>

	<div class="flex items-center justify-between gap-4 p-3 bg-muted rounded-md">
		<div class="flex flex-col gap-1">
			<span class="text-sm">Launch on startup</span>
			<span class="text-xs text-muted-foreground">Open Imago when you sign in to your computer</span>
		</div>
		<Switch checked={launchOnStartup} onCheckedChange={toggleLaunchOnStartup} disabled={!loaded} />
	</div>
 
	<div class="flex items-center justify-between gap-4 p-3 bg-muted rounded-md">
		<div class="flex flex-col gap-1">
			<span class="text-sm">Start minimized</span>
			<span class="text-xs text-muted-foreground">When launched at startup, stay in the tray instead of opening the window</span>
		</div>
		<Switch checked={startMinimized} onCheckedChange={toggleStartMinimized} disabled={!loaded || !launchOnStartup} />
	</div>
 
	<div class="flex items-center justify-between gap-4 p-3 bg-muted rounded-md">
		<div class="flex flex-col gap-1">
			<span class="text-sm">Close to tray</span>
			<span class="text-xs text-muted-foreground">Closing the window keeps Imago running in the background and watching your folders</span>
		</div>
		<Switch checked={closeToTray} onCheckedChange={toggleCloseToTray} disabled={!loaded} />
	</div>
</div>
