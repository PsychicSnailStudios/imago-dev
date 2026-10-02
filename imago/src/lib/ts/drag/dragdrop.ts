export { startDrag, endDrag } from "$ts/store/drag.svelte";

export function isDraggingFolderType(e: DragEvent): boolean {
	return e.dataTransfer?.types.includes("application/x-sonodex-folder") ?? false;
}
