//! Unit conversion for the numbers real work arrives in: the recipe in cups,
//! the fence in feet, the download in mebibytes, the forecast in fahrenheit.
//! One function, because the only thing anyone wants is "this number, in that
//! unit" - and a conversion across dimensions is an error, not a guess.

/// A unit's dimension and its size in that dimension's base unit.
fn factor_exact(unit: &str) -> Option<(&'static str, f64)> {
    return Some(match unit {
        "mm" | "millimeter" | "millimetre" => ("length", 0.001),
        "cm" | "centimeter" | "centimetre" => ("length", 0.01),
        "m" | "meter" | "metre" => ("length", 1.0),
        "km" | "kilometer" | "kilometre" => ("length", 1000.0),
        "in" | "inch" | "inches" => ("length", 0.0254),
        "ft" | "foot" | "feet" => ("length", 0.3048),
        "yd" | "yard" => ("length", 0.9144),
        "mi" | "mile" => ("length", 1609.344),
        "nmi" | "nautical_mile" => ("length", 1852.0),

        "mg" | "milligram" => ("mass", 0.000001),
        "g" | "gram" => ("mass", 0.001),
        "kg" | "kilogram" => ("mass", 1.0),
        "t" | "tonne" | "metric_ton" => ("mass", 1000.0),
        "oz" | "ounce" => ("mass", 0.028349523125),
        "lb" | "pound" => ("mass", 0.45359237),
        "stone" => ("mass", 6.35029318),

        "ml" | "milliliter" | "millilitre" => ("volume", 0.001),
        "l" | "liter" | "litre" => ("volume", 1.0),
        "gal" | "gallon" => ("volume", 3.785411784),
        "qt" | "quart" => ("volume", 0.946352946),
        "pint" => ("volume", 0.473176473),
        "cup" => ("volume", 0.2365882365),
        "floz" | "fluid_ounce" => ("volume", 0.0295735295625),
        "tbsp" | "tablespoon" => ("volume", 0.01478676478125),
        "tsp" | "teaspoon" => ("volume", 0.00492892159375),

        "byte" | "bytes" => ("data", 1.0),
        "kb" => ("data", 1000.0),
        "mb" => ("data", 1000000.0),
        "gb" => ("data", 1000000000.0),
        "tb" => ("data", 1000000000000.0),
        "kib" => ("data", 1024.0),
        "mib" => ("data", 1048576.0),
        "gib" => ("data", 1073741824.0),
        "tib" => ("data", 1099511627776.0),

        "mps" => ("speed", 1.0),
        "kph" | "kmh" => ("speed", 0.277_777_777_777_777_8),
        "mph" => ("speed", 0.44704),
        "knot" | "kn" => ("speed", 0.514_444_444_444_444_5),

        "sqm" | "square_meter" => ("area", 1.0),
        "sqkm" => ("area", 1000000.0),
        "sqft" | "square_foot" => ("area", 0.09290304),
        "sqmi" => ("area", 2589988.110336),
        "acre" => ("area", 4046.8564224),
        "ha" | "hectare" => ("area", 10000.0),

        _ => return None,
    });
}

/// Exact name first, then with one trailing s forgiven, so `miles` and
/// `pounds` read as their singulars.
fn factor(unit: &str) -> Option<(&'static str, f64)> {
    if let Some(found) = factor_exact(unit) {
        return Some(found);
    }
    return unit.strip_suffix('s').and_then(factor_exact);
}

/// Temperatures convert through kelvin, because their scales do not share a
/// zero the way every other dimension's units do.
fn temperature_to_kelvin(unit: &str, value: f64) -> Option<f64> {
    return match unit {
        "c" | "celsius" => Some(value + 273.15),
        "f" | "fahrenheit" => Some((value - 32.0) * 5.0 / 9.0 + 273.15),
        "k" | "kelvin" => Some(value),
        _ => None,
    };
}

fn kelvin_to(unit: &str, kelvin: f64) -> Option<f64> {
    return match unit {
        "c" | "celsius" => Some(kelvin - 273.15),
        "f" | "fahrenheit" => Some((kelvin - 273.15) * 9.0 / 5.0 + 32.0),
        "k" | "kelvin" => Some(kelvin),
        _ => None,
    };
}

/// This number, in that unit. Units are short names - `km`, `mi`, `kg`, `lb`,
/// `l`, `cup`, `gb`, `mib`, `mph`, `knot`, `acre`, `c`, `f` - with full names
/// and plurals forgiven. Converting length to mass is an error, not a guess.
pub fn units(value: f64, from_unit: String, to_unit: String) -> Result<f64, String> {
    let from_clean = from_unit.trim().to_lowercase().replace(' ', "_");
    let to_clean = to_unit.trim().to_lowercase().replace(' ', "_");

    let from_temperature = temperature_to_kelvin(&from_clean, value);
    let to_is_temperature = kelvin_to(&to_clean, 0.0).is_some();
    if let Some(kelvin) = from_temperature {
        if to_is_temperature {
            return Ok(kelvin_to(&to_clean, kelvin).expect("checked just above"));
        }
    }
    if from_temperature.is_some() != to_is_temperature {
        return Err(format!("convert_units: `{}` and `{}` measure different things - a temperature only converts to a temperature", from_unit.trim(), to_unit.trim()));
    }

    let (from_dimension, from_factor) = factor(&from_clean).ok_or_else(|| format!("convert_units: `{}` is not a unit this knows - it speaks length, mass, volume, area, speed, data and temperature", from_unit.trim()))?;
    let (to_dimension, to_factor) = factor(&to_clean).ok_or_else(|| format!("convert_units: `{}` is not a unit this knows - it speaks length, mass, volume, area, speed, data and temperature", to_unit.trim()))?;
    if from_dimension != to_dimension {
        return Err(format!("convert_units: `{}` measures {} and `{}` measures {} - they do not convert", from_unit.trim(), from_dimension, to_unit.trim(), to_dimension));
    }
    return Ok(value * from_factor / to_factor);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 0.0001;
    }

    #[test]
    fn the_everyday_conversions_come_out_right() {
        assert!(close(units(1.0, "mi".to_string(), "km".to_string()).unwrap(), 1.609344));
        assert!(close(units(100.0, "kg".to_string(), "lb".to_string()).unwrap(), 220.46226));
        assert!(close(units(2.0, "cup".to_string(), "ml".to_string()).unwrap(), 473.176473));
        assert!(close(units(1.0, "gb".to_string(), "mib".to_string()).unwrap(), 953.674316));
        assert!(close(units(60.0, "mph".to_string(), "kph".to_string()).unwrap(), 96.56064));
        assert!(close(units(1.0, "acre".to_string(), "sqm".to_string()).unwrap(), 4046.8564224));
    }

    #[test]
    fn temperatures_cross_their_offset_scales() {
        assert!(close(units(100.0, "c".to_string(), "f".to_string()).unwrap(), 212.0));
        assert!(close(units(32.0, "fahrenheit".to_string(), "celsius".to_string()).unwrap(), 0.0));
        assert!(close(units(0.0, "c".to_string(), "k".to_string()).unwrap(), 273.15));
    }

    #[test]
    fn plurals_and_full_names_are_forgiven() {
        assert!(close(units(3.0, "miles".to_string(), "kilometers".to_string()).unwrap(), 4.828032));
        assert!(close(units(2.0, "pounds".to_string(), "grams".to_string()).unwrap(), 907.18474));
    }

    #[test]
    fn nonsense_is_named_not_guessed() {
        assert!(units(1.0, "parsec".to_string(), "km".to_string()).unwrap_err().contains("not a unit this knows"));
        assert!(units(1.0, "kg".to_string(), "km".to_string()).unwrap_err().contains("do not convert"));
        assert!(units(1.0, "c".to_string(), "kg".to_string()).unwrap_err().contains("only converts to a temperature"));
    }
}
