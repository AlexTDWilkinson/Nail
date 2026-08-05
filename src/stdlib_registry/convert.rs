//! Convert module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Convert:
        "convert_units" => "std_lib::convert::units", (value: f, from_unit: s, to_unit: s) -> (f!e),
            "This number, in that unit. Length, mass, volume, area, speed, data, energy, power, pressure, frequency, angle and temperature. Units are short names like `km`, `mi`, `kg`, `lb`, `cup`, `gb`, `mib`, `mph`, `kwh`, `hp`, `psi`, `hz`, `deg`, `c`, `f`, with full names and plurals forgiven. Converting across dimensions is an error, not a guess.",
            "kilometers:f = danger(convert_units(3.2, `mi`, `km`));";
        "convert_fuel_economy" => "std_lib::convert::fuel_economy", (value: f, from_unit: s, to_unit: s) -> (f!e),
            "Fuel economy across its three dialects: `l100km`, `mpg` for US gallons and `mpgimp` for imperial. Bigger mpg means less fuel, an inverse relation a factor table cannot hold, so this converts through liters per 100 km. The value must be positive.",
            "economy:f = danger(convert_fuel_economy(8.0, `l100km`, `mpg`));";
    }
}
