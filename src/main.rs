// Versão de teste para o PC: `cargo run`
// No celular quem roda é o `android_main` em lib.rs.

#[cfg(not(target_os = "android"))]
fn main() -> Result<(), eframe::Error> {
    abysshub::rodar_desktop()
}

#[cfg(target_os = "android")]
fn main() {}
