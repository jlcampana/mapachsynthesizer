//! Busca el mando y enseña lo que manda cada control.
//!
//! `cargo run -p mapach-input --example probar_mando` (Ctrl+C para salir).

use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    let (stx, srx) = mpsc::channel();
    mapach_input::mando::spawn(tx, stx, || {});
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
