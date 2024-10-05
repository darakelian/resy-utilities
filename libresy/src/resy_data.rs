use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct RestaurantCityConfig {
    code: String,
    country_code: String,
    country_id: u16,
    country_name: String,
    id: u32,
    pub latitude: f32,
    pub longitude: f32,
    name: String,
    radius: u16,
    time_zone: String,
    url_slug: String
}

impl RestaurantCityConfig {
    pub fn is_match(&self, city: &String, country: &String) -> bool {
        let country_match = self.country_code.eq_ignore_ascii_case(country);
        let city_match = self.url_slug.to_ascii_lowercase().contains(&city.to_ascii_lowercase());
        
        country_match && city_match
    }
}

/// Contains search params for narrowing down availability on a particular time
#[derive(Debug, Deserialize)]
pub struct SlotFilter {
    day: String,
    party_size: u16
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GeoFilter {
    latitude: f32,
    longitude: f32,
    radius: u16
}

impl GeoFilter {
    pub fn new(latitude: f32, longitude: f32, radius: u16) -> GeoFilter {
        GeoFilter { latitude, longitude, radius }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RestaurantSearchParams {
    availability: bool,
    geo: GeoFilter,
    query: String
}

impl RestaurantSearchParams {
    pub fn new(availability: bool, geo: &GeoFilter, query: &String) -> RestaurantSearchParams {
        RestaurantSearchParams{ availability, geo: geo.clone(), query: query.clone()}
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RestaurantSearchResult {
    locality: String,
    #[serde(rename = "objectID")]
    pub object_id: String,
    url_slug: String
}