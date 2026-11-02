use glintblade::game::Options;
use glintblade::world::ZoneId;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h" || a == "help") {
        println!("GLINTBLADE\n  glintblade [--mute] [--colemak] [--zone Z] [--god] [--peaceful]   play (window)\n  glintblade shot ...      headless scripted play to PNGs (see src/tools.rs)\n  glintblade zoneview ...  headless zone renders (see src/tools.rs)\n  glintblade eightway-audit ... native actor/animation/lighting audit (see src/eightway_audit.rs)");
        return;
    }
    match args.first().map(|s| s.as_str()) {
        Some("eightway-audit") => glintblade::eightway_audit::run(&args[1..]),
        Some("zoneview") => glintblade::tools::zoneview(&args[1..]),
        Some("shot") => glintblade::tools::shot(&args[1..]),
        _ => {
            let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
            let opts = Options {
                start: get("--zone").and_then(|z| ZoneId::parse(&z)),
                god: args.iter().any(|a| a == "--god"),
                peaceful: args.iter().any(|a| a == "--peaceful"),
                seed: 7,
            };
            macroquad::Window::from_config(glintblade::app::conf(), glintblade::app::run(opts));
        }
    }
}
