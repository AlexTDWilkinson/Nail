//! Convert module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    // Both functions take stdlib enums, which need custom type imports, so
    // they use the full struct form.
    m.insert("convert_units", StdlibFunction {
        rust_path: "std_lib::convert::units".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("CONVERT_Unit", "nail::std_lib::convert")],
        module: StdlibModule::Convert,
        parameters: vec![
            nail_param!(value: f),
            StdlibParameter { name: "from_unit".to_string(), param_type: NailDataTypeDescriptor::Enum("CONVERT_Unit".to_string()), pass_by_reference: false },
            StdlibParameter { name: "to_unit".to_string(), param_type: NailDataTypeDescriptor::Enum("CONVERT_Unit".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((f!e)),
        diverging: false,
        description: "This number, in that unit. The units are CONVERT_Unit variants across length, mass, volume, area, speed, data, energy, power, pressure, frequency, angle and temperature, so a misspelled unit never compiles. Converting across dimensions is an error, not a guess.",
        example: "kilometers:f = danger(convert_units(3.2, CONVERT_Unit::Mile, CONVERT_Unit::Kilometer));",
    });

    m.insert("convert_fuel_economy", StdlibFunction {
        rust_path: "std_lib::convert::fuel_economy".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("CONVERT_FuelEconomy", "nail::std_lib::convert")],
        module: StdlibModule::Convert,
        parameters: vec![
            nail_param!(value: f),
            StdlibParameter { name: "from_unit".to_string(), param_type: NailDataTypeDescriptor::Enum("CONVERT_FuelEconomy".to_string()), pass_by_reference: false },
            StdlibParameter { name: "to_unit".to_string(), param_type: NailDataTypeDescriptor::Enum("CONVERT_FuelEconomy".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((f!e)),
        diverging: false,
        description: "Fuel economy across its three dialects, named by CONVERT_FuelEconomy. Bigger mpg means less fuel, an inverse relation a factor table cannot hold, so this converts through liters per 100 km. The value must be positive.",
        example: "economy:f = danger(convert_fuel_economy(8.0, CONVERT_FuelEconomy::LitersPer100Km, CONVERT_FuelEconomy::MpgUs));",
    });
}
