mod artwork;
mod commands;
mod connections;
mod db;
mod enrichment;
mod library_manager;
mod profiles;
mod scanner;
mod state;
pub mod thumb;
mod watcher;

use crate::connections::{lastfm_auth, spotify_auth};
use crate::db::{get_library_paths, init_lib_db, init_settings_db};
use crate::profiles::{
	get_lib_db_path, get_library_db_path, get_user_dir, get_local_library_db_path, get_settings_db_path,
	read_registry,
};
use rusqlite::Connection;
use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

// ─── Connection helpers ───────────────────────────────────────────────────────
// These are pub so command modules can use them via crate::open_*

pub fn open_settings_conn(uid: &str) -> Connection {
	let conn = Connection::open(get_settings_db_path(uid)).expect("Failed to open settings database");
	library_manager::tune_conn(&conn);
	conn
}

pub fn open_analytics_conn(uid: &str) -> Result<Connection, rusqlite::Error> {
	let path = crate::profiles::get_user_dir(uid).join("analytics.db");
	let conn = Connection::open(path)?;
	library_manager::tune_conn(&conn);
	db::analytics_manager::init_analytics_db(&conn)?;
	Ok(conn)
}

pub fn open_lib_conn(uid: &str) -> Connection {
	let conn = Connection::open(get_lib_db_path(uid)).expect("Failed to open lib database");
	library_manager::tune_conn(&conn);
	let settings_path = get_settings_db_path(uid);
	conn.execute_batch(&format!(
		"ATTACH DATABASE '{}' AS settings;",
		settings_path.to_string_lossy().replace('\'', "''")
	))
	.ok();
	conn
}

pub fn open_local_library_conn(uid: &str) -> Connection {
	let path = get_local_library_db_path(uid);
	let conn = Connection::open(&path).expect("Failed to open local library database");
	library_manager::tune_conn(&conn);
	let settings_path = get_settings_db_path(uid);
	conn.execute_batch(&format!(
		"ATTACH DATABASE '{}' AS settings;",
		settings_path.to_string_lossy().replace('\'', "''")
	))
	.ok();
	conn
}

pub fn open_merged_conn(uid: &str) -> Connection {
	let path = crate::profiles::get_merged_db_path(uid);
	if !path.exists() {
		return open_lib_conn(uid);
	}
	let conn = Connection::open(&path).expect("Failed to open merged database");
	library_manager::tune_conn(&conn);
	conn
}

// ─── Small helpers ────────────────────────────────────────────────────────────

fn read_setting(uid: &str, key: &str) -> Option<String> {
	let conn = Connection::open(get_settings_db_path(uid)).ok()?;
	conn.query_row(
		"SELECT value FROM settings WHERE key = ?1",
		rusqlite::params![key],
		|row| row.get::<_, String>(0),
	)
	.ok()
}

fn show_main_window(app: &AppHandle) {
	if let Some(w) = app.get_webview_window("main") {
		let _ = w.show();
		let _ = w.unminimize();
		let _ = w.set_focus();
	}
}

#[tauri::command]
fn quit_app(app: AppHandle) {
	app.exit(0);
}

// ─── Deep link handler ────────────────────────────────────────────────────────

pub fn handle_deep_link(app: tauri::AppHandle, url: &str) {
	use tauri::Manager;
	eprintln!("[deep-link] received url: {}", url);

	let state = app.state::<crate::state::AppState>();
	let uid = state.get_uid();

	if let Ok(parsed) = url::Url::parse(url) {
		eprintln!("[deep-link] host: {:?}", parsed.host_str());
		match parsed.host_str() {
			Some("spotify-callback") => {
				eprintln!("[deep-link] matched spotify-callback");
				if let Some(code) = parsed
					.query_pairs()
					.find(|(k, _)| k == "code")
					.map(|(_, v)| v.to_string())
				{
					eprintln!("[deep-link] got code, exchanging...");
					let verifier = state.take_spotify_verifier();
					tauri::async_runtime::spawn(async move {
						let result =
							spotify_auth::spotify_exchange_code(&uid, &code, &verifier).await;
						eprintln!("[deep-link] exchange result: {:?}", result);
						let _ = app.emit("spotify:connected", ());
					});
				} else {
					eprintln!("[deep-link] no code found in query params");
				}
			}
			_ => {
				eprintln!("[deep-link] unmatched host");
			}
		}
	} else {
		eprintln!("[deep-link] failed to parse url");
	}
}

// ─── open_in_explorer (platform-specific, stays here since it needs no db) ────

#[tauri::command]
fn open_in_explorer(path: String) -> Result<(), String> {
	#[cfg(target_os = "windows")]
	std::process::Command::new("explorer")
		.args(["/select,", &path])
		.spawn()
		.map_err(|e| e.to_string())?;

	#[cfg(target_os = "macos")]
	std::process::Command::new("open")
		.args(["-R", &path])
		.spawn()
		.map_err(|e| e.to_string())?;

	#[cfg(target_os = "linux")]
	std::process::Command::new("xdg-open")
		.arg(
			std::path::Path::new(&path)
				.parent()
				.unwrap_or(std::path::Path::new("/")),
		)
		.spawn()
		.map_err(|e| e.to_string())?;

	Ok(())
}

// ─── App entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	let registry = read_registry();
	let initial_uid = registry.active.clone();

	if !initial_uid.is_empty() {
		let settings_conn = Connection::open(get_settings_db_path(&initial_uid))
			.expect("Failed to open settings db");
		init_settings_db(&settings_conn).expect("Failed to init settings db");

		let lib_conn =
			Connection::open(get_lib_db_path(&initial_uid)).expect("Failed to open lib db");
		init_lib_db(&lib_conn).expect("Failed to init lib db");

		// Bootstrap federated library structure and run on-load merge
		library_manager::ensure_default_library(&initial_uid)
			.expect("Failed to ensure default library");
		library_manager::on_load_sync(&initial_uid).unwrap_or_else(|e| {
			eprintln!("[startup] merge sync failed: {e}");
			library_manager::MergeResult {
				rebuilt: false,
				libraries_processed: 0,
			}
		});
	}

	let app_state = AppState::new(initial_uid.clone());

	tauri::Builder::default()
		.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
			if let Some(url) = argv.iter().find(|a| a.starts_with("imago://")) {
				handle_deep_link(app.clone(), url);
			}
			show_main_window(app);
		}))
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_clipboard_manager::init())
		.manage(app_state)
		.register_asynchronous_uri_scheme_protocol("artwork", |ctx, request, responder| {
			artwork::handle(ctx.app_handle().clone(), request, responder);
		})
		.plugin(tauri_plugin_media::init())
		.plugin(tauri_plugin_log::Builder::new().build())
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_os::init())
		.plugin(tauri_plugin_deep_link::init())
		.plugin(tauri_plugin_shell::init())
		.setup(move |app| {
			let handle = app.handle().clone();
			let uid = app.state::<AppState>().get_uid();

			if !uid.is_empty() {
				let settings_conn = open_settings_conn(&uid);

				let default_lib_uid = db::library_registry::get_default_library(&settings_conn)
					.unwrap_or(None)
					.map(|l| l.uid)
					.unwrap_or_default();

				let paths = get_library_paths(&settings_conn)
					.unwrap_or_default()
					.into_iter()
					.map(|p| p.path)
					.collect();

				watcher::start_watcher(handle.clone(), uid.clone(), default_lib_uid, paths);
			}

			#[cfg(any(windows, target_os = "linux"))]
			if cfg!(debug_assertions) {
				app.deep_link().register_all()?;
			}

			app.deep_link().on_open_url(move |event| {
				for url in event.urls() {
					handle_deep_link(handle.clone(), url.as_str());
				}
			});

			#[cfg(desktop)]
			app.handle()
				.plugin(tauri_plugin_updater::Builder::new().build())?;
			#[cfg(desktop)]
			app.handle().plugin(
				tauri_plugin_window_state::Builder::default()
					.with_state_flags(
						tauri_plugin_window_state::StateFlags::all()
							& !tauri_plugin_window_state::StateFlags::VISIBLE,
					)
					.build(),
			)?;

			#[cfg(desktop)]
			{
				app.handle().plugin(tauri_plugin_autostart::init(
					tauri_plugin_autostart::MacosLauncher::LaunchAgent,
					Some(vec!["--autostart"]),
				))?;

				let show_item = MenuItem::with_id(app, "show", "Show Imago", true, None::<&str>)?;
				let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
				let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

				let mut tray = TrayIconBuilder::with_id("main-tray")
					.tooltip("Imago")
					.menu(&menu)
					.show_menu_on_left_click(false)
					.on_menu_event(|app, event| match event.id.as_ref() {
						"show" => show_main_window(app),
						"quit" => {
							let _ = app.emit("tray:quit", ());
							let fallback = app.clone();
							std::thread::spawn(move || {
								std::thread::sleep(std::time::Duration::from_secs(3));
								fallback.exit(0);
							});
						}
						_ => {}
					})
					.on_tray_icon_event(|tray, event| {
						if let TrayIconEvent::Click {
							button: MouseButton::Left,
							button_state: MouseButtonState::Up,
							..
						} = event
						{
							show_main_window(tray.app_handle());
						}
					});

				if let Some(icon) = app.default_window_icon() {
					tray = tray.icon(icon.clone());
				}
				tray.build(app)?;
			}

			let start_minimized = std::env::args().any(|a| a == "--autostart")
				&& !uid.is_empty()
				&& read_setting(&uid, "start_minimized").as_deref() == Some("true");

			if !start_minimized {
				show_main_window(app.handle());
			}

			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			open_in_explorer,
			quit_app,
			artwork::clear_artwork_cache,
			// Profiles
			commands::profiles::needs_profile_setup,
			commands::profiles::get_profiles,
			commands::profiles::get_active_profile,
			commands::profiles::get_profile_avatar,
			commands::profiles::create_profile_cmd,
			commands::profiles::update_profile_cmd,
			commands::profiles::delete_profile_cmd,
			commands::profiles::switch_profile,
			commands::profiles::set_profile_password,
			commands::profiles::remove_profile_password,
			commands::profiles::verify_profile_password,
			commands::profiles::verify_recovery_key,
			commands::profiles::profile_has_password,
			// Paths / scan
			commands::paths::add_path,
			commands::paths::remove_path,
			commands::paths::get_paths,
			commands::paths::get_paths_for_library,
			commands::paths::rescan,
			commands::paths::rescan_path_cmd,
			// Tracks
			commands::tracks::get_tracks,
			commands::tracks::get_track,
			commands::tracks::get_track_artwork,
			commands::tracks::get_duplicates,
			commands::tracks::update_track_metadata,
			commands::tracks::write_track_tags,
			commands::tracks::remove_track_from_library,
			commands::tracks::delete_track_file,
			commands::tracks::merge_remote_local_tracks,
			commands::tracks::replace_track_path,
			commands::tracks::add_uid_remap,
			commands::tracks::resolve_uid,
			commands::tracks::get_tracks_by_uids,
			// Albums
			commands::albums::get_albums,
			commands::albums::get_album,
			commands::albums::get_album_artwork,
			commands::albums::create_album_entry,
			commands::albums::update_album_entry,
			commands::albums::delete_album_entry,
			commands::albums::rename_album_in_tracks_cmd,
			commands::albums::set_album_artist_for_album_cmd,
			commands::albums::rename_album_artist_cmd,
			// Artists
			commands::artists::get_artists,
			commands::artists::get_artist,
			commands::artists::get_artist_uid_by_name,
			commands::artists::get_artist_profile_art,
			commands::artists::get_artist_banner_art,
			commands::artists::create_artist_entry,
			commands::artists::update_artist_entry,
			commands::artists::delete_artist_entry,
			commands::artists::get_artist_albums,
			commands::artists::rename_artist_in_library_cmd,
			// Playlists
			commands::playlists::get_playlists,
			commands::playlists::get_playlist,
			commands::playlists::get_playlist_artwork,
			commands::playlists::create_playlist_entry,
			commands::playlists::update_playlist_entry,
			commands::playlists::delete_playlist_entry,
			commands::playlists::rename_playlist_folder,
			commands::playlists::move_playlists_to_folder,
			// Settings
			commands::settings::get_settings,
			commands::settings::save_setting,
			commands::manage::delete_tracks_cmd,
			commands::manage::delete_albums_cmd,
			commands::manage::delete_artists_cmd,
			commands::manage::merge_artists_cmd,
			commands::manage::merge_albums_cmd,
			commands::settings::preview_filename_pattern_cmd,
			// Enrichment
			commands::enrichment::enrich_track,
			commands::enrichment::enrich_all,
			commands::enrichment::enrich_album,
			commands::enrichment::enrich_artist,
			commands::enrichment::enrich_all_albums,
			commands::enrichment::enrich_all_artists,
			commands::enrichment::spotify_enrich_track_cmd,
			commands::enrichment::spotify_enrich_album_cmd,
			commands::enrichment::spotify_enrich_artist_cmd,
			// Lyrics
			commands::lyrics::get_track_lyrics,
			commands::lyrics::fetch_track_lyrics,
			commands::lyrics::delete_track_lyrics,
			commands::lyrics::get_all_lyrics,
			// Scrobbles
			commands::scrobbles::get_scrobbles,
			commands::scrobbles::get_scrobbles_for_track,
			commands::scrobbles::get_scrobbles_since,
			commands::scrobbles::get_recent_scrobbles,
			commands::scrobbles::get_scrobbles_for_artist,
			commands::scrobbles::log_scrobble,
			commands::scrobbles::update_scrobble,
			commands::scrobbles::delete_scrobble,
			// Tags
			commands::tags::get_all_tags_cmd,
			commands::tags::add_tag_cmd,
			commands::tags::add_genre_cmd,
			commands::tags::rename_tag_cmd,
			commands::tags::delete_tag_cmd,
			commands::tags::update_tag_color_cmd,
			commands::tags::get_tag_groups_cmd,
			commands::tags::create_tag_group_cmd,
			commands::tags::update_tag_group_cmd,
			commands::tags::delete_tag_group_cmd,
			// Libraries (federated)
			commands::libraries::get_libraries,
			commands::libraries::create_library_cmd,
			commands::libraries::import_library_cmd,
			commands::libraries::export_library_cmd,
			commands::libraries::update_library_cmd,
			commands::libraries::delete_library_cmd,
			commands::libraries::sync_library_cmd,
			commands::libraries::push_library_cmd,
			commands::libraries::check_write_permission_cmd,
			commands::libraries::rebuild_merged_cmd,
			commands::libraries::get_blocklist,
			commands::libraries::add_to_blocklist_cmd,
			commands::libraries::remove_from_blocklist_cmd,
			commands::libraries::get_delete_preference_cmd,
			commands::libraries::set_delete_preference_cmd,
			// Connections
			commands::connections::lastfm_get_auth_url,
			commands::connections::lastfm_exchange_token_cmd,
			commands::connections::lastfm_disconnect_cmd,
			commands::connections::lastfm_connection_status,
			commands::connections::scrobble_track,
			commands::connections::update_now_playing,
			commands::connections::listenbrainz_connection_status,
			commands::connections::listenbrainz_validate_token_cmd,
			commands::connections::listenbrainz_connect_cmd,
			commands::connections::listenbrainz_disconnect_cmd,
			commands::connections::spotify_get_auth_url,
			commands::connections::spotify_exchange_code_cmd,
			commands::connections::spotify_disconnect_cmd,
			commands::connections::spotify_connection_status,
			commands::connections::spotify_get_playlists_cmd,
			commands::connections::spotify_import_playlist_cmd,
			commands::connections::spotify_get_playlist_info_cmd,
			commands::connections::import_spotify_history_cmd,
			// Downloads
			commands::downloads::download_track_cmd,
			commands::downloads::download_album_tracks_cmd,
			commands::downloads::download_playlist_tracks_cmd,
			commands::downloads::sync_offline_subscriptions_cmd,
			commands::downloads::unsubscribe_offline_cmd,
			// Search
			commands::search::search_tracks,
			commands::search::search_albums,
			commands::search::search_artists,
		])
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}