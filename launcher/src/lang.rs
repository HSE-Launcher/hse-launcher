use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Lang {
    English,
    Russian,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LangMessage {
    AuthMessage { url: String },
    NoConnectionToAuthServer { offline_username: Option<String> },
    AuthTimeout,
    AuthError(String),
    AuthorizedAs,
    AuthorizeUsing(String),
    Authorizing,
    Authorize,
    FetchingVersionManifest,
    NoConnectionToManifestServer,
    ErrorFetchingRemoteManifest(String),
    FetchManifest,
    SelectModpack,
    NotSelected,
    NoModpacks,
    GettingVersionMetadata,
    NoConnectionToMetadataServer,
    ErrorGettingRemoteMetadata(String),
    ErrorGettingMetadata(String),
    CheckingFiles,
    DownloadingFiles,
    SyncModpack,
    ModpackNotSynced,
    ModpackSynced,
    NoConnectionToSyncServer,
    ModpackSyncError(String),
    DownloadingJava,
    JavaInstalled { version: String },
    NeedJava { version: String },
    ErrorDownloadingJava(String),
    NoConnectionToJavaServer,
    Settings,
    SelectedJavaPath,
    NoJavaPath,
    JavaXMX,
    SelectJavaPath,
    Launch,
    LaunchError(String),
    ProcessErrorCode(String),
    Running,
    LanguageName,
    DownloadingUpdate,
    CheckingForUpdates,
    Launching,
    ErrorCheckingForUpdates(String),
    ErrorDownloadingUpdate(String),
    NoConnectionToUpdateServer(String),
    ErrorReadOnly,
    ProceedToLauncher,
    Authorization,
    ForceOverwrite,
    ForceOverwriteWarning,
    KillMinecraft,
    CloseLauncherAfterLaunch,
    DownloadAndLaunch,
    CancelLaunch,
    CancelDownload,
    Retry,
    OpenLogs,
}

impl LangMessage {
    pub fn to_string(&self, lang: &Lang) -> String {
        match self {
            LangMessage::AuthMessage { url: _ } => match lang {
                Lang::English => {
                    "Authorize in the browser window.\nOr open the link manually.".to_string()
                }
                Lang::Russian => {
                    "Авторизуйтесь в открывшемся окне браузера.\nИли откройте ссылку вручную."
                        .to_string()
                }
            },
            LangMessage::NoConnectionToAuthServer {
                offline_username: username,
            } => match lang {
                Lang::English => format!(
                    "Error: no connection to the authorization server.\nOffline username: {}",
                    username.as_ref().unwrap_or(&"None".to_string())
                ),
                Lang::Russian => format!(
                    "Ошибка: нет подключения к серверу авторизации.\nОфлайн имя пользователя: {}",
                    username.as_ref().unwrap_or(&"Отсутствует".to_string())
                ),
            },
            LangMessage::AuthTimeout => match lang {
                Lang::English => "Authorization timeout".to_string(),
                Lang::Russian => "Превышено время авторизации".to_string(),
            },
            LangMessage::AuthError(e) => match lang {
                Lang::English => format!("Authorization error: {}", e),
                Lang::Russian => format!("Ошибка авторизации: {}", e),
            },
            LangMessage::AuthorizedAs => match lang {
                Lang::English => "Authorized as".to_string(),
                Lang::Russian => "Авторизован как".to_string(),
            },
            LangMessage::AuthorizeUsing(app_name) => match lang {
                Lang::English => format!("Authorize using {}", app_name),
                Lang::Russian => format!("Авторизуйтесь через {}", app_name),
            },
            LangMessage::Authorizing => match lang {
                Lang::English => "Authorizing...".to_string(),
                Lang::Russian => "Авторизация...".to_string(),
            },
            LangMessage::Authorize => match lang {
                Lang::English => "Authorize".to_string(),
                Lang::Russian => "Авторизоваться".to_string(),
            },
            LangMessage::FetchingVersionManifest => match lang {
                Lang::English => "Fetching modpack list...".to_string(),
                Lang::Russian => "Получение списка модпаков...".to_string(),
            },
            LangMessage::NoConnectionToManifestServer => match lang {
                Lang::English => "Error: no connection to the modpack server".to_string(),
                Lang::Russian => "Ошибка: нет подключения к серверу модпаков".to_string(),
            },
            LangMessage::ErrorFetchingRemoteManifest(s) => match lang {
                Lang::English => format!("Error fetching remote modpack list: {}", s),
                Lang::Russian => format!("Ошибка получения списка модпаков с сервера: {}", s),
            },
            LangMessage::FetchManifest => match lang {
                Lang::English => "Fetch modpack list".to_string(),
                Lang::Russian => "Получить список модпаков".to_string(),
            },
            LangMessage::SelectModpack => match lang {
                Lang::English => "Select modpack:".to_string(),
                Lang::Russian => "Выберите модпак:".to_string(),
            },
            LangMessage::NotSelected => match lang {
                Lang::English => "Not selected".to_string(),
                Lang::Russian => "Не выбран".to_string(),
            },
            LangMessage::NoModpacks => match lang {
                Lang::English => "No modpacks fetched".to_string(),
                Lang::Russian => "Список модпаков пуст".to_string(),
            },
            LangMessage::GettingVersionMetadata => match lang {
                Lang::English => "Getting version metadata...".to_string(),
                Lang::Russian => "Получение метаданных версии...".to_string(),
            }
            LangMessage::NoConnectionToMetadataServer => match lang {
                Lang::English => "Error: no connection to the version metadata server".to_string(),
                Lang::Russian => "Ошибка: нет подключения к серверу метаданных версии".to_string(),
            },
            LangMessage::ErrorGettingRemoteMetadata(s) => match lang {
                Lang::English => format!("Error getting remote version metadata: {}", s),
                Lang::Russian => format!("Ошибка получения метаданных версии с сервера: {}", s),
            },
            LangMessage::ErrorGettingMetadata(s) => match lang {
                Lang::English => format!("Error getting version metadata: {}", s),
                Lang::Russian => format!("Ошибка получения метаданных версии: {}", s),
            },
            LangMessage::CheckingFiles => match lang {
                Lang::English => "Checking files...".to_string(),
                Lang::Russian => "Проверка файлов...".to_string(),
            },
            LangMessage::DownloadingFiles => match lang {
                Lang::English => "Downloading files...".to_string(),
                Lang::Russian => "Загрузка файлов...".to_string(),
            },
            LangMessage::SyncModpack => match lang {
                Lang::English => "Sync modpack".to_string(),
                Lang::Russian => "Синхронизировать модпак".to_string(),
            },
            LangMessage::ModpackNotSynced => match lang {
                Lang::English => "Modpack not synced".to_string(),
                Lang::Russian => "Модпак не синхронизирован".to_string(),
            },
            LangMessage::ModpackSynced => match lang {
                Lang::English => "Modpack up-to-date".to_string(),
                Lang::Russian => "Модпак синхронизирован".to_string(),
            },
            LangMessage::NoConnectionToSyncServer => match lang {
                Lang::English => "Error: no connection to the modpack sync server".to_string(),
                Lang::Russian => "Ошибка: нет подключения к серверу синхронизации модпаков".to_string(),
            },
            LangMessage::ModpackSyncError(e) => match lang {
                Lang::English => format!("Error syncing modpack: {}", e),
                Lang::Russian => format!("Ошибка синхронизации модпака: {}", e),
            },
            LangMessage::DownloadingJava => match lang {
                Lang::English => "Downloading Java...".to_string(),
                Lang::Russian => "Загрузка Java...".to_string(),
            },
            LangMessage::JavaInstalled { version } => match lang {
                Lang::English => format!("Java {} installed", version),
                Lang::Russian => format!("Java {} установлена", version),
            },
            LangMessage::NeedJava { version } => match lang {
                Lang::English => format!("Java {} not installed", version),
                Lang::Russian => format!("Java {} не установлена", version),
            },
            LangMessage::ErrorDownloadingJava(e) => match lang {
                Lang::English => format!("Error downloading Java: {}", e),
                Lang::Russian => format!("Ошибка загрузки Java: {}", e),
            },
            LangMessage::NoConnectionToJavaServer => match lang {
                Lang::English => "Error: no connection to the Java download server".to_string(),
                Lang::Russian => "Ошибка: нет подключения к серверу загрузки Java".to_string(),
            },
            LangMessage::Settings => match lang {
                Lang::English => "Settings".to_string(),
                Lang::Russian => "Настройки".to_string(),
            },
            LangMessage::SelectedJavaPath => match lang {
                Lang::English => "Selected Java path:".to_string(),
                Lang::Russian => "Выбранный путь к Java:".to_string(),
            },
            LangMessage::NoJavaPath => match lang {
                Lang::English => "No Java path selected".to_string(),
                Lang::Russian => "Путь к Java не выбран".to_string(),
            },
            LangMessage::JavaXMX => match lang {
                Lang::English => "Java Xmx".to_string(),
                Lang::Russian => "Java Xmx".to_string(),
            },
            LangMessage::SelectJavaPath => match lang {
                Lang::English => "Select Java path".to_string(),
                Lang::Russian => "Выберите путь к Java".to_string(),
            },
            LangMessage::Launch => match lang {
                Lang::English => "Launch".to_string(),
                Lang::Russian => "Запустить".to_string(),
            },
            LangMessage::LaunchError(e) => match lang {
                Lang::English => format!("Error launching: {}", e),
                Lang::Russian => format!("Ошибка запуска: {}", e),
            },
            LangMessage::ProcessErrorCode(e) => match lang {
                Lang::English => format!("Process exited with code: {}", e),
                Lang::Russian => format!("Процесс завершился с кодом: {}", e),
            },
            LangMessage::Running => match lang {
                Lang::English => "Running...".to_string(),
                Lang::Russian => "Запущено...".to_string(),
            },
            LangMessage::LanguageName => match lang {
                Lang::English => "English".to_string(),
                Lang::Russian => "Русский".to_string(),
            },
            LangMessage::DownloadingUpdate => match lang {
                Lang::English => "Downloading update...".to_string(),
                Lang::Russian => "Загрузка обновления...".to_string(),
            },
            LangMessage::CheckingForUpdates => match lang {
                Lang::English => "Checking for updates...".to_string(),
                Lang::Russian => "Проверка обновлений...".to_string(),
            },
            LangMessage::Launching => match lang {
                Lang::English => "Launching...".to_string(),
                Lang::Russian => "Запуск...".to_string(),
            },
            LangMessage::ErrorCheckingForUpdates(e) => match lang {
                Lang::English => format!("Error checking for updates: {}", e),
                Lang::Russian => format!("Ошибка проверки обновлений: {}", e),
            },
            LangMessage::ErrorDownloadingUpdate(e) => match lang {
                Lang::English => format!("Error downloading update: {}", e),
                Lang::Russian => format!("Ошибка загрузки обновления: {}", e),
            },
            LangMessage::NoConnectionToUpdateServer(e) => match lang {
                Lang::English => format!("Error: no connection to the update server ({})", e),
                Lang::Russian => format!("Ошибка: нет подключения к серверу обновлений ({})", e),
            },
            LangMessage::ErrorReadOnly => match lang {
                Lang::English => {
                    if cfg!(target_os = "macos") {
                        "Error: read-only mode. If running from a disk image, copy to Applications"
                            .to_string()
                    } else {
                        "Error: read-only mode".to_string()
                    }
                }
                Lang::Russian => {
                    if cfg!(target_os = "macos") {
                        "Ошибка: режим только для чтения. Если лаунчер запущен из образа диска, скопируйте в Applications".to_string()
                    } else {
                        "Ошибка: режим только для чтения".to_string()
                    }
                }
            },
            LangMessage::ProceedToLauncher => match lang {
                Lang::English => "Proceed to launcher".to_string(),
                Lang::Russian => "Перейти к лаунчеру".to_string(),
            },
            LangMessage::Authorization => match lang {
                Lang::English => "Authorization".to_string(),
                Lang::Russian => "Авторизация".to_string(),
            },
            LangMessage::ForceOverwrite => match lang {
                Lang::English => "Overwrite optional files".to_string(),
                Lang::Russian => "Перезаписать необязательные файлы".to_string(),
            },
            LangMessage::ForceOverwriteWarning => match lang {
                Lang::English => "Warning: this may overwrite such files as configs, server list, etc.".to_string(),
                Lang::Russian => "Внимание: это может перезаписать такие файлы как настройки, список серверов и т.д.".to_string(),
            },
            LangMessage::KillMinecraft => match lang {
                Lang::English => "Kill Minecraft".to_string(),
                Lang::Russian => "Закрыть Minecraft".to_string(),
            },
            LangMessage::CloseLauncherAfterLaunch => match lang {
                Lang::English => "Close launcher after launch".to_string(),
                Lang::Russian => "Закрыть лаунчер после запуска".to_string(),
            },
            LangMessage::DownloadAndLaunch => match lang {
                Lang::English => "Download and launch".to_string(),
                Lang::Russian => "Загрузить и запустить".to_string(),
            },
            LangMessage::CancelLaunch => match lang {
                Lang::English => "Cancel launch".to_string(),
                Lang::Russian => "Отменить запуск".to_string(),
            },
            LangMessage::CancelDownload => match lang {
                Lang::English => "Cancel download".to_string(),
                Lang::Russian => "Отменить загрузку".to_string(),
            },
            LangMessage::Retry => match lang {
                Lang::English => "Retry".to_string(),
                Lang::Russian => "Попробовать снова".to_string(),
            },
            LangMessage::OpenLogs => match lang {
                Lang::English => "Open logs folder".to_string(),
                Lang::Russian => "Открыть папку с логами".to_string(),
            }
        }
    }
}
