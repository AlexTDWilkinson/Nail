//! Geography over plain latitude/longitude pairs: how far apart two places
//! are, which way one lies from the other, where a heading takes you, and
//! which of many points is nearest. All of it is haversine trigonometry on
//! the WGS-84 mean earth radius - within a fraction of a percent of the true
//! ellipsoid, which is what "how far is the other property" actually needs.
//! A coordinate off the map is an error that names itself, not a quiet NaN.

/// A place on earth as a value: the pair every function here speaks. Where a
/// function answers with a location, it answers with one of these rather than
/// a bare array, so the latitude cannot be mistaken for the longitude.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GEO_Point {
    pub latitude: f64,
    pub longitude: f64,
}

/// The WGS-84 mean earth radius in kilometers - the sphere this math lives on.
const EARTH_RADIUS_KM: f64 = 6371.0088;

/// Kilometers in one statute mile, exactly.
const KILOMETERS_PER_MILE: f64 = 1.609344;

/// The sixteen compass winds, clockwise from north, each owning 22.5 degrees.
const COMPASS_WINDS: [&str; 16] = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];

/// The 32 geohash characters - the digits and the lowercase letters minus the
/// four that read like digits (a, i, l, o).
const GEOHASH_ALPHABET: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// The latitude where the square Web-Mercator map ends. Tiles cannot see
/// past it, so tile math clamps latitudes to this line first.
const WEB_MERCATOR_LATITUDE_LIMIT: f64 = 85.0511;

/// How close to a polygon edge, in degrees, still counts as on it. Generous
/// enough to absorb float rounding, far too small to matter on a map.
const ON_EDGE_TOLERANCE: f64 = 0.000000001;

/// One latitude/longitude pair checked against the map, the message naming
/// whose coordinate went off it and what it was. NaN fails both ranges.
fn check_pair(function: &str, whose: &str, latitude: f64, longitude: f64) -> Result<(), String> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(format!("{}: {} latitude is {}, and a latitude runs from -90 to 90", function, whose, latitude));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(format!("{}: {} longitude is {}, and a longitude runs from -180 to 180", function, whose, longitude));
    }
    return Ok(());
}

/// Both endpoints checked, so every two-point function fails the same way.
fn check_two_points(function: &str, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<(), String> {
    check_pair(function, "the first point's", lat1, lon1)?;
    check_pair(function, "the second point's", lat2, lon2)?;
    return Ok(());
}

/// Parallel latitude and longitude arrays checked as one list of points:
/// equal lengths, at least the required count, and every pair on the map.
fn check_point_arrays(function: &str, latitudes: &Vec<f64>, longitudes: &Vec<f64>, minimum: usize, need: &str) -> Result<(), String> {
    if latitudes.len() != longitudes.len() {
        return Err(format!("{}: {} latitudes but {} longitudes - the arrays pair up index by index, so their lengths must match", function, latitudes.len(), longitudes.len()));
    }
    if latitudes.len() < minimum {
        return Err(format!("{}: only {} points were given, and {}", function, latitudes.len(), need));
    }
    for index in 0..latitudes.len() {
        check_pair(function, &format!("point {}'s", index), latitudes[index], longitudes[index])?;
    }
    return Ok(());
}

/// A bare latitude array checked for a bounds question: non-empty and every
/// value on the map. NaN fails the range test like everywhere else here.
fn check_latitude_array(function: &str, latitudes: &Vec<f64>) -> Result<(), String> {
    if latitudes.is_empty() {
        return Err(format!("{}: the array is empty, so there is no bound to report", function));
    }
    for (index, &latitude) in latitudes.iter().enumerate() {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(format!("{}: latitude {} is {}, and a latitude runs from -90 to 90", function, index, latitude));
        }
    }
    return Ok(());
}

/// A bare longitude array checked the same way as the latitude one.
fn check_longitude_array(function: &str, longitudes: &Vec<f64>) -> Result<(), String> {
    if longitudes.is_empty() {
        return Err(format!("{}: the array is empty, so there is no bound to report", function));
    }
    for (index, &longitude) in longitudes.iter().enumerate() {
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(format!("{}: longitude {} is {}, and a longitude runs from -180 to 180", function, index, longitude));
        }
    }
    return Ok(());
}

/// A zoom level checked against the slippy-map range shared by every tile
/// server, 0 for the whole world through 22 for a single doorstep.
fn check_zoom(function: &str, zoom: i64) -> Result<(), String> {
    if !(0..=22).contains(&zoom) {
        return Err(format!("{}: the zoom is {}, and slippy-map zoom levels run from 0 to 22", function, zoom));
    }
    return Ok(());
}

/// The haversine central angle between two points, in radians. The half-chord
/// term is clamped into [0, 1] so near-antipodal points survive float
/// rounding, and the atan2 form stays accurate at both tiny and huge angles.
fn central_angle(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let half_delta_phi = (lat2 - lat1).to_radians() / 2.0;
    let half_delta_lambda = (lon2 - lon1).to_radians() / 2.0;
    let half_chord = (half_delta_phi.sin().powi(2) + phi1.cos() * phi2.cos() * half_delta_lambda.sin().powi(2)).clamp(0.0, 1.0);
    return 2.0 * half_chord.sqrt().atan2((1.0 - half_chord).sqrt());
}

/// Degrees folded into 0..360. rem_euclid can round a hair-below-zero angle
/// up to exactly 360, so that edge folds back to 0.
fn normalize_bearing(degrees: f64) -> f64 {
    let normalized = degrees.rem_euclid(360.0);
    if normalized >= 360.0 {
        return 0.0;
    }
    return normalized;
}

/// A longitude in degrees folded into -180..180, so a path across the
/// antimeridian still comes out as a coordinate a map accepts.
fn normalize_longitude(degrees: f64) -> f64 {
    return (degrees + 180.0).rem_euclid(360.0) - 180.0;
}

/// The great-circle distance between two points in kilometers - the shortest
/// path over the surface, not through the crust.
pub fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<f64, String> {
    check_two_points("geo_distance_km", lat1, lon1, lat2, lon2)?;
    return Ok(EARTH_RADIUS_KM * central_angle(lat1, lon1, lat2, lon2));
}

/// The same great-circle distance, in statute miles.
pub fn distance_miles(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<f64, String> {
    check_two_points("geo_distance_miles", lat1, lon1, lat2, lon2)?;
    return Ok(EARTH_RADIUS_KM * central_angle(lat1, lon1, lat2, lon2) / KILOMETERS_PER_MILE);
}

/// The initial compass bearing from the first point toward the second, in
/// degrees from 0 up to but not including 360. On a great circle the heading
/// drifts as you travel; this is the one you set out on.
pub fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<f64, String> {
    check_two_points("geo_bearing", lat1, lon1, lat2, lon2)?;
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();
    let east = delta_lambda.sin() * phi2.cos();
    let north = phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * delta_lambda.cos();
    return Ok(normalize_bearing(east.atan2(north).to_degrees()));
}

/// Where you end up after traveling a distance along a bearing: a two-element
/// array of latitude then longitude, since a pair travels as an array. The
/// arrival longitude is folded into -180..180.
pub fn destination(latitude: f64, longitude: f64, bearing: f64, distance_km: f64) -> Result<GEO_Point, String> {
    check_pair("geo_destination", "the starting point's", latitude, longitude)?;
    if !bearing.is_finite() {
        return Err(format!("geo_destination: the bearing is {}, which points nowhere", bearing));
    }
    if !distance_km.is_finite() || distance_km < 0.0 {
        return Err(format!("geo_destination: the distance is {} km, and travel needs a finite distance of zero or more", distance_km));
    }
    let angular_distance = distance_km / EARTH_RADIUS_KM;
    let heading = bearing.to_radians();
    let phi1 = latitude.to_radians();
    let sine_arrival = (phi1.sin() * angular_distance.cos() + phi1.cos() * angular_distance.sin() * heading.cos()).clamp(-1.0, 1.0);
    let phi2 = sine_arrival.asin();
    let east = heading.sin() * angular_distance.sin() * phi1.cos();
    let north = angular_distance.cos() - phi1.sin() * phi2.sin();
    let lambda2 = longitude.to_radians() + east.atan2(north);
    return Ok(GEO_Point { latitude: phi2.to_degrees(), longitude: normalize_longitude(lambda2.to_degrees()) });
}

/// The geographic midpoint of two points - halfway along the great circle
/// between them - as a GEO_Point. Exactly
/// antipodal points have no single midpoint; the formula settles on one.
pub fn midpoint(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Result<GEO_Point, String> {
    check_two_points("geo_midpoint", lat1, lon1, lat2, lon2)?;
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let delta_lambda = (lon2 - lon1).to_radians();
    let shifted_x = phi2.cos() * delta_lambda.cos();
    let shifted_y = phi2.cos() * delta_lambda.sin();
    let mid_phi = (phi1.sin() + phi2.sin()).atan2(((phi1.cos() + shifted_x).powi(2) + shifted_y.powi(2)).sqrt());
    let mid_lambda = lon1.to_radians() + shifted_y.atan2(phi1.cos() + shifted_x);
    return Ok(GEO_Point { latitude: mid_phi.to_degrees(), longitude: normalize_longitude(mid_lambda.to_degrees()) });
}

/// A bearing as its 16-wind compass name, `N` through `NNW`. Any finite
/// number of degrees is accepted - 725 and -45 both land on a wind - and each
/// wind owns the 22.5 degrees centered on it.
pub fn compass_point(bearing: f64) -> Result<String, String> {
    if !bearing.is_finite() {
        return Err(format!("geo_compass_point: the bearing is {}, which points nowhere", bearing));
    }
    let index = (normalize_bearing(bearing) / 22.5).round() as usize % 16;
    return Ok(COMPASS_WINDS[index].to_string());
}

/// Whether the second point lies within the given great-circle distance of
/// the first - the geofence question, with the fence line itself counting as
/// inside.
pub fn in_radius(lat1: f64, lon1: f64, lat2: f64, lon2: f64, radius_km: f64) -> Result<bool, String> {
    check_two_points("geo_in_radius", lat1, lon1, lat2, lon2)?;
    if radius_km.is_nan() || radius_km < 0.0 {
        return Err(format!("geo_in_radius: the radius is {} km, and a geofence needs a radius of zero or more", radius_km));
    }
    return Ok(EARTH_RADIUS_KM * central_angle(lat1, lon1, lat2, lon2) <= radius_km);
}

/// The index of the nearest point among parallel latitude and longitude
/// arrays - the "which branch is closest" question. Distances compare by
/// central angle, which ranks identically to kilometers. Ties go to the
/// earlier index.
pub fn closest(latitude: f64, longitude: f64, latitudes: &Vec<f64>, longitudes: &Vec<f64>) -> Result<i64, String> {
    check_pair("geo_closest", "the query point's", latitude, longitude)?;
    if latitudes.len() != longitudes.len() {
        return Err(format!("geo_closest: {} latitudes but {} longitudes - the arrays pair up index by index, so their lengths must match", latitudes.len(), longitudes.len()));
    }
    if latitudes.is_empty() {
        return Err("geo_closest: the arrays are empty, so there is no nearest point".to_string());
    }
    let mut best_index = 0;
    let mut best_angle = f64::INFINITY;
    for index in 0..latitudes.len() {
        check_pair("geo_closest", &format!("point {}'s", index), latitudes[index], longitudes[index])?;
        let angle = central_angle(latitude, longitude, latitudes[index], longitudes[index]);
        if angle < best_angle {
            best_angle = angle;
            best_index = index;
        }
    }
    return Ok(best_index as i64);
}

/// Whether the pair is a place on earth: latitude within -90 to 90 and
/// longitude within -180 to 180. NaN is not a place.
pub fn valid(latitude: f64, longitude: f64) -> bool {
    return (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
}

/// Whether the point lies inside the polygon traced by the parallel arrays -
/// the neighborhood-boundary question. Ray casting in the coordinate plane,
/// with a point on an edge or vertex counting as inside. The polygon closes
/// itself, the last vertex connecting back to the first.
pub fn point_in_polygon(latitude: f64, longitude: f64, latitudes: &Vec<f64>, longitudes: &Vec<f64>) -> Result<bool, String> {
    check_pair("geo_point_in_polygon", "the query point's", latitude, longitude)?;
    check_point_arrays("geo_point_in_polygon", latitudes, longitudes, 3, "a polygon needs at least 3 vertices")?;
    let count = latitudes.len();
    let mut inside = false;
    let mut previous = count - 1;
    for index in 0..count {
        let (lat_here, lon_here) = (latitudes[index], longitudes[index]);
        let (lat_there, lon_there) = (latitudes[previous], longitudes[previous]);
        let cross = (lon_there - lon_here) * (latitude - lat_here) - (lat_there - lat_here) * (longitude - lon_here);
        let within_span = longitude >= lon_here.min(lon_there) - ON_EDGE_TOLERANCE
            && longitude <= lon_here.max(lon_there) + ON_EDGE_TOLERANCE
            && latitude >= lat_here.min(lat_there) - ON_EDGE_TOLERANCE
            && latitude <= lat_here.max(lat_there) + ON_EDGE_TOLERANCE;
        if cross.abs() <= ON_EDGE_TOLERANCE && within_span {
            return Ok(true);
        }
        if (lat_here > latitude) != (lat_there > latitude) {
            let crossing_longitude = (lon_there - lon_here) * (latitude - lat_here) / (lat_there - lat_here) + lon_here;
            if longitude < crossing_longitude {
                inside = !inside;
            }
        }
        previous = index;
    }
    return Ok(inside);
}

/// The area of the polygon in square kilometers, by the spherical shoelace
/// formula on the earth sphere. The absolute value is taken, so the winding
/// direction of the vertices does not matter. The polygon closes itself from
/// the last vertex back to the first.
pub fn polygon_area_km2(latitudes: &Vec<f64>, longitudes: &Vec<f64>) -> Result<f64, String> {
    check_point_arrays("geo_polygon_area_km2", latitudes, longitudes, 3, "a polygon needs at least 3 vertices")?;
    let count = latitudes.len();
    let mut total = 0.0;
    for index in 0..count {
        let next = (index + 1) % count;
        total += (longitudes[next] - longitudes[index]).to_radians() * (2.0 + latitudes[index].to_radians().sin() + latitudes[next].to_radians().sin());
    }
    return Ok((total * EARTH_RADIUS_KM * EARTH_RADIUS_KM / 2.0).abs());
}

/// The southernmost latitude among the points - the bottom edge of their
/// bounding box.
pub fn bounds_south(latitudes: &Vec<f64>) -> Result<f64, String> {
    check_latitude_array("geo_bounds_south", latitudes)?;
    return Ok(latitudes.iter().cloned().fold(f64::INFINITY, f64::min));
}

/// The northernmost latitude among the points - the top edge of their
/// bounding box.
pub fn bounds_north(latitudes: &Vec<f64>) -> Result<f64, String> {
    check_latitude_array("geo_bounds_north", latitudes)?;
    return Ok(latitudes.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
}

/// The westernmost longitude among the points, as a plain minimum. Points
/// straddling the antimeridian get the honest numeric answer, a box reaching
/// past 180 degrees wide, rather than a wrapped one.
pub fn bounds_west(longitudes: &Vec<f64>) -> Result<f64, String> {
    check_longitude_array("geo_bounds_west", longitudes)?;
    return Ok(longitudes.iter().cloned().fold(f64::INFINITY, f64::min));
}

/// The easternmost longitude among the points, as a plain maximum, with the
/// same honest antimeridian caveat as the west bound.
pub fn bounds_east(longitudes: &Vec<f64>) -> Result<f64, String> {
    check_longitude_array("geo_bounds_east", longitudes)?;
    return Ok(longitudes.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
}

/// The center of the points as the mean of their 3D unit vectors, brought
/// back to the surface - correct across the antimeridian, where averaging
/// raw longitudes would land on the wrong side of the planet.
pub fn center(latitudes: &Vec<f64>, longitudes: &Vec<f64>) -> Result<GEO_Point, String> {
    check_point_arrays("geo_center", latitudes, longitudes, 1, "a center needs at least one point")?;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    for index in 0..latitudes.len() {
        let phi = latitudes[index].to_radians();
        let lambda = longitudes[index].to_radians();
        x += phi.cos() * lambda.cos();
        y += phi.cos() * lambda.sin();
        z += phi.sin();
    }
    let count = latitudes.len() as f64;
    x /= count;
    y /= count;
    z /= count;
    if (x * x + y * y + z * z).sqrt() < ON_EDGE_TOLERANCE {
        return Err("geo_center: the points balance out to the earth's core, so no point on the surface is their middle".to_string());
    }
    return Ok(GEO_Point { latitude: z.atan2((x * x + y * y).sqrt()).to_degrees(), longitude: normalize_longitude(y.atan2(x).to_degrees()) });
}

/// The point encoded as a geohash of the given precision, 1 to 12 characters
/// of the standard base32 alphabet. Longer is a smaller cell.
pub fn geohash(latitude: f64, longitude: f64, precision: i64) -> Result<String, String> {
    check_pair("geo_geohash", "the point's", latitude, longitude)?;
    if !(1..=12).contains(&precision) {
        return Err(format!("geo_geohash: the precision is {}, and a geohash runs from 1 to 12 characters", precision));
    }
    let mut latitude_range = (-90.0_f64, 90.0_f64);
    let mut longitude_range = (-180.0_f64, 180.0_f64);
    let mut encoded = String::with_capacity(precision as usize);
    let mut bits_gathered = 0;
    let mut character_value = 0usize;
    let mut longitude_turn = true;
    while encoded.len() < precision as usize {
        let (range, coordinate) = if longitude_turn { (&mut longitude_range, longitude) } else { (&mut latitude_range, latitude) };
        let middle = (range.0 + range.1) / 2.0;
        character_value <<= 1;
        if coordinate >= middle {
            character_value |= 1;
            range.0 = middle;
        } else {
            range.1 = middle;
        }
        longitude_turn = !longitude_turn;
        bits_gathered += 1;
        if bits_gathered == 5 {
            encoded.push(GEOHASH_ALPHABET[character_value] as char);
            bits_gathered = 0;
            character_value = 0;
        }
    }
    return Ok(encoded);
}

/// The center of the cell a geohash names, as a GEO_Point. Uppercase input is
/// read as its lowercase self. Every character must come from the geohash
/// alphabet, and an empty string names no cell at all.
pub fn geohash_decode(geohash: String) -> Result<GEO_Point, String> {
    if geohash.is_empty() {
        return Err("geo_geohash_decode: the geohash is empty, so it names no cell".to_string());
    }
    let mut latitude_range = (-90.0_f64, 90.0_f64);
    let mut longitude_range = (-180.0_f64, 180.0_f64);
    let mut longitude_turn = true;
    for character in geohash.chars() {
        let lowered = character.to_ascii_lowercase();
        let index = match GEOHASH_ALPHABET.iter().position(|&letter| letter as char == lowered) {
            Some(index) => index,
            None => {
                return Err(format!("geo_geohash_decode: `{}` is not a geohash character - the alphabet is the digits and the lowercase letters except a, i, l and o", character));
            }
        };
        for bit in (0..5).rev() {
            let range = if longitude_turn { &mut longitude_range } else { &mut latitude_range };
            let middle = (range.0 + range.1) / 2.0;
            if (index >> bit) & 1 == 1 {
                range.0 = middle;
            } else {
                range.1 = middle;
            }
            longitude_turn = !longitude_turn;
        }
    }
    return Ok(GEO_Point { latitude: (latitude_range.0 + latitude_range.1) / 2.0, longitude: (longitude_range.0 + longitude_range.1) / 2.0 });
}

/// The OSM slippy-map tile column holding the longitude at the given zoom,
/// 0 through 2 to the zoom minus 1, counted from the antimeridian eastward.
pub fn tile_x(longitude: f64, zoom: i64) -> Result<i64, String> {
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(format!("geo_tile_x: the longitude is {}, and a longitude runs from -180 to 180", longitude));
    }
    check_zoom("geo_tile_x", zoom)?;
    let tile_count = (1i64 << zoom) as f64;
    let column = ((longitude + 180.0) / 360.0 * tile_count).floor() as i64;
    return Ok(column.clamp(0, (1i64 << zoom) - 1));
}

/// The OSM slippy-map tile row holding the latitude at the given zoom, 0 at
/// the top of the map. Latitudes beyond the Web-Mercator limit of 85.0511
/// degrees are clamped to it first, since the square map ends there.
pub fn tile_y(latitude: f64, zoom: i64) -> Result<i64, String> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(format!("geo_tile_y: the latitude is {}, and a latitude runs from -90 to 90", latitude));
    }
    check_zoom("geo_tile_y", zoom)?;
    let phi = latitude.clamp(-WEB_MERCATOR_LATITUDE_LIMIT, WEB_MERCATOR_LATITUDE_LIMIT).to_radians();
    let tile_count = (1i64 << zoom) as f64;
    let row = ((1.0 - phi.tan().asinh() / std::f64::consts::PI) / 2.0 * tile_count).floor() as i64;
    return Ok(row.clamp(0, (1i64 << zoom) - 1));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64, within: f64) -> bool {
        return (left - right).abs() < within;
    }

    #[test]
    fn edmonton_to_calgary_comes_out_at_the_known_distance() {
        let kilometers = distance_km(53.5461, -113.4937, 51.0447, -114.0719).unwrap();
        assert!(close(kilometers, 280.0, 5.0));
        assert!(close(kilometers, 280.9064, 0.1));
        let miles = distance_miles(53.5461, -113.4937, 51.0447, -114.0719).unwrap();
        assert!(close(miles, 174.5471, 0.1));
    }

    #[test]
    fn a_quarter_of_the_equator_is_a_quarter_circumference() {
        let kilometers = distance_km(0.0, 0.0, 0.0, 90.0).unwrap();
        assert!(close(kilometers, 10007.5, 0.1));
        assert!(close(kilometers, EARTH_RADIUS_KM * std::f64::consts::FRAC_PI_2, 0.000000001));
    }

    #[test]
    fn due_east_along_the_equator_bears_ninety() {
        assert!(close(bearing(0.0, 0.0, 0.0, 90.0).unwrap(), 90.0, 0.000000001));
    }

    #[test]
    fn the_compass_names_the_sixteen_winds() {
        assert_eq!(compass_point(0.0).unwrap(), "N");
        assert_eq!(compass_point(22.5).unwrap(), "NNE");
        assert_eq!(compass_point(359.0).unwrap(), "N");
        assert_eq!(compass_point(-45.0).unwrap(), "NW");
        assert_eq!(compass_point(725.0).unwrap(), "N");
        assert!(compass_point(f64::NAN).unwrap_err().contains("points nowhere"));
    }

    #[test]
    fn a_destination_lies_the_traveled_distance_away() {
        let arrival = destination(53.5461, -113.4937, 150.0, 100.0).unwrap();
        let back = distance_km(arrival.latitude, arrival.longitude, 53.5461, -113.4937).unwrap();
        assert!(close(back, 100.0, 0.000001));
        assert!(destination(0.0, 0.0, 90.0, -5.0).unwrap_err().contains("zero or more"));
    }

    #[test]
    fn the_midpoint_of_two_equator_points_sits_on_the_equator() {
        let middle = midpoint(0.0, 10.0, 0.0, 50.0).unwrap();
        assert!(close(middle.latitude, 0.0, 0.000000001));
        assert!(close(middle.longitude, 30.0, 0.000000001));
    }

    #[test]
    fn closest_picks_the_nearest_of_the_candidates() {
        let latitudes = vec![51.0447, 52.2681, 49.2827];
        let longitudes = vec![-114.0719, -113.8112, -123.1207];
        assert_eq!(closest(53.5461, -113.4937, &latitudes, &longitudes).unwrap(), 1);
        assert!(closest(0.0, 0.0, &vec![], &vec![]).unwrap_err().contains("empty"));
        assert!(closest(0.0, 0.0, &vec![1.0], &vec![1.0, 2.0]).unwrap_err().contains("lengths must match"));
    }

    #[test]
    fn the_geofence_answers_within_and_beyond() {
        assert!(in_radius(53.5461, -113.4937, 51.0447, -114.0719, 300.0).unwrap());
        assert!(!in_radius(53.5461, -113.4937, 51.0447, -114.0719, 250.0).unwrap());
        assert!(in_radius(0.0, 0.0, 0.0, 0.0, -1.0).unwrap_err().contains("zero or more"));
    }

    #[test]
    fn a_coordinate_off_the_map_errors_naming_which() {
        let message = distance_km(91.0, 0.0, 0.0, 0.0).unwrap_err();
        assert!(message.contains("latitude"));
        assert!(message.contains("91"));
        let message = distance_km(0.0, 0.0, 0.0, 181.0).unwrap_err();
        assert!(message.contains("longitude"));
        assert!(message.contains("181"));
        assert!(!valid(91.0, 0.0));
        assert!(!valid(0.0, 181.0));
        assert!(!valid(f64::NAN, 0.0));
        assert!(valid(53.5461, -113.4937));
    }

    #[test]
    fn a_square_polygon_answers_inside_outside_and_edge() {
        let latitudes = vec![0.0, 0.0, 10.0, 10.0];
        let longitudes = vec![0.0, 10.0, 10.0, 0.0];
        assert!(point_in_polygon(5.0, 5.0, &latitudes, &longitudes).unwrap());
        assert!(!point_in_polygon(5.0, 15.0, &latitudes, &longitudes).unwrap());
        assert!(point_in_polygon(0.0, 5.0, &latitudes, &longitudes).unwrap());
        assert!(point_in_polygon(0.0, 0.0, &latitudes, &longitudes).unwrap());
    }

    #[test]
    fn a_concave_polygon_excludes_the_notch() {
        let latitudes = vec![0.0, 0.0, 10.0, 10.0, 3.0, 3.0, 10.0, 10.0];
        let longitudes = vec![0.0, 10.0, 10.0, 7.0, 7.0, 3.0, 3.0, 0.0];
        assert!(!point_in_polygon(5.0, 5.0, &latitudes, &longitudes).unwrap());
        assert!(point_in_polygon(5.0, 1.0, &latitudes, &longitudes).unwrap());
        assert!(point_in_polygon(5.0, 9.0, &latitudes, &longitudes).unwrap());
        assert!(point_in_polygon(1.0, 5.0, &latitudes, &longitudes).unwrap());
        assert!(!point_in_polygon(11.0, 5.0, &latitudes, &longitudes).unwrap());
    }

    #[test]
    fn a_polygon_question_checks_its_arrays() {
        assert!(point_in_polygon(0.0, 0.0, &vec![0.0, 1.0], &vec![0.0, 1.0, 2.0]).unwrap_err().contains("lengths must match"));
        assert!(point_in_polygon(0.0, 0.0, &vec![0.0, 1.0], &vec![0.0, 1.0]).unwrap_err().contains("at least 3 vertices"));
        assert!(point_in_polygon(91.0, 0.0, &vec![0.0, 0.0, 1.0], &vec![0.0, 1.0, 0.0]).unwrap_err().contains("latitude"));
        assert!(point_in_polygon(0.0, 0.0, &vec![0.0, 95.0, 1.0], &vec![0.0, 1.0, 0.0]).unwrap_err().contains("point 1's"));
    }

    #[test]
    fn a_one_degree_equator_square_measures_its_known_area() {
        let latitudes = vec![0.0, 0.0, 1.0, 1.0];
        let longitudes = vec![0.0, 1.0, 1.0, 0.0];
        let area = polygon_area_km2(&latitudes, &longitudes).unwrap();
        assert!(close(area, 12364.0, 12364.0 * 0.01));
        let reversed_latitudes: Vec<f64> = latitudes.iter().rev().cloned().collect();
        let reversed_longitudes: Vec<f64> = longitudes.iter().rev().cloned().collect();
        let reversed_area = polygon_area_km2(&reversed_latitudes, &reversed_longitudes).unwrap();
        assert!(close(area, reversed_area, 0.000001));
        assert!(polygon_area_km2(&vec![0.0, 1.0], &vec![0.0, 1.0]).unwrap_err().contains("at least 3 vertices"));
    }

    #[test]
    fn the_bounds_are_the_plain_extremes() {
        let latitudes = vec![53.5461, 51.0447, 52.2681];
        let longitudes = vec![-113.4937, -114.0719, -113.8112];
        assert!(close(bounds_south(&latitudes).unwrap(), 51.0447, 0.000000001));
        assert!(close(bounds_north(&latitudes).unwrap(), 53.5461, 0.000000001));
        assert!(close(bounds_west(&longitudes).unwrap(), -114.0719, 0.000000001));
        assert!(close(bounds_east(&longitudes).unwrap(), -113.4937, 0.000000001));
        let straddling = vec![170.0, -170.0];
        assert!(close(bounds_west(&straddling).unwrap(), -170.0, 0.000000001));
        assert!(close(bounds_east(&straddling).unwrap(), 170.0, 0.000000001));
        assert!(bounds_south(&vec![]).unwrap_err().contains("empty"));
        assert!(bounds_north(&vec![91.0]).unwrap_err().contains("-90 to 90"));
        assert!(bounds_west(&vec![]).unwrap_err().contains("empty"));
        assert!(bounds_east(&vec![181.0]).unwrap_err().contains("-180 to 180"));
    }

    #[test]
    fn the_center_crosses_the_antimeridian_honestly() {
        let straddle = center(&vec![10.0, 10.0], &vec![179.0, -179.0]).unwrap();
        assert!(close(straddle.latitude, 10.0, 0.01));
        assert!(close(straddle.longitude.abs(), 180.0, 0.000001));
        let equator = center(&vec![0.0, 0.0], &vec![0.0, 10.0]).unwrap();
        assert!(close(equator.latitude, 0.0, 0.000000001));
        assert!(close(equator.longitude, 5.0, 0.000000001));
        assert!(center(&vec![], &vec![]).unwrap_err().contains("at least one point"));
        assert!(center(&vec![0.0], &vec![0.0, 1.0]).unwrap_err().contains("lengths must match"));
        assert!(center(&vec![0.0, 0.0], &vec![0.0, 180.0]).unwrap_err().contains("core"));
    }

    #[test]
    fn the_geohash_anchor_encodes_as_wikipedia_says() {
        assert_eq!(geohash(57.64911, 10.40744, 11).unwrap(), "u4pruydqqvj");
        assert_eq!(geohash(57.64911, 10.40744, 6).unwrap(), "u4pruy");
        assert!(geohash(57.64911, 10.40744, 0).unwrap_err().contains("1 to 12"));
        assert!(geohash(57.64911, 10.40744, 13).unwrap_err().contains("1 to 12"));
        assert!(geohash(91.0, 0.0, 6).unwrap_err().contains("latitude"));
    }

    #[test]
    fn a_geohash_decodes_back_into_its_cell() {
        let point = geohash_decode("u4pruydqqvj".to_string()).unwrap();
        assert!(close(point.latitude, 57.64911, 0.001));
        assert!(close(point.longitude, 10.40744, 0.001));
        let upper = geohash_decode("U4PRUYDQQVJ".to_string()).unwrap();
        assert!(close(upper.latitude, point.latitude, 0.000000001));
        assert!(geohash_decode("".to_string()).unwrap_err().contains("empty"));
        assert!(geohash_decode("u4a".to_string()).unwrap_err().contains("not a geohash character"));
        assert!(geohash_decode("u4!".to_string()).unwrap_err().contains("not a geohash character"));
    }

    #[test]
    fn edmonton_lands_on_its_slippy_map_tile() {
        assert_eq!(tile_x(-113.4937, 10).unwrap(), 189);
        assert_eq!(tile_y(53.5461, 10).unwrap(), 330);
        assert_eq!(tile_x(0.0, 0).unwrap(), 0);
        assert_eq!(tile_y(0.0, 0).unwrap(), 0);
        assert_eq!(tile_x(180.0, 4).unwrap(), 15);
        assert_eq!(tile_y(89.9, 10).unwrap(), 0);
        assert_eq!(tile_y(-89.9, 10).unwrap(), 1023);
        assert!(tile_x(181.0, 10).unwrap_err().contains("-180 to 180"));
        assert!(tile_y(91.0, 10).unwrap_err().contains("-90 to 90"));
        assert!(tile_x(0.0, -1).unwrap_err().contains("0 to 22"));
        assert!(tile_y(0.0, 23).unwrap_err().contains("0 to 22"));
    }
}
