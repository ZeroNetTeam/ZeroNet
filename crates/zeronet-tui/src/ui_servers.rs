//! Drawing of the "My server" page (state and keys are in `servers`).
//!
//! One function per screen, each returning the lines of a single panel, plus
//! the QR panel the result and server screens share. Everything is plain
//! words: no jargon a person who has only just rented a server would not know.

use crate::qr::{self, QrStyle};
use crate::servers::{
    copy_text, DnsStatus, Field, Form, ProgressView, Screen, ServersState, StepState,
};
use crate::ui::{truncate, UiRenderer};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;
use zero_provision::install::{Note, Preset, Step};
use zero_provision::manage::InboundUsage;
use zero_provision::record::{ServerRecord, Service};

/// `1.4 MB`, `820 B`.
fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// What a note means, in a sentence.
pub fn note_text(note: &Note) -> String {
    match note {
        Note::PortMoved { wanted, used, held_by } => format!(
            "Port {wanted} was already used by {held_by}, so port {used} is used instead."
        ),
        Note::FirewallOpened { which } => {
            format!("The ports were opened in the server's firewall ({which}).")
        }
        Note::ProviderFirewall { tcp, udp } => {
            let list = |ports: &[u16], kind: &str| {
                (!ports.is_empty()).then(|| {
                    let p: Vec<String> = ports.iter().map(u16::to_string).collect();
                    format!("{kind} {}", p.join(", "))
                })
            };
            let parts: Vec<String> = [list(tcp, "TCP"), list(udp, "UDP")]
                .into_iter()
                .flatten()
                .collect();
            format!(
                "Your hosting provider may have its own firewall. If nothing connects, open {} in its panel.",
                parts.join(" and ")
            )
        }
        Note::Unverified => {
            "The server program was uploaded from this app and was not checked against the signed list.".into()
        }
        Note::KeyNotWorking => {
            "The app could not log back in with its own key, so it will ask for the password next time.".into()
        }
    }
}

/// The plain name of an install step.
fn step_name(step: Step, preset: Option<Preset>) -> String {
    let what = match preset {
        Some(Preset::Reality) => " (REALITY)",
        Some(Preset::Xhttp) => " (XHTTP)",
        Some(Preset::MasterDns) => " (MasterDNS)",
        None => "",
    };
    let base = match step {
        Step::Check => "Checking the server",
        Step::Packages => "Getting what it needs",
        Step::Download => "Downloading and verifying",
        Step::Install => "Installing",
        Step::Configure => "Writing the settings",
        Step::Start => "Starting",
        Step::Firewall => "Opening the ports",
        Step::Access => "Saving this app's access",
    };
    format!("{base}{what}")
}

impl UiRenderer<'_> {
    pub(crate) fn render_servers(&mut self, frame: &mut Frame, area: Rect) {
        let state = self.servers;
        let (title, lines, keys, with_qr) = match &state.screen {
            Screen::List => ("My servers", self.list_lines(state), "a add a server   ↑↓ choose   Enter open", false),
            Screen::Busy(text) => ("Please wait", vec![Line::raw(text.clone())], "Esc cancel", false),
            Screen::Form(form) => (
                "Add a server",
                self.form_lines(form),
                "Tab next   Space tick   Ctrl+V paste   Ctrl+S connect   Esc back",
                false,
            ),
            Screen::Identity { checked, key, .. } => (
                "Is this your server?",
                self.identity_lines(&checked.address, &key.fingerprint),
                "Enter yes, it matches   Esc back",
                false,
            ),
            Screen::Dns { records, status, checked, .. } => (
                "Point your domain at the server",
                self.dns_lines(records, status, &checked.address, checked.domain.as_deref().unwrap_or("")),
                "1-4 copy   c check   Enter install   Esc back",
                false,
            ),
            Screen::Progress(view) => (
                "Setting up",
                self.progress_lines(view),
                if view.failed.is_some() { "Enter go back" } else { "This takes a minute or two" },
                false,
            ),
            Screen::Result { record, notes, shown } => (
                "Your server is ready",
                self.result_lines(record, notes, *shown),
                "←→ other config   c copy   o big code   s save files   Enter open server page",
                true,
            ),
            Screen::Server { id, shown } => match state.record(id) {
                Some(record) => (
                    "Server",
                    self.server_lines(state, record, *shown),
                    "←→ config   c copy   o big code   s save   r refresh   t test   d remove   Esc back",
                    true,
                ),
                None => ("Server", vec![Line::raw("This server is gone.")], "Esc back", false),
            },
            Screen::ConfirmRemove { id, typed } => (
                "Remove everything?",
                self.confirm_lines(state, id, typed),
                "Enter confirm   Esc cancel",
                false,
            ),
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme.border))
            .title(Span::styled(
                format!(" {title} "),
                Style::default()
                    .fg(self.theme.accent_bright)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(rows[0]);
        frame.render_widget(block, rows[0]);

        let code = if with_qr {
            self.current_qr(state)
        } else {
            None
        };
        match &code {
            // The big code: on its own, so it is large enough to scan.
            Some(c) if state.show_qr => {
                let w = (c.width_cells as u16).min(inner.width);
                let at = Rect {
                    x: inner.x + inner.width.saturating_sub(w) / 2,
                    y: inner.y,
                    width: w,
                    height: (c.height_cells as u16).min(inner.height),
                };
                if inner.width < c.width_cells as u16 || inner.height < c.height_cells as u16 {
                    frame.render_widget(
                        Paragraph::new("The window is too small for the code. Make it bigger, or copy the link with c.")
                            .wrap(Wrap { trim: false }),
                        inner,
                    );
                } else {
                    self.draw_qr(frame, at, c);
                }
            }
            _ => {
                let (text_area, qr_area) = match &code {
                    Some(c)
                        if inner.width >= c.width_cells as u16 + 40
                            && inner.height >= c.height_cells as u16 =>
                    {
                        let cols = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Min(30),
                                Constraint::Length(c.width_cells as u16 + 2),
                            ])
                            .split(inner);
                        (cols[0], Some(cols[1]))
                    }
                    _ => (inner, None),
                };
                frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), text_area);
                if let (Some(code), Some(area)) = (&code, qr_area) {
                    self.draw_qr(frame, area, code);
                }
            }
        }

        let hint = match &state.notice {
            Some(notice) if matches!(state.screen, Screen::List | Screen::Server { .. }) => {
                Line::styled(
                    truncate(notice, area.width as usize),
                    Style::default().fg(self.theme.info),
                )
            }
            _ => Line::styled(
                truncate(keys, area.width as usize),
                Style::default().fg(self.theme.muted),
            ),
        };
        frame.render_widget(Paragraph::new(hint), rows[1]);
    }

    fn draw_qr(&self, frame: &mut Frame, area: Rect, code: &qr::RenderedQr) {
        let lines: Vec<Line> = code
            .lines
            .iter()
            .map(|l| Line::styled(l.as_str(), Style::default().fg(self.theme.text)))
            .collect();
        frame.render_widget(Paragraph::new(lines), area);
    }

    /// The QR of the config being shown, when it has a link.
    fn current_qr(&self, state: &ServersState) -> Option<qr::RenderedQr> {
        let (record, shown) = match &state.screen {
            Screen::Result { record, shown, .. } => (&**record, *shown),
            Screen::Server { id, shown } => (state.record(id)?, *shown),
            _ => return None,
        };
        let link = record.client_configs().get(shown)?.link.clone()?;
        qr::render(&link, QrStyle::HalfBlock).ok()
    }

    fn dim(&self, text: impl Into<String>) -> Line<'static> {
        Line::styled(text.into(), Style::default().fg(self.theme.muted))
    }

    fn strong(&self, text: impl Into<String>) -> Line<'static> {
        Line::styled(
            text.into(),
            Style::default()
                .fg(self.theme.accent_bright)
                .add_modifier(Modifier::BOLD),
        )
    }

    fn list_lines(&self, state: &ServersState) -> Vec<Line<'static>> {
        if state.records.is_empty() {
            return vec![
                self.strong("Run your own server in a minute"),
                Line::raw(""),
                Line::raw("Rent a small server (a VPS), then press  a  and type its address and password."),
                Line::raw("Zray installs and starts everything for you, then shows the configs to use."),
                Line::raw(""),
                self.dim("It also shows how many devices are connected, as a count only:"),
                self.dim("no addresses or anything that points at a person is ever shown or stored."),
            ];
        }
        let mut lines = vec![self.dim("Your servers")];
        for (i, record) in state.records.iter().enumerate() {
            let live = state
                .usage
                .get(&record.id)
                .map(|(u, _)| format!("{} connected", u.devices_now()))
                .unwrap_or_default();
            let services: Vec<&str> = record.services.iter().map(Service::title).collect();
            let text = format!(
                "{} {}   {}   {}   {}",
                if i == state.selected { "▌" } else { " " },
                record.name,
                record.host,
                services.join(" + "),
                live
            );
            let style = if i == state.selected {
                Style::default()
                    .fg(self.theme.accent_bright)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.text)
            };
            lines.push(Line::styled(text, style));
        }
        lines
    }

    fn form_lines(&self, form: &Form) -> Vec<Line<'static>> {
        let field = |label: &str, value: String, focus: Field| {
            let focused = form.focus == focus;
            let caret = if focused { "▏" } else { "" };
            Line::from(vec![
                Span::styled(
                    format!("{} {:<16}", if focused { "▌" } else { " " }, label),
                    Style::default().fg(if focused {
                        self.theme.accent_bright
                    } else {
                        self.theme.muted
                    }),
                ),
                Span::styled(
                    format!("{value}{caret}"),
                    Style::default().fg(self.theme.text),
                ),
            ])
        };
        let tick = |label: &str, on: bool, focus: Field| {
            let focused = form.focus == focus;
            Line::styled(
                format!(
                    "{} [{}] {label}",
                    if focused { "▌" } else { " " },
                    if on { "x" } else { " " }
                ),
                Style::default().fg(if focused {
                    self.theme.accent_bright
                } else {
                    self.theme.text
                }),
            )
        };
        let mut lines = vec![
            field("Name", form.name.clone(), Field::Name),
            field("Server address", form.address.clone(), Field::Address),
            field("SSH port", form.port.clone(), Field::SshPort),
            field("Login user", form.user.clone(), Field::User),
            field(
                if form.key_file.trim().is_empty() {
                    "Password"
                } else {
                    "Key passphrase"
                },
                "•".repeat(form.password.chars().count()),
                Field::Password,
            ),
            field(
                "SSH key file",
                form.key_file.clone(),
                Field::KeyFile,
            ),
            self.dim("    Instead of a password: the path of your private key (not the .pub). Used once, never saved."),
            Line::raw(""),
            self.dim("What to install"),
            tick(
                "VLESS + REALITY   (fast, looks like a normal website)",
                form.reality,
                Field::Reality,
            ),
            tick(
                "VLESS + XHTTP     (REALITY too; a different traffic shape, own port)",
                form.xhttp,
                Field::Xhttp,
            ),
            tick(
                "MasterDnsVPN      (works through DNS only; needs your own domain)",
                form.masterdns,
                Field::MasterDns,
            ),
        ];
        if form.masterdns {
            lines.push(field("Tunnel domain", form.domain.clone(), Field::Domain));
            lines.push(self.dim("    A subdomain you own, like  t.example.com  (the next screens show what to set)."));
        }
        lines.push(Line::raw(""));
        let connect_focused = form.focus == Field::Connect;
        lines.push(Line::styled(
            format!(
                "{} Connect and install",
                if connect_focused { "▌" } else { " " }
            ),
            Style::default()
                .fg(if connect_focused {
                    self.theme.ok
                } else {
                    self.theme.muted
                })
                .add_modifier(Modifier::BOLD),
        ));
        if let Some(error) = &form.error {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                error.clone(),
                Style::default().fg(self.theme.err),
            ));
        }
        lines.push(Line::raw(""));
        lines.push(self.dim("The password is used once to log in and is not kept: the app puts its own key on the server."));
        lines
    }

    fn identity_lines(&self, address: &str, fingerprint: &str) -> Vec<Line<'static>> {
        vec![
            Line::raw(format!("Connected to {address}. Its identity is:")),
            Line::raw(""),
            self.strong(fingerprint.to_string()),
            Line::raw(""),
            Line::raw("Compare it with the one in your hosting panel (many show it in the server's page or console)."),
            Line::raw("If it is the same, press Enter. If it is different or you cannot check, press Esc: someone may be between you and the server."),
            Line::raw(""),
            self.dim("Nothing secret has been sent yet."),
        ]
    }

    fn dns_lines(
        &self,
        records: &[zero_provision::guide::DnsRecord],
        status: &DnsStatus,
        address: &str,
        domain: &str,
    ) -> Vec<Line<'static>> {
        let mut lines = vec![
            self.strong("Do this once in Cloudflare (about two minutes)"),
            Line::raw("1. Open dash.cloudflare.com, pick your domain, then  DNS  >  Records  >  Add record."),
            Line::raw("2. Add these two records. Keep the cloud GREY (\"DNS only\"), never orange:"),
            Line::raw(""),
        ];
        for (i, r) in records.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {}  ", i + 1),
                    Style::default().fg(self.theme.muted),
                ),
                Span::styled(
                    format!(
                        "Type {:<3} Name {:<10} {}",
                        r.kind,
                        r.name,
                        if r.kind == "NS" {
                            "Nameserver"
                        } else {
                            "Value"
                        }
                    ),
                    Style::default().fg(self.theme.text),
                ),
                Span::styled(
                    format!(" {}", r.value),
                    Style::default().fg(self.theme.accent_bright),
                ),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(self.dim(format!(
            "Keys: 1 copy first name, 2 its value, 3 second name, 4 its value.   Domain: {domain}"
        )));
        if address.parse::<std::net::IpAddr>().is_err() {
            lines.push(Line::styled(
                "You typed a name, not an IP: use the server's IP address as the value of the first record.",
                Style::default().fg(self.theme.warn),
            ));
        }
        lines.push(Line::raw("3. Press  c  to check. It can take a few minutes to spread; you can also install now and test later."));
        lines.push(Line::raw(""));
        let (text, color) = match status {
            DnsStatus::NotChecked => ("Not checked yet.", self.theme.muted),
            DnsStatus::Checking => ("Checking…", self.theme.info),
            DnsStatus::Right => ("The record points at your server. Good.", self.theme.ok),
            DnsStatus::Wrong => (
                "The name resolves, but not to this server. Check the first record's value.",
                self.theme.warn,
            ),
            DnsStatus::Unknown => (
                "Not visible yet. If you just added it, wait a few minutes and check again.",
                self.theme.warn,
            ),
        };
        lines.push(Line::styled(text, Style::default().fg(color)));
        lines.push(Line::raw(""));
        lines.push(self.dim("Not on Cloudflare? Any registrar works: add the same two records."));
        lines
    }

    fn progress_lines(&self, view: &ProgressView) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        for (step, preset, state) in &view.steps {
            let (mark, color) = match state {
                StepState::Done => ("✓", self.theme.ok),
                StepState::Running => ("…", self.theme.accent_bright),
                StepState::Waiting => (" ", self.theme.muted),
            };
            lines.push(Line::styled(
                format!(" {mark} {}", step_name(*step, *preset)),
                Style::default().fg(color),
            ));
        }
        if let Some(failed) = &view.failed {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "It stopped:",
                Style::default()
                    .fg(self.theme.err)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::styled(
                failed.clone(),
                Style::default().fg(self.theme.err),
            ));
            lines.push(Line::raw(""));
            lines.push(
                self.dim("Nothing half-finished is left running. Fix the cause and try again."),
            );
        } else if !view.detail.is_empty() {
            lines.push(Line::raw(""));
            lines.push(self.dim(view.detail.clone()));
        }
        lines
    }

    fn config_lines(&self, record: &ServerRecord, shown: usize) -> Vec<Line<'static>> {
        let configs = record.client_configs();
        let mut lines = Vec::new();
        let Some(config) = configs.get(shown) else {
            return lines;
        };
        lines.push(self.strong(format!(
            "Config {} of {}: {} ({})",
            shown + 1,
            configs.len(),
            config.user,
            config.service
        )));
        if let Some(link) = &config.link {
            lines.push(Line::raw(link.clone()));
            lines.push(self.dim("Scan the code with the Zray app, or press c to copy the link."));
        }
        if !config.files.is_empty() {
            let names: Vec<&str> = config.files.iter().map(|f| f.name.as_str()).collect();
            lines.push(Line::raw(format!("Files: {}", names.join(", "))));
            lines.push(self.dim("Press s to save them, then give them to the MasterDNS client."));
        }
        lines
    }

    fn result_lines(
        &self,
        record: &ServerRecord,
        notes: &[Note],
        shown: usize,
    ) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::styled(
                format!("{} ({}) is running.", record.name, record.host),
                Style::default()
                    .fg(self.theme.ok)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
        ];
        lines.extend(self.config_lines(record, shown));
        for note in notes {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                note_text(note),
                Style::default().fg(self.theme.warn),
            ));
        }
        lines
    }

    fn usage_lines(&self, state: &ServersState, record: &ServerRecord) -> Vec<Line<'static>> {
        let mut lines = vec![self.strong("Live use")];
        let Some((usage, at)) = state.usage.get(&record.id) else {
            lines.push(match state.usage_error.get(&record.id) {
                Some(error) => Line::styled(error.clone(), Style::default().fg(self.theme.warn)),
                None => self.dim("Reading…"),
            });
            return lines;
        };
        for service in &record.services {
            let row: Option<&InboundUsage> = usage.inbounds.iter().find(|i| i.tag == service.tag());
            let (now, recent, sessions, traffic) = match row {
                Some(r) => (
                    r.devices_now.to_string(),
                    r.devices_recent.to_string(),
                    r.sessions.to_string(),
                    format!(
                        "up {}  down {}",
                        human_bytes(r.uplink),
                        human_bytes(r.downlink)
                    ),
                ),
                None => ("0".into(), "0".into(), "0".into(), String::new()),
            };
            lines.push(Line::styled(
                service.title(),
                Style::default()
                    .fg(self.theme.text)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::raw(format!(
                "  connected now {now}   in the last {} min {recent}   open sessions {sessions}",
                usage.window_secs / 60
            )));
            if !traffic.is_empty() {
                lines.push(self.dim(format!("  traffic since start: {traffic}")));
            } else if matches!(service, Service::MasterDns(_)) {
                lines.push(self.dim(
                    "  counted from the server's log: up to 5 minutes behind, no traffic figure.",
                ));
            }
        }
        if let Some(error) = state.usage_error.get(&record.id) {
            lines.push(Line::styled(
                error.clone(),
                Style::default().fg(self.theme.warn),
            ));
        }
        lines.push(self.dim(format!(
            "updated {} s ago. Only counts are shown: no addresses are kept or sent.",
            at.elapsed().as_secs()
        )));
        lines
    }

    fn server_lines(
        &self,
        state: &ServersState,
        record: &ServerRecord,
        shown: usize,
    ) -> Vec<Line<'static>> {
        let mut lines = vec![
            self.strong(format!("{}   {}", record.name, record.host)),
            self.dim(format!(
                "{}   Zray server {}",
                record.os,
                if record.zray_version == "local build" {
                    "(uploaded from this app)"
                } else {
                    &record.zray_version
                }
            )),
            Line::raw(""),
        ];
        lines.extend(self.usage_lines(state, record));
        lines.push(Line::raw(""));
        lines.extend(self.config_lines(record, shown));
        if copy_text(record, shown).is_none() {
            lines.push(self.dim("This config has no link."));
        }
        lines
    }

    fn confirm_lines(&self, state: &ServersState, id: &str, typed: &str) -> Vec<Line<'static>> {
        let name = state.record(id).map_or("this server", |r| r.name.as_str());
        vec![
            Line::styled(
                format!("This takes Zray off {name}."),
                Style::default().fg(self.theme.warn).add_modifier(Modifier::BOLD),
            ),
            Line::raw("It stops and deletes the server program and its settings, closes the ports it opened,"),
            Line::raw("and removes this app's key from the server. Everything else on the server stays."),
            Line::raw("Configs you gave to people stop working."),
            Line::raw(""),
            Line::raw(format!("Type  {}  and press Enter:  {typed}▏", crate::servers::CONFIRM_WORD)),
        ]
    }
}
