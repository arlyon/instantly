use crate::data::ProfileData;
use crate::data::User;
use anyhow::{anyhow, Context, Result};
use async_std::{
    fs::{create_dir_all, File},
    path::PathBuf,
};
use futures::{io::copy, stream::StreamExt};
use std::{env, rc::Rc};
use url::Url;

mod data;

#[async_std::main]
async fn main() -> Result<()> {
    let username = env::args()
        .into_iter()
        .skip(1)
        .next()
        .context("No username provided")?;

    let x = PathBuf::from(username.clone());
    create_dir_all(&x).await?;
    let folder: Rc<PathBuf> = Rc::from(x);

    let insta = Instagram::new();
    let my_images = insta
        .user(username)
        .await
        .with_context(|| "User not found.")?
        .images();

    let stream = my_images.await.map(|image| {
        let folder = folder.clone();
        async move {
            let file_name = folder.join(format!("{}.jpg", image.shortcode));
            let mut file = File::create(&file_name).await.unwrap();
            let resp = surf::get(&image.url).await.unwrap();
            copy(resp, &mut file).await.unwrap();
            println!("Downloaded {}", image);
        }
    });

    stream.buffer_unordered(20).collect::<Vec<()>>().await;

    Ok(())
}

struct Instagram {}

impl Instagram {
    fn new() -> Self {
        Instagram {}
    }

    async fn user<T: Into<String>>(self, user: T) -> Result<User> {
        let urlstr = format!("https://www.instagram.com/{}/", user.into());
        let url = Url::parse(urlstr.as_str())?;
        let mut resp = surf::get(url)
            .await
            .map_err(|err| anyhow!(err))
            .context("Failed to reach instagram.")?;

        let data: ProfileData = resp
            .body_string()
            .await
            .unwrap()
            .lines()
            .filter_map(|s| {
                let x = s
                    .trim()
                    .trim_start_matches("<script type=\"text/javascript\">window._sharedData = ")
                    .trim_end_matches(";</script>");
                serde_json::from_str(x).ok()
            })
            .next()
            .ok_or(anyhow!(
                "Page did not include profile data. Does the user exist?"
            ))?;

        data.entry_data
            .profile_page
            .first()
            .map(|p| p.graphql.user.clone())
            .ok_or(anyhow!("No user found in profile data."))
    }
}
