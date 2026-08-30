// 去掉开头的 "./" 或 "."，确保以 "/" 开头
pub fn dir_into_url_path(upload_dir: &str) -> String {
    format!(
        "/{}",
        upload_dir.trim_start_matches("./").trim_start_matches('/')
    )
}
