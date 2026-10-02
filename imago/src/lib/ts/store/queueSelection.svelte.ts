let selectedIndices = $state<Set<number>>(new Set());
let lastClickedIndex = $state<number | null>(null);

export const queueSelection = {
	get selected() { return selectedIndices; },
	get count() { return selectedIndices.size; },
	isSelected(index: number): boolean {
		return selectedIndices.has(index);
	}
};

export function selectQueueItem(index: number, event: MouseEvent, totalCount: number) {
	if (event.shiftKey && lastClickedIndex !== null) {
		const [lo, hi] = index < lastClickedIndex ? [index, lastClickedIndex] : [lastClickedIndex, index];
		const range = Array.from({ length: hi - lo + 1 }, (_, i) => lo + i);
		if (event.ctrlKey || event.metaKey) {
			selectedIndices = new Set([...selectedIndices, ...range]);
		} else {
			selectedIndices = new Set(range);
		}
		return;
	}

	if (event.ctrlKey || event.metaKey) {
		const next = new Set(selectedIndices);
		if (next.has(index)) {
			next.delete(index);
		} else {
			next.add(index);
		}
		selectedIndices = next;
	} else {
		selectedIndices = new Set([index]);
	}

	lastClickedIndex = index;
}

export function clearQueueSelection() {
	selectedIndices = new Set();
	lastClickedIndex = null;
}
