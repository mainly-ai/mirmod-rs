pub mod config;
mod debug;
pub mod sctx;

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOKEN: &str = "pxy.FBBe9XuQUyhZiRRS.uVOGRa_YIoDU6ZJ66tKvSQyKtVDCh68DySI";

    #[tokio::test]
    async fn test_get_config() {
        let config = config::MirandaConfig::new_from_default().unwrap();
    }

    #[tokio::test]
    async fn test_security_context() {
        let token = String::from(TEST_TOKEN);

        let config = config::MirandaConfig::new_from_default()
            .unwrap()
            .merge_into_new(config::PartialMirandaConfig::new_from_token_string(token).unwrap())
            .unwrap();

        let mut sc = sctx::SecurityContext::new_from_config(config)
            .await
            .unwrap();

        // this function also returns the user_id, but we don't care so discard it with ok()
        sc.renew_id().await.ok();

        println!("User id: {}", sc.user_id);
    }
}
