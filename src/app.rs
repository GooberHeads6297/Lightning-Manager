use eframe::egui;

use crate::minecraft;

struct Palette {
    bg: egui::Color32,
    card: egui::Color32,
    card_alt: egui::Color32,
    accent: egui::Color32,
    selected: egui::Color32,
    text: egui::Color32,
    text_secondary: egui::Color32,
    text_muted: egui::Color32,
    border: egui::Color32,
    danger: egui::Color32,
    danger_bg: egui::Color32,
    success: egui::Color32,
    success_bg: egui::Color32,
    warning: egui::Color32,
    warning_bg: egui::Color32,
    heading: egui::Color32,
}



#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Mods,
    Profiles,
    Launcher,
    AppData,
}

enum ConfirmAction {
    DeleteSelected,
    ClearAll,
}

pub struct ModManagerApp {
    current_tab: Tab,
    dark_mode: bool,
    show_settings: bool,
    mods: Vec<minecraft::ModEntry>,
    selected_mods: std::collections::HashSet<String>,
    mods_error: Option<String>,
    delete_status: Option<String>,
    profiles: minecraft::LauncherProfiles,
    profiles_error: Option<String>,
    profiles_saved: Option<String>,
    selected_profile_key: Option<String>,
    detected_launcher_path: Option<String>,
    custom_launcher_path: String,
    launch_status: Option<String>,
    launch_error: Option<String>,
    minecraft_path: String,
    mods_path: String,
    show_confirm: bool,
    confirm_action: Option<ConfirmAction>,
    backup_enabled: bool,
    last_backup: Option<String>,
    profile_launch_status: Option<String>,
    clear_backups_result: Option<String>,
    logging_enabled: bool,
}

impl Default for ModManagerApp {
    fn default() -> Self {
        let mc_path = minecraft::minecraft_dir().to_string_lossy().to_string();
        let md_path = minecraft::mods_dir().to_string_lossy().to_string();
        let detected_path =
            minecraft::find_launcher().map(|p| p.to_string_lossy().to_string());
        let config = minecraft::load_config();

        let mut app = Self {
            current_tab: Tab::Mods,
            dark_mode: true,
            show_settings: false,
            mods: Vec::new(),
            selected_mods: std::collections::HashSet::new(),
            mods_error: None,
            delete_status: None,
            profiles: minecraft::LauncherProfiles {
                profiles: std::collections::HashMap::new(),
                selectedProfile: None,
            },
            profiles_error: None,
            profiles_saved: None,
            selected_profile_key: None,
            detected_launcher_path: detected_path,
            custom_launcher_path: config.custom_launcher_path,
            launch_status: None,
            launch_error: None,
            minecraft_path: mc_path,
            mods_path: md_path,
            show_confirm: false,
            confirm_action: None,
            backup_enabled: true,
            last_backup: None,
            profile_launch_status: None,
            clear_backups_result: None,
            logging_enabled: config.logging_enabled,
        };
        app.refresh_mods();
        app.refresh_profiles();
        app
    }
}

impl ModManagerApp {
    fn palette(&self) -> &'static Palette {
        if self.dark_mode { &DARK } else { &LIGHT }
    }

    fn refresh_mods(&mut self) {
        self.mods_error = None;
        match minecraft::list_mods() {
            Ok(mods) => {
                self.mods = mods;
                self.selected_mods
                    .retain(|name| self.mods.iter().any(|m| m.name == *name));
            }
            Err(e) => {
                self.mods_error = Some(format!("Failed to list mods: {e}"));
                self.mods.clear();
            }
        }
    }

    fn refresh_profiles(&mut self) {
        self.profiles_error = None;
        match minecraft::read_profiles() {
            Ok(p) => {
                self.profiles = p;
                if self.selected_profile_key.is_none()
                    || !self.profiles.profiles.contains_key(
                        self.selected_profile_key.as_deref().unwrap_or(""),
                    )
                {
                    self.selected_profile_key = self
                        .profiles
                        .profiles
                        .keys()
                        .next()
                        .cloned();
                }
            }
            Err(e) => self.profiles_error = Some(e),
        }
    }

    fn persist_config(&self) {
        let config = minecraft::AppConfig {
            custom_launcher_path: self.custom_launcher_path.clone(),
            logging_enabled: self.logging_enabled,
        };
        minecraft::save_config(&config);
    }

    fn do_delete(&mut self) {
        let to_delete: Vec<_> = match &self.confirm_action {
            Some(ConfirmAction::DeleteSelected) => self
                .mods
                .iter()
                .filter(|m| self.selected_mods.contains(&m.name))
                .cloned()
                .collect(),
            Some(ConfirmAction::ClearAll) => self.mods.clone(),
            None => return,
        };

        if to_delete.is_empty() {
            self.delete_status = Some("No mods to delete.".to_string());
            self.show_confirm = false;
            self.confirm_action = None;
            return;
        }

        if self.backup_enabled {
            match minecraft::backup_mods(&to_delete) {
                Ok(path) => {
                    self.last_backup = Some(path);
                }
                Err(e) => {
                    self.delete_status = Some(format!("Backup failed: {e}"));
                    self.show_confirm = false;
                    self.confirm_action = None;
                    return;
                }
            }
        }

        let count = to_delete.len();
        let mut failed = Vec::new();
        for mod_entry in &to_delete {
            if let Err(e) = minecraft::delete_mod(&mod_entry.path) {
                failed.push(format!("{}: {e}", mod_entry.name));
            }
        }

        if failed.is_empty() {
            self.delete_status = Some(format!(
                "Deleted {count} mod(s).{}",
                if self.backup_enabled {
                    " Backed up before deletion."
                } else {
                    ""
                }
            ));
        } else {
            self.delete_status = Some(format!(
                "Deleted {} of {count} mod(s). Errors: {}",
                count - failed.len(),
                failed.join("; ")
            ));
        }

        self.selected_mods.clear();
        self.show_confirm = false;
        self.confirm_action = None;
        self.refresh_mods();
    }

    fn save_profiles(&mut self) {
        self.profiles_saved = None;
        match minecraft::save_profiles(&self.profiles) {
            Ok(()) => {
                self.profiles_saved = Some("Profiles saved successfully.".to_string());
            }
            Err(e) => self.profiles_error = Some(e),
        }
    }

    fn launcher_path_to_use(&self) -> String {
        if !self.custom_launcher_path.is_empty() {
            self.custom_launcher_path.clone()
        } else {
            self.detected_launcher_path.clone().unwrap_or_default()
        }
    }

    fn section_frame(p: &Palette) -> egui::Frame {
        egui::Frame::default()
            .fill(p.card)
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(16, 12))
            .stroke(egui::Stroke::new(1.0, p.border))
    }

    fn section_header(ui: &mut egui::Ui, p: &Palette, text: &str) {
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(text)
                    .size(14.0)
                    .color(p.heading),
            );
        });
        ui.add_space(8.0);
    }

    fn show_banner(ui: &mut egui::Ui, text: &str, bg: egui::Color32, fg: egui::Color32) {
        egui::Frame::default()
            .inner_margin(egui::Margin::symmetric(14, 8))
            .corner_radius(6)
            .fill(bg)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(text).color(fg));
            });
    }

    fn show_settings_window(&mut self, ctx: &egui::Context) {
        let p = self.palette();
        if self.show_settings {
            let mut window_open = true;
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .open(&mut window_open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                Self::section_frame(p).show(ui, |ui| {
                    Self::section_header(ui, p, "Theme");
                    ui.horizontal(|ui| {
                        let icon = if self.dark_mode { "☽ Dark" } else { "☀ Light" };
                        if ui.button(icon).clicked() {
                            self.dark_mode = !self.dark_mode;
                        }
                    });
                });

                ui.add_space(8.0);

                Self::section_frame(p).show(ui, |ui| {
                    Self::section_header(ui, p, "Launcher");
                    let path = self.launcher_path_to_use();
                    ui.label(egui::RichText::new(
                        if path.is_empty() { "None set" } else { &path },
                    ).color(p.text_secondary).size(12.0));
                });

                ui.add_space(8.0);

                Self::section_frame(p).show(ui, |ui| {
                    Self::section_header(ui, p, "Backups");
                    if ui.button("Clear All Backups").clicked() {
                        match minecraft::clear_backups() {
                            Ok(n) => self.clear_backups_result = Some(format!("Removed {n} backup folder(s).")),
                            Err(e) => self.clear_backups_result = Some(format!("Error: {e}")),
                        }
                    }
                    if let Some(result) = &self.clear_backups_result {
                        ui.add_space(4.0);
                        if result.starts_with("Error") {
                            Self::show_banner(ui, result, p.danger_bg, p.danger);
                        } else {
                            Self::show_banner(ui, result, p.success_bg, p.success);
                        }
                    }
                });

                ui.add_space(8.0);

                Self::section_frame(p).show(ui, |ui| {
                    Self::section_header(ui, p, "Logging");
                    let prev = self.logging_enabled;
                    ui.checkbox(&mut self.logging_enabled, "Write log files (launch logs, crash reports)");
                    if prev != self.logging_enabled {
                        self.persist_config();
                    }
                    if !self.logging_enabled {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Launch logs and crash reports will not be saved to disk.")
                                .size(11.0)
                                .color(p.text_muted),
                        );
                    }
                });

                ui.add_space(8.0);

                Self::section_frame(p).show(ui, |ui| {
                    Self::section_header(ui, p, "Debug Info");
                    let info = vec![
                        format!("Version: Release v1.0"),
                        format!("Dark Mode: {}", self.dark_mode),
                        format!("Profiles: {}", self.profiles.profiles.len()),
                        format!("Mods: {}", self.mods.len()),
                        format!("Minecraft: {}", self.minecraft_path),
                        format!("Mods Dir: {}", self.mods_path),
                        format!("Backups: {}", minecraft::backup_dir().to_string_lossy()),
                        format!("Launcher: {}", self.launcher_path_to_use()),
                        format!("Selected Profile: {}", self.selected_profile_key.as_deref().unwrap_or("none")),
                    ];
                    for line in &info {
                        ui.label(egui::RichText::new(line).size(11.0).color(p.text_secondary));
                    }
                });
            });
            self.show_settings = window_open;
        }
    }

    fn show_mods_tab(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        let has_mods = !self.mods.is_empty();

        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Mods").size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} mod(s)", self.mods.len()))
                        .color(p.text_secondary),
                );
            });
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let del_btn = egui::Button::new(
                egui::RichText::new("Delete Selected").color(p.danger),
            )
            .fill(p.danger_bg)
            .min_size(egui::vec2(0.0, 28.0));
            if ui.add_enabled(has_mods && !self.selected_mods.is_empty(), del_btn).clicked() {
                self.show_confirm = true;
                self.confirm_action = Some(ConfirmAction::DeleteSelected);
            }

            let clear_btn = egui::Button::new(
                egui::RichText::new("Clear All").color(p.danger),
            )
            .fill(p.danger_bg)
            .min_size(egui::vec2(0.0, 28.0));
            if ui.add_enabled(has_mods, clear_btn).clicked() {
                self.show_confirm = true;
                self.confirm_action = Some(ConfirmAction::ClearAll);
            }

            if ui.button("Open Mods Folder").clicked() {
                let _ = minecraft::open_mods_folder();
            }
            if ui.button("Refresh").clicked() {
                self.refresh_mods();
            }
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.backup_enabled, "Backup mods before deletion");
            if let Some(last) = &self.last_backup {
                if ui.button("Open Backup").clicked() {
                    let _ = minecraft::open_folder(&std::path::PathBuf::from(last));
                }
            }
        });

        if let Some(status) = &self.delete_status.clone() {
            ui.add_space(4.0);
            Self::show_banner(ui, status, p.success_bg, p.success);
        }

        if let Some(error) = &self.mods_error {
            ui.add_space(4.0);
            Self::show_banner(ui, error, p.danger_bg, p.danger);
        }

        ui.add_space(6.0);

        if self.mods.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("No mods found")
                        .size(16.0)
                        .color(p.text_secondary),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&self.mods_path)
                        .color(p.text_muted)
                        .size(12.0),
                );
                ui.add_space(8.0);
                if ui.button("Open Mods Folder").clicked() {
                    let _ = minecraft::open_mods_folder();
                }
            });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (i, mod_entry) in self.mods.clone().iter().enumerate() {
                        let bg = if i % 2 == 0 { p.card } else { p.card_alt };
                        egui::Frame::default()
                            .fill(bg)
                            .corner_radius(6)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let is_selected =
                                        self.selected_mods.contains(&mod_entry.name);
                                    let mut checked = is_selected;
                                    ui.checkbox(&mut checked, "");
                                    if checked != is_selected {
                                        if checked {
                                            self.selected_mods
                                                .insert(mod_entry.name.clone());
                                        } else {
                                            self.selected_mods
                                                .remove(&mod_entry.name);
                                        }
                                    }
                                    ui.label(
                                        egui::RichText::new(&mod_entry.name).size(13.0),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(
                                            egui::Align::Center,
                                        ),
                                        |ui| {
                                            let size_kb = mod_entry.size / 1024;
                                            let color = if size_kb > 5000 {
                                                p.warning
                                            } else {
                                                p.text_secondary
                                            };
                                            ui.label(
                                                egui::RichText::new(format!("{size_kb} KB"))
                                                    .color(color)
                                                    .size(11.0),
                                            );
                                        },
                                    );
                                });
                            });
                        ui.add_space(2.0);
                    }
                });
        }
    }

    fn show_profiles_tab(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        if let Some(error) = &self.profiles_error.clone() {
            ui.add_space(4.0);
            Self::show_banner(ui, error, p.danger_bg, p.danger);
            return;
        }

        if let Some(saved) = &self.profiles_saved.clone() {
            ui.add_space(4.0);
            Self::show_banner(ui, saved, p.success_bg, p.success);
        }

        if let Some(status) = &self.profile_launch_status.clone() {
            ui.add_space(4.0);
            if status.starts_with("Error") || status.starts_with("Failed") {
                Self::show_banner(ui, status, p.danger_bg, p.danger);
            } else {
                Self::show_banner(ui, status, p.success_bg, p.success);
            }
        }

        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Profiles").size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save Changes").clicked() {
                    self.save_profiles();
                }
                if ui.button("Refresh").clicked() {
                    self.refresh_profiles();
                }
            });
        });
        ui.add_space(4.0);

        if self.profiles.profiles.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("No profiles found")
                        .size(16.0)
                        .color(p.text_secondary),
                );
                ui.add_space(4.0);
                ui.label("Launch Minecraft at least once to create profiles.");
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Profile").color(p.text_secondary),
            );
            let selected_label = match &self.selected_profile_key {
                Some(k) => match self.profiles.profiles.get(k) {
                    Some(prof) if !prof.name.is_empty() => prof.name.as_str(),
                    _ => k.as_str(),
                },
                None => "Select...",
            };
            egui::ComboBox::from_id_salt("profile_selector")
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for key in self.profiles.profiles.keys() {
                        let display = self
                            .profiles
                            .profiles
                            .get(key)
                            .map(|p| {
                                if p.name.is_empty() {
                                    key.clone()
                                } else {
                                    p.name.clone()
                                }
                            })
                            .unwrap_or_else(|| key.clone());
                        let is_selected =
                            self.selected_profile_key.as_deref() == Some(key);
                        if ui
                            .selectable_label(is_selected, &display)
                            .clicked()
                        {
                            self.selected_profile_key = Some(key.clone());
                        }
                    }
                });
        });

        ui.add_space(6.0);

        let selected_key = self.selected_profile_key.clone();
        if let Some(ref key) = selected_key {
            let profile_opt = self.profiles.profiles.get(key).cloned();
            if let Some(mut edited) = profile_opt {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        Self::section_frame(p).show(ui, |ui| {
                            Self::section_header(ui, p, "Profile Info");
                            ui.label(
                                egui::RichText::new(format!("ID: {key}"))
                                    .color(p.text_muted)
                                    .size(11.0),
                            );
                            ui.add_space(6.0);

                            let mut display_name = edited.name.clone();
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Name")
                                        .color(p.heading),
                                );
                                ui.text_edit_singleline(&mut display_name);
                            });
                            edited.name = display_name;
                        });

                        ui.add_space(8.0);

                        Self::section_frame(p).show(ui, |ui| {
                            Self::section_header(ui, p, "Game");

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Version ID")
                                        .color(p.heading),
                                );
                                let mut val =
                                    edited.lastVersionId.clone().unwrap_or_default();
                                let resp = ui
                                    .add(
                                        egui::TextEdit::singleline(&mut val)
                                            .hint_text("Not set"),
                                    );
                                if resp.changed() {
                                    edited.lastVersionId =
                                        if val.is_empty() { None } else { Some(val) };
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Game Directory")
                                        .color(p.heading),
                                );
                                let mut val =
                                    edited.gameDir.clone().unwrap_or_default();
                                let resp = ui
                                    .add(
                                        egui::TextEdit::singleline(&mut val)
                                            .hint_text("Not set (uses default)"),
                                    );
                                if resp.changed() {
                                    edited.gameDir =
                                        if val.is_empty() { None } else { Some(val) };
                                }
                            });
                        });

                        ui.add_space(8.0);

                        Self::section_frame(p).show(ui, |ui| {
                            Self::section_header(ui, p, "Java");

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Executable")
                                        .color(p.heading),
                                );
                                let mut val =
                                    edited.javaDir.clone().unwrap_or_default();
                                let resp = ui
                                    .add(
                                        egui::TextEdit::singleline(&mut val)
                                            .hint_text("Not set (uses bundled Java)"),
                                    );
                                if resp.changed() {
                                    edited.javaDir =
                                        if val.is_empty() { None } else { Some(val) };
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Arguments")
                                        .color(p.heading),
                                );
                                let mut val =
                                    edited.javaArgs.clone().unwrap_or_default();
                                let resp = ui.add_sized(
                                    egui::vec2(ui.available_width().max(100.0), 60.0),
                                    egui::TextEdit::multiline(&mut val)
                                        .desired_rows(3),
                                );
                                if resp.changed() {
                                    edited.javaArgs =
                                        if val.is_empty() { None } else { Some(val) };
                                }
                            });
                        });

                        ui.add_space(8.0);

                        Self::section_frame(p).show(ui, |ui| {
                            Self::section_header(ui, p, "Appearance");
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Icon")
                                        .color(p.heading),
                                );
                                let mut val =
                                    edited.icon.clone().unwrap_or_default();
                                if ui.text_edit_singleline(&mut val).changed() {
                                    edited.icon =
                                        if val.is_empty() { None } else { Some(val) };
                                }
                            });
                        });

                        ui.add_space(8.0);

                        Self::section_frame(p).show(ui, |ui| {
                            Self::section_header(ui, p, "Launch");
                            ui.label(
                                egui::RichText::new(
                                    "Launches this profile directly via Java, bypassing the Minecraft Launcher."
                                ).size(12.0).color(p.text_muted),
                            );
                            ui.add_space(4.0);
                            if ui.button("Launch Directly").clicked() {
                                if let Some(version_id) = &edited.lastVersionId {
                                    match minecraft::read_version_manifest(version_id) {
                                        Ok(manifest) => {
                                            match minecraft::launch_profile_direct(&edited, &manifest, self.logging_enabled) {
                                                Ok(()) => {
                                                    self.profile_launch_status = Some(
                                                        "Minecraft launched successfully.".to_string()
                                                    );
                                                }
                                                Err(e) => {
                                                    self.profile_launch_status = Some(format!("Error: {e}"));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.profile_launch_status = Some(format!("Error: {e}"));
                                        }
                                    }
                                } else {
                                    self.profile_launch_status = Some(
                                        "Error: No version ID set for this profile.".to_string()
                                    );
                                }
                            }
                        });

                        if let Some(edit_key) = &self.selected_profile_key {
                            self.profiles
                                .profiles
                                .insert(edit_key.clone(), edited);
                        }
                    });
            }
        }
    }

    fn show_launcher_tab(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Launcher").size(20.0).strong());
        });
        ui.add_space(4.0);

        let launch_clicked = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("  Launch Minecraft")
                        .size(16.0)
                        .color(egui::Color32::WHITE),
                )
                .min_size(egui::vec2(240.0, 42.0))
                .fill(p.accent),
            )
            .clicked();

        if launch_clicked {
            self.launch_error = None;
            self.launch_status = None;
            let path = self.launcher_path_to_use();
            match minecraft::launch_minecraft_with_path(&path) {
                Ok(()) => {
                    self.launch_status =
                        Some("Launcher started successfully.".to_string());
                }
                Err(e) => {
                    self.launch_error = Some(e);
                }
            }
        }

        ui.add_space(8.0);

        if let Some(status) = &self.launch_status.clone() {
            Self::show_banner(ui, status, p.success_bg, p.success);
            ui.add_space(6.0);
        }

        if let Some(error) = &self.launch_error.clone() {
            Self::show_banner(ui, error, p.danger_bg, p.danger);
            ui.add_space(6.0);
        }

        Self::section_frame(p).show(ui, |ui| {
            Self::section_header(ui, p, "Launcher Location");

            let has_custom = !self.custom_launcher_path.is_empty();
            if let Some(path) = &self.detected_launcher_path {
                if has_custom {
                    ui.label(
                        egui::RichText::new("Detected (overridden by custom path):")
                            .size(12.0)
                            .color(p.text_muted),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(path)
                            .size(12.0)
                            .color(p.text_secondary),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Detected automatically:")
                            .size(12.0)
                            .color(p.text_muted),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(path)
                            .size(12.0)
                            .color(p.success),
                    );
                }
            } else if !has_custom {
                egui::Frame::default()
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .corner_radius(6)
                    .fill(p.warning_bg)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Minecraft Launcher not found.")
                                .color(p.warning)
                                .size(13.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Set a custom path below to tell the app where your launcher is installed.",
                            )
                            .size(12.0)
                            .color(p.text_secondary),
                        );
                    });

                ui.add_space(6.0);
                if ui.button("Scan Again").clicked() {
                    self.detected_launcher_path = minecraft::find_launcher()
                        .map(|p| p.to_string_lossy().to_string());
                }
            }
        });

        ui.add_space(8.0);

        let path_changed = self.custom_launcher_path.clone();
        Self::section_frame(p).show(ui, |ui| {
            Self::section_header(ui, p, "Custom Path");

            ui.label(
                egui::RichText::new(
                    "Point to your Minecraft Launcher executable if installed in a custom location.",
                )
                .size(11.0)
                .color(p.text_muted),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.custom_launcher_path)
                        .hint_text("C:\\Path\\To\\MinecraftLauncher.exe")
                        .desired_width(ui.available_width() - 120.0),
                );
                if ui.button("Browse").clicked() {
                    if let Some(path) = minecraft::pick_file_dialog("Select Minecraft Launcher", "Executable", &["exe"]) {
                        self.custom_launcher_path = path;
                        self.persist_config();
                    }
                }
                if !self.custom_launcher_path.is_empty()
                    && ui.button("Clear").clicked()
                {
                    self.custom_launcher_path.clear();
                    self.persist_config();
                }
            });

            if self.custom_launcher_path != path_changed {
                self.persist_config();
            }

            if !self.custom_launcher_path.is_empty() {
                let exists = std::path::Path::new(&self.custom_launcher_path).exists();
                ui.add_space(4.0);
                if exists {
                    ui.label(
                        egui::RichText::new("File exists")
                            .color(p.success)
                            .size(12.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("File not found")
                            .color(p.danger)
                            .size(12.0),
                    );
                }
            }
        });
    }

    fn show_appdata_tab(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("AppData").size(20.0).strong());
        });
        ui.add_space(4.0);

        Self::section_frame(p).show(ui, |ui| {
            Self::section_header(ui, p, "Minecraft Directory");

            ui.label(
                egui::RichText::new(&self.minecraft_path)
                    .size(13.0)
                    .color(p.text),
            );
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Open .minecraft").size(14.0),
                        )
                        .min_size(egui::vec2(180.0, 36.0)),
                    )
                    .clicked()
                {
                    let _ = minecraft::open_minecraft_folder();
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Open Mods").size(14.0),
                        )
                        .min_size(egui::vec2(180.0, 36.0)),
                    )
                    .clicked()
                {
                    let _ = minecraft::open_mods_folder();
                }
            });
        });

        ui.add_space(8.0);

        Self::section_frame(p).show(ui, |ui| {
            Self::section_header(ui, p, "Backups");

            let backup_path = minecraft::backup_dir().to_string_lossy().to_string();
            ui.label(
                egui::RichText::new(&backup_path)
                    .size(12.0)
                    .color(p.success),
            );
            ui.add_space(8.0);

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Open Backup Folder").size(14.0),
                    )
                    .min_size(egui::vec2(200.0, 34.0)),
                )
                .clicked()
            {
                let _ = minecraft::open_folder(&minecraft::backup_dir());
            }
        });
    }
}

static DARK: Palette = Palette {
    bg: egui::Color32::from_rgb(18, 18, 22),
    card: egui::Color32::from_rgb(30, 30, 38),
    card_alt: egui::Color32::from_rgb(26, 26, 34),
    accent: egui::Color32::from_rgb(108, 123, 255),
    selected: egui::Color32::from_rgb(38, 40, 52),
    text: egui::Color32::from_rgb(232, 232, 237),
    text_secondary: egui::Color32::from_rgb(154, 154, 168),
    text_muted: egui::Color32::from_rgb(106, 106, 120),
    border: egui::Color32::from_rgb(46, 46, 58),
    danger: egui::Color32::from_rgb(224, 72, 72),
    danger_bg: egui::Color32::from_rgb(58, 26, 30),
    success: egui::Color32::from_rgb(80, 200, 120),
    success_bg: egui::Color32::from_rgb(26, 58, 36),
    warning: egui::Color32::from_rgb(232, 168, 72),
    warning_bg: egui::Color32::from_rgb(58, 42, 20),
    heading: egui::Color32::from_rgb(180, 200, 255),
};

static LIGHT: Palette = Palette {
    bg: egui::Color32::from_rgb(240, 240, 245),
    card: egui::Color32::from_rgb(255, 255, 255),
    card_alt: egui::Color32::from_rgb(245, 245, 250),
    accent: egui::Color32::from_rgb(80, 95, 220),
    selected: egui::Color32::from_rgb(220, 222, 240),
    text: egui::Color32::from_rgb(26, 26, 36),
    text_secondary: egui::Color32::from_rgb(100, 100, 118),
    text_muted: egui::Color32::from_rgb(140, 140, 160),
    border: egui::Color32::from_rgb(210, 210, 220),
    danger: egui::Color32::from_rgb(200, 50, 50),
    danger_bg: egui::Color32::from_rgb(255, 230, 232),
    success: egui::Color32::from_rgb(50, 150, 80),
    success_bg: egui::Color32::from_rgb(230, 250, 236),
    warning: egui::Color32::from_rgb(190, 130, 30),
    warning_bg: egui::Color32::from_rgb(255, 246, 224),
    heading: egui::Color32::from_rgb(50, 60, 160),
};

impl eframe::App for ModManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = self.palette();

        ctx.set_visuals(egui::Visuals {
            window_corner_radius: 10.0.into(),
            window_fill: p.bg,
            panel_fill: p.bg,
            faint_bg_color: p.card_alt,
            extreme_bg_color: p.card,
            code_bg_color: p.card,
            warn_fg_color: p.warning,
            error_fg_color: p.danger,
            hyperlink_color: p.accent,
            selection: egui::style::Selection {
                bg_fill: p.accent,
                stroke: egui::Stroke::new(1.0, p.accent),
            },
            widgets: egui::style::Widgets {
                noninteractive: egui::style::WidgetVisuals {
                    bg_fill: p.card,
                    weak_bg_fill: p.card_alt,
                    bg_stroke: egui::Stroke::new(1.0, p.border),
                    corner_radius: 6.0.into(),
                    fg_stroke: egui::Stroke::new(1.0, p.text_secondary),
                    expansion: 0.0,
                },
                inactive: egui::style::WidgetVisuals {
                    bg_fill: p.card,
                    weak_bg_fill: p.card_alt,
                    bg_stroke: egui::Stroke::new(1.0, p.border),
                    corner_radius: 6.0.into(),
                    fg_stroke: egui::Stroke::new(1.0, p.text),
                    expansion: 0.0,
                },
                hovered: egui::style::WidgetVisuals {
                    bg_fill: p.card_alt,
                    weak_bg_fill: p.selected,
                    bg_stroke: egui::Stroke::new(1.0, p.accent),
                    corner_radius: 6.0.into(),
                    fg_stroke: egui::Stroke::new(1.5, p.text),
                    expansion: 1.0,
                },
                active: egui::style::WidgetVisuals {
                    bg_fill: p.selected,
                    weak_bg_fill: p.card,
                    bg_stroke: egui::Stroke::new(1.0, p.accent),
                    corner_radius: 6.0.into(),
                    fg_stroke: egui::Stroke::new(1.5, p.text),
                    expansion: 0.0,
                },
                open: egui::style::WidgetVisuals {
                    bg_fill: p.card_alt,
                    weak_bg_fill: p.card,
                    bg_stroke: egui::Stroke::new(1.0, p.accent),
                    corner_radius: 6.0.into(),
                    fg_stroke: egui::Stroke::new(1.5, p.text),
                    expansion: 0.0,
                },
            },
            ..Default::default()
        });

        if self.show_confirm {
            let mod_list = match &self.confirm_action {
                Some(ConfirmAction::DeleteSelected) => self
                    .mods
                    .iter()
                    .filter(|m| self.selected_mods.contains(&m.name))
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>(),
                Some(ConfirmAction::ClearAll) => {
                    self.mods.iter().map(|m| m.name.clone()).collect()
                }
                None => Vec::new(),
            };

            let action_label = match &self.confirm_action {
                Some(ConfirmAction::DeleteSelected) => {
                    format!("Delete {} selected mod(s)?", mod_list.len())
                }
                Some(ConfirmAction::ClearAll) => {
                    format!("Delete all {} mod(s)?", mod_list.len())
                }
                None => String::new(),
            };

            let warning = if self.backup_enabled {
                "Mods will be backed up before deletion."
            } else {
                "BACKUP IS DISABLED \u{2014} mods will be permanently deleted."
            };

            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Are you sure?")
                                .size(18.0)
                                .color(p.warning),
                        );
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(&action_label).size(14.0));
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(warning)
                                .size(12.0)
                                .color(if self.backup_enabled {
                                    p.success
                                } else {
                                    p.danger
                                }),
                        );

                        if !mod_list.is_empty() {
                            ui.add_space(8.0);
                            ui.label("Mods to delete:");
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    for name in &mod_list {
                                        ui.label(
                                            egui::RichText::new(name)
                                                .size(11.0)
                                                .color(p.text_secondary),
                                        );
                                    }
                                });
                        }

                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Cancel")
                                            .color(egui::Color32::WHITE),
                                    )
                                    .min_size(egui::vec2(100.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.show_confirm = false;
                                self.confirm_action = None;
                            }

                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Delete")
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(p.danger_bg)
                                    .min_size(egui::vec2(100.0, 30.0)),
                                )
                                .clicked()
                            {
                                self.do_delete();
                            }
                        });
                    });
                });
        }

        if self.show_settings {
            self.show_settings_window(ctx);
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Lightning Manager")
                        .color(p.heading)
                        .size(20.0),
                );
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let theme_icon = if self.dark_mode { "☀" } else { "☽" };
                        if ui.button("⚙").clicked() {
                            self.show_settings = !self.show_settings;
                        }
                        ui.add_space(2.0);
                        if ui.button(theme_icon).clicked() {
                            self.dark_mode = !self.dark_mode;
                        }
                        ui.add_space(4.0);
                    },
                );
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.add_space(4.0);
                let tabs = [
                    (Tab::Mods, "Mods"),
                    (Tab::Profiles, "Profiles"),
                    (Tab::Launcher, "Launcher"),
                    (Tab::AppData, "AppData"),
                ];
                for (tab, label) in &tabs {
                    let is_selected = self.current_tab == *tab;
                    let tb = egui::SelectableLabel::new(is_selected, *label);
                    let r = if is_selected {
                        egui::Frame::default()
                            .fill(p.selected)
                            .corner_radius(6)
                            .show(ui, |ui| ui.add(tb))
                            .inner
                    } else {
                        ui.add(tb)
                    };
                    if r.clicked() {
                        self.current_tab = *tab;
                    }
                }
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            match self.current_tab {
                Tab::Mods => self.show_mods_tab(ui),
                Tab::Profiles => self.show_profiles_tab(ui),
                Tab::Launcher => self.show_launcher_tab(ui),
                Tab::AppData => self.show_appdata_tab(ui),
            }
        });
    }
}
