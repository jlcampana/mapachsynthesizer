//! Busca el mando y enseña lo que manda cada control.
//!
//! `cargo run -p mapach-input --example probar_mando` (Ctrl+C para salir).

use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    let (stx, srx) = mpsc::channel();
    let (rtx, _rrx) = mpsc::channel();
    let shared = std::sync::Arc::new(std::sync::Mutex::new(mapach_input::mando::Shared::default()));
    mapach_input::mando::spawn(tx, stx, rtx, shared, || {});
    println!("Buscando el mando…");
    loop {
        while let Ok(s) = srx.try_recv() {
            println!("Mando: {s:?}");
        }
        if let Ok(e) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
            println!("{e:?}");
        }
    }
}
