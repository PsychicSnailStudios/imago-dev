export type TrackAlbumEntry = {
	uid: string;
	name: string;
	track_number: number | null;
	disc: number | null;
};

export type PlaylistTrackEntry = {
	uid: string;
	name: string;
	order: number;
};

export type UserOptions = {
   shuffle_link: String,
	start_trim_ms: number,
	end_trim_ms: number,
	skip_conditions: String,
	shuffle_priority: number,
}

export type Track = {
	id: number;
	uid: string;
	path: string;
	remote_path: string | null;
	last_modified: number;
	title: string | null;
	artists: string | null;
	album_artist: string | null;
	albums: string | null;
	genres: string | null;
	year: string | null;
	rating: number | null;
	tags: string | null;
	duration_ms: number | null;
	bpm: number | null;
	key: string | null;
	user_options: string | null;
	credits: string | null;
	label: string | null;
	format: string | null;
	bitrate: number | null;
	remote_data: string | null;
	track_data: string | null;
	artwork_thumb: string | null;
	source_lib_uid: string | null;
	is_local_override: boolean | null;
	date_added: number | null;
	play_source_uid?: string | null;
};

export type Album = {
	id: number;
	uid: string;
	format: string | null;
	title: string;
	rating: number | null;
	artists: string | null;
	album_artist: string | null;
	release_date: string | null;
	tags: string | null;
	genres: string | null;
	tracks: string | null;
	credits: string | null;
	label: string | null;
	artwork_path: string | null;
	emulate_type: string | null;
	artwork_thumb: string | null;
	source_lib_uid: string | null;
	is_local_override: boolean | null;
};

export type Artist = {
	id: number;
	uid: string;
	name: string;
	aka: string | null;
	about: string | null;
	tags: string | null;
	genres: string | null;
	websites: string | null;
	members: string | null;
	profile_art_path: string | null;
	banner_art_path: string | null;
	profile_art_thumb: string | null;
	banner_art_thumb: string | null;
	source_lib_uid: string | null;
	is_local_override: boolean | null;
};

export type Playlist = {
	id: number;
	uid: string;
	title: string;
	description: string | null;
	owner: string | null;
	tracks: PlaylistTrackEntry[] | null;
	artwork_path: string | null;
	folder: string | null;
	version: number;
	versions_data: string | null;
	link_url: string | null;
	emulate_type: string | null;
	emulate_settings: string | null;
	pending_tracks: string | null;
	share_settings: string | null;
	artwork_thumb: string | null;
};

export type TrackEntry = { uid: string; name: string; order: number }

export type SpotifyPlaylistSummary = {
	id: string;
	name: string;
	description: string | null;
	track_count: number;
	owner: string;
	image_url: string | null;
};

export type SpotifyPlaylistInfo = {
	id: string;
	name: string;
	description: string | null;
	owner: string;
	track_count: number;
	image_url: string | null;
};

export type Lyrics = {
	track_uid: string;
	track_id: number;
	source: string;
	plain: string | null;
	synced: string | null;
	instrumental: boolean;
};

export type AudioCatagories = "track" | "album" | "artist" | "playlist" | "unknown";


export type ParsedTrack = {
	title: string;
	artist: string;
	album: string;
	duration_ms: number | null;
	year: string | null;
	track_number: number | null;
};

export type ImportState = "idle" | "parsed" | "importing" | "done" | "error";

export type DuplicateGroup = {
	tracks: Track[];
};

// ─── Federated library types ──────────────────────────────────────────────────

export type Library = {
	uid: string;
	name: string;
	is_default: boolean;
	file_path: string;
	sync_url: string | null;
	sync_meta_url: string | null;
	has_write_permission: boolean;
	last_synced: number;
	last_data_version: number;
	sort_order: number;
};

export type LibraryUpdate = {
	name?: string | null;
	sync_url?: string | null;
	sync_meta_url?: string | null;
	write_token?: string | null;
	has_write_permission?: boolean | null;
	last_synced?: number | null;
	last_data_version?: number | null;
	sort_order?: number | null;
};

export type BlocklistEntry = {
	uid: string;
	entity_type: string;
	blocked_at: number;
	reason: string | null;
	cascade: boolean;
	source_lib_uid: string;
};

export type LibraryDeletePreference = {
	lib_uid: string;
	cascade_delete: number;
};

export type MergeResult = {
	rebuilt: boolean;
	libraries_processed: number;
};
