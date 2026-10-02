<script lang="ts">

	import * as InputGroup from "$shadcn/input-group/index.js";

	let { value = $bindable(null), placeholder, classes, Icon, type } = $props<{ value: string | number | null; placeholder: string; classes: string; Icon: any; type: string }>();
</script>

<div class={classes}>
	<InputGroup.Root>
		<InputGroup.Input type={type} placeholder={placeholder} bind:value={value} />
		<InputGroup.Addon>
			<svelte:component this={Icon} class="w-4 h-4" />
		</InputGroup.Addon>
	</InputGroup.Root>
</div>
