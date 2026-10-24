use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    mem,
    process::{Command, Stdio},
    str::FromStr,
};
use xml::{
    attribute::OwnedAttribute,
    reader::{EventReader, XmlEvent},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomCatalog {
    pub date: String,
    pub shop: MaxidomShop,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomShop {
    pub name: String,
    pub company: String,
    pub url: String,
    pub currencies: Vec<MaxidomCurrency>,
    pub categories: Vec<MaxidomCategory>,
    pub products: Vec<MaxidomProduct>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomCurrency {
    pub id: String,
    pub rate: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomCategory {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomProduct {
    pub id: String,
    pub available: Option<bool>,
    pub url: String,
    pub name: String,
    pub price: f64,
    pub currency_id: String,
    pub category_id: i64,
    pub pictures: Vec<String>,
    pub sales_notes: String,
    pub delivery: bool,
    pub local_delivery_cost: f64,
    pub vendor: String,
    pub country_of_origin: String,
    pub description: String,
    pub weight: f64,
    pub params: Vec<MaxidomProductParam>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MaxidomProductParam {
    pub name: String,
    pub unit: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
enum Field {
    ShopName,
    Company,
    ShopUrl,
    Category,
    Url,
    Name,
    Price,
    CurrencyId,
    CategoryId,
    Picture,
    SalesNotes,
    Delivery,
    LocalDeliveryCost,
    Vendor,
    CountryOfOrigin,
    Description,
    Weight,
    Param,
}

impl Field {
    fn product(tag: &str) -> Option<Self> {
        Some(match tag {
            "url" => Self::Url,
            "name" => Self::Name,
            "price" => Self::Price,
            "currencyId" => Self::CurrencyId,
            "categoryId" => Self::CategoryId,
            "picture" => Self::Picture,
            "sales_notes" => Self::SalesNotes,
            "delivery" => Self::Delivery,
            "local_delivery_cost" => Self::LocalDeliveryCost,
            "vendor" => Self::Vendor,
            "country_of_origin" => Self::CountryOfOrigin,
            "description" => Self::Description,
            "weight" => Self::Weight,
            "param" => Self::Param,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum State {
    Document,
    Catalog,
    Shop,
    Currencies,
    Categories,
    Offers,
    Offer,
    Text(Field),
    // Unknown elements are skipped with their entire subtree.
    Skip { depth: usize },
    Done,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn attribute<'a>(attributes: &'a [OwnedAttribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| a.name.local_name == name)
        .map(|a| a.value.as_str())
}

fn required_attribute<'a>(
    attributes: &'a [OwnedAttribute],
    name: &str,
) -> Result<&'a str, io::Error> {
    attribute(attributes, name).ok_or_else(|| invalid(format!("Missing attribute {name:?}")))
}

fn number<T: FromStr>(value: &str, field: &str) -> Result<T, io::Error> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("Invalid {field}: {value:?}")))
}

fn boolean(value: &str, field: &str) -> Result<bool, io::Error> {
    match value.trim() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(invalid(format!("Invalid {field}: {value:?}"))),
    }
}

pub fn parse_catalog(reader: impl Read) -> Result<MaxidomCatalog, Box<dyn Error>> {
    let mut catalog = MaxidomCatalog::default();
    let mut product = MaxidomProduct::default();
    let mut category = MaxidomCategory::default();
    let mut param = MaxidomProductParam::default();
    let mut state = State::Document;
    let mut resume = State::Document;
    let mut text = String::new();

    for event in EventReader::new(reader) {
        match event? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let tag = name.local_name.as_str();
                state = match (state, tag) {
                    (State::Document, "yml_catalog") => {
                        catalog.date = required_attribute(&attributes, "date")?.to_owned();
                        State::Catalog
                    }
                    (State::Catalog, "shop") => State::Shop,
                    (State::Shop, "name") => State::Text(Field::ShopName),
                    (State::Shop, "company") => State::Text(Field::Company),
                    (State::Shop, "url") => State::Text(Field::ShopUrl),
                    (State::Shop, "currencies") => State::Currencies,
                    (State::Shop, "categories") => State::Categories,
                    (State::Shop, "offers") => State::Offers,
                    (State::Currencies, "currency") => {
                        catalog.shop.currencies.push(MaxidomCurrency {
                            id: required_attribute(&attributes, "id")?.to_owned(),
                            rate: required_attribute(&attributes, "rate")?.to_owned(),
                        });
                        resume = State::Currencies;
                        State::Skip { depth: 1 }
                    }
                    (State::Categories, "category") => {
                        category = MaxidomCategory {
                            id: number(required_attribute(&attributes, "id")?, "category id")?,
                            parent_id: attribute(&attributes, "parentId")
                                .map(|v| number(v, "parentId"))
                                .transpose()?,
                            name: String::new(),
                        };
                        State::Text(Field::Category)
                    }
                    (State::Offers, "offer") => {
                        product = MaxidomProduct {
                            id: required_attribute(&attributes, "id")?.to_owned(),
                            available: attribute(&attributes, "available")
                                .map(|v| boolean(v, "available"))
                                .transpose()?,
                            ..Default::default()
                        };
                        State::Offer
                    }
                    (State::Offer, tag) => {
                        if let Some(field) = Field::product(tag) {
                            if matches!(field, Field::Param) {
                                param = MaxidomProductParam {
                                    name: required_attribute(&attributes, "name")?.to_owned(),
                                    unit: attribute(&attributes, "unit").map(str::to_owned),
                                    value: String::new(),
                                };
                            }
                            State::Text(field)
                        } else {
                            resume = State::Offer;
                            State::Skip { depth: 1 }
                        }
                    }
                    (State::Skip { depth }, _) => State::Skip { depth: depth + 1 },
                    (State::Text(_), _) => {
                        return Err(invalid(format!(
                            "Unexpected nested element <{tag}> in a text field"
                        ))
                        .into());
                    }
                    (State::Document | State::Done, _) => {
                        return Err(invalid("Expected a single <yml_catalog> root").into());
                    }
                    (current, _) => {
                        resume = current;
                        State::Skip { depth: 1 }
                    }
                };
            }
            // Text can arrive in several events, including entities and CDATA.
            XmlEvent::Characters(value) | XmlEvent::CData(value) | XmlEvent::Whitespace(value) => {
                if matches!(state, State::Text(_)) {
                    text.push_str(&value);
                }
            }
            XmlEvent::EndElement { .. } => {
                state = match state {
                    State::Text(field) => {
                        let value = mem::take(&mut text);
                        match field {
                            Field::ShopName => catalog.shop.name = value,
                            Field::Company => catalog.shop.company = value,
                            Field::ShopUrl => catalog.shop.url = value,
                            Field::Category => {
                                category.name = value;
                                catalog.shop.categories.push(mem::take(&mut category));
                            }
                            Field::Url => product.url = value,
                            Field::Name => product.name = value,
                            Field::Price => product.price = number(&value, "price")?,
                            Field::CurrencyId => product.currency_id = value,
                            Field::CategoryId => {
                                product.category_id = number(&value, "categoryId")?
                            }
                            Field::Picture => product.pictures.push(value),
                            Field::SalesNotes => product.sales_notes = value,
                            Field::Delivery => product.delivery = boolean(&value, "delivery")?,
                            Field::LocalDeliveryCost => {
                                product.local_delivery_cost = number(&value, "local_delivery_cost")?
                            }
                            Field::Vendor => product.vendor = value,
                            Field::CountryOfOrigin => product.country_of_origin = value,
                            Field::Description => product.description = value,
                            Field::Weight => product.weight = number(&value, "weight")?,
                            Field::Param => {
                                param.value = value;
                                product.params.push(mem::take(&mut param));
                            }
                        }
                        match field {
                            Field::ShopName | Field::Company | Field::ShopUrl => State::Shop,
                            Field::Category => State::Categories,
                            _ => State::Offer,
                        }
                    }
                    State::Offer => {
                        catalog.shop.products.push(mem::take(&mut product));
                        State::Offers
                    }
                    State::Offers | State::Currencies | State::Categories => State::Shop,
                    State::Shop => State::Catalog,
                    State::Catalog => State::Done,
                    State::Skip { depth: 1 } => resume,
                    State::Skip { depth } => State::Skip { depth: depth - 1 },
                    _ => return Err(invalid("Unexpected closing element").into()),
                };
            }
            _ => {}
        }
    }

    if !matches!(state, State::Done) {
        return Err(invalid("Incomplete catalog").into());
    }
    Ok(catalog)
}

const PRODUCT_COLUMNS: &str = "id, url, name, price, currency_id, category_id, picture, sales_notes, delivery, local_delivery_cost, vendor, country_of_origin, description, weight, params";

// COPY text escaping keeps each product on one line, including multiline descriptions.
fn write_copy_row(writer: &mut impl Write, values: &[String]) -> io::Result<()> {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            writer.write_all(b"\t")?;
        }
        let escaped = value
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        writer.write_all(escaped.as_bytes())?;
    }
    writer.write_all(b"\n")
}

fn import_products(products: &[MaxidomProduct], database: &str) -> Result<(), Box<dyn Error>> {
    if products.is_empty() {
        return Err(invalid("Catalog has no products").into());
    }
    for product in products {
        product.id.parse::<i64>()?;
        if product.pictures.len() > 1 {
            return Err(invalid(format!(
                "Offer {} has multiple pictures, but products.picture stores one",
                product.id
            ))
            .into());
        }
    }

    let mut child = Command::new("psql")
        .args([
            "-X",
            "-w",
            "--set=ON_ERROR_STOP=1",
            "--dbname",
            database,
            "--file=-",
        ])
        .stdin(Stdio::piped())
        .spawn()?;

    let write_result = (|| -> Result<(), Box<dyn Error>> {
        let mut writer = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| invalid("Missing psql stdin"))?,
        );
        writeln!(writer, "BEGIN;")?;
        writeln!(writer, "SELECT pg_advisory_xact_lock(824390217);")?;
        writeln!(
            writer,
            "SELECT current_database(), count(*) AS products_before FROM public.products;"
        )?;
        writeln!(
            writer,
            "CREATE TEMP TABLE catalog_import (LIKE public.products INCLUDING ALL) ON COMMIT DROP;"
        )?;
        writeln!(
            writer,
            "COPY catalog_import ({PRODUCT_COLUMNS}) FROM STDIN;"
        )?;
        for product in products {
            write_copy_row(
                &mut writer,
                &[
                    product.id.clone(),
                    product.url.clone(),
                    product.name.clone(),
                    product.price.to_string(),
                    product.currency_id.clone(),
                    product.category_id.to_string(),
                    product.pictures.first().cloned().unwrap_or_default(),
                    product.sales_notes.clone(),
                    product.delivery.to_string(),
                    product.local_delivery_cost.to_string(),
                    product.vendor.clone(),
                    product.country_of_origin.clone(),
                    product.description.clone(),
                    product.weight.to_string(),
                    serde_json::to_string(&product.params)?,
                ],
            )?;
        }
        writeln!(writer, "\\.")?;
        let updates = PRODUCT_COLUMNS
            .split(", ")
            .filter(|column| *column != "id")
            .map(|column| format!("{column} = EXCLUDED.{column}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            writer,
            "INSERT INTO public.products ({PRODUCT_COLUMNS}) SELECT {PRODUCT_COLUMNS} FROM catalog_import ON CONFLICT (id) DO UPDATE SET {updates};"
        )?;
        // Compare the actual stored values, including numeric rounding and all params.
        writeln!(
            writer,
            "DO $$ BEGIN IF (SELECT count(*) FROM catalog_import) <> {} OR EXISTS (SELECT 1 FROM catalog_import i LEFT JOIN public.products p USING (id) WHERE p.id IS NULL OR p IS DISTINCT FROM i) THEN RAISE EXCEPTION 'Catalog verification failed'; END IF; END $$;",
            products.len()
        )?;
        writeln!(
            writer,
            "SELECT count(*) AS products_after FROM public.products;"
        )?;
        writeln!(writer, "COMMIT;")?;
        writer.flush()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        // Disconnect before COMMIT rolls the transaction back.
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    if !child.wait()?.success() {
        return Err(invalid("PostgreSQL import failed; transaction was rolled back").into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--import") {
        println!("Usage: cargo run -- --import [catalog.xml] [database DSN]");
        return Ok(());
    }
    let path = args
        .next()
        .unwrap_or_else(|| "export_yandex_spb.xml".into());
    let database = args
        .next()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "dbname=repaircalc".into());
    if args.next().is_some() {
        return Err(invalid("Too many arguments").into());
    }
    let catalog = parse_catalog(BufReader::new(File::open(path)?))?;
    println!(
        "Parsed {} products, {} categories",
        catalog.shop.products.len(),
        catalog.shop.categories.len()
    );
    import_products(&catalog.shop.products, &database)?;
    println!(
        "Committed and verified {} products",
        catalog.shop.products.len()
    );
    Ok(())
}
