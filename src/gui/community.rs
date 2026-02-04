use serde::Deserialize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::app::store::{GifConfig, Store};

#[derive(Debug, Deserialize, Clone)]
pub struct PackManifest {
    pub version: Option<String>,
    pub last_updated: Option<String>,
    #[serde(default)]
    pub packs: Vec<PackMetadata>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PackMetadata {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub preview_url: String,
    #[serde(rename = "pack_url")]
    pub pack_url: String,
    #[serde(default)]
    pub animation_count: Option<u32>,
    #[serde(default)]
    pub file_size_mb: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct CommunityPack {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub animations: Vec<CommunityAnimation>,
}

#[derive(Debug, Deserialize)]
pub struct CommunityAnimation {
    pub filename: String,
    pub url: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub default_settings: Option<CommunityAnimationSettings>,
}

#[derive(Debug, Deserialize)]
pub struct CommunityAnimationSettings {
    pub scale: Option<f32>,
    pub fps: Option<u32>,
    pub alignment: Option<String>,
    #[serde(default)]
    pub always_on_top: Option<bool>,
}

pub fn fetch_manifest_async(
    tx: mpsc::Sender<Result<Vec<PackMetadata>, String>>,
) {
    std::thread::spawn(move || {
        let url = "https://arknight38.github.io/Gif-Engine-Library/manifest.json";
        let packs = (|| -> Result<Vec<PackMetadata>, String> {
            let resp = reqwest::blocking::get(url)
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("Failed to fetch manifest: {e}"))?;
            let manifest: PackManifest = resp
                .json()
                .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;
            Ok(manifest.packs)
        })();

        let _ = tx.send(packs);
    });
}

pub fn install_pack_async(
    meta: PackMetadata,
    store: Arc<Mutex<Store>>,
) {
    std::thread::spawn(move || {
        if let Err(e) = install_pack_blocking(&meta, &store) {
            eprintln!("[community] Failed to install pack {}: {}", meta.id, e);
        }
    });
}

fn install_pack_blocking(
    meta: &PackMetadata,
    store_arc: &Arc<Mutex<Store>>,
) -> Result<(), String> {
    let resp = reqwest::blocking::get(&meta.pack_url)
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Failed to fetch pack.json: {e}"))?;

    let pack: CommunityPack = resp
        .json()
        .map_err(|e| format!("Failed to parse pack.json: {e}"))?;

    if pack.animations.is_empty() {
        return Err("Pack has no animations".to_string());
    }

    let gifs_dir = Store::gifs_dir();
    if !gifs_dir.exists() {
        std::fs::create_dir_all(&gifs_dir)
            .map_err(|e| format!("Failed to create gifs dir: {e}"))?;
    }

    let mut store = store_arc
        .lock()
        .map_err(|_| "Store mutex poisoned".to_string())?;

    for anim in &pack.animations {
        let url = if anim.url.starts_with("http://") || anim.url.starts_with("https://") {
            anim.url.clone()
        } else {
            // Resolve relative to pack_url
            // pack_url is like "https://.../packs/foo/pack.json"
            // We want "https://.../packs/foo/" + anim.url
            if let Some(base) = meta.pack_url.rsplit_once('/') {
                format!("{}/{}", base.0, anim.url)
            } else {
                anim.url.clone() // Fallback
            }
        };

        let resp = reqwest::blocking::get(&url)
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Failed to download {}: {e}", url))?;
        let bytes = resp
            .bytes()
            .map_err(|e| format!("Failed to read bytes for {}: {e}", url))?;

        let filename = if anim.filename.is_empty() {
            match url.split('/').last() {
                Some(name) if !name.is_empty() => name.to_string(),
                _ => format!("{}.gif", meta.id),
            }
        } else {
            anim.filename.clone()
        };

        let dest_path = gifs_dir.join(&filename);
        std::fs::write(&dest_path, &bytes)
            .map_err(|e| format!("Failed to write {}: {e}", dest_path.display()))?;

        let name = anim
            .display_name
            .clone()
            .unwrap_or_else(|| filename.clone());
        let name = unique_name(&name, &store);

        let cfg = build_gif_config(&name, &dest_path, anim, &pack);
        store.gifs.insert(name, cfg);
    }

    store
        .save()
        .map_err(|e| format!("Failed to save store.json: {e}"))?;

    Ok(())
}

fn unique_name(base: &str, store: &Store) -> String {
    if !store.gifs.contains_key(base) {
        return base.to_string();
    }
    for i in 2..=9999 {
        let cand = format!("{base} ({i})");
        if !store.gifs.contains_key(&cand) {
            return cand;
        }
    }
    base.to_string()
}

fn build_gif_config(
    name: &str,
    path: &std::path::Path,
    anim: &CommunityAnimation,
    pack: &CommunityPack,
) -> GifConfig {
    let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let mut cfg = GifConfig {
        path: abs_path,
        name: name.to_string(),
        fps: None,
        scale: None,
        position: None,
        align: "center".to_string(),
        monitor: 0,
        overlay: true,
        tags: vec![],
        anchor: None,
        behavior: crate::app::store::Behavior::None,
    };

    // Merge pack-level + animation-level tags
    let mut tags = pack.tags.clone();
    tags.extend(anim.tags.clone());
    tags.sort();
    tags.dedup();
    cfg.tags = tags;

    if let Some(settings) = &anim.default_settings {
        if let Some(scale) = settings.scale {
            cfg.scale = Some(scale);
        }
        if let Some(fps) = settings.fps {
            cfg.fps = Some(fps);
        }
        if let Some(align) = &settings.alignment {
            cfg.align = align.to_lowercase();
        }
        if let Some(always_on_top) = settings.always_on_top {
            cfg.overlay = always_on_top;
        }
    }

    cfg
}


