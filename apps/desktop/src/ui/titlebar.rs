use gpui::{*, InteractiveElement};
use crate::ui::workspace::{Workspace, MainMenu};

pub struct Titlebar {
    pub active_menu: Option<MainMenu>,
}

impl Titlebar {
    pub fn new(active_menu: Option<MainMenu>) -> Self {
        Self { active_menu }
    }

    pub fn render(self, cx: &mut Context<Workspace>) -> impl IntoElement {
        let active_menu = self.active_menu;

        div()
            .h(px(48.))
            .w_full()
            .bg(rgb(0x161618))
            .border_b(px(1.))
            .border_color(rgb(0x242427))
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size_3()
                            .rounded_full()
                            .bg(rgb(0x8b5cf6)) // Violet dot logo
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("OpenCut 桌面版")
                    )
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .child(
                        self.render_header_item("檔案", MainMenu::File, active_menu, cx)
                    )
                    .child(
                        self.render_header_item("編輯", MainMenu::Edit, active_menu, cx)
                    )
                    .child(
                        self.render_header_item("檢視", MainMenu::View, active_menu, cx)
                    )
                    .child(
                        self.render_header_item("說明", MainMenu::Help, active_menu, cx)
                    )
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8e8e93))
                    .child("v0.1.0-alpha")
            )
    }

    fn render_header_item(
        &self,
        label: &'static str,
        menu: MainMenu,
        active_menu: Option<MainMenu>,
        cx: &mut Context<Workspace>,
    ) -> impl IntoElement {
        let is_active = active_menu == Some(menu);
        div()
            .id(label)
            .px_2()
            .py_1()
            .rounded(px(4.))
            .bg(if is_active { rgb(0x242427) } else { rgb(0x161618) })
            .text_xs()
            .text_color(if is_active { rgb(0xffffff) } else { rgb(0xc5c5c7) })
            .hover(|style| style.text_color(rgb(0xffffff)).bg(rgb(0x242427)))
            .cursor_pointer()
            .on_click(cx.listener(move |workspace, _, _window, cx| {
                if workspace.active_menu == Some(menu) {
                    workspace.active_menu = None;
                } else {
                    workspace.active_menu = Some(menu);
                }
                cx.notify();
            }))
            .child(label)
    }
}
