pub mod admin;
pub mod config;
mod debug;
pub mod hashcookie;
pub mod orm;
pub mod sctx;

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOKEN: &str = "pxy.kdIWqsOqbo9UgzdT.rdNKj9DRqwZoKvwkDyJNfvADfNKqtyix1RM";

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

        let mut ob = orm::find_by_id::<orm::DockerJob>(&mut sc, 1)
            .await
            .expect("Error finding job");

        let new_state = match ob.workflow_state() {
            orm::docker_job::WorkflowState::Uninitialized => {
                orm::docker_job::WorkflowState::Starting
            }
            orm::docker_job::WorkflowState::Starting => orm::docker_job::WorkflowState::Ready,
            orm::docker_job::WorkflowState::Ready => orm::docker_job::WorkflowState::ResumeReady,
            orm::docker_job::WorkflowState::ResumeReady => orm::docker_job::WorkflowState::Running,
            orm::docker_job::WorkflowState::Running => orm::docker_job::WorkflowState::Error,
            orm::docker_job::WorkflowState::Error => orm::docker_job::WorkflowState::Exited,
            orm::docker_job::WorkflowState::Exited => orm::docker_job::WorkflowState::Uninitialized,
        };
        let new_cpu_seconds = ob.cpu_seconds() + 1.2;

        ob.set_workflow_state(new_state.clone());
        ob.set_cpu_seconds(new_cpu_seconds.clone());

        orm::update(&mut sc, &mut ob)
            .await
            .expect("Error updating job");

        let ob = orm::find_by_id::<orm::DockerJob>(&mut sc, 1)
            .await
            .expect("Error finding job after update");

        if ob.workflow_state() != new_state {
            panic!("Workflow state did not update");
        }

        if ob.cpu_seconds() != new_cpu_seconds {
            panic!("CPU seconds did not update");
        }

        println!("Found job: {:?}", ob);
    }

    #[tokio::test]
    async fn test_orm_ko() {
        let token = String::from(TEST_TOKEN);

        let config = config::MirandaConfig::new_from_default()
            .unwrap()
            .merge_into_new(config::PartialMirandaConfig::new_from_token_string(token).unwrap())
            .unwrap();

        let mut sc = sctx::SecurityContext::new_from_config(config)
            .await
            .unwrap();

        let mut ob = orm::find_by_id::<orm::KnowledgeObject>(&mut sc, 1)
            .await
            .expect("Error finding KO");

        println!("Found KO: {:?}", ob);
    }

    #[tokio::test]
    async fn test_hashcookie() {
        let config = config::MirandaConfig::new_from_default().unwrap();
        let mut sctx = sctx::SecurityContext::new_from_config(config)
            .await
            .unwrap();
        sctx.renew_id().await.ok();
        let token =
            String::from("1711663072.d2ViYWRtaW4=.NvnxNf4Aw5PBBKH7O9K5CBQqlaRo2QlGwF5U_JwVAli2EIaUQFJmTxGZAqx0IX406jzhYYjc4tjPYD1pMTyfdkChmpaoJkUABaWQVhn88bZVOvPHxXsPBJ-oCtjPvo6scYV9iOk434HNDUyZajWLh51GbQo29WoVYtTZ3TS8BzajIC0gB-T45qJJJ4iZQffZ099xPIYhXwWczWo4.4Kojp-2BAi0=");
        let username = hashcookie::HashCookieTokenPayload::new(token.clone())
            .expect("Error parsing token")
            .get_username();
        let user = admin::users::find_user_by_username(&mut sctx, &username)
            .await
            .expect("Error finding user");
        println!("{:?}", user);
        if let Ok(hc) = hashcookie::HashCookieToken::new_from_token(token, user) {
            println!("{:?}", hc);
        } else {
            panic!("Error parsing token");
        }
    }
}
