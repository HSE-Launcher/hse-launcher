use eframe::egui;
use eframe::run_native;
use tokio::runtime::Runtime;

use super::auth_state::AuthState;
use super::java_state::JavaState;
use super::launch_state::ForceLaunchResult;
use super::launch_state::LaunchState;
use super::manifest_state::ManifestState;
use super::metadata_state::MetadataState;
use super::modpack_sync_state::ModpackSyncState;
use super::settings::SettingsState;
use crate::config::build_config;
use crate::config::runtime_config;
use crate::utils;

pub struct LauncherApp {
    runtime: Runtime,
    config: runtime_config::Config,
    settings_state: SettingsState,
    auth_state: AuthState,
    manifest_state: ManifestState,
    metadata_state: MetadataState,
    java_state: JavaState,
    modpack_sync_state: ModpackSyncState,
    launch_state: LaunchState,
}

pub fn run_gui(config: runtime_config::Config) {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((350.0, 300.0))
            .with_icon(utils::get_icon_data()),
        ..Default::default()
    };

    run_native(
        &build_config::get_launcher_name(),
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::new(config, &cc.egui_ctx)))
        }),
    )
    .unwrap();
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl LauncherApp {
    fn new(config: runtime_config::Config, ctx: &egui::Context) -> Self {
        let runtime = Runtime::new().unwrap();
        LauncherApp {
            settings_state: SettingsState::new(),
            auth_state: AuthState::new(ctx),
            manifest_state: ManifestState::new(),
            metadata_state: MetadataState::new(),
            java_state: JavaState::new(ctx),
            modpack_sync_state: runtime.block_on(ModpackSyncState::new(ctx, &config)),
            launch_state: LaunchState::new(),
            runtime,
            config,
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        let mut selected_metadata = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let mut need_check = false;

                need_check |= self
                    .manifest_state
                    .update(&self.runtime, &mut self.config, ctx);

                need_check |= self.manifest_state.render_ui(ui, &mut self.config);
                ui.separator();

                let selected_modpack = self.manifest_state.get_selected_modpack(&self.config);
                if let Some(selected_modpack) = selected_modpack {
                    if need_check {
                        self.metadata_state.reset();
                    }

                    need_check |= self.metadata_state.update(
                        &self.runtime,
                        &mut self.config,
                        selected_modpack,
                        ctx,
                    );

                    if self.metadata_state.render_ui(ui, &self.config) {
                        ui.separator();
                    }

                    let version_metadata = self.metadata_state.get_version_metadata();
                    if let Some(version_metadata) = version_metadata {
                        selected_metadata = Some(version_metadata.clone());
                        if need_check {
                            self.auth_state
                                .reset_auth_if_needed(version_metadata.get_auth_data());
                        }
                        need_check |= self.auth_state.update();

                        self.auth_state.render_ui(
                            ui,
                            &self.config,
                            &self.runtime,
                            ctx,
                            version_metadata.get_auth_data(),
                        );

                        let version_auth_data = self.auth_state.get_version_auth_data(version_metadata.get_auth_data());

                        if let Some(version_auth_data) = version_auth_data {
                            let manifest_online =
                                self.manifest_state.online() && self.metadata_state.online();
                            need_check |= self.modpack_sync_state.update(
                                &self.runtime,
                                selected_modpack,
                                version_metadata.clone(),
                                &self.config,
                                need_check,
                                manifest_online,
                            );

                            self.java_state.update(
                                &self.runtime,
                                &version_metadata,
                                &mut self.config,
                                ctx,
                                need_check,
                            );

                            ui.separator();
                            self.modpack_sync_state.render_ui(
                                ui,
                                &mut self.config,
                                manifest_online,
                            );

                            ui.separator();
                            self.java_state
                                .render_ui(ui, &mut self.config, &version_metadata);

                            if self.java_state.ready_for_launch()
                                && (self.modpack_sync_state.ready_for_launch() || !manifest_online)
                            {
                                self.launch_state.update();

                                self.launch_state.render_ui(
                                    &self.runtime,
                                    ui,
                                    &mut self.config,
                                    &version_metadata,
                                    version_auth_data,
                                    self.auth_state.online(),
                                );
                            } else {
                                let force_launch_result =
                                    self.launch_state.render_download_ui(ui, &mut self.config);
                                match force_launch_result {
                                    ForceLaunchResult::ForceLaunchSelected => {
                                        self.modpack_sync_state.schedule_sync_if_needed();
                                        self.java_state.schedule_download_if_needed(
                                            &self.runtime,
                                            &version_metadata,
                                            &mut self.config,
                                        );
                                    }
                                    ForceLaunchResult::CancelSelected => {
                                        self.java_state.cancel_download();
                                        self.modpack_sync_state.cancel_sync();
                                    }
                                    ForceLaunchResult::NotSelected => {}
                                }
                            }
                        }
                    }
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    let selected_metadata_ref = selected_metadata.as_deref();
                    self.settings_state.render_ui(
                        ui,
                        &self.runtime,
                        &mut self.config,
                        selected_metadata_ref,
                    );
                });
                ui.add_space(5.0);
            });
    }
}
