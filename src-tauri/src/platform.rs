//! Desktop decisions that must be the same idea on every OS, tested without a window.
//! Mirrors Gentle Dot: the app places its own windows except on native Wayland, where the
//! compositor does, and GNOME's Wayland session is steered onto XWayland so placement works.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
    Windows,
}

impl Os {
    pub const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::MacOs
    } else if cfg!(windows) {
        Self::Windows
    } else {
        Self::Linux
    };
}

fn set(env: &impl Fn(&str) -> Option<String>, key: &str) -> Option<String> {
    env(key).filter(|value| !value.is_empty())
}

fn is_hyprland(env: &impl Fn(&str) -> Option<String>) -> bool {
    set(env, "HYPRLAND_INSTANCE_SIGNATURE").is_some()
        || set(env, "XDG_CURRENT_DESKTOP").is_some_and(|desktop| desktop.to_lowercase().contains("hyprland"))
}

fn is_wayland(env: &impl Fn(&str) -> Option<String>) -> bool {
    set(env, "WAYLAND_DISPLAY").is_some() || set(env, "XDG_SESSION_TYPE").as_deref() == Some("wayland")
}

/// `Some("x11")` when a Wayland session that is not Hyprland should run under XWayland.
/// Native Wayland does not allow an app to pin an always-on-top window or drag it freely.
pub fn forced_gdk_backend(env: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    let wanted = is_wayland(&env)
        && !is_hyprland(&env)
        && set(&env, "DISPLAY").is_some()
        && set(&env, "GDK_BACKEND").is_none();
    wanted.then_some("x11")
}

/// True when the compositor, not the app, decides where the floating window sits.
pub fn compositor_places_windows(env: impl Fn(&str) -> Option<String>) -> bool {
    if !is_wayland(&env) {
        return false;
    }
    match set(&env, "GDK_BACKEND") {
        Some(backend) => !backend.split(',').next().unwrap_or_default().trim().eq_ignore_ascii_case("x11"),
        None => true,
    }
}

/// The app may move and restore the widget. False only on native Wayland.
pub fn app_places_windows(env: impl Fn(&str) -> Option<String>) -> bool {
    !compositor_places_windows(env)
}

/// Apply the XWayland choice before GTK starts. A later read of [`app_places_windows`] sees it.
pub fn prepare_session() {
    if let Some(backend) = forced_gdk_backend(|key| std::env::var(key).ok()) {
        std::env::set_var("GDK_BACKEND", backend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| pairs.iter().find(|(name, _)| *name == key).map(|(_, value)| (*value).to_string())
    }

    #[test]
    fn gnome_wayland_is_steered_to_xwayland_so_the_app_can_place_the_widget() {
        let gnome: &[(&str, &str)] = &[
            ("XDG_SESSION_TYPE", "wayland"),
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("DISPLAY", ":0"),
            ("XDG_CURRENT_DESKTOP", "ubuntu:GNOME"),
        ];
        assert_eq!(forced_gdk_backend(env_of(gnome)), Some("x11"));
        let steered: &[(&str, &str)] = &[
            ("XDG_SESSION_TYPE", "wayland"),
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("DISPLAY", ":0"),
            ("GDK_BACKEND", "x11"),
        ];
        assert!(app_places_windows(env_of(steered)));
    }

    #[test]
    fn hyprland_keeps_native_wayland_and_places_the_window_itself() {
        let hypr: &[(&str, &str)] = &[
            ("XDG_SESSION_TYPE", "wayland"),
            ("WAYLAND_DISPLAY", "wayland-1"),
            ("DISPLAY", ":1"),
            ("XDG_CURRENT_DESKTOP", "Hyprland"),
            ("HYPRLAND_INSTANCE_SIGNATURE", "abc"),
            ("GDK_BACKEND", "wayland,x11,*"),
        ];
        assert_eq!(forced_gdk_backend(env_of(hypr)), None);
        assert!(!app_places_windows(env_of(hypr)));
    }

    #[test]
    fn windows_macos_and_x11_place_their_own_windows() {
        let x11: &[(&str, &str)] = &[("XDG_SESSION_TYPE", "x11"), ("DISPLAY", ":0")];
        assert_eq!(forced_gdk_backend(env_of(x11)), None);
        assert!(app_places_windows(env_of(x11)));
        assert!(app_places_windows(env_of(&[])));
        assert_eq!(Os::CURRENT, Os::CURRENT);
    }
}
