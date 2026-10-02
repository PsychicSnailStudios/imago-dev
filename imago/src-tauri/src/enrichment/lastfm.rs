use super::{EnrichedArtist, EnrichedMetadata};
use reqwest::Client;
use serde::Deserialize;

const LASTFM_BASE: &str = "https://ws.audioscrobbler.com/2.0";

#[derive(Debug, Deserialize)]
struct LfmTrackResponse {
    track: Option<LfmTrack>,
}

#[derive(Debug, Deserialize)]
struct LfmTrack {
    name: Option<String>,
    artist: Option<LfmArtistShort>,
    album: Option<LfmAlbum>,
    toptags: Option<LfmTopTags>,
}

#[derive(Debug, Deserialize)]
struct LfmArtistShort {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LfmAlbum {
    title: Option<String>,
    image: Option<Vec<LfmImage>>,
}

#[derive(Debug, Deserialize)]
struct LfmImage {
    #[serde(rename = "#text")]
    text: String,
    size: String,
}

#[derive(Debug, Deserialize)]
struct LfmTopTags {
    tag: Option<Vec<LfmTag>>,
}

#[derive(Debug, Deserialize)]
struct LfmTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LfmArtistResponse {
    artist: Option<LfmArtistFull>,
}

#[derive(Debug, Deserialize)]
struct LfmArtistFull {
    bio: Option<LfmBio>,
    tags: Option<LfmTopTags>,
    image: Option<Vec<LfmImage>>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LfmBio {
    content: Option<String>,
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
    if api_key.is_empty() {
        return None;
    }

    let url = format!("{}/", LASTFM_BASE);

    let resp = client
        .get(&url)
        .query(&[
            ("method", "track.getInfo"),
            ("api_key", api_key),
            ("artist", artist),
            ("track", title),
            ("autocorrect", "1"),
            ("format", "json"),
        ])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let data: LfmTrackResponse = resp.json().await.ok()?;
    let track = data.track?;

    let title = track.name.clone();
    let artist_name = track.artist.as_ref().and_then(|a| a.name.clone());

    let artists = artist_name
        .as_deref()
        .map(|a| serde_json::to_string(&vec![a.to_string()]).unwrap_or_else(|_| "[]".to_string()));
    let album_artist = artist_name.clone();

    let album_name = track.album.as_ref().and_then(|a| a.title.clone());
    let albums = album_name.map(|name| {
        serde_json::to_string(&vec![
            serde_json::json!({ "name": name, "track_number": null }),
        ])
        .unwrap_or_else(|_| "[]".to_string())
    });

    let genres = track
        .toptags
        .as_ref()
        .and_then(|t| t.tag.as_ref())
        .filter(|tags| !tags.is_empty())
        .map(|tags| {
            let names: Vec<String> = tags.iter().take(3).map(|t| t.name.clone()).collect();
            serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
        });

    let artwork_url = track
        .album
        .as_ref()
        .and_then(|a| a.image.as_ref())
        .and_then(|imgs| {
            imgs.iter()
                .filter(|i| i.size == "extralarge" || i.size == "mega")
                .map(|i| i.text.as_str())
                .find(|url| !url.is_empty())
                .map(|s| s.to_string())
        });

    let artwork = match artwork_url {
        Some(url) => fetch_image(client, &url).await,
        None => None,
    };

    Some(EnrichedMetadata {
        title,
        artists,
        album_artist,
        albums,
        genres,
        artwork,
        ..Default::default()
    })
}

pub async fn search_artist(client: &Client, artist: &str, api_key: &str) -> Option<EnrichedArtist> {
    if api_key.is_empty() {
        return None;
    }

    let url = format!("{}/", LASTFM_BASE);

    let resp = client
        .get(&url)
        .query(&[
            ("method", "artist.getInfo"),
            ("api_key", api_key),
            ("artist", artist),
            ("autocorrect", "1"),
            ("format", "json"),
        ])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let data: LfmArtistResponse = resp.json().await.ok()?;
    let a = data.artist?;

    let about = a
        .bio
        .as_ref()
        .and_then(|b| b.content.as_deref())
        .map(|s| {
            if let Some(pos) = s.find("<a href") {
                s[..pos].trim().to_string()
            } else {
                s.trim().to_string()
            }
        })
        .filter(|s| !s.is_empty());

    let genres = a
        .tags
        .as_ref()
        .and_then(|t| t.tag.as_ref())
        .filter(|tags| !tags.is_empty())
        .map(|tags| {
            let names: Vec<String> = tags.iter().take(5).map(|t| t.name.clone()).collect();
            serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
        });

    let websites = a
        .url
        .as_deref()
        .map(|w| serde_json::to_string(&vec![w.to_string()]).unwrap_or_else(|_| "[]".to_string()));

    let profile_art_url = a.image.as_ref().and_then(|imgs| {
        imgs.iter()
            .filter(|i| i.size == "extralarge" || i.size == "mega")
            .map(|i| i.text.as_str())
            .find(|url| !url.is_empty())
            .map(|s| s.to_string())
    });

    let profile_art = match profile_art_url {
        Some(url) => fetch_image(client, &url).await,
        None => None,
    };

    Some(EnrichedArtist {
        about,
        genres,
        websites,
        profile_art,
        banner_art: None,
    })
}
