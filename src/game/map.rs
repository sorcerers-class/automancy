use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::{collections::HashMap, path::PathBuf};

use chrono::{Local, Utc};
use ractor::ActorRef;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use zstd::{Decoder, Encoder};

use crate::game;
use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::{GetData, SetData};
use crate::game::tile::entity::{
    data_from_raw, data_to_raw, DataMap, DataMapRaw, TileEntityMsg, TileModifier,
};
use crate::game::GameMsg;
use crate::resource::ResourceManager;
use crate::util::id::{Id, Interner};

pub const MAP_PATH: &str = "map";

const MAP_BUFFER_SIZE: usize = 256 * 1024;

pub type Tiles = HashMap<TileCoord, (Id, TileModifier)>;
pub type TileEntities = HashMap<TileCoord, ActorRef<TileEntityMsg>>;

#[derive(Debug, Clone)]
pub struct Map {
    pub map_name: String,

    pub tiles: Tiles,
    pub data: DataMap,

    pub save_time: i64,
}

#[derive(Debug, Clone)]
pub struct MapInfo {
    pub map_name: String,
    pub tiles: usize,
    pub data: usize,
    pub save_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MapHeader(Vec<(Id, String)>);

#[derive(Debug, Serialize, Deserialize)]
struct SerdeMap {
    #[serde(default)]
    pub header: MapHeader,
    #[serde(default)]
    pub serde_tiles: Vec<(TileCoord, SerdeTile)>,
    #[serde(default)]
    pub data: DataMapRaw,
    #[serde(default)]
    pub save_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerdeTile(Id, TileModifier, DataMapRaw);

impl Map {
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,

            tiles: Default::default(),
            data: Default::default(),
            save_time: Local::now().timestamp(),
        }
    }

    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}.bin"))
    }

    pub fn save(&self, runtime: &Runtime, interner: &Interner, tile_entities: TileEntities) {
        drop(std::fs::create_dir_all(MAP_PATH));

        let path = Self::path(&self.map_name);

        let file = File::create(path).unwrap();

        let writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, file);
        let mut encoder = Encoder::new(writer, 0).unwrap();

        let mut id_map = HashMap::new();

        let serde_tiles = self
            .tiles
            .iter()
            .flat_map(|(coord, (id, tile_modifier))| {
                if let Some(tile_entity) = tile_entities.get(coord) {
                    if !id_map.contains_key(id) {
                        id_map.insert(*id, interner.resolve(*id).unwrap().to_string());
                    }

                    let data = runtime
                        .block_on(tile_entity.call(GetData, None))
                        .unwrap()
                        .unwrap(); // TODO call multi
                    let data = data_to_raw(data, interner);

                    tile_entity.stop(None);

                    Some((coord, SerdeTile(*id, *tile_modifier, data)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let header = MapHeader(id_map.into_iter().collect());

        let data = data_to_raw(self.data.clone(), interner);

        let save_time = Utc::now().timestamp();

        serde_json::to_writer(&mut encoder, &(header, serde_tiles, data, save_time)).unwrap();

        encoder.do_finish().unwrap();
    }

    pub async fn load(
        game: &ActorRef<GameMsg>,
        resource_man: &ResourceManager,
        map_name: String,
    ) -> (Self, TileEntities) {
        let path = Self::path(&map_name);

        let file = if let Ok(file) = File::open(path) {
            file
        } else {
            return (Map::new_empty(map_name), Default::default());
        };

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);
        let decoder = Decoder::new(reader).unwrap();

        let decoded_map: serde_json::Result<SerdeMap> = serde_json::from_reader(decoder);

        if decoded_map.is_err() {
            log::error!("serde: {:?}", decoded_map.err());

            let err_map_name = format!("{}-ERR-{}", map_name, Local::now().format("%y%m%d%H%M%S"));

            resource_man.error_man.push(
                (
                    resource_man.registry.err_ids.invalid_map_data,
                    vec![map_name, err_map_name.clone()],
                ),
                resource_man,
            );
            return (Map::new_empty(err_map_name), Default::default());
        }
        let SerdeMap {
            header,
            serde_tiles,
            data,
            save_time,
            ..
        } = decoded_map.unwrap();

        let id_reverse = header.0.into_iter().collect::<HashMap<_, _>>();

        let mut tiles = HashMap::new();
        let mut tile_entities = HashMap::new();

        for (coord, SerdeTile(id, tile_modifier, data)) in serde_tiles.into_iter() {
            if let Some(id) = id_reverse
                .get(&id)
                .and_then(|id| resource_man.interner.get(id.as_str()))
            {
                let tile_entity = game::new_tile(game, coord, id, tile_modifier).await;
                let data = data_from_raw(data, &resource_man.interner);

                data.into_iter().for_each(|(key, value)| {
                    tile_entity.send_message(SetData(key, value)).unwrap();
                });

                tiles.insert(coord, (id, tile_modifier));
                tile_entities.insert(coord, tile_entity);
            }
        }

        let data = data_from_raw(data, &resource_man.interner);

        (
            Self {
                map_name,

                tiles,
                data,

                save_time,
            },
            tile_entities,
        )
    }
}
