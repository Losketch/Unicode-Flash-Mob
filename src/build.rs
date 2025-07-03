fn main() {
    #[cfg(windows)]
    {
        use std::env;
        use std::fs;
        use std::path::Path;

        let out_dir = env::var("OUT_DIR").unwrap();
        let rc_path = Path::new(&out_dir).join("temp_resource.rc");

        let rc_content = r#"IDI_APP ICON "icon.ico""#;
        fs::write(&rc_path, rc_content).unwrap();

        embed_resource::compile(rc_path);
    }
}