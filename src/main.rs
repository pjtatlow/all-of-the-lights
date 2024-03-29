#[macro_use]
extern crate lazy_static;

use actix_web::{get, post, web, App, HttpServer, Responder};
use blinkt::Blinkt;
use chrono::{offset::Utc, DateTime};
use serde::{Deserialize, Serialize};
use std::collections::LinkedList;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref LIGHTS: Arc<Mutex<LinkedList<LightConfig>>> =
        Arc::new(Mutex::new(LinkedList::new()));
    static ref CHANGE_FLAG: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Body {
    config: Vec<LightConfig>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct LightConfig {
    pixels: [Option<Pixel>; 8],
    end: DateTime<Utc>,
}

impl LightConfig {
    fn empty() -> LightConfig {
        LightConfig {
            pixels: [None, None, None, None, None, None, None, None],
            end: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct Pixel {
    red: u8,
    green: u8,
    blue: u8,
    brightness: f32,
}

#[post("/")]
async fn index_post(body: web::Json<Body>) -> impl Responder {
    let mut vals = LIGHTS.lock().unwrap();
    vals.clear();
    for config in &body.config {
        vals.push_back(*config);
    }
    let mut flag = CHANGE_FLAG.lock().unwrap();
    *flag = true;
    format!("Your wish is my command")
}

#[get("/")]
async fn index_get() -> impl Responder {
    let vals = LIGHTS.lock().unwrap();
    web::Json(vals.clone())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (sender, receiver) = channel::<LightConfig>();
    std::thread::spawn(move || match Blinkt::new() {
        Ok(mut blinkt) => {
            blinkt.set_clear_on_drop(true);

            blinkt.set_all_pixels_rgbb(255, 0, 0, 0.05);
            let _ = blinkt.show();
            thread::sleep(Duration::from_secs(1));

            blinkt.set_all_pixels_rgbb(255, 255, 0, 0.05);
            let _ = blinkt.show();
            thread::sleep(Duration::from_secs(1));

            blinkt.set_all_pixels_rgbb(0, 255, 0, 0.05);
            let _ = blinkt.show();
            thread::sleep(Duration::from_secs(1));

            blinkt.set_all_pixels_rgbb(0, 0, 0, 0.05);
            let _ = blinkt.show();
            loop {
                match receiver.recv() {
                    Ok(config) => {
                        blinkt.clear();
                        for i in 0_usize..8_usize {
                            if let Some(pixel) = config.pixels[i] {
                                blinkt.set_pixel_rgbb(
                                    i,
                                    pixel.red,
                                    pixel.green,
                                    pixel.blue,
                                    pixel.brightness,
                                );
                            } else {
                                blinkt.set_pixel_rgbb(i, 0, 0, 0, 0.0);
                            }
                        }
                        let _ = blinkt.show();
                    }
                    Err(_) => (),
                }
            }
        }
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
                        // println!("DROPPING CURRENT");
                    }
                    *flag = false;
                }
            }
            if let Some(val) = &current {
                let now = Utc::now();
                if now >= val.end {
                    // println!("DONE");
                    current = None;
                    let _ = sender.send(LightConfig::empty());
                    // } else {
                    // let diff = val.end - now;
                    // println!("{}", diff.num_seconds());
                }
            } else {
                let mut vals = LIGHTS.lock().unwrap();
                if let Some(val) = vals.pop_front() {
                    let mut next = val.clone();
                    next.end = next.end + chrono::Duration::days(1);
                    vals.push_back(next);
                    // println!("NEW {:?}", val);
                    current = Some(val);
                    let _ = sender.send(val);
                } else {
                    // println!("NOTHING");
                }
            }
            thread::sleep(Duration::from_secs(1))
        }
    });
    HttpServer::new(|| App::new().service(index_post).service(index_get))
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
