use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate};

use crate::road::{camera::Camera, ticket::Ticket};

type Result = anyhow::Result<Option<Ticket>>;

pub struct PlateStorage {
    plates: HashMap<String, PlateState>,
}

impl PlateStorage {
    pub fn new() -> Self {
        PlateStorage {
            plates: HashMap::new(),
        }
    }
    pub fn update_plate(&mut self, name: &str, timestamp: u32, camera: &Camera) -> Result {
        let plate = self
            .plates
            .entry(name.to_string())
            .or_insert(PlateState::new(name, timestamp, camera));
        let ticket = plate.update(timestamp, camera)?;
        Ok(ticket)
    }
}
pub struct PlateState {
    name: String,
    sightings: HashMap<u16, BTreeMap<u32, Camera>>, // time → mile
    ticketed_dates: HashSet<NaiveDate>,             // (t₀, t₁) you’ve already ticketed
}

impl PlateState {
    pub fn new(name: &str, timestamp: u32, camera: &Camera) -> Self {
        let mut sightings = HashMap::new();
        let mut b_tree = BTreeMap::new();
        b_tree.insert(timestamp, camera.clone());
        sightings.insert(camera.road, b_tree);
        Self {
            name: name.to_string(),
            sightings,
            ticketed_dates: HashSet::new(),
        }
    }

    pub fn update(&mut self, timestamp: u32, camera: &Camera) -> Result {
        let sightings_on_road = self.sightings.entry(camera.road).or_insert(BTreeMap::new());
        sightings_on_road.insert(timestamp, camera.clone());
        let ticket = self.has_ticket_for_road(&camera.road, &timestamp)?;
        if let Some(found_ticket) = &ticket {
            let date = to_date(found_ticket.timestamp2.clone());
            if self.ticketed_dates.contains(&date) {
                return Ok(None);
            }
            self.ticketed_dates.insert(date);
        }
        Ok(ticket)
    }

    pub fn has_ticket_for_road(
        &mut self,
        road: &u16,
        timestamp: &u32,
    ) -> Result {
        if let Some(sightings) = self.sightings.get(&road) {
            let sighting = match sightings.get(timestamp) {
                Some(cam) => cam,
                None => return Ok(None), // or error?
            };
            let (prev, next) = neighbors(sightings, timestamp.clone());
            if let Some((prev_timestamp, camera)) = prev {
                if camera.speeding(prev_timestamp, &sighting, &timestamp) {
                    let ticket = Ticket {
                        plate: self.name.clone(),
                        road: *road,
                        timestamp1: *prev_timestamp,
                        mile1: camera.location,
                        timestamp2: *timestamp,
                        mile2: sighting.location,
                        speed: sighting.limit,
                    };
                    return Ok(Some(ticket));
                }
            }
            if let Some((next_timestamp, camera)) = next {
                if sighting.speeding(timestamp, camera, next_timestamp) {
                    let ticket = Ticket {
                        plate: self.name.clone(),
                        road: *road,
                        timestamp1: *timestamp,
                        mile1: sighting.location,
                        timestamp2: *next_timestamp,
                        mile2: camera.location,
                        speed: camera.limit,
                    };
                    return Ok(Some(ticket));
                }
            }
        }
        Ok(None)
    }
}

fn neighbors<K: Ord + Copy, V>(
    map: &BTreeMap<K, V>,
    key: K,
) -> (Option<(&K, &V)>, Option<(&K, &V)>) {
    let prev = map.range(..key).next_back();
    let next = map
        .range((std::ops::Bound::Excluded(key), std::ops::Bound::Unbounded))
        .next();
    (prev, next)
}

fn to_date(ts: u32) -> NaiveDate {
    let datetime = DateTime::from_timestamp(ts as i64, 0).expect("Invalid timestamp");
    datetime.date_naive()
}
