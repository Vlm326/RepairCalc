use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri},
    response::IntoResponse,
};
use repair_calc::catalog::{
    CatalogClient, CatalogSnapshot, ClientConfig, Error, ProductQuery, SyncOptions,
    synchronize_to_file,
};
use serde_json::{Value, json};

struct Reply {
    path: String,
    query: Vec<(String, String)>,
    status: StatusCode,
    body: Value,
    delay: Duration,
    authorization: Option<String>,
}

fn reply(path: &str, query: &[(&str, &str)], body: Value) -> Reply {
    Reply {
        path: path.into(),
        query: query
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        status: StatusCode::OK,
        body,
        delay: Duration::ZERO,
        authorization: None,
    }
}

struct Stand {
    base_url: String,
    replies: Arc<Mutex<VecDeque<Reply>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Stand {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Stand {
    async fn new(replies: Vec<Reply>) -> Self {
        async fn handle(
            State(replies): State<Arc<Mutex<VecDeque<Reply>>>>,
            uri: Uri,
            headers: HeaderMap,
        ) -> impl IntoResponse {
            let expected = replies
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected HTTP request");
            assert_eq!(uri.path(), expected.path);
            let url = reqwest::Url::parse(&format!("http://localhost{uri}")).unwrap();
            let mut actual: Vec<_> = url
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();
            let mut query = expected.query;
            query.sort();
            actual.sort();
            assert_eq!(actual, query, "query for {uri}");
            assert_eq!(
                headers
                    .get("authorization")
                    .map(|v| v.to_str().unwrap().to_string()),
                expected.authorization
            );
            assert_eq!(headers["accept"], "application/json");
            tokio::time::sleep(expected.delay).await;
            (expected.status, axum::Json(expected.body))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        let app = Router::new().fallback(handle).with_state(replies.clone());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            base_url,
            replies,
            task,
        }
    }

    fn config(&self) -> ClientConfig {
        let mut config = ClientConfig::new(&self.base_url);
        config.retries = 0;
        config.retry_delay = Duration::ZERO;
        config
    }

    fn done(&self) {
        assert!(
            self.replies.lock().unwrap().is_empty(),
            "unconsumed requests"
        );
    }
}

fn products(category: &str, page: &str, items: Value, last: u64) -> Reply {
    reply(
        "/api/catalog/products",
        &[
            ("category_id", category),
            ("page", page),
            ("per_page", "20"),
            ("include_subsections", "0"),
            ("include_categories_tree", "1"),
            ("locale", "ru"),
        ],
        json!({"items":items,"properties":[],"meta":{"current_page":page.parse::<u64>().unwrap(),"last_page":last,"per_page":20}}),
    )
}

fn tree() -> Reply {
    reply(
        "/api/catalog/categories/tree",
        &[
            ("category_id", "1"),
            ("depth_level", "20"),
            ("locale", "ru"),
        ],
        json!({"data":[
            {"id":2,"parent_id":1,"name":"Parent","depth_level":1,"product_count":1,"children":[
                {"id":33,"parent_id":2,"name":"Child","depth_level":2,"children":[]}
            ]}
        ]}),
    )
}

fn initial_replies() -> Vec<Reply> {
    vec![
        tree(),
        reply(
            "/api/catalog/categories",
            &[("parent_id", "1"), ("depth_level", "1")],
            json!({"data":[{"id":2,"parent_id":1,"depth_level":1,"name":"Parent","description":"Parent description"}]}),
        ),
        reply(
            "/api/catalog/categories",
            &[("parent_id", "2"), ("depth_level", "2")],
            json!({"data":[{"id":33,"parent_id":2,"depth_level":2,"name":"Child","description":"Child description"}]}),
        ),
        reply(
            "/api/regions",
            &[],
            json!({"data":[{"id":1,"name":"Region","code":"spb"}]}),
        ),
    ]
}

#[tokio::test]
async fn complete_sync_paginates_fetches_cards_and_replaces_snapshot() {
    let mut replies = initial_replies();
    replies.extend([
        products("1","1", json!([]),0),
        products("2","1",json!([{"id":96611,"name":"Summary","categories_tree":[{"id":2,"name":"Parent","depth":1}]}]),2),
        // Duplicate on page 2 must not trigger a second detail request.
        products("2","2",json!([{"id":96611,"name":"Summary"}]),2),
        reply("/api/catalog/products/96611", &[("locale","ru")], json!({"data":{
            "id":96611,"sku":"0007","cod_good":"0099","name":"Full card","description":"Full description",
            "weight":28.5,"price":1800,"result_price":1650,"special_price":1500,"special_price_active":false,
            "characteristics":[{"code":"color","name":"Color","value":"Silver"}],
            "stocks":[{"region_id":1,"qty":12}]
        }})),
        products("33","1",json!([{"id":96611,"name":"Summary","categories":{"33":"Child"}}]),1),
    ]);
    let stand = Stand::new(replies).await;
    let client = CatalogClient::new(stand.config()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/catalog.json");
    let snapshot = synchronize_to_file(&client, &SyncOptions::default(), path.clone())
        .await
        .unwrap();
    assert_eq!(snapshot.products.len(), 1);
    assert_eq!(snapshot.categories.len(), 2);
    assert_eq!(
        snapshot.categories[&33].description.as_deref(),
        Some("Child description")
    );
    assert_eq!(
        snapshot.categories_tree[0].children[0]
            .description
            .as_deref(),
        Some("Child description")
    );
    let product = &snapshot.products[&96611];
    assert_eq!(product.sku.as_deref(), Some("0007"));
    assert_eq!(product.cod_good.as_deref(), Some("0099"));
    assert_eq!(product.description.as_deref(), Some("Full description"));
    assert_eq!(product.result_price, Some(1650.0));
    assert_eq!(product.weight, Some(28.5));
    assert_eq!(product.characteristics.as_ref().unwrap()[0].value, "Silver");
    assert_eq!(product.categories_tree.as_ref().unwrap()[0].id, 2);
    assert_eq!(product.categories.as_ref().unwrap()["33"], "Child");
    assert!(
        snapshot
            .regions
            .contains_key(&product.stocks.as_ref().unwrap()[0].region_id.unwrap())
    );
    assert_eq!(CatalogSnapshot::load(&path).unwrap().products.len(), 1);
    stand.done();

    // The next successful snapshot reflects source removals rather than accumulating stale items.
    let stand = Stand::new(vec![
        reply(
            "/api/catalog/categories/tree",
            &[
                ("category_id", "1"),
                ("depth_level", "20"),
                ("locale", "ru"),
            ],
            json!({"data":[]}),
        ),
        reply("/api/regions", &[], json!({"data":[]})),
        products("1", "1", json!([]), 1),
    ])
    .await;
    synchronize_to_file(
        &CatalogClient::new(stand.config()).unwrap(),
        &SyncOptions::default(),
        path.clone(),
    )
    .await
    .unwrap();
    assert!(CatalogSnapshot::load(path).unwrap().products.is_empty());
    stand.done();
}

#[tokio::test]
async fn failed_card_preserves_previous_snapshot() {
    let mut replies = initial_replies();
    let mut failure = reply(
        "/api/catalog/products/7",
        &[("locale", "ru")],
        json!({"message":"unavailable"}),
    );
    failure.status = StatusCode::SERVICE_UNAVAILABLE;
    replies.extend([
        products("1", "1", json!([{"id":7,"name":"Product"}]), 1),
        failure,
    ]);
    let stand = Stand::new(replies).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.json");
    std::fs::write(&path, b"previous snapshot").unwrap();
    assert!(
        synchronize_to_file(
            &CatalogClient::new(stand.config()).unwrap(),
            &SyncOptions::default(),
            path.clone()
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), b"previous snapshot");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    stand.done();
}

#[tokio::test]
async fn pagination_rejects_missing_meta_wrong_page_empty_middle_and_page_limit() {
    let responses = [
        json!({"items":[]}),
        json!({"items":[],"meta":{"current_page":1}}),
        json!({"items":[],"meta":{"current_page":2,"last_page":2}}),
        json!({"items":[],"meta":{"current_page":1,"last_page":2}}),
        json!({"items":[{"id":1,"name":"P"}],"meta":{"current_page":1,"last_page":10001}}),
    ];
    for body in responses {
        let mut response = products("33", "1", json!([]), 1);
        response.body = body;
        let stand = Stand::new(vec![response]).await;
        let error = CatalogClient::new(stand.config())
            .unwrap()
            .products(ProductQuery::new(33, 20))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Protocol(_)), "{error}");
        stand.done();
    }
}

#[tokio::test]
async fn transient_statuses_retry_but_authentication_errors_do_not() {
    for status in [
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        let mut failure = reply("/api/regions", &[], json!({}));
        failure.status = status;
        let stand = Stand::new(vec![
            failure,
            reply("/api/regions", &[], json!({"data":[]})),
        ])
        .await;
        let mut config = stand.config();
        config.retries = 1;
        CatalogClient::new(config).unwrap().regions().await.unwrap();
        stand.done();
    }
    let mut failure = reply("/api/regions", &[], json!({}));
    failure.status = StatusCode::UNAUTHORIZED;
    let stand = Stand::new(vec![failure]).await;
    let mut config = stand.config();
    config.retries = 2;
    assert!(
        matches!(CatalogClient::new(config).unwrap().regions().await,Err(Error::Http{source,..}) if source.status() == Some(StatusCode::UNAUTHORIZED))
    );
    stand.done();
}

#[tokio::test]
async fn token_locale_and_deployment_prefix_are_sent_correctly() {
    let mut response = reply(
        "/stand/api/catalog/products/96611",
        &[("locale", "ru RU")],
        json!({"data":{"id":96611,"name":"P"}}),
    );
    response.authorization = Some("Bearer test-token".into());
    let stand = Stand::new(vec![response]).await;
    let mut config = stand.config();
    config.base_url.push_str("/stand");
    config.locale = "ru RU".into();
    config.bearer_token = Some("test-token".into());
    assert_eq!(
        CatalogClient::new(config)
            .unwrap()
            .product(96611)
            .await
            .unwrap()
            .id,
        96611
    );
    stand.done();
}

#[tokio::test]
async fn malformed_payload_wrong_product_id_and_timeouts_fail() {
    for body in [
        json!({"invalid":true}),
        json!({"data":{"id":8,"name":"Wrong ID"}}),
    ] {
        let stand = Stand::new(vec![reply(
            "/api/catalog/products/7",
            &[("locale", "ru")],
            body,
        )])
        .await;
        assert!(
            CatalogClient::new(stand.config())
                .unwrap()
                .product(7)
                .await
                .is_err()
        );
        stand.done();
    }
    let mut response = reply("/api/regions", &[], json!({"data":[]}));
    response.delay = Duration::from_millis(100);
    let stand = Stand::new(vec![response]).await;
    let mut config = stand.config();
    config.timeout = Duration::from_millis(10);
    assert!(
        matches!(CatalogClient::new(config).unwrap().regions().await,Err(Error::Http{source,..}) if source.is_timeout())
    );
    stand.done();
}

#[test]
fn invalid_configuration_is_rejected() {
    for url in [
        "not a url",
        "ftp://localhost",
        "http://user:pass@localhost",
        "http://localhost?key=secret",
        "http://localhost#fragment",
    ] {
        assert!(CatalogClient::new(ClientConfig::new(url)).is_err());
    }
}
