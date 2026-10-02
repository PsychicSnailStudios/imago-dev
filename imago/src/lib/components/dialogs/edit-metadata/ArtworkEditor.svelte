<script lang="ts">
	
	// APP
	import { open } from "@tauri-apps/plugin-dialog";

	// COMPONENTS
	import { Upload, Clipboard, Trash2 } from "lucide-svelte";
	import Button from "$shadcn/button/button.svelte";

	// CUSTOM COMPONENTS
	import ArtworkDisplay from "$lib/components/custom/ArtworkDisplay.svelte";

	// PROPS
	let {
		entityType,
		entityUid,
		entity: entityProp,
		onchange,
	} = $props<{
		entityType: string;
		entityUid: string;
		entity?: any;
		onchange?: (path: string | null) => void;
	}>();

	// VARIABLES
	let previewPath = $state<string | null>(null);
	let cleared = $state(false);

	let entity = $derived.by(() => {
		const base = entityProp ?? { uid: entityUid };
		if (!cleared) return base;
		return { ...base, artwork_path: null, artwork_thumb: null, profile_art_path: null, profile_art_thumb: null };
	});
	
	// FUNCTIONS
	async function handleUpload() {
		const selected = await open({
			filters: [{ name: "Image", extensions: ["jpg", "jpeg", "png", "webp"] }],
			multiple: false,
		});
		if (selected && typeof selected === "string") {
			cleared = false;
			previewPath = selected;
			onchange?.(selected);
		}
	}

	async function handlePaste() {
		try {
			const items = await navigator.clipboard.read();
			for (const item of items) {
				const imageType = item.types.find((t) => t.startsWith("image/"));
				if (imageType) {
					const blob = await item.getType(imageType);
					const reader = new FileReader();
					reader.onload = () => {
						onchange?.("__paste__");
					};
					reader.readAsArrayBuffer(blob);
					return;
				}
			}
		} catch {}
	}

	function handleClear() {
		previewPath = null;
		cleared = true;
		onchange?.(null);
	}
</script>

<div class="flex justify-around items-start">
	<ArtworkDisplay entity={entity} size={164} previewPath={previewPath} />

	<div class="flex flex-col gap-2 justify-center pt-1">
		<Button variant="outline" size="sm" onclick={handleUpload} class="justify-start gap-2">
			<Upload class="w-4 h-4" />
			Upload file
		</Button>
		<!-- <Button variant="outline" size="sm" onclick={handlePaste} class="justify-start gap-2">
			<Clipboard class="w-4 h-4" />
			Paste from clipboard
		</Button> -->
		<Button variant="outline" size="sm" onclick={handleClear} class="justify-start gap-2 text-destructive hover:text-destructive">
			<Trash2 class="w-4 h-4" />
			Remove artwork
		</Button>
	</div>
</div>