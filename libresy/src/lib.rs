use std::str::FromStr;

use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};
use resy_data::{
    GeoFilter, ReservationDetails, ReservationDetailsRequest, ReservationSlot,
    RestaurantCityConfig, RestaurantSearchRequest, RestaurantSearchResult,
};
use serde::de::value;

mod resy_data;

/// Resy apparently checks if the user-agent is a "browser" agent so let's pretend to be Firefox
static USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0";
/// Base URL for location based queries
static RESY_LOCATION_BASE: &str = "https://api.resy.com/3/location";
/// URL path to fetch the location config data
static RESY_CONFIG_URL: &str = "/config";
/// Base URL for venue search queries
static RESY_VENUESEARCH_BASE: &str = "https://api.resy.com/3/venuesearch";
static RESY_VENUESEARCH_SEARCH: &str = "/search";

static RESY_FIND_URL: &str = "https://api.resy.com/4/find";

/// URL to get reservation details
static RESY_DETAILS_URL: &str = "https://api.resy.com/3/details";

static RESY_AUTH_TOKEN_HEADER: &str = "X-Resy-Auth-Token";

/// Client used for interacting with Resy. Under the hood, maintains
/// a reqwst client
#[derive(Debug)]
pub struct ResyClient {
    api_key: String,
    auth_key: String,
    no_cache: bool,
    strict_match: bool,
    client: Client,
    restaurants: Vec<RestaurantCityConfig>,
}

impl ResyClient {
    pub fn builder() -> ResyClientBuilder {
        ResyClientBuilder::default()
    }

    /// Loads the restaurant city configs from Resy. This is all cities in the Resy network
    /// that we can search for restaurants later.
    /// TODO: Make this use a caching mechanism?
    pub async fn load_config(&mut self) -> anyhow::Result<()> {
        let mapping_body = self
            .client
            .get(format!("{}{}", RESY_LOCATION_BASE, RESY_CONFIG_URL))
            .send()
            .await?
            .text()
            .await?;
        let results: Vec<RestaurantCityConfig> = serde_json::from_str(&mapping_body)?;
        self.restaurants.extend(results);
        Ok(())
    }

    /// Gets the city configuration data for a given city so that we can search for the
    /// restaurant later.
    pub fn get_restaurant_city_config(
        &self,
        city: &String,
        country: &String,
    ) -> Option<RestaurantCityConfig> {
        for restaurant_city_config in self.restaurants.iter() {
            if restaurant_city_config.is_match(city, country) {
                return Some(restaurant_city_config.clone());
            }
        }
        None
    }

    /// Tries to get the restaurant. Assumes the restaurant name provided is unique
    /// to only have one restaurant in the given city.
    pub async fn find_restaurant(
        &self,
        city_config: &RestaurantCityConfig,
        name: &String,
    ) -> anyhow::Result<Option<RestaurantSearchResult>> {
        let geo_filter = GeoFilter::new(city_config.latitude, city_config.longitude, u16::MAX);
        let restaurant_search_params = RestaurantSearchRequest::new(false, &geo_filter, name);

        let res = self
            .client
            .post(format!(
                "{}{}",
                RESY_VENUESEARCH_BASE, RESY_VENUESEARCH_SEARCH
            ))
            .json(&restaurant_search_params)
            .send()
            .await?;

        let text = res.text().await?;
        let value = serde_json::Value::from_str(&text).unwrap();
        let hits: Vec<RestaurantSearchResult> =
            serde_json::from_value::<Vec<RestaurantSearchResult>>(value["search"]["hits"].clone())
                .unwrap();
        Ok(hits.first().cloned())
    }

    /// Gets reservations for a given restaurant. Empty vec means no time slots on
    /// the given date were found.
    pub async fn get_reservations(
        &self,
        restaurant_id: &String,
        date: &String,
        party_size: u8,
    ) -> anyhow::Result<Vec<ReservationSlot>> {
        let res = self
            .client
            .get(RESY_FIND_URL)
            .query(&[("lat", "0")])
            .query(&[("long", "0")])
            .query(&[("venue_id", restaurant_id)])
            .query(&[("day", date)])
            .query(&[("party_size", &party_size.to_string())])
            .send()
            .await?;
        let text = res.text().await?;
        let value = serde_json::Value::from_str(&text).unwrap();
        let slots_value = value["results"]["venues"][0]["slots"].clone();
        let slots = serde_json::from_value(slots_value).unwrap();
        Ok(slots)
    }

    /// Retrieves the reservation details for a slot.
    pub async fn get_reservation_details(
        &self,
        reservation_slot: &ReservationSlot,
        date: &String,
        party_size: u8,
    ) -> anyhow::Result<ReservationDetails> {
        let details_request = ReservationDetailsRequest::new(
            reservation_slot.config.token.clone(),
            date.clone(),
            party_size.to_string(),
        );
        let res: ReservationDetails = self
            .client
            .post(RESY_DETAILS_URL)
            .json(&details_request)
            .send()
            .await?
            .json()
            .await?;
        Ok(res)
    }
}

#[derive(Default)]
pub struct ResyClientBuilder {
    api_key: String,
    auth_key: String,
    no_cache: bool,
    strict_match: bool,
}

impl ResyClientBuilder {
    pub fn new(api_key: String, auth_key: String) -> ResyClientBuilder {
        ResyClientBuilder {
            api_key: api_key,
            auth_key: auth_key,
            no_cache: false,
            strict_match: false,
        }
    }

    pub fn no_cache(mut self) -> ResyClientBuilder {
        self.no_cache = true;
        self
    }

    pub fn strict_match(mut self) -> ResyClientBuilder {
        self.strict_match = true;
        self
    }

    pub fn build(self) -> ResyClient {
        let mut headers = HeaderMap::new();

        let mut api_header =
            HeaderValue::from_str(&format!("ResyAPI api_key=\"{}\"", self.api_key))
                .expect("Key invalid HTTP header value");
        api_header.set_sensitive(true);

        let mut auth_key_header =
            HeaderValue::from_str(&self.auth_key).expect("Key invalid HTTP header value");
        auth_key_header.set_sensitive(true);

        headers.insert(AUTHORIZATION, api_header);
        headers.insert(RESY_AUTH_TOKEN_HEADER, auth_key_header);
        headers.insert("User-Agent", HeaderValue::from_static(&USER_AGENT));

        ResyClient {
            api_key: self.api_key,
            auth_key: self.auth_key,
            no_cache: self.no_cache,
            strict_match: self.strict_match,
            client: Client::builder()
                .default_headers(headers)
                .build()
                .expect("Unable to construct HTTP client"),
            restaurants: Vec::<RestaurantCityConfig>::new(),
        }
    }
}
