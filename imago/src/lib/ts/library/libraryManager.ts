import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "svelte-sonner";
import { emitSingleChange, reloadLibrary } from "$ts/store/library.svelte";
import { showWarning } from "$ts/ui/dialogManager.svelte";
import {
	addToBlocklist,
	getDeletePreference,
	setDeletePreference,
	showBlocklistDialog,
} from "$ts/library/libraryRegistry.svelte";
import type { DuplicateGroup } from "$ts/util/types";

const CASCADE_SINGLE = 0;
const CASCADE_ALL = 1;

type EntityKind = "tracks" | "albums" | "artists";

type ReadOnlyItem = { uid: string; source_lib_uid: string };

type DeleteResult = {
	deleted: number;
	read_only: ReadOnlyItem[];
	failed: { uid: string; error: string }[];
};

async function hideReadOnly(items: ReadOnlyItem[], kind: EntityKind): Promise<number> {
	const byLib = new Map<string, string[]>();
	for (const item of items) {
		const list = byLib.get(item.source_lib_uid) ?? [];
		list.push(item.uid);
		byLib.set(item.source_lib_uid, list);
	}

	let hidden = 0;
	for (const [libUid, uids] of byLib) {
		const pref = await getDeletePreference(libUid);
		const hasSavedChoice = !!pref && pref.cascade_delete !== 2;
		if (!hasSavedChoice) {
			const label = uids.length === 1 ? "1 item" : `${uids.length} items`;
			const choice = await showBlocklistDialog(label, kind, 0);
			if (!choice) continue;
			if (choice.remember) {
				await setDeletePreference(libUid, choice.cascade ? CASCADE_ALL : CASCADE_SINGLE);
			}
		}
		for (const uid of uids) {
			await addToBlocklist(uid, kind, false, libUid);
			hidden++;
		}
	}
	return hidden;
}

async function runDelete(
	command: string,
	kind: EntityKind,
	uids: string[],
	extra: Record<string, unknown> = {}
): Promise<void> {
	try {
		const result = await invoke<DeleteResult>(command, { uids, ...extra });

		let hidden = 0;
		if (result.read_only.length > 0) {
			hidden = await hideReadOnly(result.read_only, kind);
		}

		if (result.failed.length > 0) {
			const first = result.failed[0].error;
			toast.error(
				result.failed.length === 1
					? `Could not delete: ${first}`
					: `${result.failed.length} items could not be deleted: ${first}`
			);
		} else if (result.deleted + hidden === 0 && result.read_only.length === 0) {
			toast.info("Nothing was deleted");
		}
	} catch (e) {
		toast.error(`Delete failed: ${e}`);
	} finally {
		await reloadLibrary("all");
	}
}

// ─── Track Actions ───────────────────────────────────────────────────────────

export async function removeTrackFromLibrary(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Track?", description: "This action cannot be undone" });
	if (!confirmed) return;
	await runDelete("delete_tracks_cmd", "tracks", [uid], { deleteFiles: false });
}

export async function removeTracksFromLibrary(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Tracks?`, description: "This action cannot be undone" });
	if (!confirmed) return;
	await runDelete("delete_tracks_cmd", "tracks", uids, { deleteFiles: false });
}

export async function replaceTrackPath(uid: string): Promise<void> {
	const selected = await open({
		multiple: false,
		filters: [{ name: "Audio", extensions: ["mp3", "flac", "m4a", "aac", "wav", "aiff", "ogg"] }],
	});
	if (!selected) return;
	const newPath = typeof selected === "string" ? selected : selected[0];
	await invoke("replace_track_path", { uid, newPath });
	emitSingleChange(uid);
}

export async function openTrackInExplorer(path: string): Promise<void> {
	await invoke("open_in_explorer", { path });
}

// ─── Album Actions ───────────────────────────────────────────────────────────

export async function removeAlbum(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Album?", description: "The album is removed from its tracks. This action cannot be undone" });
	if (!confirmed) return;
	await runDelete("delete_albums_cmd", "albums", [uid]);
}

export async function removeAlbums(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Albums?`, description: "The albums are removed from their tracks. This action cannot be undone" });
	if (!confirmed) return;
	await runDelete("delete_albums_cmd", "albums", uids);
}

// ─── Artist Actions ──────────────────────────────────────────────────────────

const ARTIST_DELETE_NOTE =
	"Only the artist record is removed. Tracks that still list this artist keep the name, and the record may be recreated when those tracks are rescanned or edited.";

export async function removeArtist(uid: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Artist?", description: ARTIST_DELETE_NOTE });
	if (!confirmed) return;
	await runDelete("delete_artists_cmd", "artists", [uid]);
}

export async function removeArtists(uids: string[]): Promise<void> {
	if (uids.length === 0) return;
	const confirmed = await showWarning({ title: `Delete ${uids.length} Artists?`, description: ARTIST_DELETE_NOTE });
	if (!confirmed) return;
	await runDelete("delete_artists_cmd", "artists", uids);
}

// ─── Duplicate Actions ───────────────────────────────────────────────────────

export async function getDuplicates(): Promise<DuplicateGroup[]> {
	return await invoke<DuplicateGroup[]>("get_duplicates");
}

export async function keepTrack(keepUid: string, group: DuplicateGroup): Promise<void> {
	const toRemove = group.tracks.filter((t) => t.uid !== keepUid).map((t) => t.uid);
	if (toRemove.length === 0) return;
	await runDelete("delete_tracks_cmd", "tracks", toRemove, { deleteFiles: false });
}

export async function deleteTrackFile(uid: string, _path: string): Promise<void> {
	const confirmed = await showWarning({ title: "Delete Track?", description: "The file is deleted from disk. This action cannot be undone" });
	if (!confirmed) return;
	await runDelete("delete_tracks_cmd", "tracks", [uid], { deleteFiles: true });
}

export async function mergeKeepFirst(group: DuplicateGroup): Promise<void> {
	if (group.tracks.length < 2) return;
	await keepTrack(group.tracks[0].uid, group);
}

export async function mergeRemoteLocal(group: DuplicateGroup): Promise<void> {
	const local = group.tracks.find((t) => t.path && t.path.length > 0);
	const remote = group.tracks.find((t) => (!t.path || t.path.length === 0) && t.remote_path && t.remote_path.length > 0);
	if (!local || !remote) return;
	try {
		await invoke("merge_remote_local_tracks", { keepUid: local.uid, dropUid: remote.uid });
	} catch (e) {
		toast.error(`Merge failed: ${e}`);
	}
	await reloadLibrary("all");
}