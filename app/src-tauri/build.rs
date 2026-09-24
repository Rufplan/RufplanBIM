fn main() {
    // The Rufplan Supabase anon key (public, RLS-governed) is baked in at build time from
    // the environment or the gitignored app/.env.local. Without it, sign-in is disabled.
    println!("cargo:rerun-if-env-changed=RUFPLAN_SUPABASE_ANON_KEY");
    println!("cargo:rerun-if-changed=../.env.local");
    // Declaring rerun-if-changed turns off Cargo's rerun-on-any-change default, so list
    // what tauri-build embeds (the icon resource, config and capabilities) too.
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=capabilities");
    let key = std::env::var("RUFPLAN_SUPABASE_ANON_KEY").ok().or_else(|| {
        std::fs::read_to_string("../.env.local").ok().and_then(|s| {
            s.lines()
                .filter_map(|l| l.trim().strip_prefix("RUFPLAN_SUPABASE_ANON_KEY="))
                .map(|v| v.trim().trim_matches('"').to_owned())
                .next()
        })
    });
    if let Some(key) = key.filter(|k| !k.is_empty()) {
        println!("cargo:rustc-env=RUFPLAN_SUPABASE_ANON_KEY={key}");
    }
    tauri_build::build()
}
