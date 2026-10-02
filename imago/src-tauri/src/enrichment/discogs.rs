use super::EnrichedMetadata;
use reqwest::Client;
use serde::Deserialize;

const DISCOGS_BASE: &str = "https://api.discogs.com";

#[derive(Debug, Deserialize)]
struct DiscogsSearchResponse {
    results: Option<Vec<DiscogsResult>>,
}

#[derive(Debug, Deserialize)]
struct DiscogsResult {
    title: Option<String>,
    year: Option<serde_json::Value>,
    label: Option<Vec<String>>,
    format: Option<Vec<String>>,
    genre: Option<Vec<String>>,
    thumb: Option<String>,
    resource_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscogsRelease {
    title: Option<String>,
    year: Option<u32>,
    labels: Option<Vec<DiscogsLabel>>,
    formats: Option<Vec<DiscogsFormat>>,
    genres: Option<Vec<String>>,
    images: Option<Vec<DiscogsImage>>,
    artists: Option<Vec<DiscogsArtistRef>>,
}

#[derive(Debug, Deserialize)]
struct DiscogsLabel {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscogsFormat {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscogsImage {
    #[serde(rename = "type")]
    image_type: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscogsArtistRef {
    name: Option<String>,
}

fn parse_year(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::Number(n) => {
            let y = n.as_u64()?;
            if y > 1000 {
                Some(y.to_string())
            } else {
                None
            }
        }
        serde_json::Value::String(s) => {
            if s.len() == 4 && s.parse::<u32>().is_ok() {
                Some(s.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

async fn fetch_image(client: &Client, url: &str, api_key: &str) -> Option<Vec<u8>> {
    let resp = client
        .get(url)
        .header("Authorization", format!("Discogs token={}", api_key))
        .send()
        .await
        .ok()?;
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

    let query = format!("{} {}", artist, title);
    let url = format!("{}/database/search", DISCOGS_BASE);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Discogs token={}", api_key))
        .query(&[
            ("q", query.as_str()),
            ("type", "release"),
            ("per_page", "1"),
        ])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let data: DiscogsSearchResponse = resp.json().await.ok()?;
    let result = data.results?.into_iter().next()?;

    let full: Option<DiscogsRelease> = if let Some(ref url) = result.resource_url {
        let resp = client
            .get(url)
            .header("Authorization", format!("Discogs token={}", api_key))
            .send()
            .await
            .ok()
            .filter(|r| r.status().is_success());
        if let Some(r) = resp {
            r.json().await.ok()
        } else {
            None
        }
    } else {
        None
    };

    let (album_name, artist_name) = if let Some(ref t) = result.title {
        if let Some(pos) = t.find(" - ") {
            (
                Some(t[pos + 3..].trim().to_string()),
                Some(t[..pos].trim().to_string()),
            )
        } else {
            (Some(t.clone()), None)
        }
    } else {
        (None, None)
    };

    let artists = artist_name
        .as_deref()
        .or_else(|| {
            full.as_ref()
                .and_then(|f| f.artists.as_ref()?.first()?.name.as_deref())
        })
        .map(|a| serde_json::to_string(&vec![a.to_string()]).unwrap_or_else(|_| "[]".to_string()));

    let album_artist = artist_name.clone().or_else(|| {
        full.as_ref()
            .and_then(|f| f.artists.as_ref()?.first()?.name.clone())
    });

    let albums = album_name.map(|name| {
        serde_json::to_string(&vec![
            serde_json::json!({ "name": name, "track_number": null }),
        ])
        .unwrap_or_else(|_| "[]".to_string())
    });

    let year = result
        .year
        .as_ref()
        .and_then(parse_year)
        .or_else(|| full.as_ref().and_then(|f| f.year.map(|y| y.to_string())));

    let genres = full
        .as_ref()
        .and_then(|f| f.genres.as_ref())
        .filter(|g| !g.is_empty())
        .or_else(|| result.genre.as_ref().filter(|g| !g.is_empty()))
        .map(|g| serde_json::to_string(g).unwrap_or_else(|_| "[]".to_string()));

    let label = full
        .as_ref()
        .and_then(|f| f.labels.as_ref()?.first()?.name.clone())
        .or_else(|| result.label.as_ref()?.first().cloned());

    let format = full
        .as_ref()
        .and_then(|f| f.formats.as_ref()?.first()?.name.clone())
        .or_else(|| result.format.as_ref()?.first().cloned());

    let artwork = if let Some(ref full_release) = full {
        let primary_url = full_release.images.as_ref().and_then(|imgs| {
            imgs.iter()
                .find(|i| i.image_type.as_deref() == Some("primary"))
                .or_else(|| imgs.first())
                .and_then(|i| i.uri.as_deref().map(|s| s.to_string()))
        });
        match primary_url {
            Some(url) => fetch_image(client, &url, api_key).await,
            None => None,
        }
    } else if let Some(ref thumb) = result.thumb {
        fetch_image(client, thumb, api_key).await
    } else {
        None
    };

    Some(EnrichedMetadata {
        artists,
        album_artist,
        albums,
        year,
        genres,
        label,
        format,
        artwork,
        ..Default::default()
    })
}
