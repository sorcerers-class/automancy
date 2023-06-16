use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::Arc;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::resource::{load_recursively, Registry, ResourceManager, JSON_EXT};
use crate::util::id::{Id, IdRaw, Interner};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ItemRaw {
    pub id: IdRaw,
    pub model: IdRaw,
}

impl ItemRaw {
    pub fn intern_to_item(self, interner: &mut Interner) -> Item {
        Item {
            id: self.id.to_id(interner),
            model: self.model.to_id(interner),
        }
    }

    pub fn to_item(self, interner: &Interner) -> Option<Item> {
        let id = interner.get(self.id.to_string())?;
        let model = interner.get(self.model.to_string())?;

        Some(Item { id, model })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Item {
    pub id: Id,
    pub model: Id,
}

impl Item {
    pub fn to_item_raw(self, interner: &Interner) -> Option<ItemRaw> {
        let id = IdRaw::parse(interner.resolve(self.id)?);
        let model = IdRaw::parse(interner.resolve(self.model)?);

        Some(ItemRaw { id, model })
    }
}

pub fn id_match(registry: &Registry, id: Id, other: Id) -> bool {
    if id == other {
        return true;
    }

    if let Some(tag) = registry.tags.get(&other) {
        return tag.of(registry, id);
    }

    false
}

impl ResourceManager {
    fn load_item(&mut self, file: &Path) -> Option<()> {
        log::info!("loading item at: {file:?}");

        let item: ItemRaw = serde_json::from_str(
            &read_to_string(file).unwrap_or_else(|e| panic!("error loading {file:?} {e:?}")),
        )
        .unwrap_or_else(|e| panic!("error loading {file:?} {e:?}"));

        let item = item.intern_to_item(&mut self.interner);

        self.registry.items.insert(item.id, item);

        Some(())
    }

    pub fn load_items(&mut self, dir: &Path) -> Option<()> {
        let items = dir.join("items");

        load_recursively(&items, OsStr::new(JSON_EXT))
            .into_iter()
            .for_each(|file| {
                self.load_item(&file);
            });

        Some(())
    }

    pub fn get_items(&self, id: Id, tag_cache: &mut HashMap<Id, Arc<Vec<Item>>>) -> Arc<Vec<Item>> {
        if let Some(item) = self.registry.items.get(&id) {
            Arc::new(vec![*item])
        } else {
            tag_cache.entry(id).or_insert_with(|| {
                let items = self
                    .ordered_items
                    .iter()
                    .filter(|v| id_match(&self.registry, **v, id))
                    .flat_map(|v| self.registry.item(*v).cloned())
                    .collect();

                Arc::new(items)
            });

            tag_cache[&id].clone()
        }
    }

    pub fn ordered_items(&mut self) {
        let mut ids = self.registry.items.keys().cloned().collect::<Vec<_>>();

        ids.sort_unstable_by_key(|id| self.item_name(id));

        self.ordered_items = ids;
    }
}
