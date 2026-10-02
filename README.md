# Imago

Imago is a music library app for you digital music collection.

Imago is currently in Alpha, and acting as a proof of concept. I am currently working to re-build out the full version with all I learned from this one. I have a long list of bugs and more features to add and can't wait to have 1.0 releaced.

In the meantime if you try it out feel free to leave feedback and bug reports.

## Stack

A app built with SvelteKit + shadcn-svelte + Tauri 2.

- [SvelteKit](https://kit.svelte.dev/) — frontend framework
- [shadcn-svelte](https://shadcn-svelte.com/) — UI components
- [Tailwind CSS](https://tailwindcss.com/) — styling
- [Tauri 2](https://tauri.app/) — native desktop wrapper
- [Bun](https://bun.sh/) — package manager

## Getting Started

Install dependencies:

``bun install``

Run in dev mode (opens native window):

``bun tauri dev``

Build for production:

``bun tauri build``

## Adding Components

bun x shadcn-svelte@latest add [component-name]
