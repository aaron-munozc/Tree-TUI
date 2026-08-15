use crate::Cli;
use crate::core::{
    Easing, Target, Theme, ThemeManager, ThemeRule, TreeItem, apply_theme_to_tree, collect_tree,
    copy_to_clipboard,
};
use crate::ui::{draw_editor, draw_theme_menu, draw_tree};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, widgets::ListState};
use std::time::{Duration, Instant};
use tui_input::Input;

pub enum View {
    Tree,
    ThemeList,
    Editor,
}

#[derive(Clone, PartialEq)]
pub enum FieldMapping {
    Name,
    BranchColors,
    BranchSpeed,
    BranchEasing,
    DirColors,
    DirIcon,
    DirSpeed,
    DirEasing,
    FileColors,
    FileIcon,
    FileSpeed,
    FileEasing,
    RuleGlob(usize),
    RuleTarget(usize),
    RuleColors(usize),
    RuleIcon(usize),
    RuleSpeed(usize),
    RuleEasing(usize),
}

#[derive(Clone, PartialEq)]
pub enum FieldKind {
    Text,
    TargetDropdown,
    EasingCombo,
}

pub struct FormField {
    pub mapping: FieldMapping,
    pub label: String,
    pub input: Input,
    pub kind: FieldKind,
}

pub struct FormRow {
    pub fields: Vec<FormField>,
}

pub struct DropdownState {
    pub options: Vec<String>,
    pub selected: usize,
    pub row: usize,
    pub col: usize,
}

pub struct EditorState {
    pub path: std::path::PathBuf,
    pub pages: Vec<Vec<FormRow>>,
    pub active_page: usize,
    pub focused_row: usize,
    pub focused_col: usize,
    pub scroll_offset: usize,
    pub dropdown: Option<DropdownState>,
}

impl EditorState {
    pub fn clamp_col(&mut self) {
        let rows = &self.pages[self.active_page];
        if rows.is_empty() {
            self.focused_col = 0;
            return;
        }
        let max_cols = rows[self.focused_row].fields.len().saturating_sub(1);
        self.focused_col = self.focused_col.min(max_cols);
    }

    pub fn clamp_row(&mut self) {
        let rows = &self.pages[self.active_page];
        if rows.is_empty() {
            self.focused_row = 0;
            return;
        }
        self.focused_row = self.focused_row.min(rows.len().saturating_sub(1));
    }
}

pub struct App {
    pub cli: Cli,
    pub view: View,
    pub manager: ThemeManager,
    pub active_theme: Theme,
    pub runtime_theme: crate::core::RuntimeTheme,
    pub tree_items: Vec<TreeItem>,
    pub tree_state: ListState,
    pub menu_state: ListState,
    pub editor_state: Option<EditorState>,
    pub frame_count: usize,
    pub feedback_msg: Option<String>,
}

impl App {
    pub fn new(cli: Cli) -> Self {
        let manager = ThemeManager::new();
        let active_name = manager.active_theme_name();
        let active_theme = manager
            .load_theme(&manager.dir.join(&active_name))
            .unwrap_or_default();
        let runtime_theme = active_theme.to_runtime();

        let mut tree_items = collect_tree(
            ".",
            cli.depth,
            cli.no_ignore,
            &cli.sort_mode,
            cli.max_entries,
        );
        apply_theme_to_tree(&mut tree_items, &runtime_theme);

        let mut tree_state = ListState::default();
        if !tree_items.is_empty() {
            tree_state.select(Some(0));
        }

        Self {
            cli,
            view: View::Tree,
            manager,
            active_theme,
            runtime_theme,
            tree_items,
            tree_state,
            menu_state: ListState::default(),
            editor_state: None,
            frame_count: 0,
            feedback_msg: None,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let tick_rate = Duration::from_millis(16);
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| match self.view {
                View::Tree => draw_tree(f, self),
                View::ThemeList => draw_theme_menu(f, self),
                View::Editor => draw_editor(f, self),
            })?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if !matches!(self.view, View::Tree) {
                            self.feedback_msg = None;
                        }
                        match self.view {
                            View::Tree => self.handle_tree_events(key.code),
                            View::ThemeList => self.handle_menu_events(key.code),
                            View::Editor => self.handle_editor_events(key.code, key.modifiers),
                        }
                        if let KeyCode::Char('q') = key.code {
                            if matches!(self.view, View::Tree) && key.modifiers.is_empty() {
                                break;
                            }
                        }
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.frame_count = self.frame_count.wrapping_add(1);
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn handle_tree_events(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('t') => self.view = View::ThemeList,
            KeyCode::Char('c') => {
                if !self.cli.no_clipboard {
                    if copy_to_clipboard(&self.tree_items).is_ok() {
                        self.feedback_msg = Some(" Tree copied to clipboard!".to_string());
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self
                    .tree_state
                    .selected()
                    .map_or(0, |i| (i + 1).min(self.tree_items.len().saturating_sub(1)));
                self.tree_state.select(Some(i));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self
                    .tree_state
                    .selected()
                    .map_or(0, |i| i.saturating_sub(1));
                self.tree_state.select(Some(i));
            }
            KeyCode::PageDown => {
                let i = self
                    .tree_state
                    .selected()
                    .map_or(0, |i| (i + 15).min(self.tree_items.len().saturating_sub(1)));
                self.tree_state.select(Some(i));
            }
            KeyCode::PageUp => {
                let i = self
                    .tree_state
                    .selected()
                    .map_or(0, |i| i.saturating_sub(15));
                self.tree_state.select(Some(i));
            }
            _ => {
                self.feedback_msg = None;
            }
        }
    }

    fn handle_menu_events(&mut self, code: KeyCode) {
        let themes = self.manager.list_themes();
        match code {
            KeyCode::Esc | KeyCode::Char('q') => self.view = View::Tree,
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self
                    .menu_state
                    .selected()
                    .map_or(0, |i| (i + 1).min(themes.len().saturating_sub(1)));
                self.menu_state.select(Some(i));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self
                    .menu_state
                    .selected()
                    .map_or(0, |i| i.saturating_sub(1));
                self.menu_state.select(Some(i));
            }
            KeyCode::Char('a') => {
                if let Some(idx) = self.menu_state.selected() {
                    let path = &themes[idx];
                    let name = path.file_name().unwrap().to_string_lossy();
                    self.manager.set_active_theme(&name);
                    if let Some(t) = self.manager.load_theme(path) {
                        self.active_theme = t.clone();
                        self.runtime_theme = self.active_theme.to_runtime();
                        apply_theme_to_tree(&mut self.tree_items, &self.runtime_theme);
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(idx) = self.menu_state.selected() {
                    let path = themes[idx].clone();
                    let theme = self.manager.load_theme(&path).unwrap_or_default();
                    self.build_editor_state(theme, path, Some(0));
                    self.view = View::Editor;
                }
            }
            _ => {}
        }
    }

    fn build_editor_state(
        &mut self,
        theme: Theme,
        path: std::path::PathBuf,
        keep_page: Option<usize>,
    ) {
        let mut page0 = vec![];

        page0.push(FormRow {
            fields: vec![FormField {
                mapping: FieldMapping::Name,
                label: "Theme Name".into(),
                input: Input::default().with_value(theme.name.clone()),
                kind: FieldKind::Text,
            }],
        });

        // Branch Group
        page0.push(FormRow {
            fields: vec![
                FormField {
                    mapping: FieldMapping::BranchColors,
                    label: "Branch Gradients".into(),
                    input: Input::default().with_value(theme.branch_colors.clone()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::BranchSpeed,
                    label: "Spd".into(),
                    input: Input::default().with_value(theme.branch_speed.to_string()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::BranchEasing,
                    label: "Ease (Enter↓)".into(),
                    input: Input::default().with_value(theme.branch_easing.to_string()),
                    kind: FieldKind::EasingCombo,
                },
            ],
        });

        // Dir Group
        page0.push(FormRow {
            fields: vec![
                FormField {
                    mapping: FieldMapping::DirColors,
                    label: "Default Dir Gradients".into(),
                    input: Input::default().with_value(theme.dir_colors.clone()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::DirIcon,
                    label: "Icon".into(),
                    input: Input::default().with_value(theme.dir_icon.clone()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::DirSpeed,
                    label: "Spd".into(),
                    input: Input::default().with_value(theme.dir_speed.to_string()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::DirEasing,
                    label: "Ease (Enter↓)".into(),
                    input: Input::default().with_value(theme.dir_easing.to_string()),
                    kind: FieldKind::EasingCombo,
                },
            ],
        });

        // File Group
        page0.push(FormRow {
            fields: vec![
                FormField {
                    mapping: FieldMapping::FileColors,
                    label: "Default File Gradients".into(),
                    input: Input::default().with_value(theme.file_colors.clone()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::FileIcon,
                    label: "Icon".into(),
                    input: Input::default().with_value(theme.file_icon.clone()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::FileSpeed,
                    label: "Spd".into(),
                    input: Input::default().with_value(theme.file_speed.to_string()),
                    kind: FieldKind::Text,
                },
                FormField {
                    mapping: FieldMapping::FileEasing,
                    label: "Ease (Enter↓)".into(),
                    input: Input::default().with_value(theme.file_easing.to_string()),
                    kind: FieldKind::EasingCombo,
                },
            ],
        });

        let mut page1 = vec![];

        for (i, rule) in theme.rules.iter().enumerate() {
            // Row 1 of Rule
            page1.push(FormRow {
                fields: vec![
                    FormField {
                        mapping: FieldMapping::RuleGlob(i),
                        label: format!("R{} Glob", i + 1),
                        input: Input::default().with_value(rule.glob.clone()),
                        kind: FieldKind::Text,
                    },
                    FormField {
                        mapping: FieldMapping::RuleTarget(i),
                        label: "Target (Enter↓)".into(),
                        input: Input::default().with_value(rule.target.to_string()),
                        kind: FieldKind::TargetDropdown,
                    },
                    FormField {
                        mapping: FieldMapping::RuleIcon(i),
                        label: "Icon".into(),
                        input: Input::default().with_value(rule.icon.clone()),
                        kind: FieldKind::Text,
                    },
                ],
            });
            // Row 2 of Rule
            page1.push(FormRow {
                fields: vec![
                    FormField {
                        mapping: FieldMapping::RuleColors(i),
                        label: "Gradients".into(),
                        input: Input::default().with_value(rule.colors.clone()),
                        kind: FieldKind::Text,
                    },
                    FormField {
                        mapping: FieldMapping::RuleSpeed(i),
                        label: "Spd".into(),
                        input: Input::default().with_value(rule.anim_speed.to_string()),
                        kind: FieldKind::Text,
                    },
                    FormField {
                        mapping: FieldMapping::RuleEasing(i),
                        label: "Ease (Enter↓)".into(),
                        input: Input::default().with_value(rule.anim_easing.to_string()),
                        kind: FieldKind::EasingCombo,
                    },
                ],
            });
        }

        self.editor_state = Some(EditorState {
            path,
            pages: vec![page0, page1],
            active_page: keep_page.unwrap_or(0),
            focused_row: 0,
            focused_col: 0,
            scroll_offset: 0,
            dropdown: None,
        });

        if let Some(ed) = &mut self.editor_state {
            ed.clamp_row();
            ed.clamp_col();
        }
    }

    fn handle_editor_events(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let mut ed = match self.editor_state.take() {
            Some(state) => state,
            None => return,
        };

        let mut keep_state = true;

        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('s') => {
                    let new_theme = Self::reconstruct_theme(&ed);
                    self.manager.save_theme(&ed.path, &new_theme);

                    if self.manager.active_theme_name()
                        == ed.path.file_name().unwrap().to_string_lossy()
                    {
                        self.active_theme = new_theme.clone();
                        self.runtime_theme = self.active_theme.to_runtime();
                        apply_theme_to_tree(&mut self.tree_items, &self.runtime_theme);
                    }
                    self.view = View::ThemeList;
                    keep_state = false;
                }
                KeyCode::Char('n') => {
                    let mut t = Self::reconstruct_theme(&ed);
                    t.rules.push(ThemeRule {
                        glob: "*.new".into(),
                        target: Target::Any,
                        colors: "#FFFFFF, #888888".into(),
                        icon: "📄 ".into(),
                        anim_speed: 1.0,
                        anim_easing: Easing::Preset("sine".to_string()),
                    });
                    let path = ed.path.clone();
                    self.build_editor_state(t, path, Some(1));
                    if let Some(new_ed) = &mut self.editor_state {
                        new_ed.focused_row = new_ed.pages[1].len().saturating_sub(1);
                        new_ed.focused_col = 0;
                        new_ed.scroll_offset = new_ed.focused_row.saturating_sub(2);
                    }
                    keep_state = false;
                }
                KeyCode::Char('d') => {
                    if !ed.pages[ed.active_page].is_empty() {
                        let mapping = &ed.pages[ed.active_page][ed.focused_row].fields
                            [ed.focused_col]
                            .mapping;
                        if let FieldMapping::RuleGlob(idx)
                        | FieldMapping::RuleTarget(idx)
                        | FieldMapping::RuleColors(idx)
                        | FieldMapping::RuleIcon(idx)
                        | FieldMapping::RuleSpeed(idx)
                        | FieldMapping::RuleEasing(idx) = mapping
                        {
                            let mut t = Self::reconstruct_theme(&ed);
                            t.rules.remove(*idx);
                            let path = ed.path.clone();
                            let page = ed.active_page;
                            self.build_editor_state(t, path, Some(page));
                            keep_state = false;
                        }
                    }
                }
                _ => {}
            }
        } else if let Some(ref mut drop) = ed.dropdown {
            match code {
                KeyCode::Esc => ed.dropdown = None,
                KeyCode::Up => drop.selected = drop.selected.saturating_sub(1),
                KeyCode::Down => {
                    let max = drop.options.len().saturating_sub(1);
                    drop.selected = (drop.selected + 1).min(max);
                }
                KeyCode::Enter => {
                    let selected_val = drop.options[drop.selected].clone();
                    let r = drop.row;
                    let c = drop.col;
                    ed.pages[ed.active_page][r].fields[c].input =
                        Input::default().with_value(selected_val);
                    ed.dropdown = None;
                }
                _ => {}
            }
        } else {
            match code {
                KeyCode::Esc => self.view = View::ThemeList,
                KeyCode::Enter => {
                    let field = &ed.pages[ed.active_page][ed.focused_row].fields[ed.focused_col];
                    let options = match field.kind {
                        FieldKind::TargetDropdown => Some(vec![
                            "Any".to_string(),
                            "File".to_string(),
                            "Dir".to_string(),
                        ]),
                        FieldKind::EasingCombo => Some(vec![
                            "linear".to_string(),
                            "sine".to_string(),
                            "pingpong".to_string(),
                        ]),
                        _ => None,
                    };
                    if let Some(opts) = options {
                        ed.dropdown = Some(DropdownState {
                            options: opts,
                            selected: 0,
                            row: ed.focused_row,
                            col: ed.focused_col,
                        });
                    }
                }
                KeyCode::PageUp => {
                    ed.active_page = ed.active_page.saturating_sub(1);
                    ed.focused_row = 0;
                    ed.focused_col = 0;
                    ed.scroll_offset = 0;
                }
                KeyCode::PageDown => {
                    ed.active_page = (ed.active_page + 1).min(ed.pages.len().saturating_sub(1));
                    ed.focused_row = 0;
                    ed.focused_col = 0;
                    ed.scroll_offset = 0;
                }
                KeyCode::Up => {
                    ed.focused_row = ed.focused_row.saturating_sub(1);
                    ed.clamp_col();
                }
                KeyCode::Down => {
                    let rows = &ed.pages[ed.active_page];
                    ed.focused_row = (ed.focused_row + 1).min(rows.len().saturating_sub(1));
                    ed.clamp_col();
                }
                KeyCode::Tab => {
                    let rows = &ed.pages[ed.active_page];
                    if ed.focused_col < rows[ed.focused_row].fields.len().saturating_sub(1) {
                        ed.focused_col += 1;
                    } else if ed.focused_row < rows.len().saturating_sub(1) {
                        ed.focused_row += 1;
                        ed.focused_col = 0;
                    } else if ed.active_page < ed.pages.len().saturating_sub(1) {
                        ed.active_page += 1;
                        ed.focused_row = 0;
                        ed.focused_col = 0;
                        ed.scroll_offset = 0;
                    } else {
                        ed.active_page = 0;
                        ed.focused_row = 0;
                        ed.focused_col = 0;
                        ed.scroll_offset = 0;
                    }
                }
                KeyCode::BackTab => {
                    if ed.focused_col > 0 {
                        ed.focused_col -= 1;
                    } else if ed.focused_row > 0 {
                        ed.focused_row -= 1;
                        ed.focused_col = ed.pages[ed.active_page][ed.focused_row].fields.len() - 1;
                    } else if ed.active_page > 0 {
                        ed.active_page -= 1;
                        ed.focused_row = ed.pages[ed.active_page].len().saturating_sub(1);
                        ed.focused_col = ed.pages[ed.active_page][ed.focused_row].fields.len() - 1;
                        ed.scroll_offset = 0;
                    } else {
                        ed.active_page = ed.pages.len().saturating_sub(1);
                        ed.focused_row = ed.pages[ed.active_page].len().saturating_sub(1);
                        ed.focused_col = ed.pages[ed.active_page][ed.focused_row].fields.len() - 1;
                        ed.scroll_offset = 0;
                    }
                }
                _ => {
                    let key_event = KeyEvent::new(code, modifiers);
                    if let Some(req) =
                        tui_input::backend::crossterm::to_input_request(&Event::Key(key_event))
                    {
                        if !ed.pages[ed.active_page].is_empty() {
                            ed.pages[ed.active_page][ed.focused_row].fields[ed.focused_col]
                                .input
                                .handle(req);
                        }
                    }
                }
            }
        }

        if keep_state {
            self.editor_state = Some(ed);
        }
    }

    pub fn reconstruct_theme(ed: &EditorState) -> Theme {
        let mut t = Theme {
            rules: vec![],
            ..Default::default()
        };
        let mut max_rule_idx = 0;

        for page in &ed.pages {
            for row in page {
                for field in &row.fields {
                    if let FieldMapping::RuleGlob(idx)
                    | FieldMapping::RuleTarget(idx)
                    | FieldMapping::RuleColors(idx)
                    | FieldMapping::RuleIcon(idx)
                    | FieldMapping::RuleSpeed(idx)
                    | FieldMapping::RuleEasing(idx) = field.mapping
                    {
                        max_rule_idx = max_rule_idx.max(idx + 1);
                    }
                }
            }
        }

        t.rules = vec![ThemeRule::default(); max_rule_idx];

        for page in &ed.pages {
            for row in page {
                for field in &row.fields {
                    let val = field.input.value().to_string();
                    match field.mapping {
                        FieldMapping::Name => t.name = val,
                        FieldMapping::BranchColors => t.branch_colors = val,
                        FieldMapping::BranchSpeed => t.branch_speed = val.parse().unwrap_or(1.0),
                        FieldMapping::BranchEasing => t.branch_easing = Easing::from_str(&val),
                        FieldMapping::DirColors => t.dir_colors = val,
                        FieldMapping::DirIcon => t.dir_icon = val,
                        FieldMapping::DirSpeed => t.dir_speed = val.parse().unwrap_or(1.0),
                        FieldMapping::DirEasing => t.dir_easing = Easing::from_str(&val),
                        FieldMapping::FileColors => t.file_colors = val,
                        FieldMapping::FileIcon => t.file_icon = val,
                        FieldMapping::FileSpeed => t.file_speed = val.parse().unwrap_or(1.0),
                        FieldMapping::FileEasing => t.file_easing = Easing::from_str(&val),
                        FieldMapping::RuleGlob(i) => t.rules[i].glob = val,
                        FieldMapping::RuleTarget(i) => t.rules[i].target = Target::from_str(&val),
                        FieldMapping::RuleColors(i) => t.rules[i].colors = val,
                        FieldMapping::RuleIcon(i) => t.rules[i].icon = val,
                        FieldMapping::RuleSpeed(i) => {
                            t.rules[i].anim_speed = val.parse().unwrap_or(1.0)
                        }
                        FieldMapping::RuleEasing(i) => {
                            t.rules[i].anim_easing = Easing::from_str(&val)
                        }
                    }
                }
            }
        }
        t
    }
}
