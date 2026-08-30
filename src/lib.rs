//! Pure-Rust Open Media Transport plugin for OBS Studio.

use log::{info, LevelFilter, Log, Metadata, Record};
use obs_wrapper::frontend;
use obs_wrapper::log::Logger;
use obs_wrapper::prelude::*;
use obs_wrapper::source::Icon;
use obs_wrapper::{obs_register_module, obs_string};

mod bandwidth;
mod channel;
mod clock;
mod config;
mod filter;
mod format;
mod graphics_capture;
mod ids;
mod media_mode;
mod omt_file_log;
mod output;
mod preview;
mod receive;
mod send_session;

use filter::OmtSendFilter;
use output::{OmtOutput, OmtOutputSettings};
use receive::OmtReceiveSource;

struct OmtModule {
    context: ModuleRef,
}

impl Module for OmtModule {
    fn new(context: ModuleRef) -> Self {
        Self { context }
    }

    fn get_ctx(&self) -> &ModuleRef {
        &self.context
    }

    fn load(&mut self, load_context: &mut LoadContext) -> bool {
        omt_file_log::init();
        let _ = TeeLogger {
            obs: Logger::new().with_promote_debug(true),
        }
        .init();
        info!(
            "omt-obs-plugin {} loading (libobs API {})",
            env!("CARGO_PKG_VERSION"),
            obs_wrapper::obs_sys::LIBOBS_API_VER
        );

        if let Ok(path) = self.context.config_path(obs_string!("omtplugin.json")) {
            output::init_controller(path);
        }

        let source = load_context
            .create_source_builder::<OmtReceiveSource>()
            .enable_get_name()
            .enable_get_defaults()
            .enable_update()
            .enable_save()
            .enable_get_properties()
            .enable_activate()
            .enable_deactivate()
            .enable_show()
            .enable_hide()
            .enable_async_video()
            .enable_audio()
            .with_icon(Icon::Camera)
            .build();
        load_context.register_source(source);

        let output = load_context
            .create_output_builder::<OmtOutput>()
            .enable_get_name()
            .enable_raw_video()
            .enable_raw_audio()
            .build();
        load_context.register_output(output);

        let settings = load_context
            .create_source_builder::<OmtOutputSettings>()
            .enable_get_name()
            .enable_get_defaults()
            .enable_get_properties()
            .enable_update()
            .enable_save()
            .with_output_flags(output::settings_flags())
            .build();
        load_context.register_source(settings);

        let filter = load_context
            .create_source_builder::<OmtSendFilter>()
            .enable_get_name()
            .enable_get_defaults()
            .enable_get_properties()
            .enable_update()
            .enable_video_render()
            .enable_video_tick()
            .enable_filter_audio()
            .enable_filter_add()
            .enable_filter_remove()
            .with_icon(Icon::Camera)
            .build();
        load_context.register_source(filter);

        true
    }

    fn post_load(&mut self) {
        if frontend::has_main_window() {
            frontend::add_event_callback(output::on_frontend_event);
            frontend::add_tools_menu_item(
                obs_string!("OMT Output Settings"),
                output::show_settings,
            );
        }
    }

    fn unload(&mut self) {
        output::destroy_outputs();
        frontend::remove_event_callback();
        info!("omt-obs-plugin unloaded");
    }

    fn description() -> ObsString {
        obs_string!("Pure-Rust Open Media Transport (OMT) source, output, preview, and filter.")
    }

    fn name() -> ObsString {
        obs_string!("OMT (Rust)")
    }

    fn author() -> ObsString {
        obs_string!("MikanseiLaboratory")
    }
}

obs_register_module!(OmtModule);

struct TeeLogger {
    obs: Logger,
}

impl TeeLogger {
    fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_max_level(LevelFilter::Trace);
        log::set_boxed_logger(Box::new(self))
    }
}

impl Log for TeeLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.obs.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.obs.log(record);
        if self.enabled(record.metadata()) {
            omt_file_log::write(&format!("{}", record.args()), record.target());
        }
    }

    fn flush(&self) {
        self.obs.flush();
    }
}
