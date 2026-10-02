use super::{EnrichedAlbum, EnrichedArtist, EnrichedMetadata};
use reqwest::Client;
use serde::Deserialize;

const AUDIODB_BASE: &str = "https://www.theaudiodb.com/api/v1/json";

#[derive(Debug, Deserialize)]
struct AudioDbSearchResponse {
    track: Option<Vec<AudioDbTrack>>,
}

#[derive(Debug, Deserialize)]
struct AudioDbTrack {
    #[serde(rename = "strTrack")]
    str_track: Option<String>,
    #[serde(rename = "strArtist")]
    str_artist: Option<String>,
    #[serde(rename = "strAlbum")]
    str_album: Option<String>,
    #[serde(rename = "strAlbumStripped")]
    str_album_stripped: Option<String>,
    #[serde(rename = "intYearReleased")]
    int_year_released: Option<String>,
    #[serde(rename = "strGenre")]
    str_genre: Option<String>,
    #[serde(rename = "strMusicBrainzID")]
    str_music_brainz_id: Option<String>,
    #[serde(rename = "strTrackThumb")]
    str_track_thumb: Option<String>,
    #[serde(rename = "strArtistThumb")]
    str_artist_thumb: Option<String>,
    #[serde(rename = "intTrackBPM")]
    int_track_bpm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudioDbArtistResponse {
    artists: Option<Vec<AudioDbArtist>>,
}

#[derive(Debug, Deserialize)]
struct AudioDbArtist {
    #[serde(rename = "strArtist")]
    str_artist: Option<String>,
    #[serde(rename = "strBiographyEN")]
    str_biography_en: Option<String>,
    #[serde(rename = "strGenre")]
    str_genre: Option<String>,
    #[serde(rename = "strWebsite")]
    str_website: Option<String>,
    #[serde(rename = "strArtistThumb")]
    str_artist_thumb: Option<String>,
    #[serde(rename = "strArtistFanart")]
    str_artist_fanart: Option<String>,
    #[serde(rename = "strArtistFanart2")]
    str_artist_fanart2: Option<String>,
    #[serde(rename = "strArtistBanner")]
    str_artist_banner: Option<String>,
    #[serde(rename = "intMembers")]
    int_members: Option<String>,
}

async fn fetch_image(client: &Client, url: &str) -> Option<Vec<u8>> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.bytes().await.ok().map(|b| b.to_vec())
}

pub async fn search(
    client: &Client,
    title: &str,
    artist: &str,
    api_key: &str,
) -> Option<EnrichedMetadata> {
    let key = if api_key.is_empty() { "2" } else { api_key };
    let url = format!(
        "{}/{}/searchtrack.php?s={}&t={}",
        AUDIODB_BASE,
        key,
        urlencoding::encode(artist),
        urlencoding::encode(title)
    );

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: AudioDbSearchResponse = resp.json().await.ok()?;
    let track = data.track?.into_iter().next()?;

    let title = track.str_track.clone();
    let artists = track
        .str_artist
        .as_deref()
        .map(|a| serde_json::to_string(&vec![a.to_string()]).unwrap_or_else(|_| "[]".to_string()));
    let album_artist = track.str_artist.clone();
    let album_name = track.str_album_stripped.or(track.str_album);
    let albums = album_name.map(|name| {
        serde_json::to_string(&vec![
            serde_json::json!({ "name": name, "track_number": null }),
        ])
        .unwrap_or_else(|_| "[]".to_string())
    });
    let year = track.int_year_released;
    let genres = track
        .str_genre
        .as_deref()
        .map(|g| serde_json::to_string(&vec![g.to_string()]).unwrap_or_else(|_| "[]".to_string()));
    let mbid = track.str_music_brainz_id;
    let bpm = track
        .int_track_bpm
        .as_deref()
        .and_then(|s| s.parse::<f32>().ok());

    let artwork = match track.str_track_thumb.or(track.str_artist_thumb) {
        Some(url) => fetch_image(client, &url).await,
        None => None,
    };

    Some(EnrichedMetadata {
        title,
        artists,
        album_artist,
        albums,
        year,
        genres,
        bpm,
        artwork,
        mbid,
        ..Default::default()
    })
}

pub async fn search_artist(client: &Client, artist: &str, api_key: &str) -> Option<EnrichedArtist> {
    let key = if api_key.is_empty() { "2" } else { api_key };
    let url = format!(
        "{}/{}/search.php?s={}",
        AUDIODB_BASE,
        key,
        urlencoding::encode(artist)
    );

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: AudioDbArtistResponse = resp.json().await.ok()?;
    let a = data.artists?.into_iter().next()?;

    let genres = a
        .str_genre
        .as_deref()
        .map(|g| serde_json::to_string(&vec![g.to_string()]).unwrap_or_else(|_| "[]".to_string()));

    let websites = a.str_website.as_deref().map(|w| {
        let url = if w.starts_with("http") {
            w.to_string()
        } else {
            format!("https://{}", w)
        };
        serde_json::to_string(&vec![url]).unwrap_or_else(|_| "[]".to_string())
    });

    let profile_art = match a.str_artist_thumb {
        Some(ref url) => fetch_image(client, url).await,
        None => None,
    };

    let banner_art = match a
        .str_artist_banner
        .or(a.str_artist_fanart)
        .or(a.str_artist_fanart2)
    {
        Some(ref url) => fetch_image(client, url).await,
        None => None,
    };

    Some(EnrichedArtist {
        about: a.str_biography_en,
        genres,
        websites,
        profile_art,
        banner_art,
    })
}

#[derive(Debug, Deserialize)]
struct AudioDbAlbumResponse {
    album: Option<Vec<AudioDbAlbum>>,
}

#[derive(Debug, Deserialize)]
struct AudioDbAlbum {
    #[serde(rename = "strAlbum")]
    str_album: Option<String>,
    #[serde(rename = "strArtist")]
    str_artist: Option<String>,
    #[serde(rename = "intYearReleased")]
    int_year_released: Option<String>,
    #[serde(rename = "strGenre")]
    str_genre: Option<String>,
    #[serde(rename = "strLabel")]
    str_label: Option<String>,
    #[serde(rename = "strReleaseFormat")]
    str_release_format: Option<String>,
    #[serde(rename = "strDescriptionEN")]
    str_description_en: Option<String>,
    #[serde(rename = "strAlbumThumb")]
    str_album_thumb: Option<String>,
    #[serde(rename = "strAlbumThumbHQ")]
    str_album_thumb_hq: Option<String>,
}

pub async fn search_album(
    client: &Client,
    album: &str,
    artist: &str,
    api_key: &str,
) -> Option<EnrichedAlbum> {
    let key = if api_key.is_empty() { "2" } else { api_key };
    let url = format!(
        "{}/{}/searchalbum.php?s={}&a={}",
        AUDIODB_BASE,
        key,
        urlencoding::encode(artist),
        urlencoding::encode(album)
    );

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: AudioDbAlbumResponse = resp.json().await.ok()?;
    let a = data.album?.into_iter().next()?;

    let genres = a.str_genre.as_deref().map(|g| {
        let parts: Vec<String> = g.split('/').map(|s| s.trim().to_string()).collect();
        serde_json::to_string(&parts).unwrap_or_else(|_| "[]".to_string())
    });

    let artwork_url = a.str_album_thumb_hq.or(a.str_album_thumb);
    let artwork = match artwork_url {
        Some(ref url) => fetch_image(client, url).await,
        None => None,
    };

    Some(EnrichedAlbum {
        release_date: a.int_year_released,
        genres,
        label: a.str_label,
        format: a.str_release_format,
        description: a.str_description_en,
        artwork,
        mbid: None,
    })
}
