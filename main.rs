#![windows_subsystem = "windows"]

mod app;
mod minecraft;

use eframe::egui;
use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(
        first: u32,
        handler: Option<unsafe extern "system" fn(*mut c_void) -> i32>,
    ) -> *mut c_void;
}

unsafe extern "system" fn vectored_handler(exception_pointers: *mut c_void) -> i32 {
    let record_ptr = *(exception_pointers as *mut *mut u8);
    let exception_code = *(record_ptr as *const u32);
    let exception_address = *(record_ptr.add(16) as *const *mut u8);

    let msg = format!(
        "Lightning Manager crashed!\nExceptionCode: 0x{:08X}\nExceptionAddress: {:p}\n",
        exception_code, exception_address,
    );
    let _ = std::fs::write(
        std::path::Path::new(&format!(
            "{}/.minecraft/lightning_crash.log",
            std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
        )),
        &msg,
    );
    0
}

fn load_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../Icons/Icon.png");
    let img = image::load_from_memory(icon_bytes).ok()?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Some(egui::IconData {
        width: w,
        height: h,
        rgba: rgba.into_raw(),
    })
}

fn main() {
    unsafe {
        AddVectoredExceptionHandler(1, Some(vectored_handler));
    }

    if let Err(e) = run() {
        let msg = format!("Lightning Manager crashed: {:?}\n", e);
        let _ = std::fs::write(
            std::path::Path::new(&format!(
                "{}/.minecraft/lightning_crash.log",
                std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
            )),
            &msg,
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([700.0, 500.0])
        .with_min_inner_size([500.0, 350.0])
        .with_icon(load_icon().unwrap_or_default())
        .with_title("Lightning Manager");

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Lightning Manager",
        options,
        Box::new(|_cc| Ok(Box::<app::ModManagerApp>::default())),
    )?;

    Ok(())
}
