//! Streaming import of {"products": [...]} using a single PostgreSQL transaction.
use std::{
    fmt,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use serde::{
    Deserialize,
    de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Number, Value};

type Error = Box<dyn std::error::Error>;

pub const SCHEMA: &str = include_str!("../sql/products_json.sql");
pub const COLUMNS: &str = "id, cod_good, name, marketing_description, url_key, price, currency, brand, collection, country, categories, unit, description, weight, characteristics";

/// Numeric tokens retain their decimal representation, including precision beyond f64.
/// Unexpected product fields fail explicitly rather than being silently discarded.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub id: i64,
    pub cod_good: Option<String>,
    pub name: String,
    pub marketing_description: Option<String>,
    pub url_key: Option<String>,
    pub price: Option<Number>,
    pub currency: Option<String>,
    pub brand: Option<Value>,
    pub collection: Option<Value>,
    pub country: Option<Value>,
    pub categories: Option<Value>,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub weight: Option<Number>,
    pub characteristics: Option<Value>,
}

struct ProductsSeed<'a, F>(&'a mut F);

impl<'de, F: FnMut(Product) -> io::Result<()>> DeserializeSeed<'de> for ProductsSeed<'_, F> {
    type Value = u64;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<u64, D::Error> {
        struct ProductsVisitor<'a, F>(&'a mut F);
        impl<'de, F: FnMut(Product) -> io::Result<()>> Visitor<'de> for ProductsVisitor<'_, F> {
            type Value = u64;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an array of products")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<u64, A::Error> {
                let mut count = 0;
                while let Some(product) = seq.next_element::<Product>()? {
                    (self.0)(product).map_err(de::Error::custom)?;
                    count += 1;
                }
                Ok(count)
            }
        }
        deserializer.deserialize_seq(ProductsVisitor(self.0))
    }
}

/// Only a single product and a fixed-size input buffer are held at a time.
/// Validates the closing object and EOF before the caller can commit.
pub fn stream_products(
    reader: impl Read,
    mut consume: impl FnMut(Product) -> io::Result<()>,
) -> Result<u64, serde_json::Error> {
    struct RootVisitor<'a, F>(&'a mut F);
    impl<'de, F: FnMut(Product) -> io::Result<()>> Visitor<'de> for RootVisitor<'_, F> {
        type Value = u64;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object containing a products array")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<u64, A::Error> {
            let mut count = None;
            while let Some(key) = map.next_key::<String>()? {
                if key == "products" {
                    if count.is_some() {
                        return Err(de::Error::duplicate_field("products"));
                    }
                    count = Some(map.next_value_seed(ProductsSeed(self.0))?);
                } else {
                    map.next_value::<IgnoredAny>()?;
                }
            }
            count.ok_or_else(|| de::Error::missing_field("products"))
        }
    }
    let mut deserializer =
        serde_json::Deserializer::from_reader(BufReader::with_capacity(64 * 1024, reader));
    let count = de::Deserializer::deserialize_map(&mut deserializer, RootVisitor(&mut consume))?;
    deserializer.end()?;
    Ok(count)
}

fn json(value: Option<Value>) -> Option<String> {
    value.map(|v| v.to_string())
}

fn write_row(writer: &mut impl Write, product: Product) -> io::Result<()> {
    let fields = [
        Some(product.id.to_string()),
        product.cod_good,
        Some(product.name),
        product.marketing_description,
        product.url_key,
        product.price.map(|n| n.to_string()),
        product.currency,
        json(product.brand),
        json(product.collection),
        json(product.country),
        json(product.categories),
        product.unit,
        product.description,
        product.weight.map(|n| n.to_string()),
        json(product.characteristics),
    ];
    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            writer.write_all(b"\t")?;
        }
        match field {
            None => writer.write_all(b"\\N")?,
            Some(value) => {
                // COPY text escapes are separate from JSON escapes. This also protects COPY's terminator.
                for byte in value.bytes() {
                    match byte {
                        b'\\' => writer.write_all(b"\\\\")?,
                        b'\t' => writer.write_all(b"\\t")?,
                        b'\n' => writer.write_all(b"\\n")?,
                        b'\r' => writer.write_all(b"\\r")?,
                        _ => writer.write_all(&[byte])?,
                    }
                }
            }
        }
    }
    writer.write_all(b"\n")
}

/// Replaces the old schema and rows only after a complete, validated nonempty import.
/// No CASCADE: dependent application objects cause rollback and must be migrated explicitly.
pub fn import_file(path: impl AsRef<Path>, database: &str) -> Result<u64, Error> {
    let source = File::open(path)?;
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
    let result = (|| -> Result<u64, Error> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing psql stdin"))?;
        let mut writer = BufWriter::with_capacity(64 * 1024, stdin);
        writeln!(
            writer,
            "BEGIN;\nSET LOCAL client_encoding = 'UTF8';\nSELECT pg_advisory_xact_lock(824390217);"
        )?;
        writer.write_all(SCHEMA.as_bytes())?;
        writeln!(
            writer,
            "\nCOPY public.products_json_import ({COLUMNS}) FROM STDIN;"
        )?;
        let count = stream_products(source, |product| write_row(&mut writer, product))?;
        if count == 0 {
            return Err(io::Error::other(
                "refusing to replace the catalog with an empty products array",
            )
            .into());
        }
        writeln!(writer, "\\.")?;
        writeln!(
            writer,
            "DO $$ BEGIN IF (SELECT count(*) FROM public.products_json_import) <> {count} THEN RAISE EXCEPTION 'Imported product count mismatch'; END IF; END $$;"
        )?;
        writeln!(
            writer,
            "CREATE INDEX products_json_import_cod_good_idx ON public.products_json_import (cod_good);"
        )?;
        writeln!(
            writer,
            "CREATE INDEX products_json_import_categories_idx ON public.products_json_import USING gin (categories);"
        )?;
        writeln!(
            writer,
            "CREATE INDEX products_json_import_characteristics_idx ON public.products_json_import USING gin (characteristics);"
        )?;
        writeln!(writer, "DROP TABLE IF EXISTS public.products;")?;
        writeln!(
            writer,
            "ALTER TABLE public.products_json_import RENAME TO products;"
        )?;
        writeln!(
            writer,
            "ALTER INDEX public.products_json_import_pkey RENAME TO products_pkey;"
        )?;
        writeln!(
            writer,
            "ALTER INDEX public.products_json_import_cod_good_idx RENAME TO products_cod_good_idx;"
        )?;
        writeln!(
            writer,
            "ALTER INDEX public.products_json_import_categories_idx RENAME TO products_categories_idx;"
        )?;
        writeln!(
            writer,
            "ALTER INDEX public.products_json_import_characteristics_idx RENAME TO products_characteristics_idx;"
        )?;
        writeln!(writer, "ANALYZE public.products;\nCOMMIT;")?;
        writer.flush()?;
        Ok(count)
    })();
    match result {
        Ok(count) => {
            if !child.wait()?.success() {
                return Err(
                    io::Error::other("PostgreSQL import failed; transaction rolled back").into(),
                );
            }
            Ok(count)
        }
        Err(error) => {
            // Closing stdin without COMMIT rolls back; kill/wait also handles a blocked COPY.
            let _ = child.kill();
            let _ = child.wait();
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_objects_and_preserves_codes_decimals_and_nested_values() {
        let input = br#"{"metadata":{"ignored":[1,2]},"products":[{"id":1,"name":"first","cod_good":"000123","price":12345678901234567890.123456789,"brand":null,"categories":{"507":"Wood"},"characteristics":[{"name":"Weight","value":"400"}]},{"id":2,"name":"second","weight":0.123456789}]}"#;
        let mut ids = Vec::new();
        let count = stream_products(&input[..], |product| {
            ids.push(product.id);
            if product.id == 1 {
                assert_eq!(product.cod_good.as_deref(), Some("000123"));
                assert_eq!(
                    product.price.unwrap().to_string(),
                    "12345678901234567890.123456789"
                );
                assert!(product.brand.is_none());
                assert_eq!(product.characteristics.unwrap()[0]["value"], "400");
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(count, 2);
        assert_eq!(ids, [1, 2]);
    }

    #[test]
    fn rejects_truncation_trailing_data_duplicate_or_missing_products_and_unknown_fields() {
        for input in [
            r#"{"products":[{"id":1,"name":"p"}]"#,
            r#"{"products":[]} false"#,
            r#"{"products":[],"products":[]}"#,
            r#"{"items":[]}"#,
            r#"{"products":[{"id":1,"name":"p","unexpected":1}]}"#,
            r#"{"products":[{"id":"1","name":"p"}]}"#,
        ] {
            assert!(
                stream_products(input.as_bytes(), |_| Ok(())).is_err(),
                "{input}"
            );
        }
    }

    #[test]
    fn callback_failure_stops_parsing() {
        let mut called = 0;
        let error = stream_products(
            br#"{"products":[{"id":1,"name":"p"},{"id":2,"name":"q"}]}"#.as_slice(),
            |_| {
                called += 1;
                Err(io::Error::other("database pipe closed"))
            },
        )
        .unwrap_err();
        assert_eq!(called, 1);
        assert!(error.to_string().contains("database pipe closed"));
    }

    #[test]
    fn copy_escapes_control_characters_null_marker_and_terminator() {
        let product: Product = serde_json::from_value(serde_json::json!({"id":1,"name":"tab\tline\nreturn\rslash\\N\n\\.","brand":{"id":2,"name":"a\nb\\c"}})).unwrap();
        let mut output = Vec::new();
        write_row(&mut output, product).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("tab\\tline\\nreturn\\rslash\\\\N\\n\\\\."));
        assert_eq!(output.lines().count(), 1);
        assert_eq!(output.split('\t').count(), 15);
        assert!(output.ends_with("\t\\N\n"));
    }
}
