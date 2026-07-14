use explonz_bnd::{api, application};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    application::run(api::build_routes().await).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
