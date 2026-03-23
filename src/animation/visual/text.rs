use bevy::prelude::*;
use std::collections::HashMap;

use crate::scene::AmVisualSpawned;

use super::super::visual_helpers::extract_fill_color;

pub(super) fn handle_text_visual(
    commands: &mut Commands,
    entity: Entity,
    content: &str,
    font_name: &str,
    font_size: f32,
    align: &str,
    fill_color: &Option<crate::schema::AmFillColor>,
    wrap_width: f32,
    line_height_ratio: f32,
    fonts: &HashMap<String, Handle<Font>>,
) {
    use bevy::text::Justify;

    let color = extract_fill_color(fill_color, false);
    let justify = match align {
        "center" => Justify::Center,
        "right" => Justify::Right,
        _ => Justify::Left,
    };

    let font = fonts
        .get(font_name)
        .cloned()
        .unwrap_or_else(Handle::default);
    let anchor = bevy::sprite::Anchor::CENTER;
    let line_height = bevy::text::LineHeight::RelativeToFont(line_height_ratio);

    commands.entity(entity).insert((
        Text2d::new(content.to_string()),
        TextFont {
            font,
            font_size,
            ..default()
        },
        TextLayout::new_with_justify(justify),
        TextColor(color),
        bevy::text::TextBounds::new_horizontal(wrap_width),
        line_height,
        anchor,
        AmVisualSpawned,
    ));
}
