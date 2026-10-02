type DialogOptions = {
	title?: string;
	description?: string;
};

type DialogState = {
	open: boolean;
	title: string;
	description: string;
	resolve: ((value: boolean) => void) | null;
};

export const dialogState = $state<DialogState>({
	open: false,
	title: "Warning",
	description: "This action cannot be undone",
	resolve: null,
});

export function warnEmptyFields(hasEmpty: boolean): Promise<boolean> {
	if (!hasEmpty) return Promise.resolve(true);
	return showWarning({
		title: "Some fields are empty",
		description: "Empty fields will not be updated — their existing values will be kept. Do you want to continue?",
	});
}

export function showWarning(options: DialogOptions = {}): Promise<boolean> {
	return new Promise((resolve) => {
		dialogState.title = options.title ?? "Warning";
		dialogState.description = options.description ?? "This action cannot be undone";
		dialogState.resolve = resolve;
		dialogState.open = true;
	});
}

export function confirmDialog() {
	dialogState.resolve?.(true);
	dialogState.open = false;
	dialogState.resolve = null;
}

export function cancelDialog() {
	dialogState.resolve?.(false);
	dialogState.open = false;
	dialogState.resolve = null;
}