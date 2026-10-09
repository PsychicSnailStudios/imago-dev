type PruneState = {
	open: boolean;
	names: string[];
	resolve: ((value: string[] | null) => void) | null;
};

export const pruneDialogState = $state<PruneState>({
	open: false,
	names: [],
	resolve: null,
});

export function showPrunePicker(names: string[]): Promise<string[] | null> {
	return new Promise((resolve) => {
		pruneDialogState.names = names;
		pruneDialogState.resolve = resolve;
		pruneDialogState.open = true;
	});
}

export function resolvePrune(selected: string[] | null) {
	pruneDialogState.resolve?.(selected);
	pruneDialogState.open = false;
	pruneDialogState.resolve = null;
}
