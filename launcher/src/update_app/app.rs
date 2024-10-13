use std::sync::mpsc;
use std::sync::Arc;

use eframe::egui;
use eframe::run_native;
use log::info;
use tokio::runtime::Runtime;

use crate::app::progress_bar::GuiProgressBar;
use crate::config::build_config;
use crate::config::runtime_config;
use crate::lang::Lang;
use crate::lang::LangMessage;
use crate::launcher::update::download_new_launcher;
use crate::launcher::update::need_update;
use crate::launcher::update::replace_launcher_and_start;
use crate::utils;

use shared::progress::ProgressBar;
use shared::progress::Unit;

enum UpdateStatus {
    Checking,
    NeedUpdate,
    UpToDate,
    UpdateError(String),
    UpdateErrorOffline(String),
}

enum DownloadStatus {
    NeedDownloading,
    Downloaded(Vec<u8>),
    DownloadError(String),
    DownloadErrorOffline(String),
    ErrorReadOnly,
}

pub struct UpdateApp {
    runtime: Runtime,
    lang: Lang,
    need_update_receiver: mpsc::Receiver<UpdateStatus>,
    new_binary_receiver: Option<mpsc::Receiver<DownloadStatus>>,
    update_progress_bar: Arc<GuiProgressBar>,
    update_status: UpdateStatus,
    download_status: DownloadStatus,
    exit_on_close: bool,
}

pub fn run_gui(config: &runtime_config::Config) {
    if std::env::var("CARGO").is_ok() {
        info!("Running from cargo, skipping auto-update");
        return;
    }

    if build_config::get_version().is_none() {
        info!("Version not set, skipping auto-update");
        return;
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((300.0, 150.0))
            .with_icon(utils::get_icon_data()),
        ..Default::default()
    };

    let lang = config.lang.clone();

    run_native(
        &format!("{} Updater", build_config::get_launcher_name()),
        native_options,
        Box::new(|cc| Ok(Box::new(UpdateApp::new(lang, &cc.egui_ctx)))),
    )
    .unwrap();
}

impl eframe::App for UpdateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.exit_on_close {
            std::process::exit(0);
        }
    }
}

impl UpdateApp {
    fn new(lang: Lang, ctx: &egui::Context) -> Self {
        let runtime = Runtime::new().unwrap();

        let (need_update_sender, need_update_receiver) = mpsc::channel();
        let ctx_clone = ctx.clone();
        runtime.spawn(async move {
            let _ = need_update_sender.send(match need_update().await {
                Ok(true) => UpdateStatus::NeedUpdate,
                Ok(false) => UpdateStatus::UpToDate,
                Err(e) if utils::is_connect_error(&e) => {
                    UpdateStatus::UpdateErrorOffline(e.to_string())
                }
                Err(e) => UpdateStatus::UpdateError(e.to_string()),
            });
            ctx_clone.request_repaint();
        });

        let update_progress_bar = Arc::new(GuiProgressBar::new(ctx));
        update_progress_bar.set_unit(Unit {
            name: "MB".to_string(),
            size: 1024 * 1024,
        });

        UpdateApp {
            runtime,
            lang,
            need_update_receiver,
            new_binary_receiver: None,
            update_progress_bar,
            update_status: UpdateStatus::Checking,
            download_status: DownloadStatus::NeedDownloading,
            exit_on_close: true,
        }
    }

    fn render_close_button(&mut self, ui: &mut egui::Ui) {
        if ui
            .button(LangMessage::ProceedToLauncher.to_string(&self.lang))
            .clicked()
        {
            self.exit_on_close = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                match &self.download_status {
                    DownloadStatus::Downloaded(new_binary) => {
                        if let Some(e) = replace_launcher_and_start(new_binary).err() {
                            self.download_status = if utils::is_read_only_error(&e) {
                                DownloadStatus::ErrorReadOnly
                            } else {
                                DownloadStatus::DownloadError(e.to_string())
                            };
                        } else {
                            panic!("Launcher should have been replaced and launched");
                        }
                    }
                    _ => {}
                }

                if let Some(new_binary_receiver) = &self.new_binary_receiver {
                    if let Ok(download_status) = new_binary_receiver.try_recv() {
                        match &download_status {
                            DownloadStatus::Downloaded(_) => {
                                ui.label(LangMessage::Launching.to_string(&self.lang));
                            }
                            DownloadStatus::DownloadError(_) => {}
                            DownloadStatus::DownloadErrorOffline(_) => {}
                            DownloadStatus::NeedDownloading => {
                                panic!("Should not receive NeedDownloading");
                            }
                            DownloadStatus::ErrorReadOnly => {}
                        }
                        self.download_status = download_status;
                    }
                } else {
                    if let Ok(update_status) = self.need_update_receiver.try_recv() {
                        match &update_status {
                            UpdateStatus::NeedUpdate => {
                                let (new_binary_sender, new_binary_receiver) = mpsc::channel();
                                self.new_binary_receiver = Some(new_binary_receiver);
                                let update_progress_bar = self.update_progress_bar.clone();
                                let ctx = ctx.clone();
                                self.runtime.spawn(async move {
                                    let _ = new_binary_sender.send(
                                        match download_new_launcher(update_progress_bar).await {
                                            Ok(new_binary) => {
                                                DownloadStatus::Downloaded(new_binary)
                                            }
                                            Err(e) if utils::is_read_only_error(&e) => {
                                                DownloadStatus::ErrorReadOnly
                                            }
                                            Err(e) if utils::is_connect_error(&e) => {
                                                DownloadStatus::DownloadErrorOffline(e.to_string())
                                            }
                                            Err(e) => DownloadStatus::DownloadError(e.to_string()),
                                        },
                                    );
                                    ctx.request_repaint();
                                });
                            }
                            UpdateStatus::UpToDate => {
                                self.exit_on_close = false;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            UpdateStatus::UpdateError(_) => {}
                            UpdateStatus::UpdateErrorOffline(_) => {}
                            UpdateStatus::Checking => {
                                panic!("Should not receive Checking");
                            }
                        }
                        self.update_status = update_status;
                    }
                }

                match &self.update_status {
                    UpdateStatus::Checking => {
                        ui.label(LangMessage::CheckingForUpdates.to_string(&self.lang));
                    }
                    UpdateStatus::NeedUpdate => match &self.download_status {
                        DownloadStatus::NeedDownloading => {
                            self.update_progress_bar.render(ui, &self.lang);
                        }
                        DownloadStatus::DownloadError(e) => {
                            ui.label(
                                LangMessage::ErrorDownloadingUpdate(e.to_string())
                                    .to_string(&self.lang),
                            );
                            self.render_close_button(ui);
                        }
                        DownloadStatus::DownloadErrorOffline(e) => {
                            ui.label(
                                LangMessage::NoConnectionToUpdateServer(e.to_string())
                                    .to_string(&self.lang),
                            );
                            self.render_close_button(ui);
                        }
                        DownloadStatus::Downloaded(_) => {}
                        DownloadStatus::ErrorReadOnly => {
                            ui.label(LangMessage::ErrorReadOnly.to_string(&self.lang));
                            self.render_close_button(ui);
                        }
                    },
                    UpdateStatus::UpToDate => {}
                    UpdateStatus::UpdateError(e) => {
                        ui.label(
                            LangMessage::ErrorCheckingForUpdates(e.to_string())
                                .to_string(&self.lang),
                        );
                        self.render_close_button(ui);
                    }
                    UpdateStatus::UpdateErrorOffline(e) => {
                        ui.label(
                            LangMessage::NoConnectionToUpdateServer(e.to_string())
                                .to_string(&self.lang),
                        );
                        self.render_close_button(ui);
                    }
                }
            });
        });
    }
}
