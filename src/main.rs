#[macro_use]
extern crate lazy_static;

use actix_web::{post, web, App, HttpServer, Responder};
use blinkt::Blinkt;
use chrono::{offset::Utc, DateTime};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref LIGHTS: Arc<Mutex<Vec<LightConfig>>> = Arc::new(Mutex::new(Vec::new()));
    static ref CHANGE_FLAG: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct LightConfig {
    red: u8,
    green: u8,
    blue: u8,
    brightness: f32,
    end: DateTime<Utc>,
}

#[post("/")]
async fn index(config: web::Json<Vec<LightConfig>>) -> impl Responder {
    let mut vals = LIGHTS.lock().unwrap();
    vals.clear();
    config.0.into_iter().rev().for_each(|v| {
        if v.end > Utc::now() {
            vals.push(v)
        }
    });
    let mut flag = CHANGE_FLAG.lock().unwrap();
    *flag = true;
    format!("Your wish is my command")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (sender, receiver) = channel::<LightConfig>();
    std::thread::spawn(move || match Blinkt::new() {
        Ok(mut blinkt) => loop {
            let spacing = 360.0 / 16.0;

            match receiver.recv() {
                Ok(config) => {
                    blinkt.set_all_pixels_rgbb(
                        config.red,
                        config.green,
                        config.blue,
                        config.brightness,
                    );
                    blinkt.show();
                }
                Err(_) => (),
            }
        },
        Err(e) => eprintln!("Could not get blinkt: {}", e),
    });
    std::thread::spawn(move || {
        let mut current: Option<LightConfig> = None;
        loop {
            {
                let mut flag = CHANGE_FLAG.lock().unwrap();
                if *flag {
                    if current.is_some() {
                        current = None;
                        println!("DROPPING CURRENT");
                    }
                    *flag = false;
                }
            }
            if let Some(val) = &current {
                let now = Utc::now();
                if now >= val.end {
                    println!("DONE");
                    current = None;
                } else {
                    let diff = val.end - now;
                    println!("{}", diff.num_seconds());
                }
            } else {
                let mut vals = LIGHTS.lock().unwrap();
                if let Some(val) = vals.pop() {
                    println!("NEW {:?}", val);
                    current = Some(val);
                    let _ = sender.send(val);
                } else {
                    println!("NOTHING");
                }
            }
            thread::sleep(Duration::from_secs(1))
        }
    });
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
