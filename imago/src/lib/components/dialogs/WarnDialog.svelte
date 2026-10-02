<script lang="ts">
	import * as AlertDialog from "$shadcn/alert-dialog/index.js";

	let {
		open = $bindable(false),
		title = "Warning",
		description = "This action cannot be undone",
		onconfirm,
		oncancel,
	} = $props<{
		open: boolean;
		title?: string;
		description?: string;
		onconfirm?: () => void;
		oncancel?: () => void;
	}>();
</script>

<AlertDialog.Root bind:open={open}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{title}</AlertDialog.Title>
			<AlertDialog.Description>
				<p>{description}</p>
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel onclick={() => oncancel?.()}>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action class="bg-destructive text-destructive-foreground hover:bg-destructive/90" onclick={() => onconfirm?.()}>Continue</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
