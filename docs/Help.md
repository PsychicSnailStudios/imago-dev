# Help & Getting Started Guide

---

## The Big Picture

1. **Add a folder** that contains your music.
2. Imago **scans** it and builds your library automatically.
3. Browse by **Tracks, Albums, Artists, or Playlists**.
4. **Play** music, queue it up, shuffle it, and adjust the equalizer.
5. **Edit details**, add tags, fetch lyrics, and enrich missing info.
6. Optionally **connect services** (Last.fm, ListenBrainz, Spotify) and **link other libraries**.

Your music files are never moved or renamed by the app. Imago keeps its own database of information about them.

---

## Profiles

A profile is your own private space in Imago. Each profile has its own library, playlists, settings, and listening history. Nothing is shared between profiles.

- **First launch:** you are asked to create a profile. Your name is pre-filled from your computer's username, and you can add an avatar. This step cannot be skipped.
- **Switching profiles:** switching reloads the app so everything reflects the other profile's data.
- **Copying folders:** when creating a new profile you can copy the music folder list from an existing profile so you don't need to add them again.
- **Passwords:** a profile can optionally have a password. When you set one, you are given a **recovery key once**. Save it somewhere safe, because it is the only way back in if you forget the password. Passwords protect access inside the app but do not encrypt your files.

---

## Adding Your Music

Go to the library management area and add a folder. Imago then:

- Searches the folder and all subfolders for audio files
- Reads the information stored inside each file (title, artist, album, year, genre, artwork, and more)
- Creates tracks, albums, and artists automatically
- Shows a progress bar in the sidebar while scanning
- Tells you if it finds likely duplicate tracks

**Supported formats:** MP3, FLAC, M4A, AAC, WAV, AIFF, OGG, and Opus.

### What if a file has missing tags?

Imago can fall back to the file name and the folder structure to work out the details. Common file name patterns it understands:

- Artist - Title
- Artist - Album - Year - Title
- Year - Artist - Album - Title

Common folder layouts it understands:

- Artist / Album
- Artist / Album (2021)
- Artist / 2021 - Album
- Year / Artist / Album

These fallbacks and the order of preference can be changed in Settings.

### Staying up to date automatically

Once a folder is added, Imago **watches it**. New files, changed files, and deleted files are picked up automatically after a few seconds, so you rarely need to rescan. If something looks wrong, you can trigger a manual rescan.

### Removing a folder

Removing a folder from Imago removes its tracks from your library. Your actual files stay exactly where they are.

---

## Browsing Your Library

The main window has three areas:

- **Left sidebar:** navigation between views, plus scan progress
- **Centre:** the current view
- **Right panel:** details about whatever you have selected
- **Bottom bar:** playback controls, always visible

### Views

- **Home / Music:** a overview with albums and recent tracks
- **Tracks:** every track in one list, with search by title, artist, album, or year
- **Albums:** a grid of album covers
- **Artists:** a grid of artists
- **Playlists:** your playlists organized in folders
- **Settings:** all configuration
- **Library Manager:** folders, linked libraries, and sync

### Selecting things

Click any track, album, or artist to see its details in the right panel. The back and forward buttons in the title bar move through what you have viewed, like a browser.

- **Track details** include artwork, information, lyrics, and related items
- **Album details** include the track list, with disc dividers for multi-disc releases, and other albums by the same artist
- **Artist details** include biography, banner and profile images, top tracks, discography, members, and websites

### The track table

- Click column headers to sort. Click again to reverse, and once more to clear the sort.
- Choose which columns appear using the table settings button.
- Select multiple tracks with **Shift+click** (range) or **Ctrl+click** (individual).
- **Double-click** a track to play it.
- **Right-click** a track for the context menu: add to queue, add to playlist, go to track, edit details, and copy name.
- Each view remembers its own sorting and column choices.

### Ghost tracks

A track that appears faded is a **ghost track**. It means Imago knows about the track but has no playable file for it. This happens when:

- A file was moved or deleted outside the app
- You created a placeholder entry by hand (for example, for vinyl you own but have not digitised)
- A track came from a playlist import and has no matching file

Ghost tracks cannot be played locally and are skipped during playback.

---

## Playing Music

### Controls

The bottom bar has play/pause, previous, next, a seek bar, volume, mute, loop, and shuffle.

- **Previous:** if more than 2 seconds have played, it restarts the track. Otherwise it goes to the previous track.
- **Loop:** cycles through off, repeat one track, and repeat everything.
- **Shuffle:** cycles through off, spaced, and smart (see below).

### Keyboard shortcuts

- **Space:** play or pause
- **Left arrow:** previous
- **Right arrow:** next

These work whenever you are not typing in a text box.

### The queue

You can queue individual tracks, whole albums, or whole playlists. Playing something replaces the queue. Adding to the queue appends to it. You can reorder or remove queued items, and the queue is remembered when you close the app.

### Shuffle modes

- **Spaced shuffle:** a random order that tries to avoid putting tracks from the same album or artist next to each other
- **Smart shuffle:** goes further by also grouping tracks that suit each other, such as similar tempo, shared genres, shared tags, and matching musical key, while still keeping same-album tracks apart

### Equalizer

A 10-band equalizer (31 Hz to 16 kHz) is built in. Presets include Flat, Bass Boost, Treble Boost, Vocal, Electronic, Rock, Jazz, and Classical. You can also adjust each band by hand. Your settings are saved.

### Resuming

When you reopen the app, your last track, queue position, volume, and loop/shuffle settings are restored. Playback does **not** start automatically. Press play when you are ready.

### Local vs. streaming tracks

Some tracks can have both a local file and an online or network source. Imago prefers the local file unless the online version has a higher bitrate. If a stream cannot be reached, the app checks whether you are offline and falls back to local files, showing a notice that offers to keep you in online mode.

---

## Playlists

- Create playlists from the Playlists view or from a selection of tracks.
- Add tracks by right-click, or by dragging tracks onto a playlist in the sidebar.
- **Reorder** tracks by dragging them within the playlist.
- A track can only appear once per playlist.
- Organise playlists into **folders**, including nested folders such as Rock / 80s. Folders you create stay visible even when empty.
- Sort playlists by title, creation date, or a **custom order** you set by dragging.
- Playlists are personal to your profile. They are never merged with linked libraries.

### Importing from Spotify

After connecting Spotify, you can import any of your Spotify playlists. Imago matches each song to your own library by title and artist. Songs it cannot find are kept as pending entries so you can see what is missing.

---

## Editing Details

Open the editor from the right-click menu, or from a detail panel. You can edit track, album, artist, and playlist information.

### Tracks

Editable fields include title, artists, album artist, albums (with track and disc numbers), year, genres, tags, rating, tempo, key, label, credits, and artwork.

- **Write to file:** a switch that controls whether changes are also saved into the audio file itself. When off, changes are stored only in Imago. Rating and custom tags are never written to files.
- **Enrich:** fills in missing details from online sources (see below).
- **Fetch Lyrics:** downloads lyrics for the track.

### Automatic tidying

When you edit a track, Imago keeps related items consistent:

- New artists and albums you type are created automatically
- Renaming an artist or album updates every track that uses it
- If an edit leaves an album empty, the album is removed
- If an edit leaves an artist unused, you are asked before it is deleted

### Warnings

If you try to save with blank fields, Imago asks you to confirm first.

### Artwork

Choose an image from your computer for any track, album, artist, or playlist. An image you choose yourself takes priority over artwork embedded in the file.

---

## Tags and Genres

Tags and genres are managed as a shared dictionary rather than loose text.

- Add colours to tags and genres so they stand out in the app
- Rename a tag and every item using it is updated
- Delete a tag and it is removed from everything
- Combine tags into **groups** for organisation
- Typing in a tag field suggests existing names, and you can create new ones on the spot

---

## Enrichment (Filling in Missing Details)

Enrichment looks up your music online and fills in gaps such as year, genres, tempo, artwork, biographies, and labels.

**Sources:** MusicBrainz, TheAudioDB, Last.fm, Discogs, and Spotify (when connected).

- Last.fm and Discogs need your own API key in Settings. MusicBrainz and TheAudioDB work without one.
- You can choose a preferred main source.
- For each field, you decide whether your existing data or the online data wins. By default, **your existing data is kept** and enrichment only fills in blanks.
- You can enrich one item at a time or everything at once, with progress shown.
- Album artwork found during enrichment is also given to tracks on that album that have none.
- Settings can run enrichment automatically after each scan.

---

## Lyrics

Lyrics come from the free lrclib.net service.

- Use **Fetch Lyrics** on a track to download them.
- Both plain text and time-synced lyrics are supported when available.
- If a track is confirmed instrumental, Imago remembers that and will not keep asking.
- Automatic lyric downloading is not available yet.

---

## Listening History and Scrobbling

Imago records every play in your profile's history, including how long you listened, whether you skipped, paused, or seeked, and whether it was shuffled or offline.

### Connecting services

- **Last.fm:** click Connect, approve in your browser, and the app is linked automatically. You need to enter your Last.fm API key and secret in Settings first.
- **ListenBrainz:** paste your personal token from listenbrainz.org. The app checks it before saving.

Once connected, Imago sends a "now playing" update when a track starts, and submits a scrobble when you finish listening. A play counts as a scrobble after **30 seconds** of listening.

### Importing old Spotify listening history

Request your data export from Spotify, then point Imago at the ZIP file. Plays under 5 seconds are skipped, and the rest are matched against your library. You get a summary of how many were imported and skipped.

---

## Connecting Spotify

Spotify connection is used for three things:

- Better enrichment (artwork, genres, tempo, and key)
- Importing your playlists
- Importing your listening history

Click Connect, approve in your browser, and you are returned to the app. Tokens refresh automatically.

---

## Linked Libraries (Sharing and Syncing)

A profile can use more than one library at a time. Your own library is always there. Others can be added so that you see them all combined, as if they were one.

### How it looks

You browse one combined collection. Behind the scenes:

- Your **own library always wins** if two libraries contain the same item
- Items from another library are marked as coming from there

### Sharing your library

Use **Export** to save a copy of your library file. Host it on any simple web server, together with a small information file next to it, and others can link to it.

### Linking someone else's library

Use **Import** and give the web address of their library. Imago downloads it and adds it to your combined view. If you were given an access token, enter it to be able to send changes back.

### Read-only vs. editable

Whether you can change items from a linked library depends on your permission:

- **With permission:** edits are saved on your copy and uploaded to the host when possible. If the upload fails, the change stays on your machine and the library is marked as having unsent changes.
- **Without permission:** the library is read-only. Editing options are disabled.

### Hiding things you cannot delete

If you try to delete something from a read-only library, Imago offers to **hide** it instead:

- **This item only**, or **this item and everything related to it**
- A checkbox to remember your choice for that library

Hidden items can be restored from the blocklist at any time. Hiding only affects your own view.

### Syncing

- **Pull** checks whether the host has a newer version and downloads it. No permission is needed.
- **Push** uploads your changes. Permission is required.
- Libraries with unsent changes are flagged.

### Rebuilding the combined view

If something ever looks out of date or inconsistent, use the **rebuild** option. It is safe and rebuilds the combined view from your libraries without changing any of them.

---

## Offline Downloads

For tracks that exist online but not on your computer, you can download them for offline use.

- Download a single track, or all missing tracks on an album or playlist
- **Subscribe** to an album or playlist so that new missing tracks are downloaded the next time you sync
- Downloads can optionally be converted to MP3
- Unsubscribe at any time

---

## Duplicates

After a scan, Imago checks for tracks that look like the same song: same title, same album artist, and nearly the same length. If it finds any, you are notified so you can review them.

When a streaming entry and a local file turn out to be the same track, Imago can merge them into one entry that keeps both sources.

---

## Settings

Settings control how scanning, enrichment, and connections behave. Key areas:

- **Scanning:** which source to trust for each detail, separators between multiple artists or genres, folder-based fallbacks, and whether albums and artists are created automatically
- **Enrichment:** main source, API keys, per-field preference, and automatic enrichment
- **Connections:** Last.fm, ListenBrainz, and Spotify
- **Playback:** equalizer and related options

Changes to scanning options apply to the next file Imago processes, with no restart needed.

---

## Where Your Data Lives

Everything is stored locally on your computer in Imago's own data folder:

- Windows: your AppData folder
- macOS: Application Support
- Linux: the local share folder

Each profile has its own folder. Your music files remain wherever you keep them.

**Backing up:** back up each profile's folder. The combined-view file is temporary and can be rebuilt any time, so it does not need to be backed up.

---

## Tips and Troubleshooting

**My new music is not showing up.** Check that its folder has been added. Then try a manual rescan.

**Albums or artists look wrong or duplicated.** Check the tags inside your files, then adjust the scanning settings for how details are read. You can also edit items directly.

**A track is faded and will not play.** It is a ghost track. The file is missing or was never there. Re-add the folder it lives in, or replace its path from the track menu.

**Changes to a linked library will not stick.** You may be read-only for that library. Check its permission in the Library Manager.

**Something looks out of sync.** Use the rebuild option in the Library Manager.

**I forgot my profile password.** Use your recovery key. It was only shown once when you set the password.

**Scrobbles are not appearing.** Confirm the service shows as connected in Settings. Scrobbles are only sent after 30 seconds of listening, and failures are silent so playback is never interrupted.

---

## Quick Glossary

- **Library:** a collection of tracks, albums, and artists stored in one file
- **Combined view:** what you see in the app, made from all your linked libraries
- **Profile:** a separate user space with its own data
- **Scrobble:** a record of a listen sent to an online service
- **Enrichment:** filling in missing details from online sources
- **Ghost track:** a track with no playable file
- **Stub track:** a track entry created by hand without a file
- **Blocklist:** the list of items you have hidden from linked libraries
- **Push / Pull:** uploading your changes to a host / downloading the latest version from a host