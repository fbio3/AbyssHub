//! AbyssHub - mostra quanto tempo você joga cada jogo no celular.

pub mod atualizacao;
pub mod dados;
pub mod historico;
pub mod molduras;
pub mod plataforma;
pub mod ranking;
pub mod sessoes;
pub mod steam;
mod ui;

use std::sync::Arc;

use plataforma::Plataforma;

fn criar_app(cc: &eframe::CreationContext<'_>, plat: Arc<dyn Plataforma>) -> Box<dyn eframe::App> {
    Box::new(ui::AbyssHubApp::novo(cc, plat))
}

/// Ponto de entrada no Android (chamado pelo NativeActivity).
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("AbyssHub"),
    );

    let plat: Arc<dyn Plataforma> = Arc::new(plataforma::android::PlataformaAndroid::nova(&app));

    let opcoes = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    let resultado = eframe::run_native("AbyssHub", opcoes, Box::new(move |cc| Ok(criar_app(cc, plat))));
    if let Err(e) = resultado {
        log::error!("AbyssHub fechou com erro: {e}");
    }
}

/// Roda no PC com dados de exemplo, para testar a interface sem celular.
#[cfg(not(target_os = "android"))]
pub fn rodar_desktop() -> Result<(), eframe::Error> {
    let opcoes = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([420.0, 860.0])
            .with_title("AbyssHub (teste no PC)"),
        ..Default::default()
    };
    let plat: Arc<dyn Plataforma> = Arc::new(plataforma::desktop::PlataformaDesktop::nova());
    eframe::run_native("AbyssHub", opcoes, Box::new(move |cc| Ok(criar_app(cc, plat))))
}
