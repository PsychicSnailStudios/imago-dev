import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const sourceLibs = $state({
	map: new Map<string, string>(),
	loaded: false,
});

let loading: Promise<void> | null = null;
let listening = false;

async function fetchSourceLibraries() {
	try {
		const result = await invoke<Record<string, string>>("get_source_libraries_cmd");
		sourceLibs.map = new Map(Object.entries(result));
		sourceLibs.loaded = true;
	} catch (e) {
		console.error("Failed to load source libraries", e);
	}
}

export function ensureSourceLibraries(force = false): Promise<void> {
	if (!listening) {
		listening = true;
		listen("library:updated", () => {
			sourceLibs.loaded = false;
			loading = null;
		});
	}
	if (sourceLibs.loaded && !force) return Promise.resolve();
	if (!loading) {
		loading = fetchSourceLibraries().finally(() => {
			loading = null;
		});
	}
	return loading;
}
