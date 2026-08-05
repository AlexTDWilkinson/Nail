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

        "j" | "joule" => ("energy", 1.0),
        "kj" | "kilojoule" => ("energy", 1000.0),
        "mj" | "megajoule" => ("energy", 1000000.0),
        "wh" | "watt_hour" => ("energy", 3600.0),
        "kwh" | "kilowatt_hour" => ("energy", 3600000.0),
        "cal" | "calorie" => ("energy", 4.184),
        "kcal" | "kilocalorie" => ("energy", 4184.0),
        "btu" => ("energy", 1055.05585262),

        "w" | "watt" => ("power", 1.0),
        "kw" | "kilowatt" => ("power", 1000.0),
        "mw" | "megawatt" => ("power", 1000000.0),
        "hp" | "horsepower" => ("power", 745.699_871_582_270),

        "pa" | "pascal" => ("pressure", 1.0),
        "kpa" | "kilopascal" => ("pressure", 1000.0),
        "mpa" | "megapascal" => ("pressure", 1000000.0),
        "bar" => ("pressure", 100000.0),
        "psi" => ("pressure", 6894.757293168),
        "atm" | "atmosphere" => ("pressure", 101325.0),
        "mmhg" => ("pressure", 133.322387415),

        "hz" | "hertz" => ("frequency", 1.0),
        "khz" | "kilohertz" => ("frequency", 1000.0),
        "mhz" | "megahertz" => ("frequency", 1000000.0),
        "ghz" | "gigahertz" => ("frequency", 1000000000.0),
        "rpm" => ("frequency", 1.0 / 60.0),

        "deg" | "degree" => ("angle", 1.0),
        "rad" | "radian" => ("angle", 57.29577951308232),
        "grad" | "gradian" => ("angle", 0.9),
        "turn" => ("angle", 360.0),
        "arcmin" | "arcminute" => ("angle", 1.0 / 60.0),
        "arcsec" | "arcsecond" => ("angle", 1.0 / 3600.0),

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
/// `l`, `cup`, `gb`, `mib`, `mph`, `knot`, `acre`, `kwh`, `hp`, `psi`, `hz`,
/// `deg`, `c`, `f` - with full names and plurals forgiven. Converting length
/// to mass is an error, not a guess.
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

    let (from_dimension, from_factor) = factor(&from_clean).ok_or_else(|| format!("convert_units: `{}` is not a unit this knows - it speaks length, mass, volume, area, speed, data, energy, power, pressure, frequency, angle and temperature", from_unit.trim()))?;
    let (to_dimension, to_factor) = factor(&to_clean).ok_or_else(|| format!("convert_units: `{}` is not a unit this knows - it speaks length, mass, volume, area, speed, data, energy, power, pressure, frequency, angle and temperature", to_unit.trim()))?;
    if from_dimension != to_dimension {
        return Err(format!("convert_units: `{}` measures {} and `{}` measures {} - they do not convert", from_unit.trim(), from_dimension, to_unit.trim(), to_dimension));
    }
    return Ok(value * from_factor / to_factor);
}

/// Miles per US gallon against liters per 100 km, derived from 3.785411784
/// liters per US gallon and 1.609344 km per mile as 100 * 3.785411784 /
/// 1.609344, about 235.214583. Dividing it by an mpg figure gives l100km and
/// dividing it by an l100km figure gives mpg, because the relation is its own
/// inverse.
const MPG_US_BRIDGE: f64 = 100.0 * 3.785411784 / 1.609344;

/// The imperial twin, from 4.54609 liters per imperial gallon and the same
/// 1.609344 km per mile as 100 * 4.54609 / 1.609344, about 282.480936.
const MPG_IMPERIAL_BRIDGE: f64 = 100.0 * 4.54609 / 1.609344;

/// One bridge serves both directions: an mpg figure in yields l100km out, an
/// l100km figure in yields mpg out, and l100km itself passes through.
fn fuel_bridge(unit: &str, value: f64) -> Option<f64> {
    return match unit {
        "l100km" => Some(value),
        "mpg" => Some(MPG_US_BRIDGE / value),
        "mpgimp" => Some(MPG_IMPERIAL_BRIDGE / value),
        _ => None,
    };
}

/// Fuel economy across its three dialects: `l100km`, `mpg` for US gallons and
/// `mpgimp` for imperial. Bigger mpg means less fuel, an inverse relation no
/// plain factor table can hold, so everything converts through liters per
/// 100 km. The value must be positive.
pub fn fuel_economy(value: f64, from_unit: String, to_unit: String) -> Result<f64, String> {
    let from_clean = from_unit.trim().to_lowercase().replace(' ', "");
    let to_clean = to_unit.trim().to_lowercase().replace(' ', "");
    if value <= 0.0 {
        return Err(format!("convert_fuel_economy: {} is not a fuel economy - the value must be positive", value));
    }
    let as_l100km = fuel_bridge(&from_clean, value).ok_or_else(|| format!("convert_fuel_economy: `{}` is not a fuel economy unit this knows - it speaks l100km, mpg and mpgimp", from_unit.trim()))?;
    return fuel_bridge(&to_clean, as_l100km).ok_or_else(|| format!("convert_fuel_economy: `{}` is not a fuel economy unit this knows - it speaks l100km, mpg and mpgimp", to_unit.trim()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 0.0001;
    }

    fn close_within(left: f64, right: f64, tolerance: f64) -> bool {
        return (left - right).abs() < tolerance;
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

    #[test]
    fn energy_power_pressure_frequency_and_angle_come_out_right() {
        assert!(close_within(units(1.0, "kwh".to_string(), "j".to_string()).unwrap(), 3600000.0, 0.000001));
        assert!(close_within(units(100.0, "hp".to_string(), "w".to_string()).unwrap(), 74569.987, 0.01));
        assert!(close_within(units(1.0, "atm".to_string(), "psi".to_string()).unwrap(), 14.695949, 0.0001));
        assert!(close_within(units(90.0, "deg".to_string(), "rad".to_string()).unwrap(), 1.570796, 0.000001));
        assert!(close_within(units(3000.0, "rpm".to_string(), "hz".to_string()).unwrap(), 50.0, 0.000000001));
    }

    #[test]
    fn new_dimensions_forgive_full_names_and_refuse_to_cross() {
        assert!(close_within(units(2.0, "kilowatt hours".to_string(), "megajoules".to_string()).unwrap(), 7.2, 0.000000001));
        assert!(close_within(units(1.0, "horsepower".to_string(), "kilowatts".to_string()).unwrap(), 0.745700, 0.000001));
        assert!(close_within(units(1.0, "atmosphere".to_string(), "bar".to_string()).unwrap(), 1.01325, 0.000001));
        assert!(close_within(units(0.5, "turns".to_string(), "radians".to_string()).unwrap(), 3.14159265, 0.000001));
        assert!(units(1.0, "j".to_string(), "w".to_string()).unwrap_err().contains("do not convert"));
        assert!(units(1.0, "hz".to_string(), "deg".to_string()).unwrap_err().contains("do not convert"));
    }

    #[test]
    fn fuel_economy_crosses_its_inverse_scales() {
        assert!(close_within(fuel_economy(8.0, "l100km".to_string(), "mpg".to_string()).unwrap(), 29.401823, 0.0001));
        let there = fuel_economy(29.4, "mpg".to_string(), "l100km".to_string()).unwrap();
        let back = fuel_economy(there, "l100km".to_string(), "mpg".to_string()).unwrap();
        assert!(close_within(back, 29.4, 0.000000001));
        assert!(close_within(fuel_economy(30.0, "mpg".to_string(), "mpgimp".to_string()).unwrap(), 36.028498, 0.001));
        assert!(close_within(fuel_economy(5.0, "L100KM".to_string(), " mpg imp ".to_string()).unwrap(), 56.4961872, 0.0001));
    }

    #[test]
    fn fuel_economy_rejects_the_meaningless() {
        assert!(fuel_economy(0.0, "mpg".to_string(), "l100km".to_string()).unwrap_err().contains("must be positive"));
        assert!(fuel_economy(-4.0, "l100km".to_string(), "mpg".to_string()).unwrap_err().contains("must be positive"));
        let unknown = fuel_economy(8.0, "furlongs per firkin".to_string(), "mpg".to_string()).unwrap_err();
        assert!(unknown.contains("l100km") && unknown.contains("mpg") && unknown.contains("mpgimp"));
    }
}
