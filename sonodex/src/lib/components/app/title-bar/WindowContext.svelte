<script lang="ts">

	// IMPORTS
	import { openUrl } from "@tauri-apps/plugin-opener";

	// COMPONENTS
	import { Ellipsis } from "lucide-svelte";

	import * as DropdownMenu from "$shadcn/dropdown-menu/index.js";
	
	// SCRIPTS
   import { setView } from "$ts/store/session.svelte";
	
	// VARIABLES
  
	// APP FUNCTIONS
  
	// FUNCTIONS
	async function openHelp() {
		await openUrl("https://github.com/PsychicSnailStudios/imago-dev/blob/main/docs/Help.md");
	}
  
</script>

<DropdownMenu.Root>
    <DropdownMenu.Trigger>
		<Ellipsis class="w-4 h-4"/>
	</DropdownMenu.Trigger>

	<DropdownMenu.Content>
		<DropdownMenu.Group>
			<DropdownMenu.Item onclick={() => {setView("settings");}}>Settings</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => {setView("manage");}}>Manage Library</DropdownMenu.Item>
			<DropdownMenu.Item onclick={() => {openHelp();}}>Help</DropdownMenu.Item>
		</DropdownMenu.Group>
	</DropdownMenu.Content>
</DropdownMenu.Root>