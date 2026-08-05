//! Geo module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Geo:
        "geo_distance_km" => "std_lib::geo::distance_km", (lat1: f, lon1: f, lat2: f, lon2: f) -> (f!e),
            "Returns the great-circle distance between two latitude/longitude points in kilometers, by the haversine formula on the WGS-84 mean earth radius. Errors when a coordinate is off the map.",
            "apart_km:f = danger(geo_distance_km(53.5461, -113.4937, 51.0447, -114.0719));";
        "geo_distance_miles" => "std_lib::geo::distance_miles", (lat1: f, lon1: f, lat2: f, lon2: f) -> (f!e),
            "Returns the great-circle distance between two latitude/longitude points in statute miles. Errors when a coordinate is off the map.",
            "apart_miles:f = danger(geo_distance_miles(53.5461, -113.4937, 51.0447, -114.0719));";
        "geo_bearing" => "std_lib::geo::bearing", (lat1: f, lon1: f, lat2: f, lon2: f) -> (f!e),
            "Returns the initial compass bearing from the first point toward the second, in degrees from 0 up to 360. Errors when a coordinate is off the map.",
            "heading:f = danger(geo_bearing(53.5461, -113.4937, 51.0447, -114.0719));";
        "geo_compass_point" => "std_lib::geo::compass_point", (bearing: f) -> (s!e),
            "Returns a bearing as its 16-wind compass name, `N` through `NNW`. Any finite number of degrees is accepted and normalized first. Only NaN or infinity errors.",
            "wind:s = danger(geo_compass_point(347.0));";
        "geo_in_radius" => "std_lib::geo::in_radius", (lat1: f, lon1: f, lat2: f, lon2: f, radius_km: f) -> (b!e),
            "Returns whether the second point lies within the given great-circle distance of the first - the geofence question, with the fence line counting as inside. Errors when a coordinate is off the map or the radius is negative.",
            "nearby:b = danger(geo_in_radius(53.5461, -113.4937, 51.0447, -114.0719, 300.0));";
        "geo_closest" => "std_lib::geo::closest", (latitude: f, longitude: f, latitudes: (&[f]), longitudes: (&[f])) -> (i!e),
            "Returns the index of the nearest point among parallel latitude and longitude arrays, ties going to the earlier index. Errors when the arrays differ in length, are empty, or hold a coordinate off the map.",
            "nearest:i = danger(geo_closest(53.5461, -113.4937, store_latitudes, store_longitudes));";
        "geo_valid" => "std_lib::geo::valid", (latitude: f, longitude: f) -> b,
            "Returns whether the pair is a place on earth - latitude within -90 to 90 and longitude within -180 to 180. NaN is not a place.",
            "on_earth:b = geo_valid(53.5461, -113.4937);";
    }

    let point_import = || vec![("GEO_Point", "nail::std_lib::geo")];

    m.insert("geo_destination", StdlibFunction {
        rust_path: "std_lib::geo::destination".to_string(),
        crate_deps: vec![CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: point_import(),
        module: StdlibModule::Geo,
        parameters: vec![nail_param!(latitude: f), nail_param!(longitude: f), nail_param!(bearing: f), nail_param!(distance_km: f)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GEO_Point".to_string()))),
        diverging: false,
        description: "Returns where you end up after traveling a distance in kilometers along a compass bearing, as a GEO_Point. Errors when the start is off the map or the distance is negative.",
        example: "arrival:GEO_Point = danger(geo_destination(53.5461, -113.4937, 150.0, 100.0));",
    });

    m.insert("geo_midpoint", StdlibFunction {
        rust_path: "std_lib::geo::midpoint".to_string(),
        crate_deps: vec![CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: point_import(),
        module: StdlibModule::Geo,
        parameters: vec![nail_param!(lat1: f), nail_param!(lon1: f), nail_param!(lat2: f), nail_param!(lon2: f)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GEO_Point".to_string()))),
        diverging: false,
        description: "Returns the geographic midpoint of two points, halfway along the great circle between them, as a GEO_Point. Errors when a coordinate is off the map.",
        example: "middle:GEO_Point = danger(geo_midpoint(53.5461, -113.4937, 51.0447, -114.0719));",
    });
}
