use anyhow::{anyhow, Result};
use async_std::{fs::File, io::copy, path::PathBuf};
use futures::try_join;

use crate::data::Image;

pub enum DownloadStatus {
    Downloaded,
    AlreadyExists,
}

pub async fn download_image(
    image: &Image,
    folder: &PathBuf,
    force: bool,
) -> Result<DownloadStatus> {
    let file_name = folder.join(format!("{}.jpg", image.shortcode));
    if !force && file_name.exists().await {
        return Ok(DownloadStatus::AlreadyExists);
    };

    let (reader, mut writer) = try_join!(
        async { surf::get(&image.url).await.map_err(|e| anyhow!(e)) },
        async { File::create(&file_name).await.map_err(|e| anyhow!(e)) },
    )?;

    copy(reader, &mut writer)
        .await
        .map(|_| DownloadStatus::Downloaded)
        .map_err(|e| anyhow!(e))
}
