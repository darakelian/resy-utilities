use std::str::FromStr;

use reqwest::Client;
use resy_data::{GeoFilter, RestaurantCityConfig, RestaurantSearchParams, RestaurantSearchResult};

mod resy_data;

/// Base URL for location based queries
static RESY_LOCATION_BASE: &str = "https://api.resy.com/3/location";
/// URL path to fetch the location config data
static RESY_CONFIG_URL: &str = "/config";
static RESY_VENUESEARCH_BASE: &str = "https://api.resy.com/3/venuesearch";
static RESY_VENUESEARCH_SEARCH: &str = "/search";

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
    restaurants: Vec<RestaurantCityConfig>
}

impl ResyClient {
    pub fn builder() -> ResyClientBuilder {
        ResyClientBuilder::default()
    }

    /// Loads the restaurant city configs from Resy. This is all cities in the Resy network
    /// that we can search for restaurants later.
    /// TODO: Make this use a caching mechanism?
    pub async fn load_config(&mut self) -> anyhow::Result<()> {
        let mapping_body = self.client.get(format!("{}{}", RESY_LOCATION_BASE, RESY_CONFIG_URL))
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
    pub fn get_restaurant_city_config(&self, city: &String, country: &String) -> Option<RestaurantCityConfig> {
        for restaurant_city_config in self.restaurants.iter() {
            if restaurant_city_config.is_match(city, country) {
                return Some(restaurant_city_config.clone())
            }
        }
        None
    }

    /// Tries to get the restaurant. Assumes the restaurant name provided is unique
    /// to only have one restaurant in the given city.
    pub async fn find_restaurant(&self, city_config: &RestaurantCityConfig, name: &String) -> anyhow::Result<Option<RestaurantSearchResult>> {
        let geo_filter = GeoFilter::new(city_config.latitude, city_config.longitude, u16::MAX);
        let restaurant_search_params = RestaurantSearchParams::new(false, &geo_filter, name);
        
        let res = self.client.post(format!("{}{}", RESY_VENUESEARCH_BASE, RESY_VENUESEARCH_SEARCH))
            .header(RESY_AUTH_TOKEN_HEADER, &self.auth_key)
            .header("Authorization", format!("ResyAPI api_key=\"{}\"", self.api_key))
            .json(&restaurant_search_params)
            .send()
            .await?;
        
        let text = res.text().await?;
        let value = serde_json::Value::from_str(&text).unwrap();
        let hits: Vec<RestaurantSearchResult> = serde_json::from_value::<Vec<RestaurantSearchResult>>(value["search"]["hits"].clone()).unwrap();
        Ok(hits.first().cloned())
    }
}

#[derive(Default)]
pub struct ResyClientBuilder {
    api_key: String,
    auth_key: String,
    no_cache: bool,
    strict_match: bool
}

impl ResyClientBuilder {
    pub fn new() -> ResyClientBuilder {
        ResyClientBuilder {
            api_key: String::from(""),
            auth_key: String::from(""),
            no_cache: false,
            strict_match: false
        }
    }

    pub fn api_key(mut self, api_key: String) -> ResyClientBuilder {
        self.api_key = api_key;
        self
    }

    pub fn auth_key(mut self, auth_key: String) ->  ResyClientBuilder {
        self.auth_key = auth_key;
        self
    }

    pub fn no_cache(mut self, no_cache: bool) -> ResyClientBuilder {
        self.no_cache = no_cache;
        self
    }

    pub fn strict_match(mut self, strict_match: bool) -> ResyClientBuilder {
        self.strict_match = strict_match;
        self
    }

    pub fn build(self) -> ResyClient {
        ResyClient {
            api_key: self.api_key,
            auth_key: self.auth_key,
            no_cache: self.no_cache,
            strict_match: self.strict_match,
            client: Client::new(),
             restaurants: Vec::<RestaurantCityConfig>::new()
        }
    }
}