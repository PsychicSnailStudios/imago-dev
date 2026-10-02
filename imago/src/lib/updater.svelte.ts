import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export async function checkForUpdate(silent = false) {
    try {
        const update = await check();
        if (!update?.available) {
            if (!silent) console.log("Already on latest version");
            return null;
        }
        return update;
    } catch (e) {
        console.error("Update check failed:", e);
        return null;
    }
}

export async function downloadAndInstall(
	update: Awaited<ReturnType<typeof check>>,
	onProgress?: (downloaded: number, total: number | null) => void
) {
	if (!update) return;

	let downloaded = 0;
	let total: number | null = null;

	await update.downloadAndInstall((event) => {
		if (event.event === "Started") {
			total = event.data.contentLength ?? null;
		} else if (event.event === "Progress") {
			downloaded += event.data.chunkLength;
			onProgress?.(downloaded, total);
		}
	});

	await relaunch();
}
