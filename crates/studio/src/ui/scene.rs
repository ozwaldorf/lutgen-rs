use egui::emath::{Pos2, Rect, TSTransform, Vec2};
use egui::{
    DragPanButtons,
    InnerResponse,
    LayerId,
    PointerButton,
    Rangef,
    Response,
    Sense,
    Ui,
    UiBuilder,
};

/// A scene container that holds a single item with constrained panning and limited zoom.
///
/// Unlike [`Scene`], this container:
/// * By default has a minimum zoom of 1.0 (can zoom in but not zoom out past 1:1 ratio)
/// * Limits panning so the item edges stay within the view bounds
/// * Is designed for viewing a single item with controlled navigation
#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct ConstrainedScene {
    zoom_range: Rangef,
}

/// Centers the item in the view and scales it to fit within the available space with padding.
/// This is useful for ensuring content starts centered and properly sized with breathing room.
fn center_item(view_rect: Rect, item_rect: Rect, transform: &mut TSTransform) {
    if item_rect.is_finite() && item_rect.size() != Vec2::ZERO {
        transform.scaling = 0.95;

        // Center the scaled item
        let scaled_item_center = item_rect.center() * 0.9;
        let center_offset = view_rect.center() - scaled_item_center;
        transform.translation = center_offset;
    }
}

impl ConstrainedScene {
    #[inline]
    pub fn new(zoom_range: impl Into<Rangef>) -> Self {
        Self {
            zoom_range: zoom_range.into(),
        }
    }

    /// Show the constrained scene with a single item.
    ///
    /// The transform is provided externally and will be modified in place.
    /// The transform is constrained so the item stays within view bounds and zoom
    /// doesn't go below 1.0.
    pub fn show<R>(
        &self,
        parent_ui: &mut Ui,
        transform: &mut TSTransform,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let (outer_rect, _outer_response) =
            parent_ui.allocate_exact_size(parent_ui.available_size_before_wrap(), Sense::hover());

        let mut current_item_bounds = Rect::NOTHING;

        // Store previous item bounds from UI state for constraint calculations
        let prev_item_bounds = parent_ui
            .ctx()
            .data(|d| d.get_temp::<Rect>(parent_ui.id().with("constrained_scene_item_bounds")));

        let ret =
            self.show_with_transform(parent_ui, outer_rect, transform, prev_item_bounds, |ui| {
                let r = add_contents(ui);
                current_item_bounds = ui.min_rect();
                r
            });

        // Store current item bounds for next frame
        parent_ui.ctx().data_mut(|d| {
            d.insert_temp(
                parent_ui.id().with("constrained_scene_item_bounds"),
                current_item_bounds,
            );
        });

        // Always apply constraints, and center if this is the first time we have bounds
        if current_item_bounds.is_finite() && current_item_bounds.size() != Vec2::ZERO {
            // If we don't have previous bounds, this is likely the first frame - center the item
            if prev_item_bounds.is_none() || prev_item_bounds == Some(Rect::NOTHING) {
                center_item(outer_rect, current_item_bounds, transform);
            }

            // Constrain zoom to allowed range
            transform.scaling = self.zoom_range.clamp(transform.scaling);

            // Constrain panning to keep item within view bounds
            self.constrain_pan_to_bounds(outer_rect, current_item_bounds, transform);
        }

        ret
    }

    /// Calculates the transformed item bounds given a transform.
    fn calculate_transformed_item_bounds(
        &self,
        item_rect: Rect,
        transform: &TSTransform,
    ) -> (Vec2, Rect) {
        let scaled_item_size = item_rect.size() * transform.scaling;
        let scaled_item_min = item_rect.min.to_vec2() * transform.scaling + transform.translation;
        let transformed_item = Rect::from_min_size(scaled_item_min.to_pos2(), scaled_item_size);
        (scaled_item_size, transformed_item)
    }

    /// Constrains translation on a single axis.
    fn constrain_axis(
        &self,
        view_rect: Rect,
        item_rect: Rect,
        scaling: f32,
        transformed_rect: Rect,
        is_x_axis: bool,
    ) -> f32 {
        let (view_size, view_center, view_min, view_max) = if is_x_axis {
            (
                view_rect.width(),
                view_rect.center().x,
                view_rect.left(),
                view_rect.right(),
            )
        } else {
            (
                view_rect.height(),
                view_rect.center().y,
                view_rect.top(),
                view_rect.bottom(),
            )
        };

        let (item_size, item_center, item_min, item_max) = if is_x_axis {
            (
                item_rect.width(),
                item_rect.center().x,
                item_rect.left(),
                item_rect.right(),
            )
        } else {
            (
                item_rect.height(),
                item_rect.center().y,
                item_rect.top(),
                item_rect.bottom(),
            )
        };

        let (transformed_min, transformed_max) = if is_x_axis {
            (transformed_rect.left(), transformed_rect.right())
        } else {
            (transformed_rect.top(), transformed_rect.bottom())
        };

        if item_size <= view_size {
            // Item is smaller than view - center it
            view_center - (item_center * scaling)
        } else {
            // Item is larger than view - constrain edges
            let current_translation = transformed_min - (item_min * scaling);
            let mut constrained = current_translation;

            if transformed_min > view_min {
                // Item edge is past view edge - move to boundary
                constrained = view_min - (item_min * scaling);
            } else if transformed_max < view_max {
                // Item edge is past view edge - move to boundary
                constrained = view_max - (item_max * scaling);
            }

            constrained
        }
    }

    /// Constrains a pan delta on a single axis to prevent overscrolling.
    fn constrain_axis_delta(
        &self,
        view_rect: Rect,
        item_rect: Rect,
        current_transform: &TSTransform,
        test_transformed_rect: Rect,
        delta: f32,
        is_x_axis: bool,
    ) -> f32 {
        let (view_size, view_min, view_max) = if is_x_axis {
            (view_rect.width(), view_rect.left(), view_rect.right())
        } else {
            (view_rect.height(), view_rect.top(), view_rect.bottom())
        };

        let (item_size, item_min, item_max) = if is_x_axis {
            (item_rect.width(), item_rect.left(), item_rect.right())
        } else {
            (item_rect.height(), item_rect.top(), item_rect.bottom())
        };

        let (test_transformed_min, test_transformed_max) = if is_x_axis {
            (test_transformed_rect.left(), test_transformed_rect.right())
        } else {
            (test_transformed_rect.top(), test_transformed_rect.bottom())
        };

        if item_size <= view_size {
            // Item is smaller than view - don't allow panning
            0.0
        } else {
            // Item is larger than view - check bounds
            let translation_component = if is_x_axis {
                current_transform.translation.x
            } else {
                current_transform.translation.y
            };
            let current_item_min = item_min * current_transform.scaling + translation_component;
            let current_item_max = item_max * current_transform.scaling + translation_component;
            let mut constrained_delta = delta;

            if test_transformed_min > view_min {
                // Would scroll past edge - limit delta to reach boundary
                constrained_delta = view_min - current_item_min;
            }
            if test_transformed_max < view_max {
                // Would scroll past edge - limit delta to reach boundary
                constrained_delta = view_max - current_item_max;
            }

            constrained_delta
        }
    }

    /// Constrains the pan offset so the item stays within the view bounds.
    fn constrain_pan_to_bounds(
        &self,
        view_rect: Rect,
        item_rect: Rect,
        transform: &mut TSTransform,
    ) {
        if item_rect.is_finite() && item_rect.size() != Vec2::ZERO {
            let (_, transformed_item) =
                self.calculate_transformed_item_bounds(item_rect, transform);

            let constrained_x = self.constrain_axis(
                view_rect,
                item_rect,
                transform.scaling,
                transformed_item,
                true,
            );

            let constrained_y = self.constrain_axis(
                view_rect,
                item_rect,
                transform.scaling,
                transformed_item,
                false,
            );

            transform.translation = Vec2::new(constrained_x, constrained_y);
        }
    }

    /// Constrains a pan delta to prevent overscrolling past bounds.
    fn constrain_pan_delta(
        &self,
        view_rect: Rect,
        item_rect: Rect,
        transform: &TSTransform,
        pan_delta: Vec2,
    ) -> Vec2 {
        if !item_rect.is_finite() || item_rect.size() == Vec2::ZERO {
            return pan_delta;
        }

        let test_transform = TSTransform::from_translation(pan_delta) * *transform;
        let (_, test_item_rect) =
            self.calculate_transformed_item_bounds(item_rect, &test_transform);

        let constrained_delta_x = self.constrain_axis_delta(
            view_rect,
            item_rect,
            transform,
            test_item_rect,
            pan_delta.x,
            true,
        );

        let constrained_delta_y = self.constrain_axis_delta(
            view_rect,
            item_rect,
            transform,
            test_item_rect,
            pan_delta.y,
            false,
        );

        Vec2::new(constrained_delta_x, constrained_delta_y)
    }

    fn show_with_transform<R>(
        &self,
        parent_ui: &mut Ui,
        outer_rect: Rect,
        to_global: &mut TSTransform,
        prev_item_bounds: Option<Rect>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        // Create a new egui paint layer, where we can draw our contents:
        let scene_layer_id = LayerId::new(
            parent_ui.layer_id().order,
            parent_ui.id().with("constrained_scene_area"),
        );

        // Put the layer directly on-top of the main layer of the ui:
        parent_ui
            .ctx()
            .set_sublayer(parent_ui.layer_id(), scene_layer_id);

        // Use item bounds if available, otherwise use a large default size
        let max_rect_size = if let Some(item_rect) = prev_item_bounds {
            item_rect.size().max(Vec2::splat(100.0)) // Ensure minimum size
        } else {
            parent_ui.available_size()
        };

        let mut local_ui = parent_ui.new_child(
            UiBuilder::new()
                .layer_id(scene_layer_id)
                .max_rect(Rect::from_min_size(Pos2::ZERO, max_rect_size))
                .sense(Sense::click_and_drag()),
        );

        let mut pan_response = local_ui.response();

        // Update the `to_global` transform based on user interaction:
        self.register_constrained_pan_and_zoom(
            &local_ui,
            &mut pan_response,
            to_global,
            outer_rect,
            prev_item_bounds,
        );

        // Set a correct global clip rect:
        local_ui.set_clip_rect(to_global.inverse() * outer_rect);

        // Tell egui to apply the transform on the layer:
        local_ui
            .ctx()
            .set_transform_layer(scene_layer_id, *to_global);

        // Add the actual contents to the area:
        let ret = add_contents(&mut local_ui);

        // This ensures we catch clicks/drags/pans anywhere on the background.
        // local_ui.force_set_min_rect((to_global.inverse() * outer_rect).round_ui());

        InnerResponse {
            response: pan_response,
            inner: ret,
        }
    }

    /// Helper function to handle constrained pan and zoom interactions.
    fn register_constrained_pan_and_zoom(
        &self,
        ui: &Ui,
        resp: &mut Response,
        to_global: &mut TSTransform,
        view_rect: Rect,
        item_bounds: Option<Rect>,
    ) {
        let dragged = DragPanButtons::all().iter().any(|button| match button {
            DragPanButtons::PRIMARY => resp.dragged_by(PointerButton::Primary),
            DragPanButtons::SECONDARY => resp.dragged_by(PointerButton::Secondary),
            DragPanButtons::MIDDLE => resp.dragged_by(PointerButton::Middle),
            DragPanButtons::EXTRA_1 => resp.dragged_by(PointerButton::Extra1),
            DragPanButtons::EXTRA_2 => resp.dragged_by(PointerButton::Extra2),
            _ => false,
        });

        if dragged {
            to_global.translation += to_global.scaling * resp.drag_delta();
            resp.mark_changed();
        }

        // Handle zoom and pan input - allow zooming even when mouse is outside content
        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

        // Most of the time we can return early. This is also important to
        // avoid `to_global` to change slightly due to floating point errors.
        if zoom_delta == 1.0 && pan_delta == Vec2::ZERO {
            return;
        }

        if zoom_delta != 1.0 {
            // Determine zoom point: use mouse position if over content, otherwise use scene center
            let zoom_point = if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos())
                && resp.contains_pointer()
            {
                // Mouse is over content - zoom on pointer
                to_global.inverse() * mouse_pos
            } else {
                // Mouse is outside content or not available - zoom on scene center
                to_global.inverse() * view_rect.center()
            };

            // Zoom in on the determined point, but constrain to zoom range
            let zoom_delta = zoom_delta.clamp(
                self.zoom_range.min / to_global.scaling,
                self.zoom_range.max / to_global.scaling,
            );

            *to_global = *to_global
                * TSTransform::from_translation(zoom_point.to_vec2())
                * TSTransform::from_scaling(zoom_delta)
                * TSTransform::from_translation(-zoom_point.to_vec2());

            // Clamp to exact zoom range.
            to_global.scaling = self.zoom_range.clamp(to_global.scaling);
            resp.mark_changed();
        }

        // Pan via scroll wheel - only when mouse is over content
        if pan_delta != Vec2::ZERO
            && let Some(_mouse_pos) = ui.input(|i| i.pointer.latest_pos())
            && resp.contains_pointer()
        {
            // Pre-constrain the pan delta to prevent overscrolling
            let constrained_pan_delta = if let Some(item_rect) = item_bounds {
                self.constrain_pan_delta(view_rect, item_rect, to_global, pan_delta)
            } else {
                pan_delta
            };

            if constrained_pan_delta != Vec2::ZERO {
                *to_global = TSTransform::from_translation(constrained_pan_delta) * *to_global;
                resp.mark_changed();
            }
        }
    }
}
