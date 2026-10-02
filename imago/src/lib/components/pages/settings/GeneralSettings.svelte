<script lang="ts">
	import { setMode, mode } from "mode-watcher";
	import * as Select from "$shadcn/select/index.js";

	let themeOptions = [
		{ value: "system", label: "System" },
		{ value: "light", label: "Light" },
		{ value: "dark", label: "Dark" },
	];
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-2 p-3 bg-muted rounded-md">
		<label class="text-sm">Theme</label>
		<Select.Root
			type="single"
			value={mode.current}
			onValueChange={(value) => setMode(value as "system" | "light" | "dark")}
		>
			<Select.Trigger class="w-[180px]">{mode.current}</Select.Trigger>
			<Select.Content>
				{#each themeOptions as opt}
					<Select.Item value={opt.value}>{opt.label}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>
</div>
