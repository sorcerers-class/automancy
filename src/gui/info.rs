use ractor::rpc::CallResult;

use automancy_defs::{colors, glam::vec2, id::Id, rendering::InstanceData, stack::ItemStack};
use automancy_resources::data::Data;
use automancy_resources::types::IconMode;
use winit::keyboard::{Key, NamedKey};
use yakui::{
    widgets::{Absolute, Layer, Pad},
    Alignment, Dim2, Pivot,
};

use crate::tile_entity::TileEntityMsg;
use crate::GameState;

use super::{
    col, col_align_end, colored_label, colored_sized_text, group, item::draw_item, label, row,
    ui_game_object, window_box, LABEL_SIZE, LARGE_ICON_SIZE, PADDING_LARGE, SMALL_ICON_SIZE,
};

#[track_caller]
fn input_hint_names(state: &mut GameState) {
    for hint in &state.input_hints {
        let name = hint
            .last()
            .and_then(|action| {
                state
                    .input_handler
                    .key_map
                    .values()
                    .find(|v| v.action == *action)
            })
            .and_then(|v| v.name);

        if let Some(name) = name.and_then(|name| state.resource_man.translates.keys.get(&name)) {
            label(name);
        } else {
            label(&state.resource_man.translates.unnamed);
        }
    }
}

#[track_caller]
fn input_hint_keys(state: &mut GameState) {
    for hint in &state.input_hints {
        let hint_text = hint
            .iter()
            .flat_map(|action| {
                if let Some((key, _key_action)) = state
                    .input_handler
                    .key_map
                    .iter()
                    .find(|(_, v)| v.action == *action)
                {
                    if let Key::Character(c) = key {
                        Some(c.to_uppercase())
                    } else if let Key::Named(n) = key {
                        match n {
                            NamedKey::Alt => Some("Alt".to_string()),
                            NamedKey::Control => Some("Ctrl".to_string()),
                            NamedKey::Shift => Some("Shift".to_string()),
                            NamedKey::Delete => Some("Del".to_string()),
                            NamedKey::Backspace => Some("Backspace".to_string()),
                            NamedKey::Enter => Some("Enter".to_string()),
                            NamedKey::Escape => Some("Esc".to_string()),
                            NamedKey::Tab => Some("Tab".to_string()),
                            NamedKey::F1 => Some("F1".to_string()),
                            NamedKey::F2 => Some("F2".to_string()),
                            NamedKey::F3 => Some("F3".to_string()),
                            NamedKey::F4 => Some("F4".to_string()),
                            NamedKey::F5 => Some("F5".to_string()),
                            NamedKey::F6 => Some("F6".to_string()),
                            NamedKey::F7 => Some("F7".to_string()),
                            NamedKey::F8 => Some("F8".to_string()),
                            NamedKey::F9 => Some("F9".to_string()),
                            NamedKey::F10 => Some("F10".to_string()),
                            NamedKey::F11 => Some("F11".to_string()),
                            NamedKey::F12 => Some("F12".to_string()),
                            NamedKey::ArrowLeft => Some("Left".to_string()),
                            NamedKey::ArrowUp => Some("Up".to_string()),
                            NamedKey::ArrowDown => Some("Down".to_string()),
                            NamedKey::ArrowRight => Some("Right".to_string()),
                            _ => Some("<?>".to_string()),
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" + ");

        colored_sized_text(&hint_text, colors::GRAY, LABEL_SIZE).show();
    }
}

fn rest_of_the_info(state: &mut GameState) {
    group(|| {
        row(|| {
            col(|| {
                input_hint_names(state);
            });

            col_align_end(|| {
                input_hint_keys(state);
            });
        });
    });
}

fn tile_icon(state: &mut GameState, id: Id) {
    ui_game_object(
        InstanceData::default().with_model_matrix(IconMode::Tile.model_matrix()),
        state.resource_man.tile_model_or_missing(id),
        vec2(LARGE_ICON_SIZE, LARGE_ICON_SIZE),
        Some(IconMode::Tile.world_matrix()),
    );
}

/// Draws the info GUI.
pub fn info_ui(state: &mut GameState) {
    Absolute::new(Alignment::TOP_RIGHT, Pivot::TOP_RIGHT, Dim2::ZERO).show(|| {
        Layer::new().show(|| {
            Pad::all(PADDING_LARGE).show(|| {
                window_box(
                    state
                        .resource_man
                        .gui_str(state.resource_man.registry.gui_ids.info)
                        .to_string(),
                    || {
                        colored_label(&state.camera.pointing_at.to_string(), colors::DARK_GRAY);

                        let Some((tile, entity)) =
                            state.loop_store.pointing_cache.blocking_lock().clone()
                        else {
                            label(
                                &state
                                    .resource_man
                                    .tile_name(state.resource_man.registry.none),
                            );

                            tile_icon(state, state.resource_man.registry.none);

                            rest_of_the_info(state);

                            return;
                        };

                        label(&state.resource_man.tile_name(tile));

                        let Ok(CallResult::Success(data)) = state
                            .tokio
                            .block_on(entity.call(TileEntityMsg::GetData, None))
                        else {
                            tile_icon(state, tile);

                            rest_of_the_info(state);

                            return;
                        };

                        tile_icon(state, tile);

                        if let Some(Data::Inventory(inventory)) =
                            data.get(state.resource_man.registry.data_ids.buffer)
                        {
                            for (id, amount) in inventory.iter() {
                                draw_item(
                                    &state.resource_man,
                                    || {},
                                    ItemStack {
                                        id: *id,
                                        amount: *amount,
                                    },
                                    SMALL_ICON_SIZE,
                                    true,
                                );
                            }
                        }

                        rest_of_the_info(state);
                    },
                );
            });
        });
    });
}
