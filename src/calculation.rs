//! Pure material calculations; lengths in metres, areas in m², reserve in percent.
//! Price is per package in one currency. Callers supply product-specific consumption.
use rust_decimal::{Decimal, prelude::ToPrimitive};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CalculationError {
    #[error("{0} must be non-negative")]
    Negative(&'static str),
    #[error("{0} must be positive")]
    NotPositive(&'static str),
    #[error("openings exceed surface area")]
    OpeningsExceedArea,
    #[error("roll cannot supply a complete strip")]
    RollTooShort,
    #[error("calculation exceeds supported numeric range")]
    Overflow,
}
type Result<T> = std::result::Result<T, CalculationError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Purchase {
    /// Quantity of material in a package: litres, kg, m², or pieces.
    pub package_quantity: Decimal,
    pub package_price: Decimal,
    pub reserve_percent: Decimal,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estimate {
    /// Material quantity before reserve (litres, kg, m², or strips).
    pub base_quantity: Decimal,
    pub required_quantity: Decimal,
    pub packages: u64,
    pub purchased_quantity: Decimal,
    pub surplus_quantity: Decimal,
    /// Rounded to two decimal places, midpoint away from zero.
    pub total_cost: Decimal,
}
fn nonnegative(value: Decimal, name: &'static str) -> Result<()> {
    if value < Decimal::ZERO {
        Err(CalculationError::Negative(name))
    } else {
        Ok(())
    }
}
fn positive(value: Decimal, name: &'static str) -> Result<()> {
    if value <= Decimal::ZERO {
        Err(CalculationError::NotPositive(name))
    } else {
        Ok(())
    }
}
fn mul(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_mul(b).ok_or(CalculationError::Overflow)
}
fn div(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_div(b).ok_or(CalculationError::Overflow)
}
fn add(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_add(b).ok_or(CalculationError::Overflow)
}
fn subtract(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_sub(b).ok_or(CalculationError::Overflow)
}

pub fn net_area(length: Decimal, width: Decimal, openings_m2: Decimal) -> Result<Decimal> {
    nonnegative(length, "length")?;
    nonnegative(width, "width")?;
    nonnegative(openings_m2, "openings_m2")?;
    let area = mul(length, width)?;
    if openings_m2 > area {
        return Err(CalculationError::OpeningsExceedArea);
    }
    subtract(area, openings_m2)
}

/// Shared packaging, reserve and cost calculation. Zero area buys zero packages.
pub fn package_estimate(base_quantity: Decimal, purchase: Purchase) -> Result<Estimate> {
    nonnegative(base_quantity, "base_quantity")?;
    positive(purchase.package_quantity, "package_quantity")?;
    nonnegative(purchase.package_price, "package_price")?;
    nonnegative(purchase.reserve_percent, "reserve_percent")?;
    let factor = add(
        Decimal::ONE,
        div(purchase.reserve_percent, Decimal::ONE_HUNDRED)?,
    )?;
    let required_quantity = mul(base_quantity, factor)?;
    // Ceiling before multiplication prevents buying a fractional package.
    let packages = div(required_quantity, purchase.package_quantity)?
        .ceil()
        .to_u64()
        .ok_or(CalculationError::Overflow)?;
    let purchased_quantity = mul(Decimal::from(packages), purchase.package_quantity)?;
    let total_cost = mul(Decimal::from(packages), purchase.package_price)?
        .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    Ok(Estimate {
        base_quantity,
        required_quantity,
        packages,
        purchased_quantity,
        surplus_quantity: subtract(purchased_quantity, required_quantity)?,
        total_cost,
    })
}

/// Paint: m² × litres/m² per coat × coats. Package quantity is litres per can.
pub fn paint(
    area_m2: Decimal,
    litres_per_m2: Decimal,
    coats: u32,
    purchase: Purchase,
) -> Result<Estimate> {
    nonnegative(area_m2, "area_m2")?;
    positive(litres_per_m2, "litres_per_m2")?;
    if coats == 0 {
        return Err(CalculationError::NotPositive("coats"));
    }
    package_estimate(
        mul(mul(area_m2, litres_per_m2)?, Decimal::from(coats))?,
        purchase,
    )
}

/// Tile/laminate: package quantity is coverage in m². Cutting waste is reserve_percent.
pub fn tiles(area_m2: Decimal, purchase: Purchase) -> Result<Estimate> {
    package_estimate(area_m2, purchase)
}

/// Adhesive, plaster, primer, etc.: quantity per m² and quantity per package share a unit.
pub fn consumable(
    area_m2: Decimal,
    quantity_per_m2: Decimal,
    purchase: Purchase,
) -> Result<Estimate> {
    nonnegative(area_m2, "area_m2")?;
    positive(quantity_per_m2, "quantity_per_m2")?;
    package_estimate(mul(area_m2, quantity_per_m2)?, purchase)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wallpaper {
    pub perimeter_m: Decimal,
    pub height_m: Decimal,
    pub roll_width_m: Decimal,
    pub roll_length_m: Decimal,
    pub trim_per_strip_m: Decimal,
    /// Zero means no pattern. Straight match; offset/half-drop match is not modeled.
    pub pattern_repeat_m: Decimal,
    pub reserve_percent: Decimal,
    pub roll_price: Decimal,
}

/// Counts whole full-height strips; windows and doors are conservatively not deducted.
/// Estimate quantities are strips; packages are rolls. Reserve applies to strips.
pub fn wallpaper(input: Wallpaper) -> Result<Estimate> {
    nonnegative(input.perimeter_m, "perimeter_m")?;
    positive(input.height_m, "height_m")?;
    positive(input.roll_width_m, "roll_width_m")?;
    positive(input.roll_length_m, "roll_length_m")?;
    nonnegative(input.trim_per_strip_m, "trim_per_strip_m")?;
    nonnegative(input.pattern_repeat_m, "pattern_repeat_m")?;
    let mut strip_length = add(input.height_m, input.trim_per_strip_m)?;
    if input.pattern_repeat_m > Decimal::ZERO {
        strip_length = mul(
            div(strip_length, input.pattern_repeat_m)?.ceil(),
            input.pattern_repeat_m,
        )?;
    }
    let strips_per_roll = div(input.roll_length_m, strip_length)?.floor();
    if strips_per_roll < Decimal::ONE {
        return Err(CalculationError::RollTooShort);
    }
    let strips = div(input.perimeter_m, input.roll_width_m)?.ceil();
    package_estimate(
        strips,
        Purchase {
            package_quantity: strips_per_roll,
            package_price: input.roll_price,
            reserve_percent: input.reserve_percent,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }
    fn purchase(quantity: &str, price: &str, reserve: &str) -> Purchase {
        Purchase {
            package_quantity: d(quantity),
            package_price: d(price),
            reserve_percent: d(reserve),
        }
    }
    #[test]
    fn paint_includes_layers_reserve_and_cans() {
        let e = paint(d("40"), d("0.12"), 2, purchase("3", "1499.99", "10")).unwrap();
        assert_eq!(e.base_quantity, d("9.6"));
        assert_eq!(e.required_quantity, d("10.56"));
        assert_eq!(e.packages, 4);
        assert_eq!(e.surplus_quantity, d("1.44"));
        assert_eq!(e.total_cost, d("5999.96"));
    }
    #[test]
    fn tile_rounds_up_and_preserves_exact_boundary() {
        assert_eq!(
            tiles(d("12"), purchase("1.5", "900", "0"))
                .unwrap()
                .packages,
            8
        );
        assert_eq!(
            tiles(d("12.01"), purchase("1.5", "900", "0"))
                .unwrap()
                .packages,
            9
        );
        assert_eq!(
            tiles(d("12"), purchase("1.5", "900", "10"))
                .unwrap()
                .packages,
            9
        );
    }
    #[test]
    fn decimal_boundary_does_not_add_a_package() {
        assert_eq!(
            tiles(d("0.3"), purchase("0.1", "0.10", "0"))
                .unwrap()
                .total_cost,
            d("0.30")
        );
    }
    #[test]
    fn glue_uses_mass_consumption() {
        let e = consumable(d("10"), d("3.5"), purchase("25", "600", "10")).unwrap();
        assert_eq!(e.required_quantity, d("38.5"));
        assert_eq!(e.packages, 2);
        assert_eq!(e.total_cost, d("1200"));
    }
    fn roll() -> Wallpaper {
        Wallpaper {
            perimeter_m: d("10"),
            height_m: d("2.5"),
            roll_width_m: d("1"),
            roll_length_m: d("10"),
            trim_per_strip_m: d("0"),
            pattern_repeat_m: d("0"),
            reserve_percent: d("0"),
            roll_price: d("1000"),
        }
    }
    #[test]
    fn wallpaper_uses_whole_strips() {
        let e = wallpaper(roll()).unwrap();
        assert_eq!(e.base_quantity, d("10"));
        assert_eq!(e.packages, 3);
        assert_eq!(e.purchased_quantity, d("12"));
        assert_eq!(e.total_cost, d("3000"));
    }
    #[test]
    fn wallpaper_accounts_for_pattern_and_trim() {
        let mut r = roll();
        r.pattern_repeat_m = d("0.6");
        assert_eq!(wallpaper(r).unwrap().packages, 4);
        r.pattern_repeat_m = d("0");
        r.trim_per_strip_m = d("0.1");
        assert_eq!(wallpaper(r).unwrap().packages, 4);
    }
    #[test]
    fn wallpaper_reserve_and_partial_width() {
        let mut r = roll();
        r.perimeter_m = d("11.1");
        r.reserve_percent = d("10");
        assert_eq!(wallpaper(r).unwrap().packages, 4);
    }
    #[test]
    fn wallpaper_rejects_short_roll() {
        let mut r = roll();
        r.height_m = d("11");
        assert_eq!(wallpaper(r), Err(CalculationError::RollTooShort));
    }
    #[test]
    fn subtracts_openings() {
        assert_eq!(net_area(d("5"), d("3"), d("2")).unwrap(), d("13"));
        assert_eq!(
            net_area(d("1"), d("1"), d("2")),
            Err(CalculationError::OpeningsExceedArea)
        );
    }
    #[test]
    fn empty_surface_costs_zero() {
        assert_eq!(
            paint(d("0"), d("0.1"), 2, purchase("1", "100", "10"))
                .unwrap()
                .packages,
            0
        );
        let mut r = roll();
        r.perimeter_m = d("0");
        assert_eq!(wallpaper(r).unwrap().total_cost, Decimal::ZERO);
    }
    #[test]
    fn rejects_invalid_inputs() {
        assert!(tiles(d("-1"), purchase("1", "1", "0")).is_err());
        assert!(tiles(d("1"), purchase("0", "1", "0")).is_err());
        assert!(tiles(d("1"), purchase("-1", "1", "0")).is_err());
        assert!(tiles(d("1"), purchase("1", "-1", "0")).is_err());
        assert!(tiles(d("1"), purchase("1", "1", "-1")).is_err());
        assert!(paint(d("1"), d("0"), 1, purchase("1", "1", "0")).is_err());
        assert!(paint(d("1"), d("1"), 0, purchase("1", "1", "0")).is_err());
        assert!(consumable(d("-1"), d("1"), purchase("1", "1", "0")).is_err());
        assert!(net_area(d("-1"), d("1"), d("0")).is_err());
        let mut r = roll();
        r.roll_width_m = d("0");
        assert!(wallpaper(r).is_err());
    }
    #[test]
    fn overflow_returns_error_without_panicking() {
        assert_eq!(
            consumable(Decimal::MAX, d("2"), purchase("1", "1", "0")),
            Err(CalculationError::Overflow)
        );
        assert_eq!(
            tiles(Decimal::MAX, purchase("1", "1", "0")),
            Err(CalculationError::Overflow)
        );
        assert_eq!(
            tiles(d("2"), purchase("1", &Decimal::MAX.to_string(), "0")),
            Err(CalculationError::Overflow)
        );
    }
    #[test]
    fn rounds_money_at_end() {
        assert_eq!(
            tiles(d("3"), purchase("1", "0.335", "0"))
                .unwrap()
                .total_cost,
            d("1.01")
        );
    }
}
