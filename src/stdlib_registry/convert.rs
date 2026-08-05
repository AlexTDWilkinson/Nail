//! Convert module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Convert:
        "convert_units" => "std_lib::convert::units", (value: f, from_unit: s, to_unit: s) -> (f!e),
            "This number, in that unit. Length, mass, volume, area, speed, data and temperature. Units are short names like `km`, `mi`, `kg`, `lb`, `cup`, `gb`, `mib`, `mph`, `c`, `f`, with full names and plurals forgiven. Converting across dimensions is an error, not a guess.",
            "kilometers:f = danger(convert_units(3.2, `mi`, `km`));";
    }
}
