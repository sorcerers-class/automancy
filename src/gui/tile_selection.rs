use std::f64::consts::FRAC_PI_4;

use tokio::sync::oneshot;

use automancy_defs::glam::{dvec3, vec2, vec3, FloatExt};
use automancy_defs::id::Id;
use automancy_defs::math::{z_far, z_near, DMatrix4, Float, Matrix4};
use automancy_defs::rendering::InstanceData;
use automancy_defs::{colors, math};
use automancy_resources::data::{Data, DataMap};
use automancy_resources::format;
use yakui::{row, use_state, Alignment, Pivot, Vec2};

use crate::gui::{GameElement, LARGE_ICON_SIZE, MEDIUM_ICON_SIZE};
use crate::util::is_research_unlocked;
use crate::GameState;

use super::components::{absolute::Absolute, hover::Hover, interactive::interactive, text::label};

fn tile_hover_z_angle(elapsed: Float, hovered: bool) -> Float {
    fn angle(hovered: bool) -> Float {
        if hovered {
            0.75
        } else {
            0.25
        }
    }

    //TODO extract this
    let s = use_state(move || angle(hovered));

    let target = angle(hovered);

    let r = s.get();

    s.modify(|v| {
        let lerped = v.lerp(target, (v / target).abs() + elapsed);

        lerped.clamp(v.min(target), v.max(target))
    });

    r
}

fn has_category_item(state: &mut GameState, game_data: &mut DataMap, id: Id) -> bool {
    let category = &state.resource_man.registry.categories[&id];

    if let Some(item) = category.item {
        if let Some(Data::Inventory(inventory)) =
            game_data.get_mut(&state.resource_man.registry.data_ids.player_inventory)
        {
            inventory.get(item) > 0
        } else {
            false
        }
    } else {
        true
    }
}

/// Draws the tile selection.
fn draw_tile_selection(
    state: &mut GameState,
    game_data: &mut DataMap,
    selection_send: &mut Option<oneshot::Sender<Id>>,
    current_category: Option<Id>,
    size: Float,
) {
    let projection = DMatrix4::perspective_lh(FRAC_PI_4, 1.0, z_near(), z_far())
        * math::view(dvec3(0.0, 0.0, 2.75));
    let projection = projection.as_mat4();

    let has_item = if let Some(category) = current_category {
        has_category_item(state, game_data, category)
    } else {
        true
    };

    for id in &state.resource_man.ordered_tiles {
        if let Some(Data::Id(category)) = state.resource_man.registry.tiles[id]
            .data
            .get(&state.resource_man.registry.data_ids.category)
        {
            if Some(*category) != current_category {
                continue;
            }
        }

        let is_default_tile = match state.resource_man.registry.tiles[id]
            .data
            .get(&state.resource_man.registry.data_ids.default_tile)
        {
            Some(Data::Bool(v)) => *v,
            _ => false,
        };

        if !is_default_tile {
            if let Some(research) = state.resource_man.get_research_by_unlock(*id) {
                if !is_research_unlocked(research.id, &state.resource_man, game_data) {
                    continue;
                }
            }
        }

        let tile = state.resource_man.registry.tiles.get(id).unwrap();
        let model = state.resource_man.get_model(tile.model);

        let hovered = use_state(|| false);

        let rotate = Matrix4::from_rotation_x(tile_hover_z_angle(
            state.loop_store.elapsed.as_secs_f32(),
            hovered.get(),
        ));

        let color_offset = if is_default_tile || has_item {
            Default::default()
        } else {
            colors::INACTIVE.to_linear()
        };

        let response = interactive(|| {
            GameElement::new(
                InstanceData::default()
                    .with_model_matrix(rotate)
                    .with_world_matrix(projection)
                    .with_light_pos(vec3(0.0, 4.0, 14.0), None)
                    .with_color_offset(color_offset),
                model,
                vec2(size, size),
            )
            .show();
        });

        hovered.set(response.hovering);

        if response.hovering {
            Hover::new().show(|| {
                label(&state.resource_man.tile_name(id));

                if !(is_default_tile || has_item) {
                    if let Some(item) = current_category
                        .and_then(|id| state.resource_man.registry.categories[&id].item)
                    {
                        label(&format(
                            state.resource_man.translates.gui[&state
                                .resource_man
                                .registry
                                .gui_ids
                                .lbl_cannot_place_missing_item]
                                .as_str(),
                            &[&state.resource_man.item_name(&item)],
                        ));
                    };
                }
            });
        }

        if response.clicked && (is_default_tile || has_item) {
            if let Some(send) = selection_send.take() {
                send.send(*id).unwrap();
            }
        }
    }
}

/// Creates the tile selection GUI.
pub fn tile_selections(
    state: &mut GameState,
    game_data: &mut DataMap,
    selection_send: oneshot::Sender<Id>,
) {
    let projection = DMatrix4::perspective_lh(FRAC_PI_4, 1.0, z_near(), z_far())
        * math::view(dvec3(0.0, 0.0, 2.75));
    let projection = projection.as_mat4();

    Absolute::new(Alignment::BOTTOM_CENTER, Pivot::BOTTOM_CENTER, Vec2::ZERO).show(|| {
        row(|| {
            for id in &state.resource_man.ordered_categories {
                let category = &state.resource_man.registry.categories[id];
                let model = state.resource_man.get_model(category.icon);

                let response = interactive(|| {
                    GameElement::new(
                        InstanceData::default()
                            .with_world_matrix(projection)
                            .with_light_pos(vec3(0.0, 4.0, 14.0), None),
                        model,
                        vec2(MEDIUM_ICON_SIZE, MEDIUM_ICON_SIZE),
                    )
                    .show();
                });

                if response.clicked {
                    state.gui_state.tile_selection_category = Some(*id);
                }

                if response.hovering {
                    Hover::new().show(|| {
                        label(&state.resource_man.category_name(id));
                    });
                }
            }
        });

        row(|| {
            draw_tile_selection(
                state,
                game_data,
                &mut Some(selection_send),
                state.gui_state.tile_selection_category,
                LARGE_ICON_SIZE,
            );
        });
    });
}
