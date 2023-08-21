use std::collections::HashMap;
use std::fmt::Debug;

use crate::{
    cell::BagOfCells,
    ipfs::{IpfsLoader, IpfsLoaderConfig},
};
use async_trait::async_trait;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_traits::Num;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub use jetton::*;
pub use nft_collection::*;
pub use nft_item::*;

mod jetton;
mod nft_collection;
mod nft_item;

lazy_static! {
    static ref META_NAME: MetaDataField = MetaDataField::new("name");
    static ref META_DESCRIPTION: MetaDataField = MetaDataField::new("description");
    static ref META_IMAGE: MetaDataField = MetaDataField::new("image");
    static ref META_SYMBOL: MetaDataField = MetaDataField::new("symbol");
    static ref META_IMAGE_DATA: MetaDataField = MetaDataField::new("image_data");
    static ref META_DECIMALS: MetaDataField = MetaDataField::new("decimals");
    static ref META_URI: MetaDataField = MetaDataField::new("uri");
    static ref META_CONTENT_URL: MetaDataField = MetaDataField::new("content_url");
    static ref META_ATTRIBUTES: MetaDataField = MetaDataField::new("attributes");
    static ref META_SOCIAL_LINKS: MetaDataField = MetaDataField::new("social_links");
    static ref META_MARKETPLACE: MetaDataField = MetaDataField::new("marketplace");
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MetaDataContent {
    External { uri: String },
    Internal { dict: HashMap<String, String> },
    Unsupported { boc: BagOfCells },
}
struct MetaDataField {
    pub(crate) key: String,
}

impl MetaDataField {
    fn new(name: &str) -> MetaDataField {
        MetaDataField {
            key: Self::key_from_str(name),
        }
    }

    fn key_from_str(k: &str) -> String {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(k);
        let s = hex::encode(hasher.finalize()[..].to_vec());
        BigInt::from_str_radix(s.as_str(), 16)
            .unwrap()
            .to_str_radix(10)
    }
}

pub struct MetaLoader<'a, MetaData>
where
    MetaData: for<'de> Deserialize<'de>,
{
    http_client: reqwest::Client,
    ipfs_loader: IpfsLoader,
    meta_data_marker: std::marker::PhantomData<&'a MetaData>,
}
pub type JettonMetaLoader<'a> = MetaLoader<'a, JettonMetaData>;
pub type NftItemMetaLoader<'a> = MetaLoader<'a, NftItemMetaData>;
pub type NftColletionMetaLoader<'a> = MetaLoader<'a, NftCollectionMetaData>;

impl<'a, MetaData> MetaLoader<'a, MetaData>
where
    MetaData: for<'de> Deserialize<'de>,
{
    pub fn new(ipfs_loader_config: &IpfsLoaderConfig) -> anyhow::Result<MetaLoader<MetaData>> {
        let http_client = reqwest::Client::builder().build()?;
        let ipfs_loader = IpfsLoader::new(ipfs_loader_config)?; // Replace with actual initialization
        Ok(MetaLoader {
            http_client,
            ipfs_loader,
            meta_data_marker: std::marker::PhantomData,
        })
    }

    pub fn default() -> anyhow::Result<MetaLoader<'a, MetaData>> {
        let http_client = reqwest::Client::builder().build()?;
        let ipfs_loader = IpfsLoader::new(&IpfsLoaderConfig::default())?; // Replace with actual initialization
        Ok(MetaLoader {
            http_client,
            ipfs_loader,
            meta_data_marker: std::marker::PhantomData,
        })
    }

    pub async fn load_meta_from_uri(&self, uri: &str) -> anyhow::Result<MetaData> {
        log::trace!("Downloading metadata from {}", uri);
        let meta_str: String = if uri.starts_with("ipfs://") {
            let path: String = uri.chars().into_iter().skip(7).collect();
            self.ipfs_loader.load_utf8(path.as_str()).await?
        } else {
            let resp = self.http_client.get(uri).send().await?;
            if resp.status().is_success() {
                resp.text().await?
            } else {
                anyhow::bail!(
                    "Failed to load jetton metadata from {}. Resp status: {}",
                    uri,
                    resp.status()
                );
            }
        };

        // Deserialize using the original meta_str
        let meta: MetaData = serde_json::from_str(&meta_str)?;

        Ok(meta)
    }
}

#[async_trait]
pub trait LoadMeta<T>
where
    T: for<'de> Deserialize<'de>,
{
    async fn load(&self, content: MetaDataContent) -> anyhow::Result<T>;
}