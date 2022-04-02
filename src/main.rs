use std::time::Duration;

use blinkt::Blinkt;
use rspotify::{Credentials, OAuth, AuthCodeSpotify, scopes, clients::OAuthClient, Config};
use tokio::time::{Instant, interval, MissedTickBehavior};

fn rgb_for_playback(is_playing: bool) -> (u8, u8, u8) {
    match is_playing {
        _ => (0, 255, 0),
    }
}

async fn blink_pixel(blinkt: &mut Blinkt, pixel: usize, (r, g, b): (u8, u8, u8), max_brightness: f32) -> anyhow::Result<()> {
    let start_time = Instant::now();
    let mut interval = interval(Duration::from_millis(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let now = Instant::now();
        let time_elapsed = now - start_time;
        let animation_progress = time_elapsed.as_millis() as f32 / 500.0;
        if animation_progress > 2.0 {
            blinkt.set_pixel(pixel, 0, 0, 0);
            blinkt.show()?;
            break;
        } else if animation_progress > 1.0 {
            blinkt.set_pixel_rgbb(pixel, r, g, b, max_brightness * (2.0 - animation_progress));
        } else {
            blinkt.set_pixel_rgbb(pixel, r, g, b, max_brightness * animation_progress);
        }
        blinkt.show()?;
    }

    Ok(())
}

async fn animate_fade_down(blinkt: &mut Blinkt, from_count: usize, to_count: usize) -> anyhow::Result<()> {
    for i in 0..(from_count - to_count) {
        blinkt.set_pixel_rgbb(from_count - i, 0, 0, 0, 0.0);
        blinkt.show()?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let creds = Credentials::from_env().unwrap();
    let oauth = OAuth::from_env(scopes!("user-read-playback-state", "user-read-currently-playing")).unwrap();
    let mut spotify = AuthCodeSpotify::with_config(creds.clone(), oauth.clone(), Config {
        token_cached: true,
        token_refreshing: true,
        ..Default::default()
    });
    let url = spotify.get_authorize_url(false)?;
    spotify.prompt_for_token(&url).await?;

    let mut blinkt = Blinkt::new()?;

    let mut last_num_full = 0;

    loop {
        let playback = spotify.current_playback(None, None::<Vec<_>>).await?;
        if let Some(playback) = playback {
            if let Some((item, progress)) = playback.item.zip(playback.progress) {
                let total_duration = match item {
                    rspotify::model::PlayableItem::Track(track) => {
                        track.duration
                    },
                    rspotify::model::PlayableItem::Episode(episode) => {
                        episode.duration
                    },
                };
                let progress = progress.as_millis() as f32 / total_duration.as_millis() as f32;
                let progress = progress * 8.0;
                let num_full = progress.floor() as usize;
                // let partial = progress % 1.0;
                let (r, g, b) = rgb_for_playback(playback.is_playing);

                if last_num_full > num_full {
                    animate_fade_down(&mut blinkt, last_num_full, num_full).await?;
                }
                last_num_full = num_full;

                for i in 0..num_full {
                    blinkt.set_pixel_rgbb(i as usize, r, g, b, 0.25);
                }
                if num_full < 8 && playback.is_playing {
                    blink_pixel(&mut blinkt, num_full, (r, g, b), 0.25).await?;
                    blink_pixel(&mut blinkt, num_full, (r, g, b), 0.25).await?;
                } else {
                    tokio::time::sleep(Duration::from_millis(2000)).await;
                }
            }
        }
    }
}
