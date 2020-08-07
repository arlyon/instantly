//! Various structs that map to return fields
//! from the instagram API. Twofold, allows us
//! to parse it type-safe using serde, while
//! also adding impls to structs as needed.

use anyhow::{anyhow, Result};
use colored::*;
use ellipse::Ellipse;
use futures::stream;
use futures::Stream;
use serde::Deserialize;
use std::fmt::Display;
use url::Url;

#[derive(Deserialize, Debug, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Dimensions {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Image {
    pub shortcode: String,
    pub dimensions: Dimensions,

    #[serde(rename = "display_url")]
    pub url: Url,
    edge_media_to_caption: EdgeMediaToCaption,
}

impl Image {
    fn get_caption(&self) -> Option<String> {
        self.edge_media_to_caption
            .edges
            .first()
            .map(|e| e.node.text.clone())
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{: >11} {}",
            self.shortcode.yellow(),
            match self.get_caption() {
                Some(cap) => cap
                    .replace("\n", "")
                    .as_str()
                    .truncate_ellipse(40)
                    .bright_blue(),
                None => "no caption".blue(),
            },
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Edge<T> {
    node: T,
}

#[derive(Deserialize, Debug, Clone)]
struct Caption {
    text: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EdgeMediaToCaption {
    edges: Vec<Edge<Caption>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EdgeOwnerToTimeline {
    pub count: u32,
    pub page_info: PageInfo,
    edges: Vec<Edge<Image>>,
}

impl EdgeOwnerToTimeline {
    pub fn images(&self) -> Vec<Image> {
        self.edges.iter().map(|e| e.node.clone()).collect()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub biography: String,
    pub username: String,
    pub id: String,
    pub profile_pic_url_hd: Url,
    pub edge_owner_to_timeline_media: EdgeOwnerToTimeline,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MoreRequestDataUser {
    pub edge_owner_to_timeline_media: EdgeOwnerToTimeline,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MoreRequestData {
    user: MoreRequestDataUser,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MoreRequest {
    data: MoreRequestData,
}

impl User {
    /// Fetching the images for a given user.
    /// If provided, the query string will allow fetching
    /// multiple pages of results, streaming the images back.
    pub async fn images(self, query_hash: Option<String>) -> impl Stream<Item = Image> {
        let images: Vec<Image> = self.edge_owner_to_timeline_media.images();

        stream::unfold(
            (
                images,
                self.edge_owner_to_timeline_media.page_info,
                self.id,
                query_hash,
            ),
            |(mut images, next_page, id, query_hash)| {
                async move {
                    match (images.pop(), next_page, query_hash) {
                        (Some(image), next_page, query_hash) => {
                            Some((image, (images, next_page, id, query_hash)))
                        }
                        (
                            None,
                            PageInfo {
                                has_next_page: true,
                                end_cursor: Some(cursor),
                            },
                            Some(query_hash),
                        ) => match get_more(&id, &cursor, &query_hash).await {
                            Ok((mut new_images, next_page)) => new_images
                                .pop()
                                .map(|i| (i, (new_images, next_page, id, Some(query_hash)))),
                            Err(e) => {
                                println!("{}", e.context("Could not fetch next page of images!"));
                                None
                            }
                        },
                        _ => None, // no more images, and no next page
                    }
                }
            },
        )
    }
}

async fn get_more(
    account_id: &String,
    cursor: &String,
    query_hash: &String,
) -> Result<(Vec<Image>, PageInfo)> {
    let url = format!("https://www.instagram.com/graphql/query/?query_hash={}&variables={{\"id\":\"{}\",\"first\":50,\"after\":\"{}\"}}", query_hash, account_id, cursor);
    let url = Url::parse(&url)?;
    let MoreRequest { data } = surf::get(url)
        .await
        .map_err(|e| anyhow!(e))?
        .body_json()
        .await?;
    Ok((
        data.user.edge_owner_to_timeline_media.images(),
        data.user.edge_owner_to_timeline_media.page_info,
    ))
}

#[derive(Deserialize, Debug)]
pub struct GraphQl {
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct ProfilePage {
    pub graphql: GraphQl,
}

#[derive(Deserialize, Debug)]
pub struct EntryData {
    #[serde(rename = "ProfilePage")]
    pub profile_page: Vec<ProfilePage>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ProfileData {
    pub entry_data: EntryData,
}
