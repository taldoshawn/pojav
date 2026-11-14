//! Rust implementation of the button properties side panel; no Java runtime.
use crate::touch::{Editor, PickerStep, Placement, TouchControls};
use crate::ui_theme::{Face, Ui};

const ROW: f32 = 48.0;
const ROWS: usize = 20;
fn geometry(w: f32, h: f32, center: f32) -> [f32; 4] {
    let width = (w * 0.42).clamp(220.0, 340.0).min(w);
    [if center < 0.5 { w - width } else { 0.0 }, 0.0, width, h]
}
fn numeric_row(row: usize) -> Option<usize> {
    match row {
        1..=4 => Some(row - 1),
        9..=13 => Some(row - 5),
        _ => None,
    }
}
fn value(p: Placement, field: usize, viewport: [f32; 2]) -> String {
    let [w, h] = viewport;
    let r = p.rect(w / h.max(1.0));
    match field {
        0 => format!("{:.0}", r[2] * w),
        1 => format!("{:.0}", r[3] * h),
        2 => format!("{:.0}", r[0] * w),
        3 => format!("{:.0}", r[1] * h),
        4 => format!("{:.0}", p.opacity * 100.0),
        5 => format!("{:.0}", p.roundness),
        6 => format!("{:.1}", p.stroke_size),
        7 => format!("{:08X}", p.button_color),
        _ => format!("{:08X}", p.stroke_color),
    }
}
pub(super) fn draw(tc: &TouchControls, ed: &Editor, ui: &mut Ui) {
    let Some(index) = ed.selected_custom else {
        return;
    };
    let Some(button) = tc.settings.custom_buttons.get(index) else {
        return;
    };
    let p = tc.custom_placement(index);
    let [x, _, width, h] = geometry(ui.w, ui.h, p.center[0]);
    let theme = crate::ui_theme::active_theme();
    ui.fill(x, 0.0, width, h, theme.bg_solid);
    ui.text(
        Face::SemiBold,
        16.0,
        x + 14.0,
        14.0,
        theme.text_primary,
        "Editar botão",
    );
    ui.centered_fit(
        Face::SemiBold,
        12.0,
        [x + width - 80.0, 8.0, 72.0, 32.0],
        theme.accent,
        "Concluído",
    );
    let top = 52.0;
    ui.set_clip(Some([x, top, width, h - top]));
    let labels = [
        "Nome",
        "Largura",
        "Altura",
        "Posição X",
        "Posição Y",
        "Tecla 1",
        "Tecla 2",
        "Tecla 3",
        "Tecla 4",
        "Opacidade (%)",
        "Raio dos cantos (%)",
        "Espessura da borda",
        "Cor do botão (AARRGGBB)",
        "Cor da borda (AARRGGBB)",
        "Manter pressionado",
        "Passar toque para câmera",
        "Ativar ao deslizar",
        "Mostrar no jogo",
        "Mostrar nos menus",
        "Excluir botão",
    ];
    for (row, label) in labels.iter().enumerate() {
        let y = top + row as f32 * ROW - ed.side_scroll;
        if y + ROW < top || y > h {
            continue;
        }
        let text = if row == 0 {
            if ed.editing_button_label && ed.numeric_property.is_none() {
                format!("{}|", ed.button_label_input)
            } else {
                button.label()
            }
        } else if let Some(field) = numeric_row(row) {
            if ed.editing_button_label && ed.numeric_property == Some(field) {
                format!("{}|", ed.button_label_input)
            } else {
                value(p, field, [ui.w, ui.h])
            }
        } else if (5..=8).contains(&row) {
            let key = button.event_slot(row - 5);
            if key.is_empty() {
                "Não atribuído ▾".into()
            } else {
                format!("{} ▾", key.strip_prefix("key.keyboard.").unwrap_or(key))
            }
        } else if row < 19 {
            let on = [
                p.toggleable,
                p.passthrough,
                p.swipeable,
                p.visible_in_game,
                p.visible_in_menu,
            ][row - 14];
            if on { "☑".into() } else { "☐".into() }
        } else {
            "Excluir".into()
        };
        ui.text(
            Face::Regular,
            10.5,
            x + 14.0,
            y + 3.0,
            theme.text_muted,
            label,
        );
        ui.text_clipped(
            Face::Medium,
            13.0,
            x + 14.0,
            y + 20.0,
            width - 28.0,
            theme.text_primary,
            &text,
        );
        ui.fill(x + 14.0, y + ROW - 4.0, width - 28.0, 1.0, theme.border);
    }
    ui.set_clip(None);
    let view = (h - top).max(1.0);
    let total = ROWS as f32 * ROW;
    let thumb = (view * view / total).min(view);
    ui.fill(
        x + width - 4.0,
        top + ed.side_scroll / total * view,
        3.0,
        thumb,
        theme.accent,
    );
}

pub(super) fn touch(
    tc: &mut TouchControls,
    ed: &mut Editor,
    id: u64,
    point: [f32; 2],
    viewport: [f32; 2],
    started: bool,
    ended: bool,
) {
    let Some(index) = ed.selected_custom else {
        return;
    };
    let placement = tc.custom_placement(index);
    let [x, _, width, h] = geometry(viewport[0], viewport[1], placement.center[0]);
    if started {
        if point[0] < x || point[0] > x + width || point[1] < 52.0 {
            ed.props_open = false;
            ed.editing_button_label = false;
            ed.numeric_property = None;
            ed.side_drag = None;
            return;
        }
        ed.side_drag = Some((id, point[1], ed.side_scroll, false));
        return;
    }
    let Some((finger, origin, scroll, moved)) = ed.side_drag else {
        return;
    };
    if finger != id {
        return;
    }
    let moved = moved || (point[1] - origin).abs() > 6.0;
    ed.side_scroll =
        (scroll + origin - point[1]).clamp(0.0, (ROWS as f32 * ROW - (h - 52.0)).max(0.0));
    ed.side_drag = if ended {
        None
    } else {
        Some((id, origin, scroll, moved))
    };
    if !ended || moved {
        return;
    }
    let row = ((point[1] - 52.0 + ed.side_scroll) / ROW) as usize;
    if row >= ROWS {
        return;
    }
    super::push_undo_snapshot(ed, &tc.settings);
    if row == 0 || numeric_row(row).is_some() {
        ed.numeric_property = numeric_row(row);
        ed.property_viewport = viewport;
        ed.button_label_input = if let Some(field) = ed.numeric_property {
            value(placement, field, viewport)
        } else {
            tc.settings.custom_buttons[index].label()
        };
        ed.editing_button_label = true;
        ed.keyboard_request = true;
    } else if (5..=8).contains(&row) {
        ed.editing_button_label = false;
        ed.numeric_property = None;
        ed.picker_step = PickerStep::AskInputType {
            event_slot: row - 5,
        };
    } else if row == 19 {
        tc.settings.custom_buttons.remove(index);
        ed.selected_custom = None;
        ed.props_open = false;
    } else {
        for p in &mut tc.settings.custom_buttons[index].layouts {
            match row {
                14 => p.toggleable = !placement.toggleable,
                15 => p.passthrough = !placement.passthrough,
                16 => p.swipeable = !placement.swipeable,
                17 => p.visible_in_game = !placement.visible_in_game,
                18 => p.visible_in_menu = !placement.visible_in_menu,
                _ => {}
            }
        }
    }
    if let Err(error) = tc.settings.save() {
        ed.status_msg = Some(error.to_string());
    }
    tc.clicked = true;
}
