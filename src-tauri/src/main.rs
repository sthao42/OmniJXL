// Ensures the desktop application launches as a pure GUI window without any CLI terminal
#![windows_subsystem = "windows"]

fn main() {
    omnijxl_lib::run()
}
