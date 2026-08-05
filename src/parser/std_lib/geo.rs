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
}
