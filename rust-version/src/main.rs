//! AdGuard VPN CLI GUI — графическая оболочка для adguardvpn-cli на Linux.
//! Написано на Rust с использованием iced для GUI.

use iced::widget::{
    button, checkbox, column, container, pick_list, radio, row, scrollable, text, text_input,
    Column, Row, Space,
};
use iced::{color, Element, Length, Subscription, Task, Theme};
use regex::Regex;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

const CLI: &str = "adguardvpn-cli";

// ─── ANSI strip ────

fn strip_ansi(input: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(input, "").to_string()
}

// ─── Async CLI helpers ────

async fn run_cli(args: Vec<String>) -> (String, String) {
    match Command::new(CLI)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            (strip_ansi(&stdout), strip_ansi(&stderr))
        }
        Err(e) => ("".into(), format!("Ошибка запуска {CLI}: {e}")),
    }
}

async fn run_cli_config(args: Vec<String>) -> (String, String) {
    let mut full = vec!["config".to_string()];
    full.extend(args);
    run_cli(full).await
}

// ─── Location parsing ────

#[derive(Debug, Clone, PartialEq, Eq)]
struct Location {
    iso: String,
    label: String,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

fn parse_locations(output: &str) -> Vec<Location> {
    let mut locations = Vec::new();
    for raw_line in output.lines() {
        let line = raw_line.trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        // skip header lines
        if parts[0] == "ISO" || parts[0] == "--" || parts[0] == "ESTIMATE" {
            continue;
        }
        // format: ISO  COUNTRY...  CITY...  PING
        if parts.len() >= 3
            && parts[0].len() == 2
            && parts[0].chars().all(|c| c.is_ascii_uppercase())
        {
            let iso = parts[0].to_string();
            let ping = if parts.last().map_or(false, |p| p.chars().all(|c| c.is_ascii_digit())) {
                parts.last().unwrap().to_string()
            } else {
                String::new()
            };
            let middle_end = if ping.is_empty() {
                parts.len()
            } else {
                parts.len() - 1
            };
            let middle = parts[1..middle_end].join(" ");
            let label = if ping.is_empty() {
                format!("{iso}  {middle}")
            } else {
                format!("{iso}  {middle}  ({ping} ms)")
            };
            locations.push(Location { iso, label });
        }
    }
    locations
}

// ─── Tabs ────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Connection,
    Account,
    Settings,
    Exclusions,
    UpdatesLogs,
}

impl Tab {
    const ALL: [Tab; 5] = [
        Tab::Connection,
        Tab::Account,
        Tab::Settings,
        Tab::Exclusions,
        Tab::UpdatesLogs,
    ];

    fn label(&self) -> &str {
        match self {
            Tab::Connection => "🌐 Подключение",
            Tab::Account => "👤 Аккаунт",
            Tab::Settings => "⚙ Настройки",
            Tab::Exclusions => "🚫 Исключения",
            Tab::UpdatesLogs => "🔄 Обновления",
        }
    }
}

// ─── IP Version ────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpVersion {
    Default,
    IPv4,
    IPv6,
}

// ─── Exclusion Mode ────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExclMode {
    General,
    Selective,
}

// ─── VPN Mode / Protocol / Channel / TunRoute for pick_list ────

#[derive(Debug, Clone, PartialEq, Eq)]
struct PickOption(String);

impl std::fmt::Display for PickOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn pick_options(items: &[&str]) -> Vec<PickOption> {
    items.iter().map(|s| PickOption(s.to_string())).collect()
}

// ─── App State ────

struct App {
    active_tab: Tab,
    status_text: String,
    status_bar_text: String,

    // Connection tab
    conn_output: String,
    locations: Vec<Location>,
    selected_location: Option<Location>,
    fastest_location: bool,
    ip_version: IpVersion,
    conn_busy: bool,

    // Account tab
    license_text: String,
    account_output: String,

    // Settings tab
    mode_options: Vec<PickOption>,
    selected_mode: Option<PickOption>,
    proto_options: Vec<PickOption>,
    selected_proto: Option<PickOption>,
    channel_options: Vec<PickOption>,
    selected_channel: Option<PickOption>,
    tun_route_options: Vec<PickOption>,
    selected_tun_route: Option<PickOption>,
    dns_input: String,
    system_dns: bool,
    socks_port: String,
    socks_host: String,
    socks_user: String,
    socks_pass: String,
    flag_reports: bool,
    flag_hints: bool,
    flag_debug: bool,
    flag_notif: bool,
    settings_output: String,

    // Exclusions tab
    excl_mode: ExclMode,
    exclusions: Vec<String>,
    excl_domain_input: String,
    excl_selected: Option<usize>,
    excl_output: String,

    // Updates tab
    updates_output: String,
    log_export_path: String,

    // Timing for auto-refresh
    last_status_refresh: Instant,
}

// ─── Messages ────

#[derive(Debug, Clone)]
enum Message {
    // Tab switching
    SwitchTab(Tab),

    // Connection
    RefreshStatus,
    StatusResult(String, String),
    LoadLocations,
    LocationsResult(String, String),
    SelectLocation(Location),
    ToggleFastest(bool),
    SetIpVersion(IpVersion),
    Connect,
    Disconnect,
    ConnectResult(String, String),
    DisconnectResult(String, String),

    // Account
    Login,
    Logout,
    LogoutResult(String, String),
    RefreshLicense,
    LicenseResult(String, String),

    // Settings
    SelectMode(PickOption),
    SelectProto(PickOption),
    SelectChannel(PickOption),
    SelectTunRoute(PickOption),
    DnsInput(String),
    ToggleSystemDns(bool),
    SocksPortInput(String),
    SocksHostInput(String),
    SocksUserInput(String),
    SocksPassInput(String),
    ToggleReports(bool),
    ToggleHints(bool),
    ToggleDebug(bool),
    ToggleNotif(bool),
    ApplySettings,
    SettingsApplied(String),
    ShowConfig,
    ConfigResult(String, String),
    ClearSocksAuth,
    ClearSocksResult(String, String),

    // Exclusions
    SetExclMode(ExclMode),
    ApplyExclMode,
    ExclModeResult(String, String),
    RefreshExclusions,
    ExclusionsResult(String, String),
    ExclDomainInput(String),
    SelectExclusion(usize),
    AddExclusion,
    RemoveExclusion,
    ClearExclusions,
    ExclActionResult(String, String),

    // Updates
    CheckUpdate,
    DoUpdate,
    UpdateResult(String, String),
    LogPathInput(String),
    ExportLogs,
    ExportLogsResult(String, String),

    // Timer tick
    Tick(Instant),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = Self {
            active_tab: Tab::Connection,
            status_text: "Получение статуса…".into(),
            status_bar_text: "Готов к работе".into(),

            conn_output: String::new(),
            locations: Vec::new(),
            selected_location: None,
            fastest_location: false,
            ip_version: IpVersion::Default,
            conn_busy: false,

            license_text: "Нажмите «Обновить лицензию» для получения данных.".into(),
            account_output: String::new(),

            mode_options: pick_options(&["TUN", "SOCKS"]),
            selected_mode: Some(PickOption("TUN".into())),
            proto_options: pick_options(&["auto", "http2", "quic"]),
            selected_proto: Some(PickOption("auto".into())),
            channel_options: pick_options(&["release", "beta", "nightly"]),
            selected_channel: Some(PickOption("release".into())),
            tun_route_options: pick_options(&["AUTO", "SCRIPT", "NONE"]),
            selected_tun_route: Some(PickOption("AUTO".into())),
            dns_input: String::new(),
            system_dns: false,
            socks_port: "1080".into(),
            socks_host: String::new(),
            socks_user: String::new(),
            socks_pass: String::new(),
            flag_reports: false,
            flag_hints: false,
            flag_debug: false,
            flag_notif: false,
            settings_output: String::new(),

            excl_mode: ExclMode::General,
            exclusions: Vec::new(),
            excl_domain_input: String::new(),
            excl_selected: None,
            excl_output: String::new(),

            updates_output: String::new(),
            log_export_path: "~/adguardvpn_logs.zip".into(),

            last_status_refresh: Instant::now(),
        };

        let tasks = Task::batch([
            Task::perform(run_cli(vec!["status".into()]), |(o, e)| {
                Message::StatusResult(o, e)
            }),
            Task::perform(run_cli(vec!["list-locations".into()]), |(o, e)| {
                Message::LocationsResult(o, e)
            }),
            Task::perform(run_cli(vec!["license".into()]), |(o, e)| {
                Message::LicenseResult(o, e)
            }),
            Task::perform(
                run_cli(vec!["site-exclusions".into(), "show".into()]),
                |(o, e)| Message::ExclusionsResult(o, e),
            ),
        ]);

        (app, tasks)
    }

    fn title_static(_state: &Self) -> String {
        "AdGuard VPN — GUI Manager (Rust + iced)".into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick(Instant::now()))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ─── Tab ────
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                Task::none()
            }

            // ─── Timer ────
            Message::Tick(now) => {
                if now.duration_since(self.last_status_refresh) >= Duration::from_secs(15) {
                    self.last_status_refresh = now;
                    return Task::perform(run_cli(vec!["status".into()]), |(o, e)| {
                        Message::StatusResult(o, e)
                    });
                }
                Task::none()
            }

            // ─── Connection ────
            Message::RefreshStatus => {
                self.last_status_refresh = Instant::now();
                Task::perform(run_cli(vec!["status".into()]), |(o, e)| {
                    Message::StatusResult(o, e)
                })
            }
            Message::StatusResult(out, err) => {
                if !out.is_empty() {
                    self.status_text = out;
                } else if !err.is_empty() {
                    self.status_text = format!("⚠ {err}");
                }
                Task::none()
            }

            Message::LoadLocations => Task::perform(
                run_cli(vec!["list-locations".into()]),
                |(o, e)| Message::LocationsResult(o, e),
            ),
            Message::LocationsResult(out, err) => {
                if !out.is_empty() {
                    self.locations = parse_locations(&out);
                    self.selected_location = self.locations.first().cloned();
                }
                if !err.is_empty() {
                    self.conn_output.push_str(&format!("⚠ {err}\n"));
                }
                Task::none()
            }

            Message::SelectLocation(loc) => {
                self.selected_location = Some(loc);
                Task::none()
            }
            Message::ToggleFastest(v) => {
                self.fastest_location = v;
                Task::none()
            }
            Message::SetIpVersion(v) => {
                self.ip_version = v;
                Task::none()
            }

            Message::Connect => {
                self.conn_busy = true;
                self.status_bar_text = "Подключение…".into();
                let mut args = vec!["connect".to_string()];
                if self.fastest_location {
                    args.push("-f".into());
                } else if let Some(ref loc) = self.selected_location {
                    args.push("-l".into());
                    args.push(loc.iso.clone());
                }
                match self.ip_version {
                    IpVersion::IPv4 => args.push("-4".into()),
                    IpVersion::IPv6 => args.push("-6".into()),
                    IpVersion::Default => {}
                }
                Task::perform(run_cli(args), |(o, e)| Message::ConnectResult(o, e))
            }
            Message::ConnectResult(out, err) => {
                self.conn_busy = false;
                self.status_bar_text = "Готово".into();
                if !out.is_empty() {
                    self.conn_output.push_str(&out);
                    self.conn_output.push('\n');
                }
                if !err.is_empty() {
                    self.conn_output.push_str(&format!("⚠ {err}\n"));
                }
                self.last_status_refresh = Instant::now();
                Task::perform(run_cli(vec!["status".into()]), |(o, e)| {
                    Message::StatusResult(o, e)
                })
            }

            Message::Disconnect => {
                self.conn_busy = true;
                self.status_bar_text = "Отключение…".into();
                Task::perform(run_cli(vec!["disconnect".into()]), |(o, e)| {
                    Message::DisconnectResult(o, e)
                })
            }
            Message::DisconnectResult(out, err) => {
                self.conn_busy = false;
                self.status_bar_text = "Готово".into();
                if !out.is_empty() {
                    self.conn_output.push_str(&out);
                    self.conn_output.push('\n');
                }
                if !err.is_empty() {
                    self.conn_output.push_str(&format!("⚠ {err}\n"));
                }
                self.last_status_refresh = Instant::now();
                Task::perform(run_cli(vec!["status".into()]), |(o, e)| {
                    Message::StatusResult(o, e)
                })
            }

            // ─── Account ────
            Message::Login => {
                self.account_output
                    .push_str("Открываю терминал для авторизации…\n");
                let terminals: Vec<Vec<&str>> = vec![
                    vec!["konsole", "-e", "adguardvpn-cli login"],
                    vec!["kitty", "--", "adguardvpn-cli", "login"],
                    vec!["alacritty", "-e", "adguardvpn-cli", "login"],
                    vec!["gnome-terminal", "--", "adguardvpn-cli", "login"],
                    vec!["xfce4-terminal", "-e", "adguardvpn-cli login"],
                    vec!["xterm", "-e", "adguardvpn-cli", "login"],
                ];
                let mut opened = false;
                for cmd in &terminals {
                    if let Ok(_) = std::process::Command::new(cmd[0])
                        .args(&cmd[1..])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        self.account_output
                            .push_str(&format!("Открыт: {}\nВведите email и пароль AdGuard.\n", cmd[0]));
                        opened = true;
                        break;
                    }
                }
                if !opened {
                    self.account_output.push_str(
                        "⚠ Терминал не найден. Выполните вручную:\n  adguardvpn-cli login\n",
                    );
                }
                Task::none()
            }
            Message::Logout => {
                self.status_bar_text = "Выход…".into();
                Task::perform(run_cli(vec!["logout".into()]), |(o, e)| {
                    Message::LogoutResult(o, e)
                })
            }
            Message::LogoutResult(out, err) => {
                self.status_bar_text = "Готово".into();
                if !out.is_empty() {
                    self.account_output.push_str(&out);
                    self.account_output.push('\n');
                }
                if !err.is_empty() {
                    self.account_output
                        .push_str(&format!("⚠ {err}\n"));
                }
                Task::none()
            }

            Message::RefreshLicense => Task::perform(
                run_cli(vec!["license".into()]),
                |(o, e)| Message::LicenseResult(o, e),
            ),
            Message::LicenseResult(out, err) => {
                if !out.is_empty() {
                    self.license_text = out;
                } else if !err.is_empty() {
                    self.license_text = format!("⚠ {err}");
                }
                Task::none()
            }

            // ─── Settings inputs ────
            Message::SelectMode(v) => {
                self.selected_mode = Some(v);
                Task::none()
            }
            Message::SelectProto(v) => {
                self.selected_proto = Some(v);
                Task::none()
            }
            Message::SelectChannel(v) => {
                self.selected_channel = Some(v);
                Task::none()
            }
            Message::SelectTunRoute(v) => {
                self.selected_tun_route = Some(v);
                Task::none()
            }
            Message::DnsInput(v) => {
                self.dns_input = v;
                Task::none()
            }
            Message::ToggleSystemDns(v) => {
                self.system_dns = v;
                Task::none()
            }
            Message::SocksPortInput(v) => {
                // Allow only digits
                if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                    self.socks_port = v;
                }
                Task::none()
            }
            Message::SocksHostInput(v) => {
                self.socks_host = v;
                Task::none()
            }
            Message::SocksUserInput(v) => {
                self.socks_user = v;
                Task::none()
            }
            Message::SocksPassInput(v) => {
                self.socks_pass = v;
                Task::none()
            }
            Message::ToggleReports(v) => {
                self.flag_reports = v;
                Task::none()
            }
            Message::ToggleHints(v) => {
                self.flag_hints = v;
                Task::none()
            }
            Message::ToggleDebug(v) => {
                self.flag_debug = v;
                Task::none()
            }
            Message::ToggleNotif(v) => {
                self.flag_notif = v;
                Task::none()
            }

            Message::ApplySettings => {
                self.settings_output.clear();
                self.status_bar_text = "Применение настроек…".into();

                let mode = self
                    .selected_mode
                    .as_ref()
                    .map(|m| m.0.clone())
                    .unwrap_or("TUN".into());
                let proto = self
                    .selected_proto
                    .as_ref()
                    .map(|p| p.0.clone())
                    .unwrap_or("auto".into());
                let channel = self
                    .selected_channel
                    .as_ref()
                    .map(|c| c.0.clone())
                    .unwrap_or("release".into());
                let tun_route = self
                    .selected_tun_route
                    .as_ref()
                    .map(|t| t.0.clone())
                    .unwrap_or("AUTO".into());
                let dns = self.dns_input.clone();
                let sys_dns = if self.system_dns { "on" } else { "off" }.to_string();
                let socks_port = self.socks_port.clone();
                let socks_host = self.socks_host.clone();
                let socks_user = self.socks_user.clone();
                let socks_pass = self.socks_pass.clone();
                let reports = if self.flag_reports { "on" } else { "off" }.to_string();
                let hints = if self.flag_hints { "on" } else { "off" }.to_string();
                let debug = if self.flag_debug { "on" } else { "off" }.to_string();
                let notif = if self.flag_notif { "on" } else { "off" }.to_string();

                Task::perform(
                    async move {
                        let mut results = Vec::new();

                        let r = run_cli_config(vec!["set-mode".into(), mode]).await;
                        results.push(r);
                        let r = run_cli_config(vec!["set-protocol".into(), proto]).await;
                        results.push(r);
                        let r = run_cli_config(vec!["set-update-channel".into(), channel]).await;
                        results.push(r);
                        let r =
                            run_cli_config(vec!["set-tun-routing-mode".into(), tun_route]).await;
                        results.push(r);

                        if !dns.is_empty() {
                            let r = run_cli_config(vec!["set-dns".into(), dns]).await;
                            results.push(r);
                        }

                        let r = run_cli_config(vec!["set-system-dns".into(), sys_dns]).await;
                        results.push(r);

                        if !socks_port.is_empty() {
                            let r =
                                run_cli_config(vec!["set-socks-port".into(), socks_port]).await;
                            results.push(r);
                        }
                        if !socks_host.is_empty() {
                            let r =
                                run_cli_config(vec!["set-socks-host".into(), socks_host]).await;
                            results.push(r);
                        }
                        if !socks_user.is_empty() {
                            let r = run_cli_config(vec!["set-socks-username".into(), socks_user])
                                .await;
                            results.push(r);
                        }
                        if !socks_pass.is_empty() {
                            let r = run_cli_config(vec!["set-socks-password".into(), socks_pass])
                                .await;
                            results.push(r);
                        }

                        let r = run_cli_config(vec!["send-reports".into(), reports]).await;
                        results.push(r);
                        let r = run_cli_config(vec!["set-show-hints".into(), hints]).await;
                        results.push(r);
                        let r = run_cli_config(vec!["set-debug-logging".into(), debug]).await;
                        results.push(r);
                        let r = run_cli_config(vec!["set-show-notifications".into(), notif]).await;
                        results.push(r);

                        let mut combined = String::new();
                        for (out, err) in results {
                            if !out.is_empty() {
                                combined.push_str(&out);
                                combined.push('\n');
                            }
                            if !err.is_empty() {
                                combined.push_str(&format!("⚠ {err}\n"));
                            }
                        }
                        combined
                    },
                    Message::SettingsApplied,
                )
            }
            Message::SettingsApplied(result) => {
                self.settings_output = result;
                self.status_bar_text = "Настройки применены".into();
                Task::none()
            }

            Message::ShowConfig => {
                self.settings_output.clear();
                Task::perform(
                    run_cli(vec!["config".into(), "show".into()]),
                    |(o, e)| Message::ConfigResult(o, e),
                )
            }
            Message::ConfigResult(out, err) => {
                self.settings_output = if !out.is_empty() {
                    out
                } else {
                    format!("⚠ {err}")
                };
                Task::none()
            }

            Message::ClearSocksAuth => Task::perform(
                run_cli_config(vec!["clear-socks-auth".into()]),
                |(o, e)| Message::ClearSocksResult(o, e),
            ),
            Message::ClearSocksResult(out, err) => {
                if !out.is_empty() {
                    self.settings_output.push_str(&out);
                    self.settings_output.push('\n');
                }
                if !err.is_empty() {
                    self.settings_output
                        .push_str(&format!("⚠ {err}\n"));
                }
                Task::none()
            }

            // ─── Exclusions ────
            Message::SetExclMode(m) => {
                self.excl_mode = m;
                Task::none()
            }
            Message::ApplyExclMode => {
                let mode = match self.excl_mode {
                    ExclMode::General => "general",
                    ExclMode::Selective => "selective",
                };
                Task::perform(
                    run_cli(vec![
                        "site-exclusions".into(),
                        "mode".into(),
                        mode.into(),
                    ]),
                    |(o, e)| Message::ExclModeResult(o, e),
                )
            }
            Message::ExclModeResult(out, err) => {
                self.excl_output.clear();
                if !out.is_empty() {
                    self.excl_output.push_str(&out);
                    self.excl_output.push('\n');
                }
                if !err.is_empty() {
                    self.excl_output.push_str(&format!("⚠ {err}\n"));
                }
                Task::perform(
                    run_cli(vec!["site-exclusions".into(), "show".into()]),
                    |(o, e)| Message::ExclusionsResult(o, e),
                )
            }

            Message::RefreshExclusions => Task::perform(
                run_cli(vec!["site-exclusions".into(), "show".into()]),
                |(o, e)| Message::ExclusionsResult(o, e),
            ),
            Message::ExclusionsResult(out, _err) => {
                self.exclusions.clear();
                if !out.is_empty() {
                    for line in out.lines() {
                        let l = line.trim();
                        if !l.is_empty() && !l.starts_with("--") {
                            self.exclusions.push(l.to_string());
                        }
                    }
                }
                Task::none()
            }

            Message::ExclDomainInput(v) => {
                self.excl_domain_input = v;
                Task::none()
            }
            Message::SelectExclusion(idx) => {
                self.excl_selected = Some(idx);
                Task::none()
            }

            Message::AddExclusion => {
                let domain = self.excl_domain_input.trim().to_string();
                if domain.is_empty() {
                    return Task::none();
                }
                self.excl_domain_input.clear();
                Task::perform(
                    run_cli(vec!["site-exclusions".into(), "add".into(), domain]),
                    |(o, e)| Message::ExclActionResult(o, e),
                )
            }
            Message::RemoveExclusion => {
                let domain = if !self.excl_domain_input.trim().is_empty() {
                    self.excl_domain_input.trim().to_string()
                } else if let Some(idx) = self.excl_selected {
                    self.exclusions.get(idx).cloned().unwrap_or_default()
                } else {
                    return Task::none();
                };
                if domain.is_empty() {
                    return Task::none();
                }
                self.excl_domain_input.clear();
                Task::perform(
                    run_cli(vec!["site-exclusions".into(), "remove".into(), domain]),
                    |(o, e)| Message::ExclActionResult(o, e),
                )
            }
            Message::ClearExclusions => Task::perform(
                run_cli(vec!["site-exclusions".into(), "clear".into()]),
                |(o, e)| Message::ExclActionResult(o, e),
            ),
            Message::ExclActionResult(out, err) => {
                self.excl_output.clear();
                if !out.is_empty() {
                    self.excl_output.push_str(&out);
                    self.excl_output.push('\n');
                }
                if !err.is_empty() {
                    self.excl_output.push_str(&format!("⚠ {err}\n"));
                }
                Task::perform(
                    run_cli(vec!["site-exclusions".into(), "show".into()]),
                    |(o, e)| Message::ExclusionsResult(o, e),
                )
            }

            // ─── Updates & Logs ────
            Message::CheckUpdate => {
                self.updates_output.clear();
                self.status_bar_text = "Проверка обновлений…".into();
                Task::perform(run_cli(vec!["check-update".into()]), |(o, e)| {
                    Message::UpdateResult(o, e)
                })
            }
            Message::DoUpdate => {
                self.updates_output.clear();
                self.status_bar_text = "Обновление…".into();
                Task::perform(run_cli(vec!["update".into()]), |(o, e)| {
                    Message::UpdateResult(o, e)
                })
            }
            Message::UpdateResult(out, err) => {
                self.status_bar_text = "Готово".into();
                if !out.is_empty() {
                    self.updates_output.push_str(&out);
                    self.updates_output.push('\n');
                }
                if !err.is_empty() {
                    self.updates_output
                        .push_str(&format!("⚠ {err}\n"));
                }
                Task::none()
            }

            Message::LogPathInput(v) => {
                self.log_export_path = v;
                Task::none()
            }
            Message::ExportLogs => {
                let path = self.log_export_path.clone();
                if path.is_empty() {
                    return Task::none();
                }
                // Expand ~ to home
                let expanded = if path.starts_with('~') {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
                    path.replacen('~', &home, 1)
                } else {
                    path
                };
                self.updates_output.clear();
                self.status_bar_text = "Экспорт логов…".into();
                Task::perform(
                    run_cli(vec!["export-logs".into(), "-o".into(), expanded]),
                    |(o, e)| Message::ExportLogsResult(o, e),
                )
            }
            Message::ExportLogsResult(out, err) => {
                self.status_bar_text = "Готово".into();
                if !out.is_empty() {
                    self.updates_output.push_str(&out);
                    self.updates_output.push('\n');
                }
                if !err.is_empty() {
                    self.updates_output
                        .push_str(&format!("⚠ {err}\n"));
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // ─── Tab bar ────
        let tab_bar: Row<Message> = Tab::ALL.iter().fold(
            row![].spacing(2).padding(4),
            |row_acc, tab| {
                let is_active = *tab == self.active_tab;
                let label = text(tab.label()).size(14);
                let btn = button(label)
                    .on_press(Message::SwitchTab(*tab))
                    .padding([8, 18])
                    .style(if is_active {
                        button::primary
                    } else {
                        button::secondary
                    });
                row_acc.push(btn)
            },
        );

        // ─── Content ────
        let content: Element<Message> = match self.active_tab {
            Tab::Connection => self.view_connection(),
            Tab::Account => self.view_account(),
            Tab::Settings => self.view_settings(),
            Tab::Exclusions => self.view_exclusions(),
            Tab::UpdatesLogs => self.view_updates(),
        };

        // ─── Status bar ────
        let status_bar = container(text(&self.status_bar_text).size(12))
            .padding([4, 10])
            .width(Length::Fill)
            .style(container::rounded_box);

        let main_layout = column![
            container(tab_bar).width(Length::Fill).style(container::rounded_box),
            container(scrollable(content).height(Length::Fill))
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill),
            status_bar,
        ]
        .spacing(4);

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(6)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }

    // ─── Connection Tab View ────
    fn view_connection(&self) -> Element<'_, Message> {
        let status_section = column![
            text("Статус VPN").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            text(&self.status_text).size(13),
            row![
                button(text("⚡  Подключиться").size(13))
                    .on_press_maybe(if self.conn_busy {
                        None
                    } else {
                        Some(Message::Connect)
                    })
                    .padding([7, 18])
                    .style(button::success),
                button(text("⏹  Отключиться").size(13))
                    .on_press_maybe(if self.conn_busy {
                        None
                    } else {
                        Some(Message::Disconnect)
                    })
                    .padding([7, 18])
                    .style(button::danger),
                button(text("🔄  Обновить статус").size(13))
                    .on_press(Message::RefreshStatus)
                    .padding([7, 18]),
            ]
            .spacing(8),
        ]
        .spacing(8);

        let location_section = column![
            text("Локация").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                pick_list(
                    self.locations.as_slice(),
                    self.selected_location.as_ref(),
                    Message::SelectLocation,
                )
                .width(Length::Fill)
                .placeholder("Выберите локацию…"),
                button(text("🔄 Обновить").size(13))
                    .on_press(Message::LoadLocations)
                    .padding([7, 14]),
            ]
            .spacing(8),
            checkbox(self.fastest_location).label("Самая быстрая локация (-f)")
                .on_toggle(Message::ToggleFastest),
            row![
                text("IP-версия:").size(13),
                radio("По умолчанию", IpVersion::Default, Some(self.ip_version), Message::SetIpVersion),
                radio("Только IPv4 (-4)", IpVersion::IPv4, Some(self.ip_version), Message::SetIpVersion),
                radio("Только IPv6 (-6)", IpVersion::IPv6, Some(self.ip_version), Message::SetIpVersion),
            ]
            .spacing(12),
        ]
        .spacing(8);

        let output_section = column![
            text("Вывод команд").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(text(&self.conn_output).size(12)).height(150))
                .padding(8)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        column![status_section, location_section, output_section]
            .spacing(16)
            .padding(6)
            .into()
    }

    // ─── Account Tab View ────
    fn view_account(&self) -> Element<'_, Message> {
        let auth_section = column![
            text("Авторизация").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                button(text("🔑  Войти (Login)").size(13))
                    .on_press(Message::Login)
                    .padding([7, 18]),
                button(text("🚪  Выйти (Logout)").size(13))
                    .on_press(Message::Logout)
                    .padding([7, 18]),
                button(text("🔄  Обновить лицензию").size(13))
                    .on_press(Message::RefreshLicense)
                    .padding([7, 18]),
            ]
            .spacing(8),
        ]
        .spacing(8);

        let license_section = column![
            text("Информация о лицензии").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(text(&self.license_text).size(13))
                .padding(10)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        let output_section = column![
            text("Вывод").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(text(&self.account_output).size(12)).height(200))
                .padding(8)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        column![auth_section, license_section, output_section]
            .spacing(16)
            .padding(6)
            .into()
    }

    // ─── Settings Tab View ────
    fn view_settings(&self) -> Element<'_, Message> {
        let main_params = column![
            text("Основные параметры").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                text("Режим:").size(13).width(140),
                pick_list(
                    self.mode_options.as_slice(),
                    self.selected_mode.as_ref(),
                    Message::SelectMode,
                )
                .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Протокол:").size(13).width(140),
                pick_list(
                    self.proto_options.as_slice(),
                    self.selected_proto.as_ref(),
                    Message::SelectProto,
                )
                .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Канал обновлений:").size(13).width(140),
                pick_list(
                    self.channel_options.as_slice(),
                    self.selected_channel.as_ref(),
                    Message::SelectChannel,
                )
                .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("TUN routing mode:").size(13).width(140),
                pick_list(
                    self.tun_route_options.as_slice(),
                    self.selected_tun_route.as_ref(),
                    Message::SelectTunRoute,
                )
                .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);

        let dns_section = column![
            text("DNS").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                text("DNS-сервер:").size(13).width(140),
                text_input("например: 1.1.1.1 или tls://dns.adguard.com", &self.dns_input)
                    .on_input(Message::DnsInput)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            checkbox(self.system_dns).label("Изменять системный DNS").on_toggle(Message::ToggleSystemDns),
        ]
        .spacing(8);

        let socks_section = column![
            text("SOCKS").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                text("Порт:").size(13).width(140),
                text_input("1080", &self.socks_port)
                    .on_input(Message::SocksPortInput)
                    .width(120),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Хост:").size(13).width(140),
                text_input("127.0.0.1", &self.socks_host)
                    .on_input(Message::SocksHostInput)
                    .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Username:").size(13).width(140),
                text_input("", &self.socks_user)
                    .on_input(Message::SocksUserInput)
                    .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                text("Password:").size(13).width(140),
                text_input("", &self.socks_pass)
                    .on_input(Message::SocksPassInput)
                    .secure(true)
                    .width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            button(text("Очистить SOCKS-авторизацию").size(13))
                .on_press(Message::ClearSocksAuth)
                .padding([7, 14]),
        ]
        .spacing(8);

        let flags_section = column![
            text("Флаги").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            checkbox(self.flag_reports).label("Отправка отчётов (send-reports)")
                .on_toggle(Message::ToggleReports),
            checkbox(self.flag_hints).label("Показывать подсказки (show-hints)")
                .on_toggle(Message::ToggleHints),
            checkbox(self.flag_debug).label("Debug-логирование (debug-logging)")
                .on_toggle(Message::ToggleDebug),
            checkbox(self.flag_notif).label("Уведомления (show-notifications)")
                .on_toggle(Message::ToggleNotif),
        ]
        .spacing(6);

        let buttons = row![
            button(text("✅  Применить настройки").size(13))
                .on_press(Message::ApplySettings)
                .padding([7, 18])
                .style(button::success),
            button(text("📋  Показать текущую конфигурацию").size(13))
                .on_press(Message::ShowConfig)
                .padding([7, 18]),
        ]
        .spacing(8);

        let output = column![
            text("Вывод").size(14).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(text(&self.settings_output).size(12)).height(150))
                .padding(8)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        column![
            main_params,
            dns_section,
            socks_section,
            flags_section,
            buttons,
            output,
        ]
        .spacing(14)
        .padding(6)
        .into()
    }

    // ─── Exclusions Tab View ────
    fn view_exclusions(&self) -> Element<'_, Message> {
        let mode_section = column![
            text("Режим исключений").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                radio("General", ExclMode::General, Some(self.excl_mode), Message::SetExclMode),
                radio("Selective", ExclMode::Selective, Some(self.excl_mode), Message::SetExclMode),
                Space::new().width(Length::Fill),
                button(text("Применить режим").size(13))
                    .on_press(Message::ApplyExclMode)
                    .padding([7, 14]),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);

        // Build exclusions list as clickable buttons
        let excl_list: Column<Message> = self
            .exclusions
            .iter()
            .enumerate()
            .fold(column![].spacing(2), |col, (i, domain)| {
                let is_selected = self.excl_selected == Some(i);
                let style = if is_selected {
                    button::primary
                } else {
                    button::secondary
                };
                col.push(
                    button(text(domain).size(12))
                        .on_press(Message::SelectExclusion(i))
                        .width(Length::Fill)
                        .padding([4, 8])
                        .style(style),
                )
            });

        let list_section = column![
            text("Текущие исключения").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(excl_list).height(200))
                .padding(6)
                .width(Length::Fill)
                .style(container::rounded_box),
            button(text("🔄  Обновить список").size(13))
                .on_press(Message::RefreshExclusions)
                .padding([7, 14]),
        ]
        .spacing(6);

        let manage_section = column![
            text("Управление").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                text("Домен:").size(13),
                text_input("example.com", &self.excl_domain_input)
                    .on_input(Message::ExclDomainInput)
                    .width(Length::Fill),
                button(text("➕ Добавить").size(13))
                    .on_press(Message::AddExclusion)
                    .padding([7, 14]),
                button(text("➖ Удалить").size(13))
                    .on_press(Message::RemoveExclusion)
                    .padding([7, 14]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            button(text("🗑  Очистить все").size(13))
                .on_press(Message::ClearExclusions)
                .padding([7, 14])
                .style(button::danger),
        ]
        .spacing(8);

        let output = column![
            text("Вывод").size(14).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(text(&self.excl_output).size(12)).height(120))
                .padding(8)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        column![mode_section, list_section, manage_section, output]
            .spacing(14)
            .padding(6)
            .into()
    }

    // ─── Updates & Logs Tab View ────
    fn view_updates(&self) -> Element<'_, Message> {
        let update_section = column![
            text("Обновления").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                button(text("🔍  Проверить обновления").size(13))
                    .on_press(Message::CheckUpdate)
                    .padding([7, 18]),
                button(text("⬆  Обновить").size(13))
                    .on_press(Message::DoUpdate)
                    .padding([7, 18])
                    .style(button::success),
            ]
            .spacing(8),
        ]
        .spacing(8);

        let logs_section = column![
            text("Логи").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            row![
                text("Путь:").size(13),
                text_input("~/adguardvpn_logs.zip", &self.log_export_path)
                    .on_input(Message::LogPathInput)
                    .width(Length::Fill),
                button(text("📁  Экспортировать логи").size(13))
                    .on_press(Message::ExportLogs)
                    .padding([7, 18]),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8);

        let output = column![
            text("Результат").size(16).style(|_| text::Style {
                color: Some(color!(0x89b4fa)),
            }),
            container(scrollable(text(&self.updates_output).size(12)).height(300))
                .padding(8)
                .width(Length::Fill)
                .style(container::rounded_box),
        ]
        .spacing(6);

        column![update_section, logs_section, output]
            .spacing(16)
            .padding(6)
            .into()
    }
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .title(App::title_static)
        .window_size((860.0, 700.0))
        .run()
}
