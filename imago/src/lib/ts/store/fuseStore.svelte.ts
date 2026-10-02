// fuseStore.svelte.ts
// Search is now fully delegated to the Rust backend via searchTracks,
// searchAlbums, and searchArtists in library.svelte.ts.
//
// The Tauri commands expected on the backend:
//   search_tracks(query: string)  -> Track[]
//   search_albums(query: string)  -> Album[]
//   search_artists(query: string) -> Artist[]
//
// These should use SQLite FTS5 (or LIKE with indexes) and mirror the
// field weights previously used by Fuse:
//   tracks:  title (0.5), artists (0.25), album_artist (0.15), albums (0.1), tags (0.05), genres (0.05)
//   albums:  title (0.5), artists (0.25), album_artist (0.15), year (0.1), tags (0.05), genres (0.05)
//   artists: name (0.5), akas (0.35), tags (0.05), genres (0.05)
//
// Re-export search functions from library.svelte.ts so that any existing
// import of fuseStore continues to work without changes at call sites.

export { searchTracks, searchAlbums, searchArtists } from "$ts/store/library.svelte";

// warmupFuseIndexes is a no-op now that indexing is handled by the backend.
// Kept as an export so existing call sites don't break.
export function warmupFuseIndexes(): void {}