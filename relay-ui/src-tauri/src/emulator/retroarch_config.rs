//! Generates the per-launch `--appendconfig` file for RetroArch (REL-106), overlaying per-game
//! save/state/screenshot directories on top of whatever the user's own retroarch.cfg already has
//! (input bindings, video driver, etc, configured once through RetroArch's own menu) rather than
//! replacing it outright. Ported from the Electron MVP's launcher/retroarchConfig.ts, minus the
//! RetroAchievements cheevos_* keys -- no RA integration exists in this rewrite yet.

use std::path::Path;

// RetroArch's own network command interface, verified against the real source
// (github.com/libretro/RetroArch, command.h/command.c) rather than guessed:
// - network_cmd_enable defaults to false (config.def.h) -- forced on per-launch, since we don't
//   want to touch the user's own base retroarch.cfg just to get this one feature.
// - network_cmd_port defaults to 55355 (config.def.h) -- kept at the default rather than made
//   configurable; nothing about a single-device console needs a second RetroArch instance
//   competing for the port.
pub const NETWORK_CMD_PORT: u16 = 55355;

// Relay owns the whole "leave this game" experience end to end (REL-23/REL-140): pause, save,
// and quit all go through the quick menu's own UI, driven over the network command port above,
// not RetroArch's own input handling or menu. Forced here on every launch for the same reason as
// network_cmd_enable -- without it, RetroArch's own defaults let a player either drop straight to
// the OS with its exit hotkey (Escape, unbound on a gamepad by default but always live on a
// keyboard) or land in RetroArch's own Quick Menu (its default menu-toggle hotkey), neither of
// which the quick menu can see or recover from.
// - input_exit_emulator: keyboard-bound quit hotkey, defaults to Escape. "nul" is RetroArch's own
//   convention for "unbound" (verified against real user reports of this exact syntax, since
//   config.def.h defines hotkey *bindings* -- as opposed to the feature toggles below -- in a
//   separate keybind table this rewrite hasn't needed to fetch bodily).
// - input_menu_toggle: keyboard-bound RetroArch-menu-open hotkey, defaults to F1. Gamepad-combo
//   equivalents for both (quit_gamepad_combo/menu_toggle_gamepad_combo) default to
//   INPUT_COMBO_NONE already (config.def.h) -- nothing to override there on a stock install.
const DISABLE_RETROARCH_OWN_EXIT_AND_MENU: &str = "input_exit_emulator = \"nul\"\ninput_menu_toggle = \"nul\"\n";

// The on-load "core name + content title" banner (REL-140) -- confirmed against real user
// reports of this exact combination, since none of these are individually documented as "the"
// load banner toggle:
// - menu_enable_widgets: the animated icon+text toast system this banner actually renders
//   through (defaults true). Off replaces it with a plain-text OSD message instead of removing it
//   outright, which show_core_name/menu_show_load_content_animation below narrow further.
// - menu_show_load_content_animation: the load-content flourish specifically.
// - show_core_name: the core-name portion of it.
// video_font_enable (the base on-screen-text system) is deliberately left alone -- real error/
// status messages during a launch are still worth seeing, this is just the flashy load banner.
const DISABLE_LOAD_CONTENT_BANNER: &str =
    "menu_enable_widgets = \"false\"\nmenu_show_load_content_animation = \"false\"\nshow_core_name = \"false\"\n";

// config_save_on_exit defaults to true (config.def.h, "Save configuration file on exit") --
// without this, RetroArch can silently write its own state back to the user's base retroarch.cfg
// or a per-core config on exit (e.g. a core auto-persisting an option change), leaking outside
// Relay's own per-launch appendconfig instead of staying scoped to it.
const DISABLE_CONFIG_SAVE_ON_EXIT: &str = "config_save_on_exit = \"false\"\n";

pub struct GameLaunchDirs<'a> {
    pub saves_dir: &'a Path,
    pub save_states_dir: &'a Path,
    pub screenshots_dir: &'a Path,
}

/// The subset of Settings (REL-142) that affect how a game actually plays/looks, as opposed to
/// GameLaunchDirs' per-game bookkeeping paths -- verified against real RetroArch config keys
/// (config.def.h), not guessed:
/// - video_smooth: bilinear-filters the image; off is the sharp/pixel-perfect look.
/// - video_scale_integer: only scales in whole-pixel steps, avoiding uneven pixel sizes.
/// - run_ahead_enabled: runs the core ahead and rewinds to mask input lag, at a CPU cost.
pub struct RetroarchPreferences {
    pub video_smooth: bool,
    pub video_scale_integer: bool,
    pub run_ahead_enabled: bool,
}

impl RetroarchPreferences {
    fn to_config_lines(&self) -> String {
        format!(
            "video_smooth = \"{}\"\nvideo_scale_integer = \"{}\"\nrun_ahead_enabled = \"{}\"\n",
            self.video_smooth, self.video_scale_integer, self.run_ahead_enabled,
        )
    }
}

pub fn append_config_contents(dirs: &GameLaunchDirs, preferences: &RetroarchPreferences) -> String {
    format!(
        "network_cmd_enable = \"true\"\nnetwork_cmd_port = \"{}\"\nsavefile_directory = \"{}\"\nsavestate_directory = \"{}\"\nscreenshot_directory = \"{}\"\n{}{}{}{}",
        NETWORK_CMD_PORT,
        dirs.saves_dir.to_string_lossy(),
        dirs.save_states_dir.to_string_lossy(),
        dirs.screenshots_dir.to_string_lossy(),
        DISABLE_RETROARCH_OWN_EXIT_AND_MENU,
        DISABLE_LOAD_CONTENT_BANNER,
        DISABLE_CONFIG_SAVE_ON_EXIT,
        preferences.to_config_lines(),
    )
}

/// Creates the per-game save/state/screenshot directories (RetroArch won't create them itself)
/// and writes the appendconfig file pointing at them.
pub async fn write_launch_config(
    config_path: &Path,
    dirs: &GameLaunchDirs<'_>,
    preferences: &RetroarchPreferences,
) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dirs.saves_dir).await?;
    tokio::fs::create_dir_all(dirs.save_states_dir).await?;
    tokio::fs::create_dir_all(dirs.screenshots_dir).await?;
    tokio::fs::write(config_path, append_config_contents(dirs, preferences)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_preferences() -> RetroarchPreferences {
        RetroarchPreferences { video_smooth: false, video_scale_integer: true, run_ahead_enabled: false }
    }

    #[test]
    fn append_config_contents_includes_network_cmd_and_all_three_directories() {
        let dirs = GameLaunchDirs {
            saves_dir: Path::new("/home/user/Relay/saves/nes/game-1"),
            save_states_dir: Path::new("/home/user/Relay/savestates/nes/game-1"),
            screenshots_dir: Path::new("/home/user/Relay/screenshots/nes/game-1"),
        };

        let contents = append_config_contents(&dirs, &default_preferences());

        assert_eq!(
            contents,
            "network_cmd_enable = \"true\"\n\
             network_cmd_port = \"55355\"\n\
             savefile_directory = \"/home/user/Relay/saves/nes/game-1\"\n\
             savestate_directory = \"/home/user/Relay/savestates/nes/game-1\"\n\
             screenshot_directory = \"/home/user/Relay/screenshots/nes/game-1\"\n\
             input_exit_emulator = \"nul\"\n\
             input_menu_toggle = \"nul\"\n\
             menu_enable_widgets = \"false\"\n\
             menu_show_load_content_animation = \"false\"\n\
             show_core_name = \"false\"\n\
             config_save_on_exit = \"false\"\n\
             video_smooth = \"false\"\n\
             video_scale_integer = \"true\"\n\
             run_ahead_enabled = \"false\"\n"
        );
    }

    #[test]
    fn append_config_contents_disables_retroarchs_own_exit_menu_load_banner_and_config_save() {
        let dirs = GameLaunchDirs {
            saves_dir: Path::new("/saves"),
            save_states_dir: Path::new("/states"),
            screenshots_dir: Path::new("/screenshots"),
        };

        let contents = append_config_contents(&dirs, &default_preferences());

        // The quick menu (REL-23/REL-140) is meant to be the only way out of a running game --
        // these keep RetroArch's own exit hotkey and menu from ever being reachable, and the
        // icon+title banner from ever popping up on load, regardless of what a base retroarch.cfg
        // (or a future RetroArch default change) might otherwise set. config_save_on_exit keeps
        // RetroArch from writing any of its own state back to that base config in the first place.
        assert!(contents.contains("input_exit_emulator = \"nul\""));
        assert!(contents.contains("input_menu_toggle = \"nul\""));
        assert!(contents.contains("menu_enable_widgets = \"false\""));
        assert!(contents.contains("menu_show_load_content_animation = \"false\""));
        assert!(contents.contains("show_core_name = \"false\""));
        assert!(contents.contains("config_save_on_exit = \"false\""));
    }

    #[test]
    fn append_config_contents_reflects_the_given_video_and_runahead_preferences() {
        let dirs = GameLaunchDirs { saves_dir: Path::new("/saves"), save_states_dir: Path::new("/states"), screenshots_dir: Path::new("/shots") };
        let preferences = RetroarchPreferences { video_smooth: true, video_scale_integer: false, run_ahead_enabled: true };

        let contents = append_config_contents(&dirs, &preferences);

        assert!(contents.contains("video_smooth = \"true\""));
        assert!(contents.contains("video_scale_integer = \"false\""));
        assert!(contents.contains("run_ahead_enabled = \"true\""));
    }

    #[tokio::test]
    async fn write_launch_config_creates_directories_and_writes_the_config_file() {
        let temp = tempfile::tempdir().unwrap();
        let saves_dir = temp.path().join("saves/nes/game-1");
        let save_states_dir = temp.path().join("savestates/nes/game-1");
        let screenshots_dir = temp.path().join("screenshots/nes/game-1");
        let config_path = temp.path().join("launch.cfg");

        let dirs = GameLaunchDirs { saves_dir: &saves_dir, save_states_dir: &save_states_dir, screenshots_dir: &screenshots_dir };
        write_launch_config(&config_path, &dirs, &default_preferences()).await.unwrap();

        assert!(saves_dir.is_dir());
        assert!(save_states_dir.is_dir());
        assert!(screenshots_dir.is_dir());

        let contents = tokio::fs::read_to_string(&config_path).await.unwrap();
        assert!(contents.contains(&format!("savefile_directory = \"{}\"", saves_dir.to_string_lossy())));
    }
}
