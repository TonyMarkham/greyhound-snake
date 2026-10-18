use std::path::{Path, PathBuf};

use crate::classes;
use crate::data_yaml;
use crate::drag::Drag;
use crate::image_set::ImageSet;
use crate::menu::ContextMenu;
use crate::yolo::{self, YoloBox};

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, SharedString, Size,
    Window, point, px, size,
};
use image::ImageReader;
use rfd::FileDialog;
use smol::channel;

/// Boxes smaller than this fraction of the image side are treated as
/// stray clicks and discarded on mouse-up; resizes also stop here.
const MIN_BOX_SIDE: f32 = 0.005;
/// Wheel zoom limits.
const MAX_ZOOM: f32 = 8.0;
/// Half-width of the grab zone around a selected box's edges, in
/// screen pixels — also the size of the drawn corner handles.
pub(crate) const HANDLE_PX: f32 = 8.0;
/// One wheel tick zooms by this factor per line scrolled.
const ZOOM_PER_LINE: f32 = 0.15;

pub(crate) struct LabelerApp {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) folder: Option<PathBuf>,
    pub(crate) set: Option<ImageSet>,
    pub(crate) classes: Vec<SharedString>,
    /// Path of the governing `data.yaml` (target folder or one up)
    /// when one exists; class renames write back to it.
    pub(crate) names_yaml: Option<PathBuf>,
    /// True while a `data.yaml` exists near the target — class renames
    /// then never write `classes.txt`.
    pub(crate) yaml_present: bool,
    pub(crate) labels: Vec<YoloBox>,
    pub(crate) dirty: bool,
    pub(crate) active_class: u32,
    pub(crate) selected: Option<usize>,
    pub(crate) drag: Option<Drag>,
    /// Bounds of the image area, captured each frame by the layout probe
    /// in `render`; the bridge between window-pixel mouse events and
    /// normalized YOLO coordinates.
    pub(crate) view_bounds: Option<Bounds<Pixels>>,
    /// Natural pixel size of the current image, probed from the header;
    /// drives the letterboxed "plane" the boxes are positioned on.
    pub(crate) image_size: Option<(u32, u32)>,
    /// The current image's label file text as last loaded or saved —
    /// the never-wipe gate for saves.
    pub(crate) label_existing: String,
    /// Zoom factor applied to the fitted image (1.0 = fit) and the raw
    /// pan offset in view-local pixels; clamped when the plane is built.
    pub(crate) zoom: f32,
    pub(crate) pan: Point<f32>,
    /// In-progress "+ class" text input state; keystrokes are routed
    /// through the root's `on_key_down` while active. Mutually exclusive
    /// with `renaming` — both use `class_draft`.
    pub(crate) adding_class: bool,
    /// Class index being renamed via its chip's context menu.
    pub(crate) renaming: Option<usize>,
    pub(crate) class_draft: String,
    /// Open right-click menu on a class chip, if any.
    pub(crate) context_menu: Option<ContextMenu>,
    /// Hint/error notes shown in the status line; the file/position/box
    /// count part is computed live at render time.
    pub(crate) status: SharedString,
}

impl Focusable for LabelerApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl LabelerApp {
    pub(crate) fn new(initial_folder: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            focus_handle: cx.focus_handle(),
            folder: None,
            set: None,
            classes: vec![SharedString::from(classes::DEFAULT_CLASS)],
            names_yaml: None,
            yaml_present: false,
            labels: Vec::new(),
            dirty: false,
            active_class: 0,
            selected: None,
            drag: None,
            view_bounds: None,
            image_size: None,
            label_existing: String::new(),
            zoom: 1.0,
            pan: point(0.0, 0.0),
            adding_class: false,
            renaming: None,
            class_draft: String::new(),
            context_menu: None,
            status: String::new().into(),
        };
        match initial_folder {
            Some(dir) => app.open_folder(&dir, cx),
            None => {
                app.status = "Ctrl-O: choose a folder of images to label.".into();
                app.start_open_folder(cx);
            }
        }
        app
    }

    // -- folder & files ------------------------------------------------------------------------ //

    pub(crate) fn open_folder(&mut self, dir: &Path, cx: &mut Context<Self>) {
        match ImageSet::scan(dir) {
            Ok(set) => {
                // The tree's data.yaml one directory up owns the class
                // names: while it exists, classes.txt is never written.
                // Names come from the yaml, or stay session placeholders
                // until it carries names.
                let yaml_path = data_yaml::find_yaml(dir);
                let yaml_present = yaml_path.is_some();
                let (mut names, names_yaml) = match yaml_path
                    .as_ref()
                    .and_then(|path| data_yaml::load_names(path).map(|names| (path.clone(), names)))
                {
                    Some((path, names)) => (names, Some(path)),
                    None if yaml_present => (Vec::new(), None),
                    None => match classes::load_or_seed(dir) {
                        Ok(names) => (names, None),
                        Err(err) => {
                            self.status = format!("could not read classes.txt: {err}").into();
                            (vec![SharedString::from(classes::DEFAULT_CLASS)], None)
                        }
                    },
                };
                let extended = classes::extend_to_cover(&mut names, &set);
                if extended
                    && !yaml_present
                    && let Err(err) = classes::save(dir, &names)
                {
                    self.status = format!("could not update classes.txt: {err}").into();
                }
                self.folder = Some(dir.to_path_buf());
                self.classes = names;
                self.names_yaml = names_yaml;
                self.yaml_present = yaml_present;
                self.set = Some(set);
                self.load_current(cx);
            }
            Err(err) => {
                self.status = format!("could not read folder {}: {err}", dir.display()).into();
                cx.notify();
            }
        }
    }

    /// Opens the native folder picker on its own thread — the native
    /// dialog needs its own message loop, and blocking gpui's UI thread
    /// would freeze the window. The pick comes back through a channel
    /// awaited on gpui's executor.
    pub(crate) fn start_open_folder(&mut self, cx: &mut Context<Self>) {
        let (tx, rx) = channel::bounded::<Option<PathBuf>>(1);
        std::thread::spawn(move || {
            let picked = FileDialog::new().pick_folder();
            let _ = tx.send_blocking(picked);
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Some(dir)) = rx.recv().await {
                let _ = this.update(cx, |app, cx| app.open_folder(&dir, cx));
            }
        })
        .detach();
    }

    fn load_current(&mut self, cx: &mut Context<Self>) {
        self.selected = None;
        self.drag = None;
        self.zoom = 1.0;
        self.pan = point(0.0, 0.0);
        let Some((path, label_path)) = self.set.as_ref().and_then(|set| {
            set.current()
                .map(|path| (path.to_path_buf(), set.label_path_for(path)))
        }) else {
            self.labels.clear();
            self.image_size = None;
            self.dirty = false;
            self.label_existing.clear();
            self.status = "No images in this folder.".into();
            cx.notify();
            return;
        };
        let existing = std::fs::read_to_string(&label_path).unwrap_or_default();
        self.labels = yolo::parse_labels(&existing);
        self.label_existing = existing;
        self.dirty = false;
        self.image_size = probe_size(&path);
        self.status = if self.image_size.is_some() {
            String::new().into()
        } else {
            "could not read image dimensions".into()
        };
        cx.notify();
    }

    /// Writes the current labels only when they changed, so plain
    /// navigation never touches a folder it is only viewing.
    pub(crate) fn save_current(&mut self, cx: &mut Context<Self>) {
        if !self.dirty {
            return;
        }
        let Some(label_path) = self
            .set
            .as_ref()
            .and_then(|set| set.current().map(|path| set.label_path_for(path)))
        else {
            return;
        };
        let text = yolo::format_labels(&self.labels);
        if yolo::wipes_labels(&self.label_existing, &text) {
            self.status = "never-wipe: refusing to empty a label file that has lines".into();
            cx.notify();
            return;
        }
        match std::fs::write(&label_path, &text) {
            Ok(()) => {
                self.dirty = false;
                self.label_existing = text;
            }
            Err(err) => {
                self.status = format!("could not save {}: {err}", label_path.display()).into()
            }
        }
        cx.notify();
    }

    // -- actions -------------------------------------------------------------------------------- //

    pub(crate) fn go_next(&mut self, cx: &mut Context<Self>) {
        self.save_current(cx);
        if let Some(set) = &mut self.set {
            set.next();
        }
        self.load_current(cx);
    }

    pub(crate) fn go_prev(&mut self, cx: &mut Context<Self>) {
        self.save_current(cx);
        if let Some(set) = &mut self.set {
            set.prev();
        }
        self.load_current(cx);
    }

    pub(crate) fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.selected.take() {
            self.labels.remove(index);
            self.dirty = true;
            cx.notify();
        }
    }

    pub(crate) fn deselect(&mut self, cx: &mut Context<Self>) {
        if self.selected.take().is_some() || self.drag.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn reset_view(&mut self, cx: &mut Context<Self>) {
        if self.zoom != 1.0 || self.pan.x != 0.0 || self.pan.y != 0.0 {
            self.zoom = 1.0;
            self.pan = point(0.0, 0.0);
            cx.notify();
        }
    }

    /// Keys `1..=9` pick the active class (re-classing the selected box
    /// too); `Tab` cycles through the whole class list.
    pub(crate) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.is_some() && event.keystroke.key == "escape" {
            self.context_menu = None;
            cx.notify();
            return;
        }
        if self.adding_class {
            self.class_input_key(event, cx);
            return;
        }
        if self.renaming.is_some() {
            self.rename_input_key(event, cx);
            return;
        }
        match event.keystroke.key.as_str() {
            "tab" => {
                if !self.classes.is_empty() {
                    let len = u32::try_from(self.classes.len()).unwrap_or(u32::MAX);
                    self.active_class = (self.active_class + 1) % len;
                    cx.notify();
                }
            }
            key => {
                let Ok(digit) = key.parse::<u32>() else {
                    return;
                };
                if digit != 0 && self.set_active_class(digit - 1) {
                    cx.notify();
                }
            }
        }
    }

    // -- class editing -------------------------------------------------------------------------- //

    pub(crate) fn start_add_class(&mut self, cx: &mut Context<Self>) {
        self.adding_class = true;
        self.renaming = None;
        self.class_draft.clear();
        cx.notify();
    }

    // -- class renaming ------------------------------------------------------------------------ //

    pub(crate) fn open_context_menu(
        &mut self,
        class: usize,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if class < self.classes.len() {
            self.context_menu = Some(ContextMenu {
                class,
                position: point(f32::from(position.x), f32::from(position.y)),
            });
            cx.notify();
        }
    }

    pub(crate) fn start_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.context_menu.take()
            && let Some(name) = self.classes.get(menu.class)
        {
            self.adding_class = false;
            self.renaming = Some(menu.class);
            self.class_draft = name.to_string();
            cx.notify();
        }
    }

    /// The rename input is a root-level key sink, prefilled with the
    /// current name: characters accumulate, `Enter` commits, `Esc`
    /// cancels.
    fn rename_input_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => self.commit_rename(cx),
            "escape" => {
                self.renaming = None;
                self.class_draft.clear();
            }
            "backspace" => {
                self.class_draft.pop();
            }
            "space" => self.class_draft.push(' '),
            _ => {
                if let Some(text) = event.keystroke.key_char.as_ref() {
                    self.class_draft.push_str(text);
                }
            }
        }
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(index) = self.renaming.take() {
            let name = self.class_draft.trim().to_string();
            self.class_draft.clear();
            let changed = self
                .classes
                .get(index)
                .is_some_and(|old| !name.is_empty() && *old != name);
            if changed {
                self.classes[index] = SharedString::from(name);
                if let Some(dir) = &self.folder {
                    let save_err = match &self.names_yaml {
                        // Names came from the tree's data.yaml: write
                        // them back there.
                        Some(yaml_path) => data_yaml::update_names(yaml_path, &self.classes).err(),
                        // No yaml near the target: classes.txt is the
                        // class-name source.
                        None if !self.yaml_present => classes::save(dir, &self.classes).err(),
                        // A yaml exists but carries no usable names:
                        // renames stay session-only rather than write a
                        // forbidden classes.txt.
                        None => None,
                    };
                    if let Some(err) = save_err {
                        self.status = format!("could not save class names: {err}").into();
                    }
                }
            }
            cx.notify();
        }
    }

    /// The "+ class" input is a root-level key sink: printable characters
    /// accumulate in the draft, `Enter` commits, `Esc` cancels.
    fn class_input_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" => self.commit_class(cx),
            "escape" => {
                self.adding_class = false;
                self.class_draft.clear();
            }
            "backspace" => {
                self.class_draft.pop();
            }
            "space" => self.class_draft.push(' '),
            _ => {
                if let Some(text) = event.keystroke.key_char.as_ref() {
                    self.class_draft.push_str(text);
                }
            }
        }
        cx.notify();
    }

    fn commit_class(&mut self, cx: &mut Context<Self>) {
        self.adding_class = false;
        let name = self.class_draft.trim().to_string();
        self.class_draft.clear();
        if !name.is_empty() {
            self.classes.push(SharedString::from(name));
            self.active_class = u32::try_from(self.classes.len() - 1).unwrap_or(0);
            // classes.txt is only the class-name source when no
            // data.yaml sits one directory up; with one present,
            // classes.txt is never written and adds stay session-only.
            if !self.yaml_present
                && let Some(dir) = &self.folder
                && let Err(err) = classes::save(dir, &self.classes)
            {
                self.status = format!("could not save classes.txt: {err}").into();
            }
        }
        cx.notify();
    }

    fn set_active_class(&mut self, index: u32) -> bool {
        if !usize::try_from(index).is_ok_and(|index| index < self.classes.len()) {
            return false;
        }
        self.active_class = index;
        if let Some(i) = self.selected
            && let Some(label) = self.labels.get_mut(i)
        {
            label.class = index;
            self.dirty = true;
        }
        true
    }

    // -- geometry ------------------------------------------------------------------------------- //

    /// Zoom-independent fit: the letterboxed rect the image occupies at
    /// 1× zoom, in the image area's own coordinates.
    fn fit_geometry(&self) -> Option<(Size<f32>, Point<f32>)> {
        let view = self.view_bounds?;
        let (width, height) = self.image_size?;
        if width == 0 || height == 0 {
            return None;
        }
        let scale = (f32::from(view.size.width) / width as f32)
            .min(f32::from(view.size.height) / height as f32);
        let fit_size = size(width as f32 * scale, height as f32 * scale);
        let fit_origin = point(
            (f32::from(view.size.width) - fit_size.width) / 2.0,
            (f32::from(view.size.height) - fit_size.height) / 2.0,
        );
        Some((fit_size, fit_origin))
    }

    /// The rectangle the image currently occupies in the image area's
    /// own coordinates — fit rect scaled by `zoom` and shifted by the
    /// clamped `pan` so it always covers the fitted viewport. Boxes are
    /// positioned on it and mouse positions are mapped back through it,
    /// so overlay and editing stay consistent by construction.
    pub(crate) fn image_plane(&self) -> Option<Bounds<Pixels>> {
        let (fit_size, fit_origin) = self.fit_geometry()?;
        let w = px(fit_size.width * self.zoom);
        let h = px(fit_size.height * self.zoom);
        let pan_x = self.pan.x.clamp(fit_size.width - f32::from(w), 0.0);
        let pan_y = self.pan.y.clamp(fit_size.height - f32::from(h), 0.0);
        Some(Bounds::new(
            point(px(fit_origin.x + pan_x), px(fit_origin.y + pan_y)),
            size(w, h),
        ))
    }

    /// Window-pixel position → normalized image coordinates. With
    /// `strict`, positions outside the plane return `None` (hit testing);
    /// otherwise they clamp to the border (dragging).
    fn normalize(&self, position: Point<Pixels>, strict: bool) -> Option<Point<f32>> {
        let view = self.view_bounds?;
        let plane = self.image_plane()?;
        let x = f32::from(position.x) - (f32::from(view.origin.x) + f32::from(plane.origin.x));
        let y = f32::from(position.y) - (f32::from(view.origin.y) + f32::from(plane.origin.y));
        if strict
            && (x < 0.0
                || y < 0.0
                || x > f32::from(plane.size.width)
                || y > f32::from(plane.size.height))
        {
            return None;
        }
        Some(point(
            (x / f32::from(plane.size.width)).clamp(0.0, 1.0),
            (y / f32::from(plane.size.height)).clamp(0.0, 1.0),
        ))
    }

    fn cursor_local(&self, position: Point<Pixels>) -> Option<Point<f32>> {
        let view = self.view_bounds?;
        Some(point(
            f32::from(position.x) - f32::from(view.origin.x),
            f32::from(position.y) - f32::from(view.origin.y),
        ))
    }

    /// Screen-pixel grab zone around the selected box's edges/corners,
    /// as the fixed anchor(s) a resize drags against. Corners fix both
    /// axes, edges one, the interior `None`/`None` (that's a move, not a
    /// resize).
    fn resize_anchor(
        &self,
        position: Point<f32>,
        box_: &YoloBox,
    ) -> Option<(Option<f32>, Option<f32>)> {
        let plane = self.image_plane()?;
        let hw = HANDLE_PX / f32::from(plane.size.width);
        let hh = HANDLE_PX / f32::from(plane.size.height);
        let (x0, y0, x1, y1) = (box_.x0(), box_.y0(), box_.x1(), box_.y1());
        if position.x < x0 - hw
            || position.x > x1 + hw
            || position.y < y0 - hh
            || position.y > y1 + hh
        {
            return None;
        }
        let fx = if (position.x - x0).abs() <= hw {
            Some(x0)
        } else if (position.x - x1).abs() <= hw {
            Some(x1)
        } else {
            None
        };
        let fy = if (position.y - y0).abs() <= hh {
            Some(y0)
        } else if (position.y - y1).abs() <= hh {
            Some(y1)
        } else {
            None
        };
        match (fx, fy) {
            (Some(fx), Some(fy)) => Some((Some(fx), Some(fy))),
            (Some(fx), None) if position.y >= y0 && position.y <= y1 => Some((Some(fx), None)),
            (None, Some(fy)) if position.x >= x0 && position.x <= x1 => Some((None, Some(fy))),
            _ => None,
        }
    }

    // -- mouse handling on the image plane ------------------------------------------------------ //

    pub(crate) fn on_plane_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.button {
            MouseButton::Middle => {
                if let Some(cursor) = self.cursor_local(event.position) {
                    self.drag = Some(Drag::Panning {
                        start_cursor: cursor,
                        start_pan: self.pan,
                    });
                    cx.notify();
                }
            }
            MouseButton::Left => self.left_press(event, cx),
            _ => {}
        }
    }

    fn left_press(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let Some(position) = self.normalize(event.position, true) else {
            return;
        };
        // Resizing wins over everything on the selected box's border.
        if let Some(index) = self.selected
            && let Some(box_) = self.labels.get(index)
            && let Some((fixed_x, fixed_y)) = self.resize_anchor(position, box_)
        {
            self.drag = Some(Drag::Resizing {
                index,
                fixed_x,
                fixed_y,
            });
            cx.notify();
            return;
        }
        // Topmost box wins: later boxes paint above earlier ones.
        if let Some(index) = self
            .labels
            .iter()
            .rposition(|box_| box_.contains(position.x, position.y))
        {
            self.selected = Some(index);
            let box_ = &self.labels[index];
            let offset = point(position.x - box_.cx, position.y - box_.cy);
            self.drag = Some(Drag::Moving { index, offset });
        } else {
            self.selected = None;
            self.drag = Some(Drag::Creating {
                origin: position,
                corner: position,
            });
        }
        cx.notify();
    }

    pub(crate) fn on_plane_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.drag {
            Some(Drag::Panning {
                start_cursor,
                start_pan,
            }) => {
                if event.pressed_button != Some(MouseButton::Middle) {
                    return;
                }
                if let Some(cursor) = self.cursor_local(event.position) {
                    self.pan = point(
                        start_pan.x + cursor.x - start_cursor.x,
                        start_pan.y + cursor.y - start_cursor.y,
                    );
                    cx.notify();
                }
            }
            Some(Drag::Creating { .. }) => {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if let Some(position) = self.normalize(event.position, false) {
                    if let Some(Drag::Creating { corner, .. }) = &mut self.drag {
                        *corner = position;
                    }
                    cx.notify();
                }
            }
            Some(Drag::Moving { index, offset }) => {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if let Some(position) = self.normalize(event.position, false) {
                    let Some(box_) = self.labels.get_mut(index) else {
                        self.drag = None;
                        return;
                    };
                    // Keep the box fully inside the image.
                    box_.cx = (position.x - offset.x).clamp(box_.w / 2.0, 1.0 - box_.w / 2.0);
                    box_.cy = (position.y - offset.y).clamp(box_.h / 2.0, 1.0 - box_.h / 2.0);
                    self.dirty = true;
                    cx.notify();
                }
            }
            Some(Drag::Resizing {
                index,
                fixed_x,
                fixed_y,
            }) => {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if let Some(position) = self.normalize(event.position, false) {
                    self.resize_to(index, fixed_x, fixed_y, position);
                    self.dirty = true;
                    cx.notify();
                }
            }
            None => {}
        }
    }

    fn resize_to(
        &mut self,
        index: usize,
        fixed_x: Option<f32>,
        fixed_y: Option<f32>,
        position: Point<f32>,
    ) {
        let Some(box_) = self.labels.get(index) else {
            self.drag = None;
            return;
        };
        let (x0, y0, x1, y1) = (box_.x0(), box_.y0(), box_.x1(), box_.y1());
        let class = box_.class;
        // Pull the cursor to the min-size side of the fixed anchor so a
        // resize can never invert or collapse the box.
        let mut cursor_x = position.x;
        let mut cursor_y = position.y;
        if let Some(fx) = fixed_x {
            cursor_x = if fx <= x0 + 1e-9 {
                cursor_x.max(fx + MIN_BOX_SIDE)
            } else {
                cursor_x.min(fx - MIN_BOX_SIDE)
            };
        }
        if let Some(fy) = fixed_y {
            cursor_y = if fy <= y0 + 1e-9 {
                cursor_y.max(fy + MIN_BOX_SIDE)
            } else {
                cursor_y.min(fy - MIN_BOX_SIDE)
            };
        }
        let xa = match fixed_x {
            Some(fx) => fx.min(cursor_x),
            None => x0,
        };
        let xb = match fixed_x {
            Some(fx) => fx.max(cursor_x),
            None => x1,
        };
        let ya = match fixed_y {
            Some(fy) => fy.min(cursor_y),
            None => y0,
        };
        let yb = match fixed_y {
            Some(fy) => fy.max(cursor_y),
            None => y1,
        };
        if let Some(box_) = self.labels.get_mut(index) {
            *box_ = YoloBox::from_corners(class, xa, ya, xb, yb);
        }
    }

    pub(crate) fn on_plane_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(Drag::Creating { origin, corner }) = self.drag.take()
            && (corner.x - origin.x).abs() > MIN_BOX_SIDE
            && (corner.y - origin.y).abs() > MIN_BOX_SIDE
        {
            self.labels.push(YoloBox::from_corners(
                self.active_class,
                origin.x,
                origin.y,
                corner.x,
                corner.y,
            ));
            self.selected = Some(self.labels.len() - 1);
            self.dirty = true;
        }
        cx.notify();
    }

    pub(crate) fn on_plane_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(plane) = self.image_plane() else {
            return;
        };
        let Some((_, fit_origin)) = self.fit_geometry() else {
            return;
        };
        let dy = match event.delta {
            ScrollDelta::Pixels(delta) => f32::from(delta.y) / 40.0,
            ScrollDelta::Lines(lines) => lines.y,
        };
        if dy == 0.0 {
            return;
        }
        let new_zoom = (self.zoom * (1.0 + ZOOM_PER_LINE * dy)).clamp(1.0, MAX_ZOOM);
        if (new_zoom - self.zoom).abs() < 1e-3 {
            return;
        }
        // The image point under the cursor stays under the cursor: take
        // that fraction of the old plane, then place the new plane so it
        // maps back to the same screen position. Raw pan is clamped when
        // the plane is built.
        let Some(cursor) = self.cursor_local(event.position) else {
            return;
        };
        let old_origin = point(f32::from(plane.origin.x), f32::from(plane.origin.y));
        let u = point(
            ((cursor.x - old_origin.x) / f32::from(plane.size.width)).clamp(0.0, 1.0),
            ((cursor.y - old_origin.y) / f32::from(plane.size.height)).clamp(0.0, 1.0),
        );
        let ratio = new_zoom / self.zoom;
        let new_width = f32::from(plane.size.width) * ratio;
        let new_height = f32::from(plane.size.height) * ratio;
        self.zoom = new_zoom;
        self.pan = point(
            cursor.x - u.x * new_width - fit_origin.x,
            cursor.y - u.y * new_height - fit_origin.y,
        );
        cx.notify();
    }

    /// The live first status line: current file, position, box count.
    /// Computed at render time so edits never show a stale count.
    pub(crate) fn info_line(&self) -> String {
        let Some((path, position, count)) = self
            .set
            .as_ref()
            .and_then(|set| set.current().map(|path| (path, set.position(), set.len())))
        else {
            return "no folder".to_string();
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?");
        format!(
            "{name}  ·  {}/{}  ·  {} boxes",
            position + 1,
            count,
            self.labels.len()
        )
    }
}

/// Header-only dimension probe; no pixel decoding. `None` disables box
/// alignment for that image (shown in the status line).
fn probe_size(path: &Path) -> Option<(u32, u32)> {
    ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
}
