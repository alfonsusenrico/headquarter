//! Headquarter desktop app: a client of the Headquarter host showing sessions beside
//! each agent's own terminal interface. See `openspec/specs/desktop-app/`.

mod app_log;
mod app_state;
mod header;
mod loading;
mod marks;
mod menus;
mod modal;
mod notifications;
mod settings;
mod setup;
mod shell;
mod shortcuts;
mod sidebar;
mod terminal;
mod theme;
mod themes;
mod update;
mod workspace;

use std::time::Duration;

use gpui::{AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px, size};

fn main() {
    app_log::init();
    let quit_after = quit_after_from_args();
    gpui_platform::application()
        .with_assets(gpui_kit_assets::Assets)
        .run(move |cx| {
            gpui_component::init(cx);
            terminal::bind_keys(cx);
            menus::install(cx);
            // A build without a platform text system draws no text and says nothing; make
            // that loud (the `font-kit` feature on macOS).
            if cx.text_system().all_font_names().is_empty() {
                eprintln!("headquarter-app: no fonts available; text will not be drawn");
            }
            // Read once here for the window's background; the workspace reads again.
            let (appearance, _) = settings::read(&mut settings::watcher(), None);
            let bounds = kept_bounds(cx)
                .unwrap_or_else(|| Bounds::centered(None, size(px(1200.), px(780.)), cx));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Headquarter".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.), px(18.))),
                }),
                window_background: settings::window_background(&appearance),
                // Linux desktops match the window to `headquarter.desktop` by this id
                // (Wayland app id, X11 WM_CLASS) to show its icon; macOS ignores it.
                app_id: Some("headquarter".into()),
                ..Default::default()
            };
            let show_setup = setup::should_show(
                settings::path().is_some_and(|path| path.exists()),
                setup::asked_from_args(),
            );
            let opened = cx.open_window(options, |window, cx| {
                let view = cx.new(|cx| shell::Shell::new(show_setup, window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            });
            if let Err(error) = opened {
                eprintln!("headquarter-app: cannot open a window: {error:#}");
                cx.quit();
                return;
            }
            if let Some(seconds) = quit_after {
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(seconds))
                        .await;
                    cx.update(|cx| cx.quit());
                })
                .detach();
            }
        });
}

/// Where the window was when the app last closed, if that still fits a connected
/// display (add-session-restore D3).
fn kept_bounds(cx: &gpui::App) -> Option<Bounds<gpui::Pixels>> {
    let place = app_state::path().and_then(|path| app_state::load_window(&path))?;
    let displays: Vec<app_state::WindowPlace> = cx
        .displays()
        .iter()
        .map(|display| {
            let bounds = display.bounds();
            app_state::WindowPlace {
                x: f32::from(bounds.origin.x),
                y: f32::from(bounds.origin.y),
                width: f32::from(bounds.size.width),
                height: f32::from(bounds.size.height),
            }
        })
        .collect();
    app_state::fits(place, &displays).then(|| {
        Bounds::new(
            point(px(place.x), px(place.y)),
            size(px(place.width), px(place.height)),
        )
    })
}

/// `--quit-after <seconds>`: a hidden option for smoke checks that must not leave a
/// window open.
fn quit_after_from_args() -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|arg| arg == "--quit-after")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse().ok())
}
