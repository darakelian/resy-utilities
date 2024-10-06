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
pub struct RestaurantSearchRequest {
    availability: bool,
    geo: GeoFilter,
    query: String
}

impl RestaurantSearchRequest {
    pub fn new(availability: bool, geo: &GeoFilter, query: &String) -> RestaurantSearchRequest {
        RestaurantSearchRequest{ availability, geo: geo.clone(), query: query.clone()}
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RestaurantSearchResult {
    locality: String,
    #[serde(rename = "objectID")]
    pub object_id: String,
    url_slug: String
}

/// Contains the useful information to actually book a reservation.
#[derive(Debug, Deserialize, Clone)]
pub struct ReservationSlotConfig {
    /// Some sort of ID, unsure if this value is actually needed anywhere.
    pub id: u32,
    /// This field seems to be whether the reservation is "Indoors", "Outside", etc
    /// matching the UI. Should allow further filtering for a user's preferences.
    #[serde(rename = "type")]
    pub slot_type: String,
    /// Resy-specific URI that can be used later to request a booking token.
    pub token: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReservationSlot {
    pub config: ReservationSlotConfig
}

/// Request params to get details on a reservation. The response will include the
/// booking token that is needed to actually book a reservation.
#[derive(Debug, Deserialize, Serialize)]
pub struct ReservationDetailsRequest {
    config_id: String,
    date: String,
    party_size: String
}

impl ReservationDetailsRequest {
    pub fn new(config_id: String, date: String, party_size: String) -> ReservationDetailsRequest {
        ReservationDetailsRequest { config_id, date, party_size }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct PaymentMethod {
    id: String
}

#[derive(Debug, Deserialize, Clone)]
struct DetailsUser {
    payment_methods: Vec<PaymentMethod>
}

#[derive(Debug, Deserialize, Clone)]
struct BookToken {
    date_expires: String,
    value: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReservationDetails {
    user: DetailsUser,
    book_token: BookToken
}

impl ReservationDetails {
    /// Tries to get the first payment ID a user has. A user is not guaranteed to
    /// have any payment methods on file.
    pub fn get_payment_id(&self) -> Option<String> {
        match self.user.payment_methods.first() {
            Some(p) => Some(p.id.clone()),
            None => None
        }
    }

    pub fn get_booking_token(&self) -> &String {
        &self.book_token.value
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BookingRequest {
    resy_token: String,
    reservation_id: u32,
    venue_opt_in: bool
}