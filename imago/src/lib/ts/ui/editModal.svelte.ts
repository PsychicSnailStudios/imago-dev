export type EditModalTarget =
	| { type: "track"; uid: string }
	| { type: "album"; uid: string }
	| { type: "artist"; uid: string }
	| { type: "playlist"; uid: string };

export let editModal = $state({
	open: false,
	target: null as EditModalTarget | null,
	onsave: null as (() => void) | null,
});

export function openEditModal(target: EditModalTarget, onsave?: () => void) {
	editModal.target = target;
	editModal.open = true;
}

export function closeEditModal() {
	editModal.open = false;
	editModal.target = null;
	editModal.onsave = null;
}