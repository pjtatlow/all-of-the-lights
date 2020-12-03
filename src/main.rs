#[macro_use]
extern crate lazy_static;

use actix_web::{post, web, App, HttpServer, Responder};
use blinkt::Blinkt;
use chrono::{offset::Utc, DateTime};
use colourado::{Color, ColorPalette, PaletteType};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref LIGHTS: Arc<Mutex<Vec<LightConfig>>> = Arc::new(Mutex::new(Vec::new()));
    static ref CHANGE_FLAG: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Serialize, Deserialize, Debug)]
struct LightConfig {
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
    let (sender, receiver) = channel::<u32>();
    std::thread::spawn(move || match Blinkt::new().unwrap() {
        Ok(mut blinkt) => loop {
            let spacing = 360.0 / 16.0;

            match receiver.recv() {
                Ok(_) => {
                    let end = Utc::now() + chrono::Duration::seconds(5);
                    while Utc::now() < end {
                        let hue = (Utc::now().timestamp_millis() % 360) as f32;
                        for x in 0..8 {
                            let offset = (x as f32) * spacing;
                            let h = ((hue + offset) % 360.0) / 360.0;
                            let color = Color::hsv_to_rgb(h, 1.0, 1.0);
                            blinkt.set_pixel(
                                x as usize,
                                (color.red * 255) as u8,
                                (color.green * 255) as u8,
                                (color.blue * 255) as u8,
                            );
                        }
                        blinkt.show();
                        thread::sleep_ms(1);
                    }
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
                    sender.send(1);
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
