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
        "geo_point_in_polygon" => "std_lib::geo::point_in_polygon", (latitude: f, longitude: f, latitudes: (&[f]), longitudes: (&[f])) -> (b!e),
            "Returns whether the point lies inside the polygon traced by parallel latitude and longitude arrays - the neighborhood-boundary question, a point on the boundary counting as inside. The polygon closes itself from the last vertex back to the first. Errors when the arrays differ in length, hold fewer than 3 vertices, or hold a coordinate off the map.",
            "in_zone:b = danger(geo_point_in_polygon(53.52, -113.50, zone_latitudes, zone_longitudes));";
        "geo_polygon_area_km2" => "std_lib::geo::polygon_area_km2", (latitudes: (&[f]), longitudes: (&[f])) -> (f!e),
            "Returns the area of the polygon in square kilometers, by the spherical shoelace formula. The winding direction does not matter, and the polygon closes itself from the last vertex back to the first. Errors when the arrays differ in length, hold fewer than 3 vertices, or hold a coordinate off the map.",
            "zone_km2:f = danger(geo_polygon_area_km2(zone_latitudes, zone_longitudes));";
        "geo_bounds_south" => "std_lib::geo::bounds_south", (latitudes: (&[f])) -> (f!e),
            "Returns the southernmost latitude in the array - the bottom edge of the points' bounding box. Errors when the array is empty or holds a latitude off the map.",
            "south:f = danger(geo_bounds_south(store_latitudes));";
        "geo_bounds_north" => "std_lib::geo::bounds_north", (latitudes: (&[f])) -> (f!e),
            "Returns the northernmost latitude in the array - the top edge of the points' bounding box. Errors when the array is empty or holds a latitude off the map.",
            "north:f = danger(geo_bounds_north(store_latitudes));";
        "geo_bounds_west" => "std_lib::geo::bounds_west", (longitudes: (&[f])) -> (f!e),
            "Returns the westernmost longitude in the array as a plain minimum. Points straddling the antimeridian get the honest numeric answer, a box reaching past 180 degrees wide, rather than a wrapped one. Errors when the array is empty or holds a longitude off the map.",
            "west:f = danger(geo_bounds_west(store_longitudes));";
        "geo_bounds_east" => "std_lib::geo::bounds_east", (longitudes: (&[f])) -> (f!e),
            "Returns the easternmost longitude in the array as a plain maximum, with the same honest antimeridian caveat as geo_bounds_west. Errors when the array is empty or holds a longitude off the map.",
            "east:f = danger(geo_bounds_east(store_longitudes));";
        "geo_geohash" => "std_lib::geo::geohash", (latitude: f, longitude: f, precision: i) -> (s!e),
            "Returns the point encoded as a geohash of the given precision, 1 to 12 characters of the standard base32 alphabet. Longer is a smaller cell. Errors when the coordinate is off the map or the precision is outside 1 to 12.",
            "cell:s = danger(geo_geohash(57.64911, 10.40744, 6));";
        "geo_tile_x" => "std_lib::geo::tile_x", (longitude: f, zoom: i) -> (i!e),
            "Returns the OSM slippy-map tile column holding the longitude at the given zoom, 0 through 2 to the zoom minus 1. Errors when the longitude is off the map or the zoom is outside 0 to 22.",
            "column:i = danger(geo_tile_x(-113.4937, 10));";
        "geo_tile_y" => "std_lib::geo::tile_y", (latitude: f, zoom: i) -> (i!e),
            "Returns the OSM slippy-map tile row holding the latitude at the given zoom, 0 at the top of the map. Latitudes beyond the Web-Mercator limit of 85.0511 degrees are clamped to it first, since the square map ends there. Errors when the latitude is off the map or the zoom is outside 0 to 22.",
            "row:i = danger(geo_tile_y(53.5461, 10));";
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

    m.insert("geo_center", StdlibFunction {
        rust_path: "std_lib::geo::center".to_string(),
        crate_deps: vec![CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: point_import(),
        module: StdlibModule::Geo,
        parameters: vec![nail_param!(latitudes: (&[f])), nail_param!(longitudes: (&[f]))],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GEO_Point".to_string()))),
        diverging: false,
        description: "Returns the center of the points as the mean of their 3D unit vectors brought back to the surface, correct across the antimeridian, as a GEO_Point. Errors when the arrays differ in length, are empty, hold a coordinate off the map, or the points balance out exactly.",
        example: "middle:GEO_Point = danger(geo_center(store_latitudes, store_longitudes));",
    });

    m.insert("geo_geohash_decode", StdlibFunction {
        rust_path: "std_lib::geo::geohash_decode".to_string(),
        crate_deps: vec![CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: point_import(),
        module: StdlibModule::Geo,
        parameters: vec![nail_param!(geohash: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("GEO_Point".to_string()))),
        diverging: false,
        description: "Returns the center of the cell a geohash names, as a GEO_Point. Uppercase input is read as its lowercase self. Errors when the string is empty or holds a character outside the geohash alphabet.",
        example: "cell_center:GEO_Point = danger(geo_geohash_decode(`u4pruy`));",
    });
}
