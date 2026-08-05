//! Sched module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("sched_run", StdlibFunction {
        rust_path: "std_lib::sched::run".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Chrono, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("SCHED_Job", "nail::std_lib::sched")],
        module: StdlibModule::Sched,
        parameters: vec![StdlibParameter {
            name: "jobs".to_string(),
            param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("SCHED_Job".to_string()))),
            pass_by_reference: false,
        }],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Runs jobs on their cron schedules, forever - each due moment calls the program's handle_job(name:s):v function with the job's name. Jobs run one at a time in this loop, so they never overlap. Blocks forever, so it runs in a spawn block beside the rest of the program. The error case is a cron expression that does not parse.",
        example: "spawn { danger(sched_run([SCHED_Job { name: `cleanup`, cron: `0 3 * * *` }])); }",
    });

    simple_fns! { m, Sched:
        "sched_every" [Tokio] => "std_lib::sched::every", (name: s, seconds: i) -> (v!e),
            "Calls the program's handle_job(name:s):v function with the given name every so many seconds, forever. The wait is between finishes, not starts, so slow work never overlaps itself. Blocks forever, so it runs in a spawn block.",
            "spawn { danger(sched_every(`heartbeat`, 60)); }";
    }
}
