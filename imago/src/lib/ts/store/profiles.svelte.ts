import { invoke } from "@tauri-apps/api/core";

export type Profile = {
	uid: string;
	name: string;
	avatar_blob: number[] | null;
};

export const profileState = $state({
	active: null as Profile | null,
	all: [] as Profile[],
	loaded: false,
});

export async function loadProfiles() {
	const [active, all] = await Promise.all([
		invoke<Profile>("get_active_profile"),
		invoke<Profile[]>("get_profiles"),
	]);
	profileState.active = active;
	profileState.all = all;
	profileState.loaded = true;
}

export async function switchProfile(uid: string) {
	await invoke("switch_profile", { uid });
	location.reload();
}

export async function createProfile(
	name: string,
	avatarBlob: number[] | null,
	copyPathsFrom: string | null
) {
	const profile = await invoke<Profile>("create_profile_cmd", {
		name,
		avatarBlob,
		copyPathsFrom,
	});
	await loadProfiles();
	return profile;
}

export async function updateProfile(
	uid: string,
	name: string | null,
	avatarBlob: number[] | null
) {
	await invoke("update_profile_cmd", { uid, name, avatarBlob });
	await loadProfiles();
}

export async function deleteProfile(uid: string) {
	await invoke("delete_profile_cmd", { uid });
	await loadProfiles();
}

export async function getProfileAvatar(uid: string): Promise<number[] | null> {
	return invoke<number[] | null>("get_profile_avatar", { uid });
}

export async function profileHasPassword(uid: string): Promise<boolean> {
	return invoke<boolean>("profile_has_password", { uid });
}

export async function setProfilePassword(uid: string, password: string): Promise<string> {
	return invoke<string>("set_profile_password", { uid, password });
}

export async function removeProfilePassword(uid: string): Promise<void> {
	return invoke<void>("remove_profile_password", { uid });
}

export async function verifyProfilePassword(uid: string, password: string): Promise<boolean> {
	return invoke<boolean>("verify_profile_password", { uid, password });
}

export async function verifyRecoveryKey(uid: string, key: string): Promise<boolean> {
	return invoke<boolean>("verify_recovery_key", { uid, key });
}
