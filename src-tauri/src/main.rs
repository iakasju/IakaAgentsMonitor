// Empeche l'ouverture d'une console additionnelle sous Windows en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    iakaagentsmonitor_lib::run();
}
