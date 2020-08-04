use colored::*;
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
            "{}: {}",
            self.shortcode.yellow(),
            match self.get_caption() {
                Some(cap) => cap.bright_blue(),
                None => "No caption".blue(),
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
    pub async fn images(self) -> impl Stream<Item = Image> {
        let images: Vec<Image> = self.edge_owner_to_timeline_media.images();

        stream::unfold(
            (images, self.edge_owner_to_timeline_media.page_info, self.id),
            |(mut images, next_page, id)| {
                async move {
                    match (images.pop(), next_page) {
                        (Some(image), next_page) => Some((image, (images, next_page, id))),
                        (
                            None,
                            PageInfo {
                                has_next_page: true,
                                end_cursor: Some(cursor),
                            },
                        ) => {
                            let (mut new_images, next_page) = get_more(id.clone(), cursor).await;
                            new_images.pop().map(|i| (i, (new_images, next_page, id)))
                        }
                        _ => None, // no more images, and no next page
                    }
                }
            },
        )
    }
}

async fn get_more(account_id: String, cursor: String) -> (Vec<Image>, PageInfo) {
    let url = format!("https://www.instagram.com/graphql/query/?query_hash=7437567ae0de0773fd96545592359a6b&variables={{\"id\":\"{}\",\"first\":50,\"after\":\"{}\"}}", account_id, cursor);
    let url = Url::parse(&url).unwrap();
    let resp: MoreRequest = surf::get(url).await.unwrap().body_json().await.unwrap();
    (
        resp.data.user.edge_owner_to_timeline_media.images(),
        resp.data.user.edge_owner_to_timeline_media.page_info,
    )
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
