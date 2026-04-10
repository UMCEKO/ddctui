use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::{
    collections::HashMap,
    io::stdout,
    process::Command,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Monitor {
    bus: u8,
    name: String,
    model: String,
    controls: Vec<Control>,
}

#[derive(Clone)]
struct Control {
    code: u8,
    name: String,
    current: u16,
    max: u16,
    kind: ControlKind,
}

#[derive(Clone)]
enum ControlKind {
    Continuous,
    NonContinuous { values: Vec<(u16, String)> },
}

fn vcp_name(code: u8) -> &'static str {
    match code {
        0x02 => "New Control Value",
        0x04 => "Restore Factory Defaults",
        0x05 => "Restore Factory Brightness/Contrast",
        0x08 => "Restore Color Defaults",
        0x0B => "Color Temperature Increment",
        0x0C => "Color Temperature Request",
        0x0E => "Clock",
        0x10 => "Brightness",
        0x12 => "Contrast",
        0x14 => "Color Preset",
        0x16 => "Video Gain: Red",
        0x18 => "Video Gain: Green",
        0x1A => "Video Gain: Blue",
        0x1E => "Auto Setup",
        0x20 => "Horizontal Position",
        0x22 => "Horizontal Size",
        0x24 => "Horizontal Pincushion",
        0x26 => "Horizontal Pincushion Balance",
        0x28 => "Horizontal Convergence R/B",
        0x2A => "Horizontal Linearity",
        0x2C => "Horizontal Linearity Balance",
        0x30 => "Vertical Position",
        0x32 => "Vertical Size",
        0x34 => "Vertical Pincushion",
        0x36 => "Vertical Pincushion Balance",
        0x38 => "Vertical Convergence R/B",
        0x3A => "Vertical Linearity",
        0x3C => "Vertical Linearity Balance",
        0x3E => "Clock Phase",
        0x40 => "Horizontal Parallelogram",
        0x42 => "Vertical Parallelogram",
        0x44 => "Horizontal Keystone",
        0x46 => "Vertical Keystone",
        0x48 => "Rotation",
        0x4A => "Top Corner Flare",
        0x4C => "Top Corner Hook",
        0x4E => "Bottom Corner Flare",
        0x50 => "Bottom Corner Hook",
        0x52 => "Active Control",
        0x54 => "Performance Preservation",
        0x56 => "Horizontal Moire",
        0x58 => "Vertical Moire",
        0x5A => "Six Axis Saturation: Red",
        0x5C => "Six Axis Saturation: Yellow",
        0x5E => "Six Axis Saturation: Green",
        0x60 => "Input Source",
        0x62 => "Audio Speaker Volume",
        0x64 => "Audio Microphone Volume",
        0x66 => "Audio Speaker Select",
        0x68 => "Audio Speaker Pair Select",
        0x6C => "Video Black Level: Red",
        0x6E => "Video Black Level: Green",
        0x70 => "Video Black Level: Blue",
        0x72 => "Gamma",
        0x7A => "Adjust Zoom",
        0x7C => "Zoom",
        0x7E => "Trapezoid",
        0x80 => "Keystone",
        0x82 => "Horizontal Mirror",
        0x84 => "Vertical Mirror",
        0x86 => "Display Scaling",
        0x87 => "Sharpness",
        0x88 => "Velocity Scan Modulation",
        0x8A => "Color Saturation",
        0x8C => "TV Sharpness",
        0x8D => "Audio Mute/Screen Blank",
        0x8E => "TV Contrast",
        0x90 => "Hue",
        0x92 => "TV Black Level/Luminance",
        0x94 => "TV Overscan/Underscan",
        0x96 => "Horizontal Convergence M/G",
        0x98 => "Vertical Convergence M/G",
        0x9A => "Window Background",
        0x9B => "Six Axis Hue: Red",
        0x9C => "Six Axis Hue: Yellow",
        0x9D => "Six Axis Hue: Green",
        0x9E => "Six Axis Hue: Cyan",
        0x9F => "Six Axis Hue: Blue",
        0xA0 => "Six Axis Hue: Magenta",
        0xA2 => "Auto Color Setup",
        0xA4 => "Window Control On/Off",
        0xA5 => "Window Select",
        0xAA => "Screen Orientation",
        0xAC => "Horizontal Frequency",
        0xAE => "Vertical Frequency",
        0xB0 => "Settings",
        0xB2 => "Flat Panel Sub-Pixel Layout",
        0xB4 => "Source Timing Mode",
        0xB6 => "Display Technology Type",
        0xC0 => "Display Usage Time",
        0xC6 => "Application Enable Key",
        0xC8 => "Display Controller Type",
        0xC9 => "Display Firmware Level",
        0xCA => "OSD",
        0xCC => "OSD Language",
        0xD4 => "Stereo Video Mode",
        0xD6 => "Power Mode",
        0xDC => "Display Mode",
        0xDE => "Scratch Pad",
        0xDF => "VCP Version",
        _ => "Unknown",
    }
}

fn input_source_name(val: u16) -> &'static str {
    match val {
        0x01 => "VGA-1",
        0x02 => "VGA-2",
        0x03 => "DVI-1",
        0x04 => "DVI-2",
        0x05 => "Composite-1",
        0x06 => "Composite-2",
        0x07 => "S-Video-1",
        0x08 => "S-Video-2",
        0x09 => "Tuner-1",
        0x0A => "Tuner-2",
        0x0B => "Tuner-3",
        0x0C => "Component-1",
        0x0D => "Component-2",
        0x0E => "Component-3",
        0x0F => "DisplayPort-1",
        0x10 => "DisplayPort-2",
        0x11 => "HDMI-1",
        0x12 => "HDMI-2",
        0x13 => "HDMI-3",
        0x14 => "HDMI-4",
        _ => "Unknown",
    }
}

fn color_preset_name(val: u16) -> &'static str {
    match val {
        0x01 => "sRGB",
        0x02 => "Display Native",
        0x03 => "4000K",
        0x04 => "5000K",
        0x05 => "6500K",
        0x06 => "7500K",
        0x07 => "8200K",
        0x08 => "9300K",
        0x09 => "10000K",
        0x0A => "11500K",
        0x0B => "User 1",
        0x0C => "User 2",
        0x0D => "User 3",
        _ => "Unknown",
    }
}

fn power_mode_name(val: u16) -> &'static str {
    match val {
        0x01 => "On",
        0x02 => "Standby",
        0x03 => "Suspend",
        0x04 => "Off (soft)",
        0x05 => "Off (hard)",
        _ => "Unknown",
    }
}

fn osd_language_name(val: u16) -> &'static str {
    match val {
        0x01 => "Chinese (Traditional)",
        0x02 => "English",
        0x03 => "French",
        0x04 => "German",
        0x05 => "Italian",
        0x06 => "Japanese",
        0x07 => "Korean",
        0x08 => "Portuguese",
        0x09 => "Russian",
        0x0A => "Spanish",
        0x0C => "Turkish",
        0x0D => "Chinese (Simplified)",
        0x11 => "Croatian",
        0x12 => "Czech",
        0x14 => "Dutch",
        0x1A => "Hungarian",
        0x1E => "Polish",
        0x1F => "Romanian",
        0x23 => "Thai",
        _ => "Unknown",
    }
}

fn snc_value_label(code: u8, val: u16) -> String {
    let known = match code {
        0x60 => input_source_name(val),
        0x14 => color_preset_name(val),
        0xD6 => power_mode_name(val),
        0xCC => osd_language_name(val),
        _ => "Unknown",
    };
    if known != "Unknown" {
        format!("{known} (0x{val:02x})")
    } else {
        format!("0x{val:02x}")
    }
}

/// Parsed capabilities: map of VCP code -> optional list of allowed values
struct CapabilitiesInfo {
    features: HashMap<u8, Vec<u16>>,
}

fn parse_capabilities(bus: u8) -> Option<CapabilitiesInfo> {
    let output = Command::new("ddcutil")
        .args(["capabilities", "--bus", &bus.to_string()])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut features: HashMap<u8, Vec<u16>> = HashMap::new();
    let mut current_code: Option<u8> = None;
    let mut current_values: Vec<u16> = Vec::new();
    let mut has_any = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Feature: ") {
            // Save previous feature
            if let Some(code) = current_code {
                features.insert(code, std::mem::take(&mut current_values));
            }
            let hex = trimmed
                .strip_prefix("Feature: ")
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            if let Ok(code) = u8::from_str_radix(hex, 16) {
                current_code = Some(code);
                has_any = true;
            } else {
                current_code = None;
            }
        } else if current_code.is_some() && trimmed.starts_with("Values:") {
            // "Values: 00 0A 14 ..." (inline) or just "Values:" (followed by lines)
            let rest = trimmed.strip_prefix("Values:").unwrap_or("").trim();
            if !rest.is_empty() {
                // Inline values like "Values: 00 0A 14 1E 28 (interpretation unavailable)"
                for token in rest.split_whitespace() {
                    // Stop at non-hex tokens like "(interpretation"
                    if token.starts_with('(') {
                        break;
                    }
                    if let Ok(v) = u16::from_str_radix(token, 16) {
                        current_values.push(v);
                    }
                }
            }
        } else if current_code.is_some() && !trimmed.is_empty() && !trimmed.starts_with("Feature:") {
            // Value line: "05: 6500 K" or "0f: DisplayPort-1"
            if let Some(hex) = trimmed.split(':').next() {
                let hex = hex.trim();
                if hex.len() <= 4 {
                    if let Ok(v) = u16::from_str_radix(hex, 16) {
                        current_values.push(v);
                    }
                }
            }
        }
    }

    // Save last feature
    if let Some(code) = current_code {
        features.insert(code, current_values);
    }

    if has_any {
        Some(CapabilitiesInfo { features })
    } else {
        None
    }
}

fn detect_monitors() -> Result<Vec<Monitor>> {
    let output = Command::new("ddcutil")
        .args(["detect", "--terse"])
        .output()
        .context("Failed to run ddcutil. Is it installed?")?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut monitors = Vec::new();

    let mut bus: Option<u8> = None;
    let mut name = String::new();
    let mut model = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("I2C bus:") {
            if let Some(b) = bus {
                if !model.is_empty() {
                    monitors.push((b, std::mem::take(&mut name), std::mem::take(&mut model)));
                }
            }
            bus = trimmed
                .split("/dev/i2c-")
                .nth(1)
                .and_then(|b| b.trim().parse().ok());
        } else if trimmed.starts_with("Monitor:") {
            let info = trimmed.strip_prefix("Monitor:").unwrap_or("").trim();
            let parts: Vec<&str> = info.splitn(3, ':').collect();
            if parts.len() >= 2 {
                name = parts[0].trim().to_string();
                model = parts[1].trim().to_string();
            }
        } else if trimmed.starts_with("Mfg id:") {
            name = trimmed
                .strip_prefix("Mfg id:")
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    if let Some(b) = bus {
        if !model.is_empty() {
            monitors.push((b, name, model));
        }
    }

    let mut result = Vec::new();
    for (i, (b, n, m)) in monitors.iter().enumerate() {
        eprintln!(
            "  [{}/{}] Probing {} {} (bus {})...",
            i + 1,
            monitors.len(),
            n,
            m,
            b
        );
        let controls = discover_controls(*b);
        result.push(Monitor {
            bus: *b,
            name: n.clone(),
            model: m.clone(),
            controls,
        });
    }

    Ok(result)
}

fn discover_controls(bus: u8) -> Vec<Control> {
    let caps = parse_capabilities(bus);

    let codes_to_probe: Vec<u8> = if let Some(ref caps) = caps {
        caps.features.keys().copied().collect()
    } else {
        eprintln!("    Capabilities parse failed, probing common VCP codes...");
        vec![
            0x10, 0x12, 0x14, 0x16, 0x18, 0x1A, 0x60, 0x62, 0x6C, 0x6E, 0x70, 0x87, 0x8A, 0x8D,
            0x90, 0x92, 0xD6, 0xDC, 0xCC,
        ]
    };

    let mut sorted = codes_to_probe;
    sorted.sort();

    let mut controls = Vec::new();
    for code in sorted {
        let allowed_values = caps
            .as_ref()
            .and_then(|c| c.features.get(&code))
            .cloned()
            .unwrap_or_default();

        if let Some(ctrl) = read_vcp(bus, code, &allowed_values) {
            controls.push(ctrl);
        }
    }
    controls
}

fn read_vcp(bus: u8, code: u8, allowed_values: &[u16]) -> Option<Control> {
    let output = Command::new("ddcutil")
        .args([
            "getvcp",
            &format!("0x{code:02x}"),
            "--bus",
            &bus.to_string(),
            "--terse",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split_whitespace().collect();

    if parts.len() < 4 || parts[0] != "VCP" {
        return None;
    }

    let name = vcp_name(code).to_string();
    let vcp_type = parts[2];

    match vcp_type {
        // Continuous: VCP <code> C <current> <max>
        "C" if parts.len() >= 5 => {
            let current: u16 = parts[3].parse().ok()?;
            let max: u16 = parts[4].parse().ok()?;
            if max == 0 {
                return None;
            }
            Some(Control {
                code,
                name,
                current,
                max,
                kind: ControlKind::Continuous,
            })
        }
        // Simple Non-Continuous: VCP <code> SNC x<val>
        "SNC" => {
            let val_str = parts[3].strip_prefix('x').unwrap_or(parts[3]);
            let current = u16::from_str_radix(val_str, 16).ok()?;
            let values = build_snc_values(code, current, allowed_values);
            let max = values.iter().map(|(v, _)| *v).max().unwrap_or(current);

            Some(Control {
                code,
                name,
                current,
                max,
                kind: ControlKind::NonContinuous { values },
            })
        }
        // Complex Non-Continuous: VCP <code> CNC x<mh> x<ml> x<sh> x<sl>
        // Current value is sl (last byte), max is ml (second byte)
        "CNC" if parts.len() >= 5 => {
            // Format: VCP <code> CNC x<max_hi> x<max_lo> x<cur_hi> x<cur_lo>
            // In practice for most features: CNC x00 x0b x00 x05
            // means max=0x0b, current=0x05
            let parse_hex = |s: &str| -> Option<u16> {
                let s = s.strip_prefix('x').unwrap_or(s);
                u16::from_str_radix(s, 16).ok()
            };

            // CNC has 4 values after type: max_hi, max_lo, cur_hi, cur_lo
            if parts.len() >= 7 {
                let max = parse_hex(parts[4])?;
                let current = parse_hex(parts[6])?;
                let values = build_snc_values(code, current, allowed_values);
                let max = if !values.is_empty() {
                    values.iter().map(|(v, _)| *v).max().unwrap_or(max)
                } else {
                    max
                };
                Some(Control {
                    code,
                    name,
                    current,
                    max,
                    kind: ControlKind::NonContinuous { values },
                })
            } else {
                // Fewer parts — try simpler parse
                let current_str = parts.last()?;
                let current = parse_hex(current_str)?;
                let values = build_snc_values(code, current, allowed_values);
                let max = values.iter().map(|(v, _)| *v).max().unwrap_or(current);
                Some(Control {
                    code,
                    name,
                    current,
                    max,
                    kind: ControlKind::NonContinuous { values },
                })
            }
        }
        _ => None,
    }
}

fn build_snc_values(code: u8, current: u16, allowed_values: &[u16]) -> Vec<(u16, String)> {
    let mut values: Vec<(u16, String)> = if !allowed_values.is_empty() {
        allowed_values
            .iter()
            .map(|&v| (v, snc_value_label(code, v)))
            .collect()
    } else {
        // No capabilities data — include current value only
        vec![(current, snc_value_label(code, current))]
    };

    // Ensure current value is in the list
    if !values.iter().any(|(v, _)| *v == current) {
        values.push((current, snc_value_label(code, current)));
    }

    values.sort_by_key(|(v, _)| *v);
    values.dedup_by_key(|(v, _)| *v);
    values
}

fn set_vcp(bus: u8, code: u8, value: u16) -> Result<()> {
    Command::new("ddcutil")
        .args([
            "setvcp",
            &format!("0x{code:02x}"),
            &value.to_string(),
            "--bus",
            &bus.to_string(),
        ])
        .output()
        .context("Failed to set VCP value")?;
    Ok(())
}

struct PendingWrite {
    bus: u8,
    code: u8,
    value: u16,
    deadline: Instant,
}

struct App {
    monitors: Vec<Monitor>,
    selected_monitor: usize,
    selected_control: usize,
    scroll_offset: usize,
    quit: bool,
    pending: Option<PendingWrite>,
}

const DEBOUNCE: Duration = Duration::from_millis(500);

impl App {
    fn new(monitors: Vec<Monitor>) -> Self {
        Self {
            monitors,
            selected_monitor: 0,
            selected_control: 0,
            scroll_offset: 0,
            quit: false,
            pending: None,
        }
    }

    fn current_monitor(&self) -> &Monitor {
        &self.monitors[self.selected_monitor]
    }

    fn flush_pending(&mut self) {
        if let Some(p) = self.pending.take() {
            let _ = set_vcp(p.bus, p.code, p.value);
        }
    }

    fn tick(&mut self) {
        if let Some(ref p) = self.pending {
            if Instant::now() >= p.deadline {
                self.flush_pending();
            }
        }
    }

    fn adjust(&mut self, delta: i16) {
        let mon = &mut self.monitors[self.selected_monitor];
        if let Some(ctrl) = mon.controls.get_mut(self.selected_control) {
            let new_val = match &ctrl.kind {
                ControlKind::Continuous => {
                    (ctrl.current as i16 + delta).clamp(0, ctrl.max as i16) as u16
                }
                ControlKind::NonContinuous { values } => {
                    if values.len() <= 1 {
                        return;
                    }
                    let cur_idx = values
                        .iter()
                        .position(|(v, _)| *v == ctrl.current)
                        .unwrap_or(0);
                    let new_idx = if delta > 0 {
                        (cur_idx + 1).min(values.len() - 1)
                    } else if cur_idx > 0 {
                        cur_idx - 1
                    } else {
                        0
                    };
                    values[new_idx].0
                }
            };

            if new_val != ctrl.current {
                ctrl.current = new_val;
                if let Some(ref p) = self.pending {
                    if p.bus != mon.bus || p.code != ctrl.code {
                        let bus = p.bus;
                        let code = p.code;
                        let value = p.value;
                        self.pending = None;
                        let _ = set_vcp(bus, code, value);
                    }
                }
                self.pending = Some(PendingWrite {
                    bus: mon.bus,
                    code: ctrl.code,
                    value: new_val,
                    deadline: Instant::now() + DEBOUNCE,
                });
            }
        }
    }

    fn ensure_visible(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        if self.selected_control < self.scroll_offset {
            self.scroll_offset = self.selected_control;
        } else if self.selected_control >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected_control - visible_rows + 1;
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.flush_pending();
                self.quit = true;
            }
            KeyCode::Tab => {
                if !self.monitors.is_empty() {
                    self.flush_pending();
                    self.selected_monitor = (self.selected_monitor + 1) % self.monitors.len();
                    self.selected_control = 0;
                    self.scroll_offset = 0;
                }
            }
            KeyCode::BackTab => {
                if !self.monitors.is_empty() {
                    self.flush_pending();
                    self.selected_monitor = if self.selected_monitor == 0 {
                        self.monitors.len() - 1
                    } else {
                        self.selected_monitor - 1
                    };
                    self.selected_control = 0;
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.flush_pending();
                let len = self.current_monitor().controls.len();
                if len > 0 {
                    self.selected_control = if self.selected_control == 0 {
                        len - 1
                    } else {
                        self.selected_control - 1
                    };
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.flush_pending();
                let len = self.current_monitor().controls.len();
                if len > 0 {
                    self.selected_control = (self.selected_control + 1) % len;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => self.adjust(1),
            KeyCode::Left | KeyCode::Char('h') => self.adjust(-1),
            KeyCode::Char('+') | KeyCode::Char('=') => self.adjust(5),
            KeyCode::Char('-') => self.adjust(-5),
            _ => {}
        }
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Monitor tabs
    let tab_titles: Vec<String> = app
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let marker = if i == app.selected_monitor {
                "▸ "
            } else {
                "  "
            };
            format!("{marker}{} {}", m.name, m.model)
        })
        .collect();

    let tabs = Paragraph::new(tab_titles.join("  │  ")).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Monitors (Tab to switch) "),
    );
    frame.render_widget(tabs, outer[0]);

    // Controls
    let mon = app.current_monitor().clone();
    let controls_area = outer[1];

    if mon.controls.is_empty() {
        let msg = Paragraph::new("No DDC/CI controls detected on this monitor.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", mon.model)),
        );
        frame.render_widget(msg, controls_area);
    } else {
        let row_height = 3u16;
        let visible_rows = (controls_area.height / row_height) as usize;
        app.ensure_visible(visible_rows);

        let visible_controls: Vec<(usize, &Control)> = mon
            .controls
            .iter()
            .enumerate()
            .skip(app.scroll_offset)
            .take(visible_rows)
            .collect();

        let constraints: Vec<Constraint> = visible_controls
            .iter()
            .map(|_| Constraint::Length(row_height))
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let control_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(controls_area);

        for (vi, (gi, ctrl)) in visible_controls.iter().enumerate() {
            let selected = *gi == app.selected_control;

            let border_style = if selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title = format!(
                " {} {} [0x{:02X}] ",
                if selected { "▸" } else { " " },
                ctrl.name,
                ctrl.code,
            );

            match &ctrl.kind {
                ControlKind::Continuous => {
                    let ratio = ctrl.current as f64 / ctrl.max as f64;
                    let label = Span::styled(
                        format!("{}/{}", ctrl.current, ctrl.max),
                        Style::default().fg(Color::Black).bold(),
                    );

                    let bar_color = if selected {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    };

                    let gauge = Gauge::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(border_style)
                                .title(title),
                        )
                        .gauge_style(Style::default().fg(bar_color).bg(Color::Black))
                        .ratio(ratio.clamp(0.0, 1.0))
                        .label(label);

                    frame.render_widget(gauge, control_area[vi]);
                }
                ControlKind::NonContinuous { values } => {
                    let current_label = values
                        .iter()
                        .find(|(v, _)| *v == ctrl.current)
                        .map(|(_, l)| l.as_str())
                        .unwrap_or("?");

                    let text = if values.len() > 1 {
                        format!("◀ {} ▶", current_label)
                    } else {
                        current_label.to_string()
                    };

                    let style = if selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let para = Paragraph::new(text).style(style).centered().block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border_style)
                            .title(title),
                    );

                    frame.render_widget(para, control_area[vi]);
                }
            }
        }

        // Scrollbar if needed
        if mon.controls.len() > visible_rows {
            let mut scrollbar_state =
                ScrollbarState::new(mon.controls.len()).position(app.scroll_offset);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                controls_area,
                &mut scrollbar_state,
            );
        }
    }

    // Help bar
    let help =
        Paragraph::new(" ←/→ adjust  +/- jump 5  ↑/↓ select  Tab switch monitor  q quit")
            .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, outer[2]);
}

fn main() -> Result<()> {
    eprintln!("Detecting monitors...");
    let monitors = detect_monitors()?;

    if monitors.is_empty() {
        eprintln!("No monitors detected. Make sure ddcutil is installed and you have permissions.");
        eprintln!("Try: sudo ddcutil detect");
        return Ok(());
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App::new(monitors);

    while !app.quit {
        terminal.draw(|f| ui(f, &mut app))?;

        app.tick();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
