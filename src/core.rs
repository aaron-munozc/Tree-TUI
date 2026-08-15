use arboard::Clipboard;
use clap::ValueEnum;
use directories::ProjectDirs;
use glob::Pattern;
use ignore::WalkBuilder;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// --- ENUMS & TYPES ---
#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum SortMode {
    Name,
    Extension,
    Size,
    Modified,
}

impl fmt::Display for SortMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name => write!(f, "name"),
            Self::Extension => write!(f, "extension"),
            Self::Size => write!(f, "size"),
            Self::Modified => write!(f, "modified"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum Target {
    #[default]
    Any,
    File,
    Dir,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::Any => write!(f, "Any"),
            Target::File => write!(f, "File"),
            Target::Dir => write!(f, "Dir"),
        }
    }
}

impl Target {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "file" => Target::File,
            "dir" | "folder" => Target::Dir,
            _ => Target::Any,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Easing {
    Preset(String), // e.g., "linear", "sine", "pingpong"
    Custom(f32),    // Evaluates a custom numerical curve exponent
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Preset("linear".to_string())
    }
}

impl std::fmt::Display for Easing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Easing::Preset(p) => write!(f, "{}", p),
            Easing::Custom(c) => write!(f, "{}", c),
        }
    }
}

impl Easing {
    pub fn from_str(s: &str) -> Self {
        let s = s.trim();
        if let Ok(val) = s.parse::<f32>() {
            Easing::Custom(val)
        } else {
            Easing::Preset(s.to_string())
        }
    }
}

// --- THEME STRUCTS ---

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ThemeRule {
    pub glob: String,
    pub target: Target,
    pub colors: String,
    pub icon: String,
    pub anim_speed: f32,
    pub anim_easing: Easing,
}

// RESTORED: Required for `..Default::default()` syntax inside the Theme rules
impl Default for ThemeRule {
    fn default() -> Self {
        Self {
            glob: "*".to_string(),
            target: Target::Any,
            colors: "#FFFFFF".to_string(),
            icon: "📄".to_string(),
            anim_speed: 1.0,
            anim_easing: Easing::Preset("linear".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    // Branch Formatting
    pub branch_colors: String,
    pub branch_speed: f32,
    pub branch_easing: Easing,
    // Base Dir Defaults
    pub dir_colors: String,
    pub dir_icon: String,
    pub dir_speed: f32,
    pub dir_easing: Easing,
    // Base File Defaults
    pub file_colors: String,
    pub file_icon: String,
    pub file_speed: f32,
    pub file_easing: Easing,
    // Overriding Rules
    pub rules: Vec<ThemeRule>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Catppuccin Macchiato".to_string(),

            // Subtle, clean styling for tree branch structures
            branch_colors: "#494D64, #5B6078".to_string(),
            branch_speed: 0.2,
            branch_easing: Easing::Preset("linear".to_string()),

            // Vibrant directory default (Sky Blue gradient)
            dir_colors: "#8AADF4, #7DC4E4".to_string(),
            dir_icon: "󰉋 ".to_string(),
            dir_speed: 0.4,
            dir_easing: Easing::Preset("sine".to_string()),

            // Clean neutral text color for fallback files
            file_colors: "#CAD3F5".to_string(),
            file_icon: "󰈔 ".to_string(),
            file_speed: 0.0,
            file_easing: Easing::Preset("linear".to_string()),

            rules: vec![
                // --- SPECIAL & SYSTEM DIRECTORIES ---
                ThemeRule {
                    glob: ".git".to_string(),
                    target: Target::Dir,
                    colors: "#F5A97F".to_string(), // Peach
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "node_modules".to_string(),
                    target: Target::Dir,
                    colors: "#ED8796".to_string(), // Maroon/Red (Heavy weight)
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "target".to_string(),
                    target: Target::Dir,
                    colors: "#5B6078".to_string(), // Muted Gray (Build artifact)
                    icon: "󰚳 ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: ".github".to_string(),
                    target: Target::Dir,
                    colors: "#B7BDF8".to_string(), // Lavender
                    icon: " ".to_string(),
                    anim_speed: 0.5,
                    anim_easing: Easing::Preset("sine".to_string()),
                },
                // --- CORE PROGRAMMING LANGUAGES ---
                ThemeRule {
                    glob: "*.rs".to_string(),
                    target: Target::File,
                    colors: "#EED49F, #F5A97F".to_string(), // Rust Orange/Gold gradient
                    icon: "🦀".to_string(),
                    anim_speed: 1.5,
                    anim_easing: Easing::Preset("sine".to_string()),
                },
                ThemeRule {
                    glob: "*.js".to_string(),
                    target: Target::File,
                    colors: "#EED49F".to_string(), // JS Yellow
                    icon: "󰚵 ".to_string(),
                    anim_speed: 1.0,
                    anim_easing: Easing::Preset("pingpong".to_string()),
                },
                ThemeRule {
                    glob: "*.ts".to_string(),
                    target: Target::File,
                    colors: "#8AADF4".to_string(), // TS Blue
                    icon: "󰛦 ".to_string(),
                    anim_speed: 1.0,
                    anim_easing: Easing::Preset("linear".to_string()),
                },
                ThemeRule {
                    glob: "*.py".to_string(),
                    target: Target::File,
                    colors: "#8AADF4, #EED49F".to_string(), // Python Blue/Yellow gradient
                    icon: " ".to_string(),
                    anim_speed: 0.8,
                    anim_easing: Easing::Preset("sine".to_string()),
                },
                ThemeRule {
                    glob: "*.go".to_string(),
                    target: Target::File,
                    colors: "#7DC4E4".to_string(), // Go Cyan
                    icon: " ".to_string(),
                    anim_speed: 1.2,
                    ..Default::default()
                },
                // --- CONFIGS, DATA, & MARKUP ---
                ThemeRule {
                    glob: "*.toml".to_string(),
                    target: Target::File,
                    colors: "#F5A97F".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "*.json".to_string(),
                    target: Target::File,
                    colors: "#EED49F".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "*.yaml".to_string(),
                    target: Target::File,
                    colors: "#A6DA95".to_string(), // Green
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "*.yml".to_string(),
                    target: Target::File,
                    colors: "#A6DA95".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "*.md".to_string(),
                    target: Target::File,
                    colors: "#C6A0F6".to_string(), // Mauve
                    icon: "󰍔 ".to_string(),
                    anim_speed: 0.5,
                    anim_easing: Easing::Preset("pingpong".to_string()),
                },
                // --- DOCKER & CI/CD ---
                ThemeRule {
                    glob: "Dockerfile".to_string(),
                    target: Target::File,
                    colors: "#8AADF4, #7DC4E4".to_string(),
                    icon: "󰡨 ".to_string(),
                    anim_speed: 1.0,
                    anim_easing: Easing::Preset("sine".to_string()),
                },
                ThemeRule {
                    glob: "docker-compose.yml".to_string(),
                    target: Target::File,
                    colors: "#8AADF4".to_string(),
                    icon: "󰡨 ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                // --- PACKAGE MANAGERS & LOCKFILES ---
                ThemeRule {
                    glob: "uv.lock".to_string(),
                    target: Target::File,
                    colors: "#CBA6F7".to_string(), // Purple
                    icon: "🔒".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "bun.lockb".to_string(),
                    target: Target::File,
                    colors: "#F5E0DC".to_string(), // Soft Rosewater
                    icon: "🥟".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "Cargo.lock".to_string(),
                    target: Target::File,
                    colors: "#6E738D".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: "package-lock.json".to_string(),
                    target: Target::File,
                    colors: "#6E738D".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
                ThemeRule {
                    glob: ".gitignore".to_string(),
                    target: Target::File,
                    colors: "#ED8796".to_string(),
                    icon: " ".to_string(),
                    anim_speed: 0.0,
                    ..Default::default()
                },
            ],
        }
    }
}

pub struct RuntimeRule {
    pub pattern: Pattern,
    pub target: Target,
    pub colors: Vec<Color>,
    pub icon: String,
    pub anim_speed: f32,
    pub anim_easing: Easing,
}

pub struct RuntimeTheme {
    pub branch_colors: Vec<Color>,
    pub branch_speed: f32,
    pub branch_easing: Easing,

    pub dir_colors: Vec<Color>,
    pub dir_icon: String,
    pub dir_speed: f32,
    pub dir_easing: Easing,

    pub file_colors: Vec<Color>,
    pub file_icon: String,
    pub file_speed: f32,
    pub file_easing: Easing,

    pub rules: Vec<RuntimeRule>,
}

pub fn parse_hex(hex: &str) -> Color {
    let s = hex.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}

pub fn parse_gradient(hex_list: &str) -> Vec<Color> {
    let colors: Vec<Color> = hex_list
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(parse_hex(trimmed))
            }
        })
        .collect();
    if colors.is_empty() {
        vec![Color::White]
    } else {
        colors
    }
}

impl Theme {
    pub fn to_runtime(&self) -> RuntimeTheme {
        let rules = self
            .rules
            .iter()
            .filter_map(|r| {
                Pattern::new(&r.glob).ok().map(|p| RuntimeRule {
                    pattern: p,
                    target: r.target.clone(),
                    colors: parse_gradient(&r.colors),
                    icon: r.icon.clone(),
                    anim_speed: r.anim_speed,
                    anim_easing: r.anim_easing.clone(),
                })
            })
            .collect();

        RuntimeTheme {
            branch_colors: parse_gradient(&self.branch_colors),
            branch_speed: self.branch_speed,
            branch_easing: self.branch_easing.clone(),

            dir_colors: parse_gradient(&self.dir_colors),
            dir_icon: self.dir_icon.clone(),
            dir_speed: self.dir_speed,
            dir_easing: self.dir_easing.clone(),

            file_colors: parse_gradient(&self.file_colors),
            file_icon: self.file_icon.clone(),
            file_speed: self.file_speed,
            file_easing: self.file_easing.clone(),
            rules,
        }
    }
}

// --- THEME MANAGER ---

pub struct ThemeManager {
    pub dir: PathBuf,
    pub current_ptr: PathBuf,
}

impl ThemeManager {
    pub fn new() -> Self {
        let proj_dirs = ProjectDirs::from("com", "user", "tree-tui").unwrap();
        let dir = proj_dirs.config_dir().join("themes");
        let current_ptr = proj_dirs.config_dir().join("current_theme.txt");

        if !dir.exists() {
            fs::create_dir_all(&dir).unwrap();
        }

        let default_path = dir.join("default.toml");
        if !default_path.exists() {
            let toml_str = toml::to_string_pretty(&Theme::default()).unwrap();
            fs::write(&default_path, toml_str).unwrap();
        }
        if !current_ptr.exists() {
            fs::write(&current_ptr, "default.toml").unwrap();
        }

        Self { dir, current_ptr }
    }

    pub fn list_themes(&self) -> Vec<PathBuf> {
        let mut out = vec![];
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "toml") {
                    out.push(entry.path());
                }
            }
        }
        out.sort();
        out
    }

    pub fn active_theme_name(&self) -> String {
        fs::read_to_string(&self.current_ptr)
            .unwrap_or_else(|_| "default.toml".to_string())
            .trim()
            .to_string()
    }

    pub fn set_active_theme(&self, filename: &str) {
        let _ = fs::write(&self.current_ptr, filename);
    }

    pub fn load_theme(&self, path: &Path) -> Option<Theme> {
        let s = fs::read_to_string(path).ok()?;
        toml::from_str(&s).ok()
    }

    pub fn save_theme(&self, path: &Path, theme: &Theme) {
        if let Ok(s) = toml::to_string_pretty(theme) {
            let _ = fs::write(path, s);
        }
    }
}

// --- EXACT TREE LOGIC & CACHING ---

#[derive(Clone)]
pub struct TreeItem {
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_last: bool,
    pub ancestor_is_last: Vec<bool>,

    pub cached_colors: Vec<Color>,
    pub cached_icon: String,
    pub cached_speed: f32,
    pub cached_easing: Easing,
}

pub fn collect_tree(
    path: &str,
    depth_limit: Option<usize>,
    no_ignore: bool,
    sort_mode: &SortMode,
    max_entries: Option<usize>,
) -> Vec<TreeItem> {
    let mut builder = WalkBuilder::new(path);
    if no_ignore {
        builder.standard_filters(false);
    }
    if let Some(max) = depth_limit {
        builder.max_depth(Some(max));
    }

    // 1. Sort entries inside each directory prior to traversing them
    let mode = sort_mode.clone();
    builder.sort_by_file_path(move |a, b| match mode {
        SortMode::Name => a.file_name().cmp(&b.file_name()),
        SortMode::Extension => {
            let ext_a = a.extension();
            let ext_b = b.extension();
            ext_a
                .cmp(&ext_b)
                .then_with(|| a.file_name().cmp(&b.file_name()))
        }
        SortMode::Size => {
            let size_a = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
            let size_b = std::fs::metadata(b).map(|m| m.len()).unwrap_or(0);
            size_b
                .cmp(&size_a)
                .then_with(|| a.file_name().cmp(&b.file_name()))
        }
        SortMode::Modified => {
            let time_a = std::fs::metadata(a)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let time_b = std::fs::metadata(b)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            time_b
                .cmp(&time_a)
                .then_with(|| a.file_name().cmp(&b.file_name()))
        }
    });

    let entries: Vec<_> = builder.build().filter_map(Result::ok).collect();

    // 2. Filter entries PER DIRECTORY
    let mut filtered_entries = Vec::new();
    let mut child_counts: HashMap<std::path::PathBuf, usize> = HashMap::new();
    let mut omitted_counts: HashMap<std::path::PathBuf, usize> = HashMap::new();

    for entry in entries {
        let depth = entry.depth();
        let path = entry.path().to_path_buf();

        if depth == 0 {
            filtered_entries.push(entry);
            continue;
        }

        if let Some(parent) = path.parent().map(|p| p.to_path_buf()) {
            let is_dir = entry.file_type().map_or(false, |f| f.is_dir());

            if let Some(max) = max_entries {
                // Exonerate folders: Only apply the max_entries limit to files
                if !is_dir {
                    let count = child_counts.entry(parent.clone()).or_insert(0);
                    if *count >= max {
                        *omitted_counts.entry(parent).or_insert(0) += 1;
                        continue;
                    }
                    *count += 1;
                }
            }

            filtered_entries.push(entry);
        } else {
            filtered_entries.push(entry);
        }
    }

    // 3. Map to TreeItems and seamlessly inject "omitted" entries dynamically
    let mut path_is_last = vec![false; 512];
    let mut items = Vec::new();
    let mut stack: Vec<(std::path::PathBuf, usize)> = Vec::new();

    for i in 0..filtered_entries.len() {
        let entry = &filtered_entries[i];
        let depth = entry.depth();
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|f| f.is_dir()).unwrap_or(false);
        let path = entry.path().to_path_buf();
        let parent = path.parent().map(|p| p.to_path_buf());

        // A. If the depth decreases, we are leaving a directory. Pop it and add its omitted text.
        while let Some(&(ref top_path, top_depth)) = stack.last() {
            if depth <= top_depth {
                if let Some(&omit_count) = omitted_counts.get(top_path) {
                    let omit_depth = top_depth + 1;
                    if omit_depth < path_is_last.len() {
                        path_is_last[omit_depth] = true; // The omitted node is ALWAYS the last child
                    }
                    let ancestor_is_last = if omit_depth > 1 {
                        path_is_last[1..omit_depth].to_vec()
                    } else {
                        vec![]
                    };
                    items.push(TreeItem {
                        name: format!("... {} omitted", omit_count),
                        is_dir: false,
                        depth: omit_depth,
                        is_last: true,
                        ancestor_is_last,
                        cached_colors: vec![Color::Rgb(165, 173, 203)], // Muted Subtext color
                        cached_icon: "⋯ ".to_string(),
                        cached_speed: 0.0,
                        cached_easing: Easing::Preset("linear".to_string()),
                    });
                }
                stack.pop();
            } else {
                break;
            }
        }

        // B. Calculate `is_last` for the current regular item
        let mut is_last = true;
        for j in (i + 1)..filtered_entries.len() {
            let next_depth = filtered_entries[j].depth();
            if next_depth < depth {
                break;
            }
            if next_depth == depth {
                is_last = false;
                break;
            }
        }

        // C. Crucial fix: If a parent has omitted files, the CURRENT regular child is NO LONGER the last child.
        if is_last {
            if let Some(p) = &parent {
                if omitted_counts.contains_key(p) {
                    is_last = false;
                }
            }
        }

        if depth < path_is_last.len() {
            path_is_last[depth] = is_last;
        }

        let ancestor_is_last = if depth > 1 {
            path_is_last[1..depth].to_vec()
        } else {
            vec![]
        };

        items.push(TreeItem {
            name,
            is_dir,
            depth,
            is_last,
            ancestor_is_last,
            cached_colors: vec![Color::White],
            cached_icon: String::new(),
            cached_speed: 1.0,
            cached_easing: Easing::Preset("linear".to_string()),
        });

        if is_dir {
            stack.push((path.clone(), depth));
        }
    }

    // 4. Drain any remaining directories in the stack at the end of the file tree
    while let Some((top_path, top_depth)) = stack.pop() {
        if let Some(&omit_count) = omitted_counts.get(&top_path) {
            let omit_depth = top_depth + 1;
            if omit_depth < path_is_last.len() {
                path_is_last[omit_depth] = true;
            }
            let ancestor_is_last = if omit_depth > 1 {
                path_is_last[1..omit_depth].to_vec()
            } else {
                vec![]
            };
            items.push(TreeItem {
                name: format!("... {} omitted", omit_count),
                is_dir: false,
                depth: omit_depth,
                is_last: true,
                ancestor_is_last,
                cached_colors: vec![Color::Rgb(165, 173, 203)],
                cached_icon: "⋯ ".to_string(),
                cached_speed: 0.0,
                cached_easing: Easing::Preset("linear".to_string()),
            });
        }
    }

    items
}

pub fn apply_theme_to_tree(items: &mut [TreeItem], theme: &RuntimeTheme) {
    for item in items.iter_mut() {
        // Skip applying rule colours to our special omission items
        if item.name.starts_with("... ") && item.name.ends_with(" omitted") {
            continue;
        }

        // 1. Assign defaults
        if item.is_dir {
            item.cached_colors = theme.dir_colors.clone();
            item.cached_icon = theme.dir_icon.clone();
            item.cached_speed = theme.dir_speed;
            item.cached_easing = theme.dir_easing.clone();
        } else {
            item.cached_colors = theme.file_colors.clone();
            item.cached_icon = theme.file_icon.clone();
            item.cached_speed = theme.file_speed;
            item.cached_easing = theme.file_easing.clone();
        }

        // 2. Override based on glob match AND target validation
        for r in &theme.rules {
            let target_match = match r.target {
                Target::File => !item.is_dir,
                Target::Dir => item.is_dir,
                Target::Any => true,
            };

            if target_match && r.pattern.matches(&item.name) {
                item.cached_colors = r.colors.clone();
                item.cached_icon = r.icon.clone();
                item.cached_speed = r.anim_speed;
                item.cached_easing = r.anim_easing.clone();
                break;
            }
        }
    }
}

pub fn copy_to_clipboard(items: &[TreeItem]) -> Result<(), String> {
    let mut out = String::new();
    for item in items {
        if item.depth > 0 {
            for &anc_last in &item.ancestor_is_last {
                out.push_str(if anc_last { "    " } else { "│   " });
            }
            out.push_str(if item.is_last {
                "└── "
            } else {
                "├── "
            });
        }
        out.push_str(&item.cached_icon);
        out.push(' ');
        out.push_str(&item.name);
        out.push('\n');
    }

    Clipboard::new()
        .and_then(|mut ctx| ctx.set_text(out))
        .map_err(|e| e.to_string())
}
