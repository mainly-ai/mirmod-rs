pub mod config;
mod debug;
pub mod orm;
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

    #[tokio::test]
    async fn test_orm() {
        let token = String::from(TEST_TOKEN);

        let config = config::MirandaConfig::new_from_default()
            .unwrap()
            .merge_into_new(config::PartialMirandaConfig::new_from_token_string(token).unwrap())
            .unwrap();

        let mut sc = sctx::SecurityContext::new_from_config(config)
            .await
            .unwrap();

        let ob = orm::find_by_id::<orm::docker_job::DockerJob>(&mut sc, 1)
            .await
            .expect("Error finding job");
        println!("ob: {:?}", ob);
        orm::MirandaLog::create(
            &mut sc,
            "test".to_string(),
            0,
            orm::MirandaClasses::DockerJob,
            -1,
        )
        .await
        .expect("Error creating log");
    }

    #[tokio::test]
    async fn test_update() {
        let token = String::from(TEST_TOKEN);

        let config = config::MirandaConfig::new_from_default()
            .unwrap()
            .merge_into_new(config::PartialMirandaConfig::new_from_token_string(token).unwrap())
            .unwrap();

        let mut sc = sctx::SecurityContext::new_from_config(config)
            .await
            .unwrap();

        let mut ob = orm::find_by_id::<orm::DockerJob>(&mut sc, 1)
            .await
            .expect("Error finding job");
        println!("ob: {:?}", ob);
        ob.set_workflow_state(orm::DockerJobWorkflowState::Uninitialized);
        orm::update(&mut sc, &mut ob)
            .await
            .expect("Error updating job");
    }
}
