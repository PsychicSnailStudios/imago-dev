import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import type {
	Library,
	LibraryUpdate,
	BlocklistEntry,
	LibraryDeletePreference,
	MergeResult,
} from "$ts/util/types";
import { getLibraryByUid, loadLibrary, loadLibraryRegistry, emitLibraryChange } from "$ts/store/library.svelte";

const CASCADE_SINGLE = 0;
const CASCADE_ALL = 1;
const CASCADE_ASK = 2;

// ─── Library registry ─────────────────────────────────────────────────────────

export async function getLibraries(): Promise<Library[]> {
	return invoke<Library[]>("get_libraries");
}

export async function createLibrary(name: string): Promise<Library> {
	return invoke<Library>("create_library_cmd", { name });
}

export async function updateLibrary(libUid: string, update: LibraryUpdate): Promise<void> {
	await invoke("update_library_cmd", { libUid, update });
	emitLibraryChange("libraries:changed");
}

export async function deleteLibrary(libUid: string, deleteFile: boolean): Promise<void> {
	await invoke("delete_library_cmd", { libUid, deleteFile });
	emitLibraryChange("libraries:changed");
}

// ─── Import / Export ──────────────────────────────────────────────────────────

export async function importLibrary(
	name: string,
	syncUrl: string,
	writeToken?: string
): Promise<Library> {
	const lib = await invoke<Library>("import_library_cmd", {
		name,
		syncUrl,
		writeToken: writeToken ?? null,
	});
	await loadLibrary();
	emitLibraryChange("libraries:changed");
	return lib;
}

export async function exportLibrary(libUid: string): Promise<void> {
	const destPath = await save({
		filters: [{ name: "Sonodex Library", extensions: ["db"] }],
		defaultPath: "library.db",
	});
	if (!destPath) return;
	await invoke("export_library_cmd", { libUid, destPath });
}

// ─── Sync / Push ─────────────────────────────────────────────────────────────

export async function syncLibrary(libUid: string): Promise<boolean> {
	const pulled = await invoke<boolean>("sync_library_cmd", { libUid });
	if (pulled) {
		await loadLibrary();
		emitLibraryChange("tracks:changed");
		emitLibraryChange("albums:changed");
		emitLibraryChange("artists:changed");
		emitLibraryChange("playlists:changed");
		emitLibraryChange("libraries:changed");
	}
	return pulled;
}

export async function pushLibrary(libUid: string): Promise<boolean> {
	return invoke<boolean>("push_library_cmd", { libUid });
}

export async function checkWritePermission(libUid: string): Promise<boolean> {
	return invoke<boolean>("check_write_permission_cmd", { libUid });
}

// Returns true if the given source_lib_uid belongs to the default library.
export function isDefaultLibrary(sourceLibUid: string | null): boolean {
	if (!sourceLibUid) return true;
	const lib = getLibraryByUid(sourceLibUid);
	return lib?.is_default ?? false;
}

// Returns true if the entity is editable — either it came from the local
// default library, or it has a local override, or the source lib has write permission.
export function isEntityWritable(
	sourceLibUid: string | null,
	isLocalOverride: boolean | null
): boolean {
	if (!sourceLibUid) return true;
	if (isLocalOverride) return true;
	const lib = getLibraryByUid(sourceLibUid);
	return lib?.has_write_permission ?? false;
}

// ─── Merge cache ─────────────────────────────────────────────────────────────

export async function rebuildMerged(): Promise<MergeResult> {
	const result = await invoke<MergeResult>("rebuild_merged_cmd");
	await loadLibrary();
	emitLibraryChange("tracks:changed");
	emitLibraryChange("albums:changed");
	emitLibraryChange("artists:changed");
	emitLibraryChange("playlists:changed");
	return result;
}

// ─── Blocklist ────────────────────────────────────────────────────────────────

export async function getBlocklist(): Promise<BlocklistEntry[]> {
	return invoke<BlocklistEntry[]>("get_blocklist");
}

export async function addToBlocklist(
	uid: string,
	entityType: string,
	cascade: boolean,
	sourceLibUid: string
): Promise<void> {
	return invoke("add_to_blocklist_cmd", { uid, entityType, cascade, sourceLibUid });
}

export async function removeFromBlocklist(uid: string): Promise<void> {
	await invoke("remove_from_blocklist_cmd", { uid });
	emitLibraryChange("tracks:changed");
	emitLibraryChange("albums:changed");
	emitLibraryChange("artists:changed");
}

// ─── Delete preferences ───────────────────────────────────────────────────────

export async function getDeletePreference(
	libUid: string
): Promise<LibraryDeletePreference | null> {
	return invoke<LibraryDeletePreference | null>("get_delete_preference_cmd", { libUid });
}

export async function setDeletePreference(
	libUid: string,
	cascadeDelete: number
): Promise<void> {
	return invoke("set_delete_preference_cmd", { libUid, cascadeDelete });
}

// ─── Library path management ──────────────────────────────────────────────────

export async function addPathToLibrary(path: string, libUid?: string): Promise<void> {
	await invoke("add_path", { path, libUid: libUid ?? null });
}

export async function browseAndAddPath(libUid?: string): Promise<void> {
	const selected = await open({ directory: true, multiple: false });
	if (!selected) return;
	const path = typeof selected === "string" ? selected : selected[0];
	await addPathToLibrary(path, libUid);
}

// ─── Read-only delete flow ────────────────────────────────────────────────────

// Handles deleting an entity that comes from a read-only library.
// Shows a dialog asking whether to block just this item or cascade.
// If the user has a saved preference for this library it uses that instead.
// Returns true if the item was blocked (hidden), false if the user cancelled.
export async function handleReadOnlyDelete(
	uid: string,
	entityType: "tracks" | "albums" | "artists",
	entityName: string,
	sourceLibUid: string,
	// relatedUids: any albums/artists to cascade-block if the user chooses cascade
	relatedUids?: { uid: string; entityType: "albums" | "artists" }[]
): Promise<boolean> {
	const pref = await getDeletePreference(sourceLibUid);

	let cascadeChoice: boolean;
	let remember = false;

	if (pref && pref.cascade_delete !== 2) {
		cascadeChoice = pref.cascade_delete === 1;
	} else {
		const result = await showBlocklistDialog(entityName, entityType, relatedUids?.length ?? 0);
		if (!result) return false;
		cascadeChoice = result.cascade;
		remember = result.remember;
	}

	await addToBlocklist(uid, entityType, false, sourceLibUid);

	if (cascadeChoice && relatedUids) {
		for (const rel of relatedUids) {
			await addToBlocklist(rel.uid, rel.entityType, true, sourceLibUid);
		}
	}

	if (remember) {
		await setDeletePreference(sourceLibUid, cascadeChoice ? CASCADE_ALL : CASCADE_SINGLE);
	}

	// Emit the changed event for whichever domain was affected
	emitLibraryChange(`${entityType}:changed` as any);
	if (cascadeChoice && relatedUids && relatedUids.length > 0) {
		const affectedTypes = new Set(relatedUids.map(r => r.entityType));
		for (const t of affectedTypes) {
			emitLibraryChange(`${t}:changed` as any);
		}
	}

	return true;
}

// ─── Dialog helper ────────────────────────────────────────────────────────────

type BlocklistDialogResult = {
	cascade: boolean;
	remember: boolean;
} | null;

// This drives a native dialog via a simple Promise-based pattern.
// The actual dialog UI component listens to this store and resolves the promise.
let _resolveBlocklistDialog: ((result: BlocklistDialogResult) => void) | null = null;

export const blocklistDialogState = $state({
	open: false,
	entityName: "",
	entityType: "" as "tracks" | "albums" | "artists",
	hasRelated: false,
});

export function showBlocklistDialog(
	entityName: string,
	entityType: "tracks" | "albums" | "artists",
	relatedCount: number
): Promise<BlocklistDialogResult> {
	blocklistDialogState.open = true;
	blocklistDialogState.entityName = entityName;
	blocklistDialogState.entityType = entityType;
	blocklistDialogState.hasRelated = relatedCount > 0;

	return new Promise((resolve) => {
		_resolveBlocklistDialog = resolve;
	});
}

export function resolveBlocklistDialog(result: BlocklistDialogResult): void {
	blocklistDialogState.open = false;
	_resolveBlocklistDialog?.(result);
	_resolveBlocklistDialog = null;
}