use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use egui::{CentralPanel, Color32, Context as EguiContext, RichText, ScrollArea, SidePanel, Ui};
use flume::Receiver;

use crate::{
    context::{Context, ContextNotification},
    project::Project,
};

#[derive(Clone, Debug)]
pub struct ProjectDescription {
    pub root: PathBuf,
    pub name: String,
    pub is_indexing_lsp: bool,
    pub is_indexing_docs: bool,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum SidebarTab {
    Projects,
    Info,
    Settings,
}

#[derive(Clone, Debug)]
pub struct TimestampedEvent(DateTime<Utc>, ContextNotification);

impl PartialEq for TimestampedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

pub struct App {
    context: Context,
    receiver: Receiver<ContextNotification>,
    selected_project: Option<PathBuf>,
    logs: Vec<String>,
    events: HashMap<String, Vec<TimestampedEvent>>,
    selected_sidebar_tab: SidebarTab,
    selected_event: Option<TimestampedEvent>,
    projects: Vec<ProjectDescription>,
    settings: Settings,
}

impl App {
    pub fn new(
        context: Context,
        receiver: Receiver<ContextNotification>,
        projects: Vec<ProjectDescription>,
        settings: Settings,
    ) -> Self {
        Self {
            context,
            receiver,
            selected_project: None,
            logs: Vec::new(),
            events: HashMap::new(),
            selected_sidebar_tab: SidebarTab::Projects,
            selected_event: None,
            projects,
            settings,
        }
    }

    fn handle_notifications(&mut self) -> bool {
        let mut has_new_events = false;
        while let Ok(notification) = self.receiver.try_recv() {
            if let ContextNotification::ProjectDescriptions(projects) = notification {
                self.projects = projects;
                has_new_events = true;
                continue;
            }

            self.context.request_projects();

            if matches!(notification, ContextNotification::Lsp(_)) {
                has_new_events = true;
                continue;
            }

            has_new_events = true;
            tracing::debug!("Received notification: {:?}", notification);
            let project_path = notification.notification_path();
            let Some(project) = find_root_project(&project_path, &self.projects) else {
                tracing::error!("Project not found: {:?}", project_path);
                continue;
            };
            let project_name = project.file_name().unwrap().to_string_lossy().to_string();
            let timestamped_event = TimestampedEvent(Utc::now(), notification);
            self.events
                .entry(project_name)
                .or_default()
                .push(timestamped_event);
        }
        has_new_events
    }

    fn draw_left_sidebar(&mut self, ui: &mut Ui, projects: &[ProjectDescription]) {
        ui.add_space(15.0);
        ui.columns(3, |columns| {
            columns[0].with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    ui.selectable_value(
                        &mut self.selected_sidebar_tab,
                        SidebarTab::Projects,
                        RichText::new("Projects").text_style(egui::TextStyle::Button),
                    );
                },
            );

            columns[1].with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    ui.selectable_value(
                        &mut self.selected_sidebar_tab,
                        SidebarTab::Info,
                        RichText::new("Info").text_style(egui::TextStyle::Button),
                    );
                },
            );

            columns[2].with_layout(
                egui::Layout::top_down_justified(egui::Align::Center),
                |ui| {
                    ui.selectable_value(
                        &mut self.selected_sidebar_tab,
                        SidebarTab::Settings,
                        RichText::new("Settings").text_style(egui::TextStyle::Button),
                    );
                },
            );
        });

        match self.selected_sidebar_tab {
            SidebarTab::Projects => {
                self.draw_projects_tab(ui, projects);
            }
            SidebarTab::Info => {
                self.draw_info_tab(ui);
            }
            SidebarTab::Settings => {
                self.draw_settings_tab(ui);
            }
        }
    }

    fn draw_projects_tab(&mut self, ui: &mut Ui, projects: &[ProjectDescription]) {
        ScrollArea::vertical().show(ui, |ui| {
            let selected_path = self.selected_project.clone();
            for project in projects {
                let is_spinning = project.is_indexing_lsp || project.is_indexing_docs;
                let is_selected = selected_path.as_ref() == Some(&project.root);

                let cell = ListCell::new(&project.name, is_selected, is_spinning);
                let response = cell.show(ui);

                if response.clicked() {
                    self.selected_project = Some(project.root.clone());
                    ui.ctx().request_repaint();
                }
            }
        });

        ui.vertical_centered_justified(|ui| {
            if ui.button("Add Project").clicked() {
                if let Some(path_buf) = rfd::FileDialog::new().pick_folder() {
                    tracing::debug!("Adding project: {:?}", path_buf);

                    let context = self.context.clone();
                    tokio::spawn(async move {
                        if let Err(e) = context
                            .add_project(Project {
                                root: path_buf,
                                ignore_crates: vec![],
                            })
                            .await
                        {
                            tracing::error!("Failed to add project: {}", e);
                        } else {
                            tracing::debug!("Project added successfully.");
                        }
                    });
                }
            }

            let remove_enabled = self.selected_project.is_some();
            if ui
                .add_enabled(remove_enabled, egui::Button::new("Remove Project"))
                .clicked()
            {
                if let Some(selected_root) = self.selected_project.take() {
                    let context = self.context.clone();
                    tokio::spawn(async move {
                        let _ = context.remove_project(&selected_root).await;
                    });
                }
            }
        });
    }

    fn draw_info_tab(&mut self, ui: &mut Ui) {
        let (host, port) = self.context.address_information();
        let config_file = self.context.configuration_file();
        ui.label(format!("Address: {}", host));
        ui.label(format!("Port: {}", port));

        ui.add_space(10.0);

        ui.vertical_centered_justified(|ui| {
            if ui.button("Copy MCP JSON").clicked() {
                let config = self.context.mcp_configuration();
                ui.ctx().copy_text(config);
            }
            ui.small("Place this in your .cursor/mcp.json file");

            if ui.button("Open Conf").clicked() {
                if let Err(e) = open::that(shellexpand::tilde(&config_file).to_string()) {
                    tracing::error!("Failed to open config file: {}", e);
                }
            }
            if ui.button("Copy Conf Path").clicked() {
                let path = shellexpand::tilde(&config_file).to_string();
                ui.ctx().copy_text(path);
            }
            ui.small(&config_file);
            ui.small("To manually edit projects");
        });
    }

    fn draw_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Application Settings");

        ui.add_space(20.0);

        ui.label("UI Scale");

        use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
        static SCALE_INITIALIZED: AtomicBool = AtomicBool::new(false);
        static TEMP_SCALE_VALUE: AtomicU32 = AtomicU32::new(0);

        if !SCALE_INITIALIZED.load(Ordering::SeqCst) {
            let scale_as_u32 = (self.settings.scale * 100.0) as u32;
            TEMP_SCALE_VALUE.store(scale_as_u32, Ordering::SeqCst);
            SCALE_INITIALIZED.store(true, Ordering::SeqCst);
        }

        let mut scale_as_u32 = TEMP_SCALE_VALUE.load(Ordering::SeqCst);
        let mut scale = (scale_as_u32 as f32) / 100.0;

        if ui
            .add(
                egui::Slider::new(&mut scale, 0.5..=2.5)
                    .step_by(0.1)
                    .text("Scale factor")
                    .clamping(egui::SliderClamping::Always),
            )
            .changed()
        {
            scale_as_u32 = (scale * 100.0) as u32;
            TEMP_SCALE_VALUE.store(scale_as_u32, Ordering::SeqCst);
        }

        ui.add_space(10.0);

        if ui.button("Save & Restart").clicked() {
            let context = self.context.clone();
            let scale = TEMP_SCALE_VALUE.load(Ordering::SeqCst) as f32 / 100.0;

            use std::env;
            use std::process::Command;

            let current_exe = env::current_exe().expect("Could not get current executable path");

            let handle = tokio::runtime::Handle::current();
            tokio::task::spawn_blocking(move || {
                if let Err(e) = handle.block_on(async { context.set_settings_scale(scale).await }) {
                    tracing::error!("Failed to save UI scale: {}", e);
                } else {
                    tracing::info!("UI scale saved: {}. Restarting application...", scale);

                    Command::new(current_exe)
                        .spawn()
                        .expect("Failed to restart application");

                    std::process::exit(0);
                }
            });
        }

        if ui.button("Reset to Default (1.0)").clicked() {
            TEMP_SCALE_VALUE.store(100, Ordering::SeqCst);
        }

        ui.add_space(20.0);
        ui.separator();

        ui.label("Note: Changing UI scale affects text size, spacing, and window elements.");
        ui.label("To apply scale changes, click 'Save & Restart Application' button.");
        ui.label("Changes will be saved and the application will restart automatically.");
    }

    fn draw_main_area(&mut self, ui: &mut Ui, projects: &[ProjectDescription]) {
        if let Some(selected_root) = &self.selected_project {
            let config_path = PathBuf::from(selected_root).join(".cursor/mcp.json");
            if let Some(project) = projects.iter().find(|p| p.root == *selected_root) {
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Update docs index").clicked() {
                            if let Some(ref selected_project) = self.selected_project {
                                let context = self.context.clone();
                                let selected_project = selected_project.clone();
                                tokio::spawn(async move {
                                    if let Err(e) =
                                        context.force_index_docs(&selected_project).await
                                    {
                                        tracing::error!("Failed to update docs index: {}", e);
                                    }
                                });
                            }
                            self.logs
                                .push(format!("Update docs index clicked for: {}", project.name));
                        }
                        if ui.button("Open Project").clicked() {
                            if let Err(e) = open::that(project.root.to_string_lossy().to_string()) {
                                tracing::error!("Failed to open project: {}", e);
                            }
                        }
                        if !config_path.exists()
                            && ui
                                .button("Install mcp.json")
                                .on_hover_text("Create a .cursor/mcp.json file in the project root")
                                .clicked()
                        {
                            let config = self.context.mcp_configuration();
                            if let Err(e) = create_mcp_configuration_file(&project.root, config) {
                                tracing::error!("Failed to create mcp.json: {}", e);
                            }
                        }
                        ui.add_space(10.0);
                        if project.is_indexing_lsp {
                            ui.add(egui::Spinner::new());
                            ui.label("Indexing LSP...");
                        }
                        ui.add_space(10.0);
                        if project.is_indexing_docs {
                            ui.add(egui::Spinner::new());
                            ui.label("Indexing docs...");
                        }
                    });

                    let remaining_space = ui.available_size_before_wrap();
                    ui.allocate_ui(remaining_space, |ui| {
                        egui::Frame::dark_canvas(ui.style())
                            .fill(Color32::from_black_alpha(128))
                            .inner_margin(egui::Margin::same(4))
                            .show(ui, |ui| {
                                ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        if let Some(project_events) = self.events.get(&project.name)
                                        {
                                            let mut event_to_select = None;
                                            for event_tuple in project_events.iter().rev() {
                                                if matches!(
                                                    event_tuple.1,
                                                    ContextNotification::Lsp(_)
                                                ) {
                                                    continue;
                                                }
                                                let TimestampedEvent(timestamp, event) =
                                                    event_tuple;

                                                let timestamp_str =
                                                    timestamp.format("%H:%M:%S").to_string();

                                                let event_details_str = event.description();

                                                let full_event_str = format!(
                                                    "{} - {}",
                                                    timestamp_str, event_details_str
                                                );

                                                let is_selected = self.selected_event.as_ref()
                                                    == Some(event_tuple);

                                                let truncated_str = if full_event_str.len() > 120 {
                                                    format!("{}...", &full_event_str[..117])
                                                } else {
                                                    full_event_str
                                                };
                                                let response =
                                                    ui.selectable_label(is_selected, truncated_str);
                                                if response.clicked() {
                                                    event_to_select = Some(event_tuple.clone());
                                                }
                                            }
                                            if let Some(selected) = event_to_select {
                                                self.selected_event = Some(selected);
                                            }
                                        }
                                    });
                            });
                    });
                });
            } else {
                ui.label("Error: Selected project not found.");
                if self.selected_project.is_some() {
                    self.selected_event = None;
                }
                self.selected_project = None;
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select or add a project");
                ui.label("Added projects first need to be indexed for LSP and docs before they can be used.");
            });
            if self.selected_event.is_some() {
                self.selected_event = None;
            }
        }
    }

    #[allow(dead_code)]
    fn draw_bottom_bar(&mut self, ui: &mut Ui) {
        ui.label("Logs:");
        ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            for log_entry in &self.logs {
                ui.label(log_entry);
            }
        });
    }

    fn draw_right_sidebar(&mut self, ui: &mut Ui, event: TimestampedEvent) {
        ui.horizontal(|ui| {
            if ui.button("Close").on_hover_text("Close").clicked() {
                self.selected_event = None;
            }
            if ui.button("Copy").on_hover_text("Copy").clicked() {
                ui.ctx().copy_text(format!("{:#?}", event.1));
            }
            ui.heading("Details");
        });
        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!(
                "Timestamp: {}",
                event.0.format("%Y-%m-%d %H:%M:%S.%3f")
            ));
            ui.separator();
            ui.monospace(format!("{:#?}", event.1));
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &EguiContext, _frame: &mut eframe::Frame) {
        let has_new_events = self.handle_notifications();
        let projects = self.projects.clone();

        let sidebar_frame = egui::Frame {
            fill: egui::Color32::from_rgb(32, 32, 32),
            ..egui::Frame::side_top_panel(&ctx.style())
        };

        let panel_width = 300.0;

        SidePanel::left("left_sidebar")
            .frame(sidebar_frame)
            .resizable(true)
            .min_width(panel_width)
            .default_width(panel_width)
            .show(ctx, |ui| {
                self.draw_left_sidebar(ui, &projects);
            });

        if let Some(event) = self.selected_event.clone() {
            SidePanel::right("right_sidebar")
                .resizable(true)
                .min_width(panel_width)
                .default_width(panel_width)
                .show(ctx, |ui| {
                    self.draw_right_sidebar(ui, event);
                });
        }

        CentralPanel::default().show(ctx, |ui| {
            self.draw_main_area(ui, &projects);
        });

        if has_new_events {
            ctx.request_repaint();
        }
    }
}

struct ListCell<'a> {
    text: &'a str,
    is_selected: bool,
    is_spinning: bool,
}

impl<'a> ListCell<'a> {
    fn new(text: &'a str, is_selected: bool, is_spinning: bool) -> Self {
        Self {
            text,
            is_selected,
            is_spinning,
        }
    }

    fn show(self, ui: &mut Ui) -> egui::Response {
        let desired_size = egui::vec2(
            ui.available_width(),
            ui.text_style_height(&egui::TextStyle::Body) + 2.0 * ui.style().spacing.item_spacing.y,
        );

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        let bg_fill = if self.is_selected {
            ui.style().visuals.selection.bg_fill
        } else if response.hovered() {
            ui.style().visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        if bg_fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(
                rect.expand(ui.style().spacing.item_spacing.x / 2.0),
                0,
                bg_fill,
            );
        }

        let content_rect = rect.shrink(ui.style().spacing.item_spacing.x);
        #[allow(deprecated)]
        let mut content_ui = ui.child_ui(
            content_rect,
            egui::Layout::left_to_right(egui::Align::Center),
            None,
        );

        content_ui.horizontal(|ui| {
            let text_color = if self.is_selected {
                ui.style().visuals.strong_text_color()
            } else {
                ui.style().visuals.text_color()
            };

            let label = egui::Label::new(RichText::new(self.text).color(text_color))
                .selectable(false)
                .sense(egui::Sense::hover());
            ui.add(label);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.is_spinning {
                    ui.add(egui::Spinner::new().color(text_color));
                }
            });
        });

        response
    }
}
fn find_root_project(mut path: &Path, projects: &[ProjectDescription]) -> Option<PathBuf> {
    if let Some(project) = projects.iter().find(|p| p.root == *path) {
        return Some(project.root.clone());
    }

    while let Some(parent) = path.parent() {
        path = parent;
        if let Some(project) = projects.iter().find(|p| p.root == *path) {
            return Some(project.root.clone());
        }
    }

    None
}

fn create_mcp_configuration_file(path: &Path, contents: String) -> anyhow::Result<()> {
    let config_path = PathBuf::from(path).join(".cursor/mcp.json");
    std::fs::create_dir_all(&config_path)?;
    std::fs::write(config_path, contents)?;
    Ok(())
}
