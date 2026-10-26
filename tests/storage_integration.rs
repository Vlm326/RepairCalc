//! Run explicitly against a PostgreSQL role allowed to CREATE DATABASE.
use repair_calc::storage::*;
use rust_decimal::Decimal;
use sqlx::PgPool;

fn d(s: &str) -> Decimal {
    s.parse().unwrap()
}
fn product() -> ProductInput {
    ProductInput {
        sku: "paint-1".into(),
        name: "Paint".into(),
        description: "Washable".into(),
        unit: "l".into(),
        package_quantity: d("2.5"),
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires PostgreSQL and DATABASE_URL; run with --ignored"]
async fn catalog_crud(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = CatalogStore::new(pool);
    store.migrate().await?; // Applied migrations are safe to run again.
    let mut ci = CategoryInput {
        name: "Paints".into(),
        description: "Interior".into(),
    };
    let category = store.create_category(&ci).await?;
    ci.name = "Coatings".into();
    assert_eq!(
        store.update_category(category.id, &ci).await?.unwrap().name,
        "Coatings"
    );
    assert_eq!(
        store.get_category(category.id).await?.unwrap().name,
        "Coatings"
    );
    assert_eq!(store.list_categories(10, 0).await?.len(), 1);

    let mut pi = product();
    let p = store.create_product(&pi).await?;
    pi.name = "White paint".into();
    assert_eq!(
        store.update_product(p.id, &pi).await?.unwrap().name,
        "White paint"
    );
    assert_eq!(
        store.get_product(p.id).await?.unwrap().package_quantity,
        d("2.5")
    );
    assert_eq!(store.list_products(10, 0).await?.len(), 1);
    assert!(store.list_products(10, 1).await?.is_empty());
    store.add_category(p.id, category.id).await?;
    store.add_category(p.id, category.id).await?;
    assert_eq!(
        store.products_in_category(category.id, 10, 0).await?.len(),
        1
    );
    assert!(store.remove_category(p.id, category.id).await?);
    assert!(!store.remove_category(p.id, category.id).await?);
    store.add_category(p.id, category.id).await?;

    let mut ai = CharacteristicInput {
        code: "color".into(),
        name: "Color".into(),
        unit: None,
    };
    let a = store.create_characteristic(&ai).await?;
    ai.name = "Colour".into();
    assert_eq!(
        store.update_characteristic(a.id, &ai).await?.unwrap().name,
        "Colour"
    );
    assert_eq!(store.get_characteristic(a.id).await?.unwrap().code, "color");
    assert_eq!(store.list_characteristics(10, 0).await?.len(), 1);

    let mut vi = ProductCharacteristicInput {
        product_id: p.id,
        characteristic_id: a.id,
        value: "white".into(),
    };
    let v = store.create_product_characteristic(&vi).await?;
    vi.value = "ivory".into();
    assert_eq!(
        store
            .update_product_characteristic(v.id, &vi)
            .await?
            .unwrap()
            .value,
        "ivory"
    );
    assert_eq!(
        store.get_product_characteristic(v.id).await?.unwrap().value,
        "ivory"
    );
    assert_eq!(store.list_product_characteristics(10, 0).await?.len(), 1);
    assert!(store.delete_characteristic(a.id).await.is_err()); // referenced definition
    assert!(store.delete_product_characteristic(v.id).await?);
    assert!(store.delete_characteristic(a.id).await?);

    let mut price = PriceInput {
        product_id: p.id,
        region: "spb".into(),
        currency: "RUB".into(),
        amount: d("1200.01"),
    };
    let price_row = store.create_price(&price).await?;
    price.amount = d("1300.99");
    assert_eq!(
        store
            .update_price(price_row.id, &price)
            .await?
            .unwrap()
            .amount,
        d("1300.99")
    );
    assert_eq!(
        store.get_price(price_row.id).await?.unwrap().amount,
        d("1300.99")
    );
    assert_eq!(store.list_prices(10, 0).await?.len(), 1);
    assert!(store.create_price(&price).await.is_err());
    assert!(store.delete_price(price_row.id).await?);
    store.create_price(&price).await?;

    let mut stock = StockInput {
        product_id: p.id,
        region: "spb".into(),
        warehouse: "north".into(),
        quantity: d("4"),
    };
    let stock_row = store.create_stock(&stock).await?;
    stock.quantity = d("3.5");
    assert_eq!(
        store
            .update_stock(stock_row.id, &stock)
            .await?
            .unwrap()
            .quantity,
        d("3.5")
    );
    assert_eq!(
        store.get_stock(stock_row.id).await?.unwrap().quantity,
        d("3.5")
    );
    assert_eq!(store.list_stocks(10, 0).await?.len(), 1);
    assert!(store.delete_stock(stock_row.id).await?);
    store.create_stock(&stock).await?;

    assert!(store.delete_product(p.id).await?);
    assert!(!store.delete_product(p.id).await?);
    assert!(store.get_product(p.id).await?.is_none());
    assert!(store.update_product(p.id, &pi).await?.is_none());
    assert!(store.list_prices(10, 0).await?.is_empty());
    assert!(store.list_stocks(10, 0).await?.is_empty());
    assert!(
        store
            .products_in_category(category.id, 10, 0)
            .await?
            .is_empty()
    );
    assert!(store.delete_category(category.id).await?);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires PostgreSQL and DATABASE_URL; run with --ignored"]
async fn constraints_and_cascade(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let store = CatalogStore::new(pool);
    let mut input = product();
    input.package_quantity = d("0");
    assert!(store.create_product(&input).await.is_err());
    input.package_quantity = d("1");
    input.name = "  ".into();
    assert!(store.create_product(&input).await.is_err());
    let p = store.create_product(&product()).await?;
    assert!(store.create_product(&product()).await.is_err());
    let mut price = PriceInput {
        product_id: p.id,
        region: "spb".into(),
        currency: "RUB".into(),
        amount: d("-1"),
    };
    assert!(store.create_price(&price).await.is_err());
    price.amount = d("0");
    price.currency = "ruble".into();
    assert!(store.create_price(&price).await.is_err());
    price.currency = "RUB".into();
    price.product_id = -1;
    assert!(store.create_price(&price).await.is_err());
    let stock = StockInput {
        product_id: p.id,
        region: "spb".into(),
        warehouse: "a".into(),
        quantity: d("-1"),
    };
    assert!(store.create_stock(&stock).await.is_err());
    let a = store
        .create_characteristic(&CharacteristicInput {
            code: "size".into(),
            name: "Size".into(),
            unit: Some("m".into()),
        })
        .await?;
    store
        .create_product_characteristic(&ProductCharacteristicInput {
            product_id: p.id,
            characteristic_id: a.id,
            value: "2".into(),
        })
        .await?;
    store.delete_product(p.id).await?;
    assert!(store.list_product_characteristics(10, 0).await?.is_empty());
    assert!(store.delete_characteristic(a.id).await?);
    Ok(())
}
