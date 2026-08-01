use crate::app::{App, FieldMapping};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

// ─── Catppuccin Macchiato Palette ────────────────────────────────────────────
const BG_MANTLE: Color = Color::Rgb(30,  32,  48);   // #1E2030
const SURFACE0:  Color = Color::Rgb(54,  58,  79);   // #363A4F
const SURFACE1:  Color = Color::Rgb(73,  77, 100);   // #494D64
const OVERLAY0:  Color = Color::Rgb(110, 115, 141);  // #6E738D
const OVERLAY1:  Color = Color::Rgb(128, 135, 162);  // #8087A2
const SUBTEXT0:  Color = Color::Rgb(165, 173, 203);  // #A5ADCB
const TEXT:      Color = Color::Rgb(202, 211, 245);  // #CAD3F5
const LAVENDER:  Color = Color::Rgb(183, 189, 248);  // #B7BDF8
const BLUE:      Color = Color::Rgb(138, 173, 244);  // #8AADF4
const SAPPHIRE:  Color = Color::Rgb(125, 196, 228);  // #7DC4E4
const TEAL:      Color = Color::Rgb(139, 213, 202);  // #8BD5CA
const GREEN:     Color = Color::Rgb(166, 218, 149);  // #A6DA95
const YELLOW:    Color = Color::Rgb(238, 212, 159);  // #EED49F
const PEACH:     Color = Color::Rgb(245, 169, 127);  // #F5A97F
const RED:       Color = Color::Rgb(237, 135, 150);  // #ED8796
const MAUVE:     Color = Color::Rgb(198, 160, 246);  // #C6A0F6

// ─── Animation Utils ─────────────────────────────────────────────────────────

fn lerp_color(c1: Color, c2: Color, t: f32) -> Color {
    let (r1, g1, b1) = match c1 { Color::Rgb(r, g, b) => (r, g, b), _ => (255, 255, 255) };
    let (r2, g2, b2) = match c2 { Color::Rgb(r, g, b) => (r, g, b), _ => (255, 255, 255) };
    Color::Rgb(
        (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
        (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
        (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
    )
}

fn multi_lerp(colors: &[Color], mut t: f32) -> Color {
    if colors.is_empty() { return Color::White; }
    if colors.len() == 1 { return colors[0]; }
    t = t.clamp(0.0, 1.0);
    let segments = colors.len() - 1;
    let scaled = t * segments as f32;
    let index = scaled.floor() as usize;
    let c1 = colors[index];
    let c2 = if index + 1 < colors.len() { colors[index + 1] } else { colors[index] };
    lerp_color(c1, c2, scaled - index as f32)
}

fn calculate_t(frame: usize, offset: f32, speed: f32, easing: &crate::core::Easing) -> f32 {
    let phase = offset - (frame as f32 * 0.05 * speed);
    match easing {
        crate::core::Easing::Preset(p) => match p.as_str() {
            "linear"   => (phase % 2.0).abs() / 2.0,
            "pingpong" => { let p = phase % 2.0; if p < 1.0 { p } else { 2.0 - p } }
            _          => (phase.sin() + 1.0) / 2.0,
        },
        crate::core::Easing::Custom(val) => {
            ((phase.sin() + 1.0) / 2.0).powf(val.max(0.01))
        }
    }
}

fn is_valid_hex(hex: &str) -> bool {
    let s = hex.trim().trim_start_matches('#');
    s.len() == 6 && u32::from_str_radix(s, 16).is_ok()
}

// ─── Section accent colours ───────────────────────────────────────────────────

fn section_accent(mapping: &FieldMapping) -> Color {
    match mapping {
        FieldMapping::Name         => LAVENDER,
        FieldMapping::BranchColors => SAPPHIRE,
        FieldMapping::DirColors    => BLUE,
        FieldMapping::FileColors   => TEAL,
        FieldMapping::RuleGlob(_)  => PEACH,
        _                          => OVERLAY1,
    }
}

// ─── Row title helper (unchanged logic) ──────────────────────────────────────

fn get_row_title(mapping: &FieldMapping) -> Option<String> {
    match mapping {
        FieldMapping::Name         => Some(" General Settings ".to_string()),
        FieldMapping::BranchColors => Some(" Branch Formatting ".to_string()),
        FieldMapping::DirColors    => Some(" Directory Defaults ".to_string()),
        FieldMapping::FileColors   => Some(" File Defaults ".to_string()),
        FieldMapping::RuleGlob(i)  => Some(format!(" Overriding Rule {} ", i + 1)),
        _                          => None,
    }
}

// ─── Tree View ────────────────────────────────────────────────────────────────

pub fn draw_tree(f: &mut Frame, app: &mut App) {
    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let mut indent = String::new();
            if item.depth > 0 {
                for &anc_last in &item.ancestor_is_last {
                    indent.push_str(if anc_last { "    " } else { "│   " });
                }
                indent.push_str(if item.is_last { "└── " } else { "├── " });
            }

            let phase_offset = index as f32 * 0.1;
            let t = calculate_t(app.frame_count, phase_offset, item.cached_speed, &item.cached_easing);
            let live_color = multi_lerp(&item.cached_colors, t);

            let branch_t = calculate_t(
                app.frame_count,
                phase_offset,
                app.runtime_theme.branch_speed,
                &app.runtime_theme.branch_easing,
            );
            let branch_color = multi_lerp(&app.runtime_theme.branch_colors, branch_t);

            let style = Style::default().fg(live_color);
            let final_style = if item.is_dir { style.add_modifier(Modifier::BOLD) } else { style };

            ListItem::new(Line::from(vec![
                Span::styled(indent, Style::default().fg(branch_color)),
                Span::styled(item.cached_icon.clone(), final_style),
                Span::raw(" "),
                Span::styled(item.name.clone(), final_style),
            ]))
        })
        .collect();

    // Styled footer: green confirmation message or keybinding hints
    let footer_line = if let Some(msg) = &app.feedback_msg {
        Line::from(Span::styled(
            format!(" ✔  {} ", msg),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(vec![
            Span::raw(" "),
            Span::styled("[t]", Style::default().fg(LAVENDER)),
            Span::styled(" Theme  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[c]", Style::default().fg(LAVENDER)),
            Span::styled(" Copy  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[q]", Style::default().fg(LAVENDER)),
            Span::styled(" Quit ", Style::default().fg(SUBTEXT0)),
        ])
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SURFACE1))
                .title_bottom(footer_line.alignment(Alignment::Center)),
        )
        .highlight_style(
            Style::default()
                .bg(SURFACE0)
                .fg(TEXT)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, f.area(), &mut app.tree_state);
}

// ─── Theme Menu ───────────────────────────────────────────────────────────────

pub fn draw_theme_menu(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Animated title cycling through palette accent colours
    let t = calculate_t(app.frame_count, 0.0, 1.5, &crate::core::Easing::Preset("sine".into()));
    let c = multi_lerp(&[BLUE, MAUVE, PEACH, YELLOW, BLUE], t);

    let title_para = Paragraph::new(Line::from(vec![
        Span::styled(" ✦ ", Style::default().fg(OVERLAY1)),
        Span::styled("Theme Config", Style::default().fg(c).add_modifier(Modifier::BOLD)),
        Span::styled(" ✦ ", Style::default().fg(OVERLAY1)),
    ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(SURFACE1)),
        )
        .alignment(Alignment::Center);
    f.render_widget(title_para, chunks[0]);

    let themes = app.manager.list_themes();
    let active = app.manager.active_theme_name();

    let items: Vec<ListItem> = themes
        .iter()
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            let is_active = name == active;
            let (prefix, style) = if is_active {
                (" ✦ ", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
            } else {
                ("   ", Style::default().fg(SUBTEXT0))
            };
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(SURFACE1)),
        )
        .highlight_style(
            Style::default()
                .bg(SURFACE0)
                .fg(TEXT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.menu_state);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("[Enter/e]", Style::default().fg(LAVENDER)),
            Span::styled(" Edit  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[a]", Style::default().fg(LAVENDER)),
            Span::styled(" Apply  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[Esc/q]", Style::default().fg(LAVENDER)),
            Span::styled(" Back ", Style::default().fg(SUBTEXT0)),
        ]))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(SURFACE1)),
            )
            .alignment(Alignment::Center),
        chunks[2],
    );
}

// ─── Editor ───────────────────────────────────────────────────────────────────

pub fn draw_editor(f: &mut Frame, app: &mut App) {
    let ed = app.editor_state.as_mut().unwrap();
    let area = f.area();

    // Animated outer title
    let title_t = calculate_t(
        app.frame_count,
        0.0,
        0.8,
        &crate::core::Easing::Preset("sine".into()),
    );
    let title_color = multi_lerp(&[BLUE, LAVENDER, MAUVE, BLUE], title_t);

    let editor_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(SURFACE1))
        .title(Line::from(vec![
            Span::styled(" ✦ Editing: ", Style::default().fg(OVERLAY1)),
            Span::styled(
                ed.path.file_name().unwrap().to_string_lossy().to_string(),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ✦ ", Style::default().fg(OVERLAY1)),
        ]))
        .title_alignment(Alignment::Center);

    let inner_area = editor_block.inner(area);
    f.render_widget(editor_block, area);

    // Body + help-bar footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner_area);

    // Sidebar + workspace
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Sidebar
            Constraint::Length(2),      // Spacer
            Constraint::Percentage(80), // Form + preview
        ])
        .split(main_chunks[0]);

    // ── Sidebar ──────────────────────────────────────────────────────────────
    let page_labels = ["General Config", "Theme Rules"];
    let sidebar_items: Vec<ListItem> = page_labels
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let is_active = i == ed.active_page;
            let (prefix, style) = if is_active {
                (
                    " ▶ ",
                    Style::default().fg(BLUE).bg(SURFACE0).add_modifier(Modifier::BOLD),
                )
            } else {
                ("   ", Style::default().fg(OVERLAY1))
            };
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect();

    f.render_widget(
        List::new(sidebar_items).block(
            Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(SURFACE1)),
        ),
        body_chunks[0],
    );

    // ── Right workspace: form (top) + preview panel (bottom) ─────────────────
    let right_workspace = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Scrolling form
            Constraint::Length(1), // Spacer
            Constraint::Length(7), // Preview panel (extended: swatch + info row)
        ])
        .split(body_chunks[2]);

    let form_area    = right_workspace[0];
    let preview_area = right_workspace[2];

    let current_page   = &ed.pages[ed.active_page];
    let available_height = form_area.height;

    // Dynamic scroll
    loop {
        let mut used_h = 0u16;
        let mut visible_count = 0usize;

        for row in &current_page[ed.scroll_offset..] {
            let needed = if get_row_title(&row.fields[0].mapping).is_some() { 3 } else { 2 };
            if used_h + needed > available_height && visible_count > 0 { break; }
            used_h += needed;
            visible_count += 1;
        }

        if ed.focused_row < ed.scroll_offset {
            ed.scroll_offset = ed.focused_row;
        } else if ed.focused_row >= ed.scroll_offset + visible_count {
            ed.scroll_offset += 1;
        } else {
            break;
        }
    }

    let mut layout_constraints = vec![];
    let mut end_idx = ed.scroll_offset;
    let mut used_h  = 0u16;

    for row in &current_page[ed.scroll_offset..] {
        let is_header = get_row_title(&row.fields[0].mapping).is_some();
        let needed    = if is_header { 3 } else { 2 };

        if used_h + needed > available_height && end_idx > ed.scroll_offset { break; }
        used_h  += needed;
        end_idx += 1;

        layout_constraints.push(if is_header { Constraint::Length(1) } else { Constraint::Length(0) });
        layout_constraints.push(Constraint::Length(1)); // Input row
        layout_constraints.push(Constraint::Length(1)); // Spacing
    }

    let visible_rows = &current_page[ed.scroll_offset..end_idx];
    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(layout_constraints)
        .split(form_area);

    let mut dropdown_area     = None;
    let mut dropdown_options  = vec![];
    let mut dropdown_selected = 0;

    for (r_idx, row) in visible_rows.iter().enumerate() {
        let global_r_idx = ed.scroll_offset + r_idx;
        let chunk_base   = r_idx * 3;

        let header_area = row_chunks[chunk_base];
        let input_area  = row_chunks[chunk_base + 1];

        // Section header with palette-coloured title
        if let Some(title) = get_row_title(&row.fields[0].mapping) {
            let accent = section_accent(&row.fields[0].mapping);
            f.render_widget(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(SURFACE1))
                    .title(Line::from(vec![
                        Span::raw(" "),
                        Span::styled(
                            title.trim().to_string(),
                            Style::default().fg(accent).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                    ])),
                header_area,
            );
        }

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Ratio(1, row.fields.len() as u32);
                row.fields.len()
            ])
            .split(input_area);

        for (c_idx, field) in row.fields.iter().enumerate() {
            let is_focused = global_r_idx == ed.focused_row && c_idx == ed.focused_col;

            // Layout: accent bar | label | gap | input | right padding
            //         [0]          [1]     [2]   [3]     [4]
            let field_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(1),       // [0] accent bar
                    Constraint::Percentage(40),  // [1] label
                    Constraint::Length(1),       // [2] gap
                    Constraint::Min(5),          // [3] input
                    Constraint::Length(1),       // [4] right padding
                ])
                .split(col_chunks[c_idx]);

            // Accent bar — visible left-edge indicator when focused
            f.render_widget(
                Paragraph::new(if is_focused { "▌" } else { " " }).style(
                    Style::default().fg(if is_focused { BLUE } else { SURFACE1 }),
                ),
                field_layout[0],
            );

            // Label
            f.render_widget(
                Paragraph::new(format!("{} ", field.label))
                    .style(if is_focused {
                        Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(OVERLAY0)
                    })
                    .alignment(Alignment::Right),
                field_layout[1],
            );

            // Input field
            let (input_bg, input_fg) = if is_focused {
                (SURFACE1, TEXT)
            } else {
                (SURFACE0, SUBTEXT0)
            };
            f.render_widget(
                Paragraph::new(format!(" {} ", field.input.value()))
                    .style(Style::default().bg(input_bg).fg(input_fg)),
                field_layout[3],
            );

            if is_focused {
                let cursor_x = field_layout[3].x + 1 + field.input.visual_cursor() as u16;
                let cursor_y = field_layout[3].y;

                if cursor_x < field_layout[3].x + field_layout[3].width {
                    if ed.dropdown.is_none() {
                        f.set_cursor_position(Position { x: cursor_x, y: cursor_y });
                    }
                }

                if let Some(drop) = &ed.dropdown {
                    if drop.row == global_r_idx && drop.col == c_idx {
                        let mut area = field_layout[3];
                        area.y += 1;
                        area.height = drop.options.len() as u16 + 2;
                        if area.y + area.height > f.area().bottom() {
                            area.y = area.y.saturating_sub(area.height + 1);
                        }
                        dropdown_area     = Some(area);
                        dropdown_options  = drop.options.clone();
                        dropdown_selected = drop.selected;
                    }
                }
            }
        }
    }

    // ── Preview Panel ─────────────────────────────────────────────────────────
    let focused_mapping = &ed.pages[ed.active_page][ed.focused_row].fields[ed.focused_col].mapping;
    let current_theme   = App::reconstruct_theme(ed);

    let mut errors         = vec![];
    let mut preview_colors = vec![Color::White];
    let mut preview_speed  = 1.0f32;
    let mut preview_easing = crate::core::Easing::Preset("linear".to_string());

    let (preview_text, preview_icon) = match focused_mapping {
        FieldMapping::Name => (
            format!("Editing Theme: {}", current_theme.name),
            "🏷️ ".to_string(),
        ),
        FieldMapping::BranchColors | FieldMapping::BranchSpeed | FieldMapping::BranchEasing => {
            for hex in current_theme.branch_colors.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !is_valid_hex(hex) { errors.push("Invalid Hex (use #RRGGBB)"); }
            }
            preview_colors = crate::core::parse_gradient(&current_theme.branch_colors);
            preview_speed  = current_theme.branch_speed;
            preview_easing = current_theme.branch_easing.clone();
            ("├── └── │   (Branch formatting preview)".to_string(), String::new())
        }
        FieldMapping::DirColors | FieldMapping::DirIcon | FieldMapping::DirSpeed | FieldMapping::DirEasing => {
            for hex in current_theme.dir_colors.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !is_valid_hex(hex) { errors.push("Invalid Hex (use #RRGGBB)"); }
            }
            preview_colors = crate::core::parse_gradient(&current_theme.dir_colors);
            preview_speed  = current_theme.dir_speed;
            preview_easing = current_theme.dir_easing.clone();
            ("example_folder".to_string(), current_theme.dir_icon.clone())
        }
        FieldMapping::FileColors | FieldMapping::FileIcon | FieldMapping::FileSpeed | FieldMapping::FileEasing => {
            for hex in current_theme.file_colors.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !is_valid_hex(hex) { errors.push("Invalid Hex (use #RRGGBB)"); }
            }
            preview_colors = crate::core::parse_gradient(&current_theme.file_colors);
            preview_speed  = current_theme.file_speed;
            preview_easing = current_theme.file_easing.clone();
            ("example_file.txt".to_string(), current_theme.file_icon.clone())
        }
        FieldMapping::RuleGlob(i)
        | FieldMapping::RuleTarget(i)
        | FieldMapping::RuleColors(i)
        | FieldMapping::RuleIcon(i)
        | FieldMapping::RuleSpeed(i)
        | FieldMapping::RuleEasing(i) => {
            let rule = &current_theme.rules[*i];
            if glob::Pattern::new(&rule.glob).is_err() { errors.push("Invalid Glob Pattern"); }
            for hex in rule.colors.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !is_valid_hex(hex) { errors.push("Invalid Hex (use #RRGGBB)"); }
            }
            preview_colors = crate::core::parse_gradient(&rule.colors);
            preview_speed  = rule.anim_speed;
            preview_easing = rule.anim_easing.clone();
            let name = if rule.glob.contains('*') { rule.glob.replace('*', "example") } else { rule.glob.clone() };
            (name, rule.icon.clone())
        }
    };

    let (border_color, preview_title_str) = if !errors.is_empty() {
        (RED, " ⚠ Preview ")
    } else {
        (GREEN, " ✦ Preview ")
    };

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(Span::styled(
            preview_title_str,
            Style::default().fg(border_color).add_modifier(Modifier::BOLD),
        )))
        .title_alignment(Alignment::Left);

    let inner_preview = preview_block.inner(preview_area);
    f.render_widget(preview_block, preview_area);

    if !errors.is_empty() {
        // Vertically-centred error row
        let v_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Min(0)])
            .split(inner_preview);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("⚠  ", Style::default().fg(YELLOW)),
                Span::styled(errors.join("  ·  "), Style::default().fg(RED)),
            ]))
                .alignment(Alignment::Center),
            v_layout[1],
        );
    } else {
        let t = calculate_t(app.frame_count, 0.0, preview_speed, &preview_easing);
        let c = multi_lerp(&preview_colors, t);

        // Five inner rows: padding | content | spacing | swatch | info
        let preview_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // top padding
                Constraint::Length(1), // icon + name with animated colour
                Constraint::Length(1), // spacing
                Constraint::Length(1), // gradient swatch bar
                Constraint::Length(1), // speed / easing info
            ])
            .split(inner_preview);

        // Animated preview content
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(format!("{} ", preview_icon)),
                Span::styled(preview_text, Style::default().fg(c).add_modifier(Modifier::BOLD)),
            ]))
                .alignment(Alignment::Center),
            preview_layout[1],
        );

        // Full-width gradient swatch: ◈ ███████████████
        let swatch_width = inner_preview.width.saturating_sub(8) as usize;
        if swatch_width > 1 {
            let mut swatch_spans: Vec<Span> = vec![
                Span::styled("  ◈ ", Style::default().fg(OVERLAY1)),
            ];
            for i in 0..swatch_width {
                let t = i as f32 / (swatch_width - 1).max(1) as f32;
                let sc = multi_lerp(&preview_colors, t);
                swatch_spans.push(Span::styled("█", Style::default().fg(sc)));
            }
            f.render_widget(
                Paragraph::new(Line::from(swatch_spans)),
                preview_layout[3],
            );
        }

        // Animation metadata row
        let easing_str = match &preview_easing {
            crate::core::Easing::Preset(p) => p.clone(),
            crate::core::Easing::Custom(v) => format!("{:.2}", v),
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  speed ", Style::default().fg(OVERLAY1)),
                Span::styled(format!("{:.1}", preview_speed), Style::default().fg(SUBTEXT0)),
                Span::styled("  ·  easing ", Style::default().fg(OVERLAY1)),
                Span::styled(easing_str, Style::default().fg(SUBTEXT0)),
            ])),
            preview_layout[4],
        );
    }

    // ── Help Bar ──────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("[↑↓/Tab]", Style::default().fg(LAVENDER)),
            Span::styled(" Nav  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[Enter]", Style::default().fg(LAVENDER)),
            Span::styled(" Drop  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[^N]", Style::default().fg(LAVENDER)),
            Span::styled(" Add  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[^D]", Style::default().fg(LAVENDER)),
            Span::styled(" Del  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[^S]", Style::default().fg(LAVENDER)),
            Span::styled(" Save  ", Style::default().fg(SUBTEXT0)),
            Span::styled("[Esc]", Style::default().fg(LAVENDER)),
            Span::styled(" Back ", Style::default().fg(SUBTEXT0)),
        ]))
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG_MANTLE)),
        main_chunks[1],
    );

    // ── Dropdown overlay ──────────────────────────────────────────────────────
    if let Some(area) = dropdown_area {
        f.render_widget(ratatui::widgets::Clear, area);

        let items: Vec<ListItem> = dropdown_options
            .into_iter()
            .enumerate()
            .map(|(i, opt)| {
                let style = if i == dropdown_selected {
                    Style::default()
                        .bg(BLUE)
                        .fg(BG_MANTLE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                ListItem::new(format!(" {} ", opt)).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BLUE))
                    .style(Style::default().bg(BG_MANTLE)),
            ),
            area,
        );
    }
}