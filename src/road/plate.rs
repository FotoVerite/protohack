use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate};

use crate::road::{camera::Camera, ticket::Ticket};

type Result = anyhow::Result<Vec<Ticket>>;

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

    pub fn update(&mut self, timestamp: u32, camera: &Camera) -> anyhow::Result<Vec<Ticket>> {
        println!("adding sighting {} for {:?}", timestamp, camera);
        let sightings_on_road = self.sightings.entry(camera.road).or_insert(BTreeMap::new());
        sightings_on_road.insert(timestamp, camera.clone());
        let tickets = self.has_ticket_for_road(&camera.road, &timestamp)?;
        let actionable_tickets = tickets
            .iter()
            .filter_map(|ticket| {
                let start_date = to_date(ticket.timestamp1.clone());
                let end_date = to_date(ticket.timestamp2.clone());

                if self.ticketed_dates.contains(&start_date)
                    || self.ticketed_dates.contains(&end_date)
                {
                    return None;
                }

                self.ticketed_dates.insert(start_date);
                self.ticketed_dates.insert(end_date);

                Some(ticket.clone())
            })
            .collect();
        Ok(actionable_tickets)
    }

    pub fn has_ticket_for_road(
        &mut self,
        road: &u16,
        timestamp: &u32,
    ) -> anyhow::Result<Vec<Ticket>> {
        let mut tickets = vec![];

        if let Some(sightings) = self.sightings.get(&road) {
            let sighting = match sightings.get(timestamp) {
                Some(cam) => cam,
                None => return Ok(vec![]), // or error?
            };
            let (prev, next) = neighbors(sightings, timestamp.clone());
            if let Some((prev_timestamp, camera)) = prev {
                if let Some(speed) = camera.speeding(prev_timestamp, &sighting, &timestamp) {
                    let ticket = Ticket {
                        plate: self.name.clone(),
                        road: *road,
                        timestamp1: *prev_timestamp,
                        mile1: camera.location,
                        timestamp2: *timestamp,
                        mile2: sighting.location,
                        speed: speed * 100,
                    };
                    tickets.push(ticket);
                }
            }
            if let Some((next_timestamp, camera)) = next {
                if let Some(speed) = sighting.speeding(timestamp, camera, next_timestamp) {
                    let ticket = Ticket {
                        plate: self.name.clone(),
                        road: *road,
                        timestamp1: *timestamp,
                        mile1: sighting.location,
                        timestamp2: *next_timestamp,
                        mile2: camera.location,
                        speed: speed * 100,
                    };
                    tickets.push(ticket);
                }
            }
        }
        Ok(tickets)
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
