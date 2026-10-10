use crate::storage;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow, Window};

const EDGE_INSET: f64 = 16.0;

/// Logical offsets from the current screen's usable top-left corner, independent of its DPI.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SavedPosition {
    pub x: f64,
    pub y: f64,
}

impl SavedPosition {
    pub(crate) fn valid(self) -> bool {
        [self.x, self.y]
            .iter()
            .all(|value| value.is_finite() && (0.0..=100_000.0).contains(value))
    }
}

#[derive(Clone, Copy)]
struct Screen {
    origin: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale: f64,
}

impl From<&Monitor> for Screen {
    fn from(monitor: &Monitor) -> Self {
        let area = monitor.work_area();
        Self {
            origin: area.position,
            size: area.size,
            scale: monitor.scale_factor(),
        }
    }
}

impl Screen {
    fn clamp(
        self,
        position: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
    ) -> PhysicalPosition<i32> {
        let axis = |value: i32, origin: i32, extent: u32, window: u32| {
            let travel = extent.saturating_sub(window) as f64;
            (value as f64).clamp(origin as f64, origin as f64 + travel) as i32
        };
        PhysicalPosition::new(
            axis(position.x, self.origin.x, self.size.width, size.width),
            axis(position.y, self.origin.y, self.size.height, size.height),
        )
    }

    fn restore(
        self,
        saved: Option<SavedPosition>,
        size: PhysicalSize<u32>,
    ) -> PhysicalPosition<i32> {
        let position = if let Some(saved) = saved.filter(|position| position.valid()) {
            PhysicalPosition::new(
                (self.origin.x as f64 + saved.x * self.scale).round() as i32,
                (self.origin.y as f64 + saved.y * self.scale).round() as i32,
            )
        } else {
            let x_travel = self.size.width.saturating_sub(size.width) as f64;
            let y_travel = self.size.height.saturating_sub(size.height) as f64;
            PhysicalPosition::new(
                (self.origin.x as f64 + x_travel - (EDGE_INSET * self.scale).min(x_travel / 2.0))
                    as i32,
                (self.origin.y as f64 + (EDGE_INSET * self.scale).min(y_travel / 2.0)) as i32,
            )
        };
        self.clamp(position, size)
    }

    fn remember(self, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> SavedPosition {
        let position = self.clamp(position, size);
        SavedPosition {
            x: (position.x as f64 - self.origin.x as f64) / self.scale,
            y: (position.y as f64 - self.origin.y as f64) / self.scale,
        }
    }
}

#[derive(Default)]
struct Placement {
    saved: Option<SavedPosition>,
    automatic_moves: VecDeque<PhysicalPosition<i32>>,
    revision: u64,
    persisted_revision: u64,
}

impl Placement {
    fn target(
        &self,
        screen: Screen,
        size: PhysicalSize<u32>,
        visible: bool,
        current: PhysicalPosition<i32>,
    ) -> PhysicalPosition<i32> {
        if visible {
            // Status/height changes keep the current point, including a drag onto another screen.
            screen.clamp(current, size)
        } else {
            screen.restore(self.saved, size)
        }
    }

    fn expect_move(&mut self, position: PhysicalPosition<i32>) {
        // Multiple show/resize requests can arrive before their native move notifications.
        if self.automatic_moves.len() == 8 {
            self.automatic_moves.pop_front();
        }
        self.automatic_moves.push_back(position);
    }

    fn moved(
        &mut self,
        position: PhysicalPosition<i32>,
        screen: Screen,
        size: PhysicalSize<u32>,
        visible: bool,
    ) -> Option<u64> {
        if let Some(index) = self
            .automatic_moves
            .iter()
            .position(|expected| *expected == position)
        {
            self.automatic_moves.remove(index);
            return None;
        }
        if !visible {
            return None;
        }
        self.saved = Some(screen.remember(position, size));
        self.revision = self.revision.wrapping_add(1);
        Some(self.revision)
    }
}

pub(crate) struct OverlayWindowState(Mutex<Placement>);

impl OverlayWindowState {
    pub(crate) fn load() -> Self {
        Self(Mutex::new(Placement {
            saved: storage::load_overlay_position(),
            ..Placement::default()
        }))
    }

    fn persist(&self, revision: Option<u64>) {
        let Ok(mut placement) = self.0.lock() else {
            return;
        };
        if revision.is_some_and(|revision| placement.revision != revision)
            || placement.persisted_revision == placement.revision
        {
            return;
        }
        if let Some(saved) = placement.saved {
            match storage::save_overlay_position(&saved) {
                Ok(()) => placement.persisted_revision = placement.revision,
                Err(error) => storage::append_diagnostic(&format!(
                    "overlay position could not be saved: {error}"
                )),
            }
        }
    }
}

pub(crate) fn flush_position(app: &AppHandle) {
    // Quit can happen before the last debounced move has reached disk.
    app.state::<OverlayWindowState>().persist(None);
}

pub(crate) fn reposition(app: &AppHandle) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    let Ok(visible) = window.is_visible() else {
        return;
    };
    let monitor = if visible {
        window.current_monitor().ok().flatten()
    } else {
        app.cursor_position()
            .ok()
            .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
            .or_else(|| app.primary_monitor().ok().flatten())
    };
    let Some(monitor) = monitor else { return };
    let (Ok(size), Ok(current)) = (window.outer_size(), window.outer_position()) else {
        return;
    };
    let state = app.state::<OverlayWindowState>();
    let Ok(mut placement) = state.0.lock() else {
        return;
    };
    let target = placement.target(Screen::from(&monitor), size, visible, current);
    if target == current {
        return;
    }
    placement.expect_move(target);
    drop(placement);
    if window.set_position(target).is_err() {
        if let Ok(mut placement) = state.0.lock() {
            placement
                .automatic_moves
                .retain(|position| *position != target);
        }
    }
}

pub(crate) fn remember_move(window: &Window, position: PhysicalPosition<i32>) {
    let app = window.app_handle();
    let Some(state) = app.try_state::<OverlayWindowState>() else {
        return;
    };
    let (Ok(Some(monitor)), Ok(size), Ok(visible)) = (
        window.current_monitor(),
        window.outer_size(),
        window.is_visible(),
    ) else {
        return;
    };
    let revision =
        state.0.lock().ok().and_then(|mut placement| {
            placement.moved(position, Screen::from(&monitor), size, visible)
        });
    let Some(revision) = revision else { return };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Coalesce native move notifications so dragging does not write to disk on every pixel.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let _ = tokio::task::spawn_blocking(move || {
            app.state::<OverlayWindowState>().persist(Some(revision));
        })
        .await;
    });
}

#[tauri::command]
pub(crate) fn start_overlay_drag(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "overlay" {
        return Err("Само прозорецът за запис може да бъде преместен.".into());
    }
    if let Ok(mut placement) = window.state::<OverlayWindowState>().0.lock() {
        placement.automatic_moves.clear();
    }
    window.start_dragging().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Screen {
        Screen {
            origin: PhysicalPosition::new(0, 25),
            size: PhysicalSize::new(1440, 875),
            scale: 1.0,
        }
    }

    fn size() -> PhysicalSize<u32> {
        PhysicalSize::new(552, 156)
    }

    #[test]
    fn first_show_uses_top_right_of_usable_screen() {
        assert_eq!(
            screen().restore(None, size()),
            PhysicalPosition::new(872, 41)
        );
    }

    #[test]
    fn retina_screen_with_negative_origin_uses_logical_insets() {
        let screen = Screen {
            origin: PhysicalPosition::new(-2880, 50),
            size: PhysicalSize::new(2880, 1750),
            scale: 2.0,
        };
        assert_eq!(
            screen.restore(None, PhysicalSize::new(1104, 312)),
            PhysicalPosition::new(-1136, 82)
        );
    }

    #[test]
    fn status_and_height_changes_do_not_move_a_dragged_window() {
        let mut placement = Placement::default();
        for point in [PhysicalPosition::new(200, 250), screen().origin] {
            placement.moved(point, screen(), size(), true);
            for height in [132, 156, 200, 260, 132] {
                assert_eq!(
                    placement.target(screen(), PhysicalSize::new(552, height), true, point),
                    point
                );
            }
            assert_eq!(screen().restore(placement.saved, size()), point);
        }
    }

    #[test]
    fn saved_position_survives_restart_and_a_different_dpi() {
        let point = PhysicalPosition::new(200, 250);
        let saved = screen().remember(point, size());
        let serialized = serde_json::to_string(&saved).unwrap();
        let restored = serde_json::from_str(&serialized).unwrap();
        assert_eq!(screen().restore(Some(restored), size()), point);
        let retina = Screen {
            origin: PhysicalPosition::new(-2880, 50),
            size: PhysicalSize::new(2880, 1750),
            scale: 2.0,
        };
        assert_eq!(
            retina.restore(Some(restored), PhysicalSize::new(1104, 312)),
            PhysicalPosition::new(-2480, 500)
        );
    }

    #[test]
    fn smaller_screen_clamps_saved_position_inside_usable_area() {
        let small = Screen {
            origin: PhysicalPosition::new(0, 25),
            size: PhysicalSize::new(800, 575),
            scale: 1.0,
        };
        assert_eq!(
            small.restore(
                Some(SavedPosition {
                    x: 1200.0,
                    y: 900.0
                }),
                size()
            ),
            PhysicalPosition::new(248, 444)
        );
        let tiny = Screen {
            size: PhysicalSize::new(400, 100),
            ..screen()
        };
        assert_eq!(tiny.restore(None, size()), tiny.origin);
    }

    #[test]
    fn automatic_moves_never_replace_the_user_preference() {
        let saved = SavedPosition { x: 200.0, y: 225.0 };
        let mut placement = Placement {
            saved: Some(saved),
            ..Placement::default()
        };
        let first = PhysicalPosition::new(872, 41);
        let second = PhysicalPosition::new(872, 50);
        placement.expect_move(first);
        placement.expect_move(second);
        assert!(placement.moved(first, screen(), size(), true).is_none());
        assert!(placement.moved(second, screen(), size(), true).is_none());
        assert_eq!(placement.saved, Some(saved));
        assert!(placement
            .moved(PhysicalPosition::new(300, 400), screen(), size(), true)
            .is_some());
        assert_eq!(placement.saved, Some(SavedPosition { x: 300.0, y: 375.0 }));
    }

    #[test]
    fn invalid_preferences_fall_back_to_top_right() {
        for x in [-1.0, f64::NAN, f64::INFINITY, 100_001.0] {
            assert_eq!(
                screen().restore(Some(SavedPosition { x, y: 0.0 }), size()),
                screen().restore(None, size())
            );
        }
    }

    #[test]
    fn persistence_round_trip_does_not_modify_dictation_settings() {
        let root =
            std::env::temp_dir().join(format!("aidoo-overlay-test-{}", uuid::Uuid::new_v4()));
        storage::with_test_data_dir(root.clone(), || {
            storage::ensure_directories().unwrap();
            storage::save_settings(&crate::models::AppSettings::default()).unwrap();
            let before = std::fs::read(root.join("settings.json")).unwrap();
            let position = SavedPosition { x: 310.0, y: 270.0 };
            storage::save_overlay_position(&position).unwrap();
            assert_eq!(storage::load_overlay_position(), Some(position));
            assert_eq!(std::fs::read(root.join("settings.json")).unwrap(), before);
            std::fs::write(root.join("overlay-position.json"), br#"{"x":-1,"y":0}"#).unwrap();
            assert_eq!(storage::load_overlay_position(), None);
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quit_flushes_the_last_move_even_before_its_debounce_finishes() {
        let root =
            std::env::temp_dir().join(format!("aidoo-overlay-quit-test-{}", uuid::Uuid::new_v4()));
        storage::with_test_data_dir(root.clone(), || {
            storage::ensure_directories().unwrap();
            let state = OverlayWindowState(Mutex::new(Placement::default()));
            {
                let mut placement = state.0.lock().unwrap();
                placement.moved(PhysicalPosition::new(200, 250), screen(), size(), true);
                placement.moved(PhysicalPosition::new(310, 295), screen(), size(), true);
            }
            state.persist(Some(1));
            assert!(
                storage::load_overlay_position().is_none(),
                "obsolete debounce must not save"
            );
            state.persist(None);
            assert_eq!(
                storage::load_overlay_position(),
                Some(SavedPosition { x: 310.0, y: 270.0 })
            );
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
