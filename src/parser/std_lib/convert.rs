//! Unit conversion for the numbers real work arrives in: the recipe in cups,
//! the fence in feet, the download in mebibytes, the forecast in fahrenheit.
//! One function, because the only thing anyone wants is "this number, in that
//! unit" - and a conversion across dimensions is an error, not a guess.

use serde::{Deserialize, Serialize};

/// A unit convert_units speaks, across length, mass, volume, area, speed,
/// data, energy, power, pressure, frequency, angle and temperature. The type
/// checker rules out a misspelled unit, and crossing dimensions stays a
/// runtime error, because the dimension is a fact about the value, not the
/// type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CONVERT_Unit {
    // length
    Millimeter,
    Centimeter,
    Meter,
    Kilometer,
    Inch,
    Foot,
    Yard,
    Mile,
    NauticalMile,
    // mass
    Milligram,
    Gram,
    Kilogram,
    Tonne,
    Ounce,
    Pound,
    Stone,
    // volume
    Milliliter,
    Liter,
    Gallon,
    Quart,
    Pint,
    Cup,
    FluidOunce,
    Tablespoon,
    Teaspoon,
    // data, decimal then binary
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Kibibyte,
    Mebibyte,
    Gibibyte,
    Tebibyte,
    // speed
    MetersPerSecond,
    KilometersPerHour,
    MilesPerHour,
    Knot,
    // area
    SquareMeter,
    SquareKilometer,
    SquareFoot,
    SquareMile,
    Acre,
    Hectare,
    // energy
    Joule,
    Kilojoule,
    Megajoule,
    WattHour,
    KilowattHour,
    Calorie,
    Kilocalorie,
    Btu,
    // power
    Watt,
    Kilowatt,
    Megawatt,
    Horsepower,
    // pressure
    Pascal,
    Kilopascal,
    Megapascal,
    Bar,
    Psi,
    Atmosphere,
    MillimeterOfMercury,
    // frequency
    Hertz,
    Kilohertz,
    Megahertz,
    Gigahertz,
    Rpm,
    // angle
    Degree,
    Radian,
    Gradian,
    Turn,
    Arcminute,
    Arcsecond,
    // temperature
    Celsius,
    Fahrenheit,
    Kelvin,
}

/// A unit's dimension and its size in that dimension's base unit.
/// Temperatures are absent because their scales do not share a zero, so they
/// convert through kelvin instead of through a factor.
fn factor(unit: CONVERT_Unit) -> Option<(&'static str, f64)> {
    return Some(match unit {
        CONVERT_Unit::Millimeter => ("length", 0.001),
        CONVERT_Unit::Centimeter => ("length", 0.01),
        CONVERT_Unit::Meter => ("length", 1.0),
        CONVERT_Unit::Kilometer => ("length", 1000.0),
        CONVERT_Unit::Inch => ("length", 0.0254),
        CONVERT_Unit::Foot => ("length", 0.3048),
        CONVERT_Unit::Yard => ("length", 0.9144),
        CONVERT_Unit::Mile => ("length", 1609.344),
        CONVERT_Unit::NauticalMile => ("length", 1852.0),

        CONVERT_Unit::Milligram => ("mass", 0.000001),
        CONVERT_Unit::Gram => ("mass", 0.001),
        CONVERT_Unit::Kilogram => ("mass", 1.0),
        CONVERT_Unit::Tonne => ("mass", 1000.0),
        CONVERT_Unit::Ounce => ("mass", 0.028349523125),
        CONVERT_Unit::Pound => ("mass", 0.45359237),
        CONVERT_Unit::Stone => ("mass", 6.35029318),

        CONVERT_Unit::Milliliter => ("volume", 0.001),
        CONVERT_Unit::Liter => ("volume", 1.0),
        CONVERT_Unit::Gallon => ("volume", 3.785411784),
        CONVERT_Unit::Quart => ("volume", 0.946352946),
        CONVERT_Unit::Pint => ("volume", 0.473176473),
        CONVERT_Unit::Cup => ("volume", 0.2365882365),
        CONVERT_Unit::FluidOunce => ("volume", 0.0295735295625),
        CONVERT_Unit::Tablespoon => ("volume", 0.01478676478125),
        CONVERT_Unit::Teaspoon => ("volume", 0.00492892159375),

        CONVERT_Unit::Byte => ("data", 1.0),
        CONVERT_Unit::Kilobyte => ("data", 1000.0),
        CONVERT_Unit::Megabyte => ("data", 1000000.0),
        CONVERT_Unit::Gigabyte => ("data", 1000000000.0),
        CONVERT_Unit::Terabyte => ("data", 1000000000000.0),
        CONVERT_Unit::Kibibyte => ("data", 1024.0),
        CONVERT_Unit::Mebibyte => ("data", 1048576.0),
        CONVERT_Unit::Gibibyte => ("data", 1073741824.0),
        CONVERT_Unit::Tebibyte => ("data", 1099511627776.0),

        CONVERT_Unit::MetersPerSecond => ("speed", 1.0),
        CONVERT_Unit::KilometersPerHour => ("speed", 0.277_777_777_777_777_8),
        CONVERT_Unit::MilesPerHour => ("speed", 0.44704),
        CONVERT_Unit::Knot => ("speed", 0.514_444_444_444_444_5),

        CONVERT_Unit::SquareMeter => ("area", 1.0),
        CONVERT_Unit::SquareKilometer => ("area", 1000000.0),
        CONVERT_Unit::SquareFoot => ("area", 0.09290304),
        CONVERT_Unit::SquareMile => ("area", 2589988.110336),
        CONVERT_Unit::Acre => ("area", 4046.8564224),
        CONVERT_Unit::Hectare => ("area", 10000.0),

        CONVERT_Unit::Joule => ("energy", 1.0),
        CONVERT_Unit::Kilojoule => ("energy", 1000.0),
        CONVERT_Unit::Megajoule => ("energy", 1000000.0),
        CONVERT_Unit::WattHour => ("energy", 3600.0),
        CONVERT_Unit::KilowattHour => ("energy", 3600000.0),
        CONVERT_Unit::Calorie => ("energy", 4.184),
        CONVERT_Unit::Kilocalorie => ("energy", 4184.0),
        CONVERT_Unit::Btu => ("energy", 1055.05585262),

        CONVERT_Unit::Watt => ("power", 1.0),
        CONVERT_Unit::Kilowatt => ("power", 1000.0),
        CONVERT_Unit::Megawatt => ("power", 1000000.0),
        CONVERT_Unit::Horsepower => ("power", 745.699_871_582_270),

        CONVERT_Unit::Pascal => ("pressure", 1.0),
        CONVERT_Unit::Kilopascal => ("pressure", 1000.0),
        CONVERT_Unit::Megapascal => ("pressure", 1000000.0),
        CONVERT_Unit::Bar => ("pressure", 100000.0),
        CONVERT_Unit::Psi => ("pressure", 6894.757293168),
        CONVERT_Unit::Atmosphere => ("pressure", 101325.0),
        CONVERT_Unit::MillimeterOfMercury => ("pressure", 133.322387415),

        CONVERT_Unit::Hertz => ("frequency", 1.0),
        CONVERT_Unit::Kilohertz => ("frequency", 1000.0),
        CONVERT_Unit::Megahertz => ("frequency", 1000000.0),
        CONVERT_Unit::Gigahertz => ("frequency", 1000000000.0),
        CONVERT_Unit::Rpm => ("frequency", 1.0 / 60.0),

        CONVERT_Unit::Degree => ("angle", 1.0),
        CONVERT_Unit::Radian => ("angle", 57.29577951308232),
        CONVERT_Unit::Gradian => ("angle", 0.9),
        CONVERT_Unit::Turn => ("angle", 360.0),
        CONVERT_Unit::Arcminute => ("angle", 1.0 / 60.0),
        CONVERT_Unit::Arcsecond => ("angle", 1.0 / 3600.0),

        CONVERT_Unit::Celsius | CONVERT_Unit::Fahrenheit | CONVERT_Unit::Kelvin => return None,
    });
}

/// Temperatures convert through kelvin, because their scales do not share a
/// zero the way every other dimension's units do.
fn temperature_to_kelvin(unit: CONVERT_Unit, value: f64) -> Option<f64> {
    return match unit {
        CONVERT_Unit::Celsius => Some(value + 273.15),
        CONVERT_Unit::Fahrenheit => Some((value - 32.0) * 5.0 / 9.0 + 273.15),
        CONVERT_Unit::Kelvin => Some(value),
        _ => None,
    };
}

fn kelvin_to(unit: CONVERT_Unit, kelvin: f64) -> Option<f64> {
    return match unit {
        CONVERT_Unit::Celsius => Some(kelvin - 273.15),
        CONVERT_Unit::Fahrenheit => Some((kelvin - 273.15) * 9.0 / 5.0 + 32.0),
        CONVERT_Unit::Kelvin => Some(kelvin),
        _ => None,
    };
}

/// This number, in that unit. The units are CONVERT_Unit variants, so a
/// misspelled unit never compiles. Converting length to mass is still an
/// error, not a guess.
pub fn units(value: f64, from_unit: CONVERT_Unit, to_unit: CONVERT_Unit) -> Result<f64, String> {
    let from_temperature = temperature_to_kelvin(from_unit, value);
    let to_is_temperature = kelvin_to(to_unit, 0.0).is_some();
    if let Some(kelvin) = from_temperature {
        if to_is_temperature {
            return Ok(kelvin_to(to_unit, kelvin).expect("checked just above"));
        }
    }
    if from_temperature.is_some() != to_is_temperature {
        return Err(format!("convert_units: {:?} and {:?} measure different things - a temperature only converts to a temperature", from_unit, to_unit));
    }

    let (from_dimension, from_factor) = factor(from_unit).expect("every non-temperature unit has a factor");
    let (to_dimension, to_factor) = factor(to_unit).expect("every non-temperature unit has a factor");
    if from_dimension != to_dimension {
        return Err(format!("convert_units: {:?} measures {} and {:?} measures {} - they do not convert", from_unit, from_dimension, to_unit, to_dimension));
    }
    return Ok(value * from_factor / to_factor);
}

/// A spelling of fuel economy. The three dialects do not share a direction:
/// bigger mpg means less fuel, bigger liters per 100 km means more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CONVERT_FuelEconomy {
    /// Liters burned per 100 kilometers - smaller is thriftier.
    LitersPer100Km,
    /// Miles per US gallon - bigger is thriftier.
    MpgUs,
    /// Miles per imperial gallon - bigger is thriftier.
    MpgImperial,
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
fn fuel_bridge(unit: CONVERT_FuelEconomy, value: f64) -> f64 {
    return match unit {
        CONVERT_FuelEconomy::LitersPer100Km => value,
        CONVERT_FuelEconomy::MpgUs => MPG_US_BRIDGE / value,
        CONVERT_FuelEconomy::MpgImperial => MPG_IMPERIAL_BRIDGE / value,
    };
}

/// Fuel economy across its three dialects. Bigger mpg means less fuel, an
/// inverse relation no plain factor table can hold, so everything converts
/// through liters per 100 km. The value must be positive.
pub fn fuel_economy(value: f64, from_unit: CONVERT_FuelEconomy, to_unit: CONVERT_FuelEconomy) -> Result<f64, String> {
    if value <= 0.0 {
        return Err(format!("convert_fuel_economy: {} is not a fuel economy - the value must be positive", value));
    }
    let as_l100km = fuel_bridge(from_unit, value);
    return Ok(fuel_bridge(to_unit, as_l100km));
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
        assert!(close(units(1.0, CONVERT_Unit::Mile, CONVERT_Unit::Kilometer).unwrap(), 1.609344));
        assert!(close(units(100.0, CONVERT_Unit::Kilogram, CONVERT_Unit::Pound).unwrap(), 220.46226));
        assert!(close(units(2.0, CONVERT_Unit::Cup, CONVERT_Unit::Milliliter).unwrap(), 473.176473));
        assert!(close(units(1.0, CONVERT_Unit::Gigabyte, CONVERT_Unit::Mebibyte).unwrap(), 953.674316));
        assert!(close(units(60.0, CONVERT_Unit::MilesPerHour, CONVERT_Unit::KilometersPerHour).unwrap(), 96.56064));
        assert!(close(units(1.0, CONVERT_Unit::Acre, CONVERT_Unit::SquareMeter).unwrap(), 4046.8564224));
    }

    #[test]
    fn temperatures_cross_their_offset_scales() {
        assert!(close(units(100.0, CONVERT_Unit::Celsius, CONVERT_Unit::Fahrenheit).unwrap(), 212.0));
        assert!(close(units(32.0, CONVERT_Unit::Fahrenheit, CONVERT_Unit::Celsius).unwrap(), 0.0));
        assert!(close(units(0.0, CONVERT_Unit::Celsius, CONVERT_Unit::Kelvin).unwrap(), 273.15));
    }

    #[test]
    fn crossing_dimensions_is_named_not_guessed() {
        assert!(units(1.0, CONVERT_Unit::Kilogram, CONVERT_Unit::Kilometer).unwrap_err().contains("do not convert"));
        assert!(units(1.0, CONVERT_Unit::Celsius, CONVERT_Unit::Kilogram).unwrap_err().contains("only converts to a temperature"));
        assert!(units(1.0, CONVERT_Unit::Joule, CONVERT_Unit::Watt).unwrap_err().contains("do not convert"));
        assert!(units(1.0, CONVERT_Unit::Hertz, CONVERT_Unit::Degree).unwrap_err().contains("do not convert"));
    }

    #[test]
    fn energy_power_pressure_frequency_and_angle_come_out_right() {
        assert!(close_within(units(1.0, CONVERT_Unit::KilowattHour, CONVERT_Unit::Joule).unwrap(), 3600000.0, 0.000001));
        assert!(close_within(units(100.0, CONVERT_Unit::Horsepower, CONVERT_Unit::Watt).unwrap(), 74569.987, 0.01));
        assert!(close_within(units(1.0, CONVERT_Unit::Atmosphere, CONVERT_Unit::Psi).unwrap(), 14.695949, 0.0001));
        assert!(close_within(units(90.0, CONVERT_Unit::Degree, CONVERT_Unit::Radian).unwrap(), 1.570796, 0.000001));
        assert!(close_within(units(3000.0, CONVERT_Unit::Rpm, CONVERT_Unit::Hertz).unwrap(), 50.0, 0.000000001));
        assert!(close_within(units(2.0, CONVERT_Unit::KilowattHour, CONVERT_Unit::Megajoule).unwrap(), 7.2, 0.000000001));
        assert!(close_within(units(1.0, CONVERT_Unit::Horsepower, CONVERT_Unit::Kilowatt).unwrap(), 0.745700, 0.000001));
        assert!(close_within(units(1.0, CONVERT_Unit::Atmosphere, CONVERT_Unit::Bar).unwrap(), 1.01325, 0.000001));
        assert!(close_within(units(0.5, CONVERT_Unit::Turn, CONVERT_Unit::Radian).unwrap(), 3.14159265, 0.000001));
    }

    #[test]
    fn fuel_economy_crosses_its_inverse_scales() {
        assert!(close_within(fuel_economy(8.0, CONVERT_FuelEconomy::LitersPer100Km, CONVERT_FuelEconomy::MpgUs).unwrap(), 29.401823, 0.0001));
        let there = fuel_economy(29.4, CONVERT_FuelEconomy::MpgUs, CONVERT_FuelEconomy::LitersPer100Km).unwrap();
        let back = fuel_economy(there, CONVERT_FuelEconomy::LitersPer100Km, CONVERT_FuelEconomy::MpgUs).unwrap();
        assert!(close_within(back, 29.4, 0.000000001));
        assert!(close_within(fuel_economy(30.0, CONVERT_FuelEconomy::MpgUs, CONVERT_FuelEconomy::MpgImperial).unwrap(), 36.028498, 0.001));
        assert!(close_within(fuel_economy(5.0, CONVERT_FuelEconomy::LitersPer100Km, CONVERT_FuelEconomy::MpgImperial).unwrap(), 56.4961872, 0.0001));
    }

    #[test]
    fn fuel_economy_rejects_the_meaningless() {
        assert!(fuel_economy(0.0, CONVERT_FuelEconomy::MpgUs, CONVERT_FuelEconomy::LitersPer100Km).unwrap_err().contains("must be positive"));
        assert!(fuel_economy(-4.0, CONVERT_FuelEconomy::LitersPer100Km, CONVERT_FuelEconomy::MpgUs).unwrap_err().contains("must be positive"));
    }
}
