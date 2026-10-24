use std::{collections::BTreeMap, time::Duration};

use reqwest::{StatusCode, Url, header};
use serde::{Serialize, de::DeserializeOwned};

use crate::models::{catalog::*, region::RegionListResponse};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid catalog configuration: {0}")]
    Config(String),
    #[error("catalog request failed ({endpoint}): {source}")]
    Http {
        endpoint: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("invalid catalog response: {0}")]
    Protocol(String),
    #[error("catalog snapshot I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("catalog snapshot JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalog snapshot worker: {0}")]
    Worker(#[from] tokio::task::JoinError),
}

/// `base_url` is the stand's origin (or deployment prefix), without `/api`.
/// Authentication is optional; the supplied document does not specify it.
#[derive(Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub locale: String,
    pub bearer_token: Option<String>,
    pub timeout: Duration,
    pub retries: u32,
    pub retry_delay: Duration,
    pub max_pages: u64,
}

impl ClientConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            locale: "ru".into(),
            bearer_token: None,
            timeout: Duration::from_secs(30),
            retries: 2,
            retry_delay: Duration::from_millis(500),
            max_pages: 10_000,
        }
    }
}

#[derive(Clone)]
pub struct CatalogClient {
    http: reqwest::Client,
    base_url: Url,
    config: ClientConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductQuery {
    pub category_id: u64,
    pub include_subsections: u8,
    pub include_categories_tree: u8,
    pub per_page: u64,
    pub page: u64,
}

impl ProductQuery {
    pub fn new(category_id: u64, per_page: u64) -> Self {
        Self {
            category_id,
            include_subsections: 0,
            include_categories_tree: 1,
            per_page,
            page: 1,
        }
    }
}

impl CatalogClient {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let mut base_url = Url::parse(&config.base_url)
            .map_err(|_| Error::Config("base_url must be an absolute HTTP(S) URL".into()))?;
        if !matches!(base_url.scheme(), "http" | "https")
            || base_url.host_str().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            return Err(Error::Config(
                "base_url must be HTTP(S), without credentials, query or fragment".into(),
            ));
        }
        if config.locale.is_empty() || config.timeout.is_zero() || config.max_pages == 0 {
            return Err(Error::Config(
                "locale, timeout and max_pages must be nonempty/positive".into(),
            ));
        }
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        if let Some(token) = &config.bearer_token {
            let mut value = header::HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_| Error::Config("invalid bearer token".into()))?;
            value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, value);
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .connect_timeout(config.timeout.min(Duration::from_secs(10)))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|source| Error::Http {
                endpoint: "client initialization".into(),
                source,
            })?;
        Ok(Self {
            http,
            base_url,
            config,
        })
    }

    async fn get<T: DeserializeOwned, Q: Serialize + ?Sized>(
        &self,
        endpoint: &str,
        query: &Q,
    ) -> Result<T, Error> {
        let url = self
            .base_url
            .join(endpoint)
            .map_err(|_| Error::Config("invalid endpoint URL".into()))?;
        let fail = |source: reqwest::Error| Error::Http {
            endpoint: endpoint.into(),
            source,
        };
        for attempt in 0..=self.config.retries {
            let response = self.http.get(url.clone()).query(query).send().await;
            let (retry, delay) = match &response {
                Ok(response) => (
                    response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error(),
                    response
                        .headers()
                        .get(header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(Duration::from_secs),
                ),
                Err(error) => (error.is_timeout() || error.is_connect(), None),
            };
            if retry && attempt < self.config.retries {
                let delay = delay.unwrap_or(self.config.retry_delay.saturating_mul(attempt + 1));
                // Do not retry earlier than a long server-requested delay; return the error instead.
                if delay <= Duration::from_secs(30) {
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
            let response = response.map_err(fail)?.error_for_status().map_err(fail)?;
            if !response.status().is_success() {
                return Err(Error::Protocol(format!(
                    "{endpoint}: unexpected HTTP status {}",
                    response.status()
                )));
            }
            return response.json().await.map_err(fail);
        }
        unreachable!("the final attempt always returns")
    }

    pub async fn category_tree(
        &self,
        category_id: u64,
        depth_level: u32,
    ) -> Result<CategoryTreeResponse, Error> {
        self.get(
            "api/catalog/categories/tree",
            &[
                ("category_id", category_id.to_string()),
                ("depth_level", depth_level.to_string()),
                ("locale", self.config.locale.clone()),
            ],
        )
        .await
    }

    pub async fn categories(
        &self,
        parent_id: u64,
        depth_level: u32,
    ) -> Result<CategoryListResponse, Error> {
        self.get(
            "api/catalog/categories",
            &[
                ("parent_id", parent_id.to_string()),
                ("depth_level", depth_level.to_string()),
            ],
        )
        .await
    }

    pub async fn products_page(&self, query: &ProductQuery) -> Result<ProductListResponse, Error> {
        if query.page == 0
            || query.per_page == 0
            || query.include_subsections > 1
            || query.include_categories_tree > 1
        {
            return Err(Error::Config(
                "invalid product pagination or inclusion flags".into(),
            ));
        }
        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(flatten)]
            products: &'a ProductQuery,
            locale: &'a str,
        }
        self.get(
            "api/catalog/products",
            &Query {
                products: query,
                locale: &self.config.locale,
            },
        )
        .await
    }

    /// Requires pagination metadata: a missing `last_page` must never silently truncate a sync.
    pub async fn products(&self, mut query: ProductQuery) -> Result<Vec<Product>, Error> {
        query.page = 1;
        let mut products = BTreeMap::new();
        loop {
            let response = self.products_page(&query).await?;
            let meta = response
                .meta
                .ok_or_else(|| Error::Protocol("missing product meta".into()))?;
            let last = meta
                .last_page
                .ok_or_else(|| Error::Protocol("missing last_page".into()))?;
            if meta.current_page != Some(query.page)
                || last > self.config.max_pages
                || (last < query.page
                    && !(last == 0 && query.page == 1 && response.items.is_empty()))
                || (query.page < last && response.items.is_empty())
                || (response.items.is_empty() && meta.total.is_some_and(|total| total > 0))
            {
                return Err(Error::Protocol(format!(
                    "invalid pagination for category {}, page {}",
                    query.category_id, query.page
                )));
            }
            for mut product in response.items {
                if let Some(previous) = products.remove(&product.id) {
                    super::merge_category_context(&mut product, previous);
                }
                products.insert(product.id, product);
            }
            if query.page >= last {
                return Ok(products.into_values().collect());
            }
            query.page += 1;
        }
    }

    pub async fn product(&self, id: u64) -> Result<Product, Error> {
        let response: ProductDetailsResponse = self
            .get(
                &format!("api/catalog/products/{id}"),
                &[("locale", &self.config.locale)],
            )
            .await?;
        if response.data.id != id {
            return Err(Error::Protocol(format!(
                "requested product {id}, received {}",
                response.data.id
            )));
        }
        Ok(response.data)
    }

    pub async fn regions(&self) -> Result<RegionListResponse, Error> {
        self.get("api/regions", &[] as &[(&str, &str)]).await
    }
}
