//! Environment module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Env:
        "env_get" => "std_lib::env::get", (key: s) -> (s!e),
            "Returns the value of an environment variable; errors if it is not set.",
            "home:s = danger(env_get(`HOME`));";
        "env_set" => "std_lib::env::set", (key: s, value: s) -> (v!e),
            "Sets an environment variable for the current process.",
            "danger(env_set(`MODE`, `production`));";
        "env_remove" => "std_lib::env::remove", (key: s) -> (v!e),
            "Unsets an environment variable for this process. Removing one that was never set is not an error.",
            "danger(env_remove(`AWS_PROFILE`));";
        "env_all" [DashMap] => "std_lib::env::all", () -> (h s s),
            "Returns every environment variable the process has, as a hashmap.",
            "settings:h<s,s> = env_all();";
        "env_current_dir" => "std_lib::env::current_dir", () -> (s!e),
            "Returns the directory the program is running in, which every relative path is relative to.",
            "here:s = danger(env_current_dir());";
        "env_set_current_dir" => "std_lib::env::set_current_dir", (path: s) -> (v!e),
            "Moves the program into another directory so relative paths resolve from there; errors if it cannot be entered.",
            "danger(env_set_current_dir(`/srv/app`));";
        "env_home_dir" => "std_lib::env::home_dir", () -> (s!e),
            "Returns the home directory of the user running the program; errors when HOME is not set.",
            "home:s = danger(env_home_dir());";
        "env_hostname" => "std_lib::env::hostname", () -> (s!e),
            "Returns the name of this machine, read from the kernel where possible.",
            "machine:s = danger(env_hostname());";
        "env_user" => "std_lib::env::user", () -> (s!e),
            "Returns the name of the user running the program; errors when neither USER nor LOGNAME is set.",
            "who:s = danger(env_user());";
        "env_os" => "std_lib::env::os", () -> s,
            "Returns which operating system this build runs on: linux, macos, windows.",
            "system:s = env_os();";
        "env_arch" => "std_lib::env::arch", () -> s,
            "Returns which processor this build is for: x86_64, aarch64, and so on.",
            "processor:s = env_arch();";
        "env_pid" => "std_lib::env::pid", () -> i,
            "Returns the process id of the running program - what goes in a pid file or a log line.",
            "identifier:i = env_pid();";
        "env_cpu_count" => "std_lib::env::cpu_count", () -> i,
            "Returns how many processors the program may actually use - the number to size a worker pool by.",
            "workers:i = env_cpu_count();";
        "env_args" => "std_lib::env::args", () -> [s],
            "Returns all command-line arguments, including the program name.",
            "arguments:a:s = env_args();";
        "env_load_dotenv" [DashMap] => "std_lib::env::load_dotenv", (path: s) -> ((h s s)!e),
            "Reads a .env file, sets every variable in it that is not already set, and returns what it read. Variables the process was started with always win.",
            "settings:h<s,s> = danger(env_load_dotenv(`.env`));";
    }
}
