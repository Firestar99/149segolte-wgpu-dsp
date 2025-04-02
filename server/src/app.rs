use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use tokio::runtime::Builder;

pub struct Application {
    cache: Arc<RwLock<HashMap<String, SearchStatus>>>,
    tx: tokio::sync::mpsc::Sender<InternalMessage>,
}

impl Application {
    pub fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let cache_clone = cache.clone();
        tokio::spawn(async move {
            let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
            loop {
                match rx.recv().await {
                    Some(InternalMessage::Search) => {
                        runtime.spawn(search(cache_clone.clone()));
                    }
                    Some(InternalMessage::Check) => {
                        runtime.spawn(check(cache_clone.clone()));
                    }
                    None => (),
                }
            }
        });
        Self { cache, tx }
    }
}

#[derive(Debug, Clone)]
pub enum InternalMessage {
    Search,
    Check,
}

#[derive(Debug, Clone, Serialize)]
struct Star {
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct Galaxy {
    seed: u32,
    star_count: u8,
    multipler: f32,
    stars: Vec<Star>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalaxyResponse {
    names: Vec<String>,
    galaxy: Galaxy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StarRules {
    seed: u32,
    star: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GalaxyRules {
    seed: u32,
    star: u8,
    galaxy: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub enum SearchStatus {
    StarSearch(Vec<(u32, u8)>),
    GalaxySearch(Vec<u32>),
    Running,
}

impl Application {
    pub fn galaxy_details(&self, seed: u32, star_count: u8, multipler: f32) -> GalaxyResponse {
        let galaxy = generate(seed, star_count, multipler);
        let names = galaxy.stars.iter().map(|star| star.name.clone()).collect();
        GalaxyResponse { names, galaxy }
    }

    pub fn find_star(&self, payload: StarRules) -> String {
        todo!("{:?}", payload)
    }

    pub fn find_galaxy(&self, payload: GalaxyRules) -> String {
        todo!("{:?}", payload)
    }

    pub fn find_star_status(&self, hash: String) -> Option<SearchStatus> {
        todo!("{}", hash)
    }

    pub fn find_galaxy_status(&self, hash: String) -> Option<SearchStatus> {
        todo!("{}", hash)
    }
}

fn generate(seed: u32, star_count: u8, multipler: f32) -> Galaxy {
    let is_infinite_resource = multipler >= 99.5;
    let is_rare_resource = multipler <= 0.1001;
    let oil_amount_multipler = if is_rare_resource { 0.5 } else { 1.0 };
    let gas_coef = if is_rare_resource { 0.8 } else { 1.0 };

    let mut stars = vec![];
    for i in 0..star_count {
        let name = format!("Star {}", i);
        stars.push(Star { name });
    }

    Galaxy {
        seed,
        star_count,
        multipler,
        stars,
    }
}

async fn search(cache: Arc<RwLock<HashMap<String, SearchStatus>>>) {
    todo!("{:?}", cache)
}

async fn check(cache: Arc<RwLock<HashMap<String, SearchStatus>>>) {
    todo!("{:?}", cache)
}
