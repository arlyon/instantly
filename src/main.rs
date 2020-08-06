#![deny(unsafe_code, clippy::unwrap_used)]

use anyhow::{anyhow, Context, Result};
use async_std::{fs::create_dir_all, path::PathBuf};
use clap::Clap;
use colored::*;
use futures::stream::StreamExt;
use human_panic::setup_panic;
use std::rc::Rc;
use url::Url;

use crate::data::ProfileData;
use crate::data::User;
use crate::util::{download_image, DownloadStatus};

mod data;
mod util;

/// Download instagram photos instantly. To download multiple
/// pages of pictures, you need the 'query hash' which can be
/// obtained from an official instagram request. It's generated
/// in-client, and changes frequently.
#[derive(Clap)]
#[clap(version = "1.0", author = "Alexander Lyon <alex@arlyon.dev>")]
struct Opts {
    /// The username to fetch.
    username: String,

    /// Forces download, overwriting any files that may already exist locally.
    #[clap(short, long)]
    force: bool,

    /// The query hash to use when requesting the images.
    #[clap(short, long)]
    query_hash: Option<String>,

    /// Maximum number of photos to download simultaneously.
    #[clap(long, default_value = "16")]
    buffer_size: usize,
}

#[async_std::main]
async fn main() -> Result<()> {
    setup_panic!();
    let Opts {
        force,
        username,
        query_hash,
        buffer_size,
    } = Opts::parse();

    let x = PathBuf::from(username.clone());
    create_dir_all(&x).await?;
    let folder: Rc<PathBuf> = Rc::from(x);

    let insta = Instagram::new();
    let my_images = insta
        .user(username)
        .await
        .with_context(|| "User not found.")?
        .images(query_hash);

    my_images
        .await
        .map(|image| {
            let folder = folder.clone();
            async move {
                match download_image(&image, &folder, force).await {
                    Ok(DownloadStatus::Downloaded) => println!("Downloaded:     {}", image),
                    Ok(DownloadStatus::ReDownloaded) => println!("Re-downloaded:  {}", image),
                    Ok(DownloadStatus::AlreadyExists) => println!("Already Exists: {}", image),
                    Err(e) => println!("Couldn't download {}: {}", image.shortcode.yellow(), e),
                }
            }
        })
        .buffer_unordered(buffer_size)
        .collect::<Vec<()>>()
        .await;

    Ok(())
}

struct Instagram {}

impl Instagram {
    fn new() -> Self {
        Instagram {}
    }

    async fn user<T: Into<String>>(self, user: T) -> Result<User> {
        let urlstr = format!("https://www.instagram.com/{}/", user.into());
        let url = Url::parse(&urlstr)?;
        let mut resp = surf::get(url)
            .await
            .map_err(|err| anyhow!(err))
            .context("Failed to reach instagram.")?;

        let data: ProfileData = resp
            .body_string()
            .await
            .map_err(|err| anyhow!(err))
            .context("Failed to extract body from response.")?
            .lines()
            .filter_map(|s| {
                let x = s
                    .trim()
                    .trim_start_matches("<script type=\"text/javascript\">window._sharedData = ")
                    .trim_end_matches(";</script>");
                serde_json::from_str(x).ok()
            })
            .next()
            .ok_or_else(|| anyhow!("Page did not include profile data. Does the user exist?"))?;

        data.entry_data
            .profile_page
            .first()
            .map(|p| p.graphql.user.clone())
            .ok_or_else(|| anyhow!("No user found in profile data."))
    }
}
