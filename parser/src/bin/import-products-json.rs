use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "products_final.json".into());
    if path == "--help" || path == "-h" {
        println!(
            "Usage: import-products-json [products_final.json] [database DSN]\nReplaces public.products atomically. DSN defaults to DATABASE_URL or dbname=repaircalc."
        );
        return Ok(());
    }
    let database = args
        .next()
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "dbname=repaircalc".into());
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let count = parser::json_import::import_file(&path, &database)?;
    println!("Committed and verified {count} products from {path}");
    Ok(())
}
