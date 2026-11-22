//! What each `command_run` command takes. The table and the runner moved
//! to core (`operandi_core::command_exec`, extensibility stage 2) so the
//! app's command line runs the same commands; this module keeps the old
//! paths working.
pub use operandi_core::command_exec::{
    Arg, CommandSpec, check_args, check_keys, help, help_entry, markdown, spec, specs, summary,
};
