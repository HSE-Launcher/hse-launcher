use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix_web::{App, HttpResponse, HttpServer, web};
use clone_macro::clone;
use serde::Deserialize;
use tokio::sync::Notify;
use tokio::task;

slint::include_modules!();

const CLIENT_ID: &str = "dvasmp1";
const CLIENT_SECRET: &str = "ICLLyGfzhLCyHhAiH5kmcL4QOt7N7RNrQbDkrUE1kB5GOu_EPo503iz3nsiZ34mq";
const LISTEN_PORT: i32 = 18741;
const REDIRECT_URI: &str = "http://127.0.0.1:18741/callback";

#[derive(Deserialize)]
struct ElyByOauthCallbackData {
    code: String,
}

#[derive(Deserialize)]
struct ElyByOauthTokenResponse {
    access_token: String,
    token_type: String,
}


fn main() {
    let ui = AppWindow::new().unwrap();

    // let (tx, rx) = mpsc::channel();
    thread::spawn({
        let ui_handle = ui.as_weak();
        move || {
            let url = format!("https://account.ely.by/oauth2/v1\
            ?client_id={}\
            &redirect_uri={}\
            &response_type=code\
            &scope=account_info minecraft_server_session\
            &prompt=select_account", CLIENT_ID, REDIRECT_URI);
            webbrowser::open(url.as_str()).unwrap();
            let mut access_token = String::new();

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let got_response_flag = Arc::new(Notify::new());

                    let http_server = HttpServer::new(clone!([got_response_flag, mut access_token], move || {
                        App::new().route(
                            "/callback",
                            web::get().to(clone!([got_response_flag, mut access_token], move |req: web::Query<ElyByOauthCallbackData>| clone!([got_response_flag, mut access_token], async move {
                                let client = reqwest::Client::new();
                                let token_response = client
                                    .post("https://account.ely.by/api/oauth2/v1/token")
                                    .form(&[
                                        ("client_id", CLIENT_ID),
                                        ("client_secret", CLIENT_SECRET),
                                        ("redirect_uri", REDIRECT_URI),
                                        ("grant_type", "authorization_code"),
                                        ("code", &req.code),
                                    ])
                                    .send()
                                    .await
                                    .unwrap()
                                    .json::<ElyByOauthTokenResponse>()
                                    .await
                                    .unwrap();
                                access_token.clear();
                                access_token.push_str(token_response.access_token.as_str());
                                got_response_flag.notify_one();
                                HttpResponse::Ok().body("meow")
                            }))),
                        )
                    }))
                        .bind(("127.0.0.1", 18741))?
                        .run();

                    let server_handle = http_server.handle();

                    task::spawn(async move {
                        got_response_flag.notified().await;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        server_handle.stop(false).await;
                    });

                    http_server.await
                })
                .unwrap();
            println!("meow {}", access_token);

            ui_handle.upgrade_in_event_loop(|ui| {}).unwrap();
        }
    });

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 0.1);
        }
    });

    ui.run().unwrap();
}
