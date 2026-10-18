use crate::app::HANDLE_PX;
use crate::app::LabelerApp;
use crate::classes;
use crate::drag::Drag;
use crate::image_set::ImageSet;
use crate::menu::ContextMenu;
use crate::yolo::YoloBox;
use crate::{DeleteSelected, Deselect, NextImage, OpenFolder, PrevImage, ResetView, SaveLabels};

use gpui::{
    App, Bounds, ClickEvent, Context, CursorStyle, Div, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ObjectFit, ParentElement, Pixels, Render, Stateful,
    StatefulInteractiveElement, Styled, StyledImage, Window, canvas, div, img,
    prelude::FluentBuilder, px, relative, rgb, rgba,
};

impl Render for LabelerApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let probe_target = cx.entity().downgrade();
        let plane = self.image_plane();

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xdddddd))
            .text_sm()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NextImage, _window, cx| this.go_next(cx)))
            .on_action(cx.listener(|this, _: &PrevImage, _window, cx| this.go_prev(cx)))
            .on_action(cx.listener(|this, _: &SaveLabels, _window, cx| this.save_current(cx)))
            .on_action(
                cx.listener(|this, _: &DeleteSelected, _window, cx| this.delete_selected(cx)),
            )
            .on_action(cx.listener(|this, _: &Deselect, _window, cx| this.deselect(cx)))
            .on_action(cx.listener(|this, _: &OpenFolder, _window, cx| this.start_open_folder(cx)))
            .on_action(cx.listener(|this, _: &ResetView, _window, cx| this.reset_view(cx)))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .id("image-area")
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .bg(rgb(0x101010))
                    .on_scroll_wheel(cx.listener(Self::on_plane_scroll))
                    .child(
                        canvas(
                            move |bounds, _window, _cx| bounds.size,
                            move |bounds, _size, _window, cx| {
                                // Layout probe: record the image area's
                                // bounds so mouse handlers can map window
                                // pixels to normalized YOLO coordinates.
                                // Guard against equality or every paint
                                // schedules another frame forever.
                                let _ = probe_target.update(cx, |app, _cx| {
                                    if app.view_bounds != Some(bounds) {
                                        app.view_bounds = Some(bounds);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .children(plane.map(|plane| self.render_plane(plane, cx))),
            )
            .child(self.render_status_bar(cx))
            .children(
                self.context_menu
                    .as_ref()
                    .map(|menu| self.render_context_menu(menu, cx)),
            )
    }
}

impl LabelerApp {
    fn render_toolbar(&self, cx: &mut Context<Self>) -> Div {
        let position = match &self.set {
            Some(set) => format!("{}/{}", set.position() + 1, set.len()),
            None => "no folder".to_string(),
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(rgb(0x333333))
            .child(self.button(
                "open-folder",
                "Open Folder (Ctrl-O)",
                cx.listener(|this, _: &ClickEvent, _window, cx| this.start_open_folder(cx)),
            ))
            .child(self.button(
                "prev",
                "< Prev (Left)",
                cx.listener(|this, _: &ClickEvent, _window, cx| this.go_prev(cx)),
            ))
            .child(self.button(
                "next",
                "Next (Right) >",
                cx.listener(|this, _: &ClickEvent, _window, cx| this.go_next(cx)),
            ))
            .child(self.button(
                "save",
                "Save (Ctrl-S)",
                cx.listener(|this, _: &ClickEvent, _window, cx| this.save_current(cx)),
            ))
            .child(self.button(
                "reset-view",
                "Reset View (Ctrl-0)",
                cx.listener(|this, _: &ClickEvent, _window, cx| this.reset_view(cx)),
            ))
            .child(div().flex_1())
            .child(div().text_color(rgb(0x999999)).child(position))
    }

    fn button(
        &self,
        id: &'static str,
        label: &str,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .cursor_pointer()
            .px_2()
            .py_1()
            .rounded_sm()
            .bg(rgb(0x333333))
            .hover(|style| style.bg(rgb(0x454545)))
            .on_click(on_click)
            .child(label.to_string())
    }

    fn render_plane(&self, plane: Bounds<Pixels>, cx: &mut Context<Self>) -> Div {
        let mut plane_el = div()
            .absolute()
            .left(plane.origin.x)
            .top(plane.origin.y)
            .w(plane.size.width)
            .h(plane.size.height)
            .bg(rgb(0x000000))
            .cursor(CursorStyle::Crosshair)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_plane_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_plane_mouse_down))
            .on_mouse_move(cx.listener(Self::on_plane_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_plane_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_plane_mouse_up));
        if let Some(path) = self.set.as_ref().and_then(ImageSet::current) {
            plane_el = plane_el.child(
                img(path.to_path_buf())
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            );
        }
        for (index, label) in self.labels.iter().enumerate() {
            let class_name = self
                .classes
                .get(label.class as usize)
                .cloned()
                .unwrap_or_else(|| format!("class {}", label.class).into());
            plane_el = plane_el.child(render_box(
                label,
                &class_name,
                self.selected == Some(index),
                false,
            ));
        }
        if let Some(Drag::Creating { origin, corner }) = &self.drag {
            let draft =
                YoloBox::from_corners(self.active_class, origin.x, origin.y, corner.x, corner.y);
            let class_name = self
                .classes
                .get(draft.class as usize)
                .cloned()
                .unwrap_or_else(|| format!("class {}", draft.class).into());
            plane_el = plane_el.child(render_box(&draft, &class_name, false, true));
        }
        // Corner handles on the selected box: grab zones for resizing.
        if let Some(selected) = self.selected
            && let Some(box_) = self.labels.get(selected)
        {
            let plane_w = f32::from(plane.size.width);
            let plane_h = f32::from(plane.size.height);
            let half = HANDLE_PX / 2.0;
            let color = classes::class_color(box_.class);
            for (x, y) in [
                (box_.x0(), box_.y0()),
                (box_.x1(), box_.y0()),
                (box_.x0(), box_.y1()),
                (box_.x1(), box_.y1()),
            ] {
                plane_el = plane_el.child(
                    div()
                        .absolute()
                        .left(px(x * plane_w - half))
                        .top(px(y * plane_h - half))
                        .w(px(HANDLE_PX))
                        .h(px(HANDLE_PX))
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(color),
                );
            }
        }
        plane_el
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> Div {
        let mut line = self.info_line();
        if self.dirty {
            line.push_str("  ·  unsaved");
        }
        if !self.status.is_empty() {
            line.push_str("  ·  ");
            line.push_str(&self.status);
        }
        let mut chips = div().flex().flex_wrap().items_center().gap_1();
        for (index, name) in self.classes.iter().enumerate() {
            let Ok(class) = u32::try_from(index) else {
                continue;
            };
            // The chip being renamed renders as an inline editor; its
            // keystrokes are routed through the root's on_key_down.
            if self.renaming == Some(index) {
                chips = chips.child(
                    div()
                        .px_1()
                        .rounded_sm()
                        .text_xs()
                        .whitespace_nowrap()
                        .bg(rgb(0x252525))
                        .border_1()
                        .border_color(rgb(0xffffff))
                        .text_color(rgb(0xdddddd))
                        .child(format!("{}▏", self.class_draft)),
                );
                continue;
            }
            let color = classes::class_color(class);
            chips = chips.child(
                div()
                    .px_1()
                    .rounded_sm()
                    .text_xs()
                    .whitespace_nowrap()
                    .bg(color)
                    .text_color(rgb(0x000000))
                    .when(class == self.active_class, |chip| {
                        chip.border_1().border_color(rgb(0xffffff))
                    })
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                            this.open_context_menu(index, event.position, cx);
                        }),
                    )
                    .child(format!("{}) {name}", index + 1)),
            );
        }
        chips = if self.adding_class {
            chips.child(
                div()
                    .px_1()
                    .rounded_sm()
                    .text_xs()
                    .whitespace_nowrap()
                    .bg(rgb(0x252525))
                    .border_1()
                    .border_color(rgb(0x888888))
                    .text_color(rgb(0xdddddd))
                    .child(if self.class_draft.is_empty() {
                        "class name… (Enter adds, Esc cancels)".to_string()
                    } else {
                        format!("{}▏", self.class_draft)
                    }),
            )
        } else {
            chips.child(
                div()
                    .id("add-class")
                    .cursor_pointer()
                    .px_1()
                    .rounded_sm()
                    .text_xs()
                    .whitespace_nowrap()
                    .bg(rgb(0x333333))
                    .hover(|style| style.bg(rgb(0x454545)))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _window, cx| this.start_add_class(cx)),
                    )
                    .child("+ class"),
            )
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(rgb(0x333333))
            .child(div().text_color(rgb(0x999999)).child(line))
            .child(chips)
    }

    fn render_context_menu(&self, menu: &ContextMenu, cx: &mut Context<Self>) -> Div {
        let class = menu.class;
        let name = self.classes.get(class).cloned().unwrap_or_default();
        div()
            .absolute()
            .left(px(menu.position.x))
            // Open above the cursor: the chip row sits at the window's
            // bottom edge, so a downward menu would leave the window.
            .top(px(menu.position.y - 36.0))
            .min_w(px(140.0))
            .py_1()
            .bg(rgb(0x2a2a2a))
            .border_1()
            .border_color(rgb(0x555555))
            .rounded_sm()
            .shadow_lg()
            .on_mouse_down_out(cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                this.context_menu = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("menu-rename")
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .whitespace_nowrap()
                    .hover(|style| style.bg(rgb(0x3d3d3d)))
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _window, cx| this.start_rename(cx)),
                    )
                    .child(format!("Rename \"{name}\"")),
            )
    }
}

fn render_box(label: &YoloBox, class_name: &str, selected: bool, draft: bool) -> Div {
    let color = classes::class_color(label.class);
    let mut el = div()
        .absolute()
        .left(relative(label.cx - label.w / 2.0))
        .top(relative(label.cy - label.h / 2.0))
        .w(relative(label.w))
        .h(relative(label.h))
        .border_2()
        .border_color(color);
    if selected {
        el = el.border_4().border_color(rgb(0xffffff));
    }
    if draft {
        el = el.bg(rgba(0xffffff20));
    }
    // Anchored so the chip's bottom-left sits on the box's top-left:
    // bottom = 100% of the box height puts the chip's bottom edge on
    // the box's top edge, left_0 aligns the left edges.
    el.child(
        div()
            .absolute()
            .left_0()
            .bottom(relative(1.0))
            .px_1()
            .text_xs()
            .whitespace_nowrap()
            .bg(color)
            .text_color(rgb(0x000000))
            .child(class_name.to_string()),
    )
}
