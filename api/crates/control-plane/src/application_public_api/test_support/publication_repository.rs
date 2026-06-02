use super::*;

#[async_trait]
impl ApplicationPublicationRepository for ApplicationPublicApiTestRepository {
    async fn create_active_application_publication_version(
        &self,
        input: &CreateApplicationPublicationVersionInput,
    ) -> Result<publications::ApplicationPublicationVersionRecord> {
        let mut inner = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned");

        if !inner.applications.contains_key(&input.application_id) {
            return Err(ControlPlaneError::NotFound("application").into());
        }

        inner.next_publication_ordinal += 1;
        let ordinal = inner.next_publication_ordinal;
        let version_sequence = inner
            .publications
            .values()
            .filter(|publication| publication.application_id == input.application_id)
            .map(|publication| publication.version_sequence)
            .max()
            .unwrap_or(0)
            + 1;

        for publication in inner
            .publications
            .values_mut()
            .filter(|publication| publication.application_id == input.application_id)
        {
            publication.active = false;
        }

        let publication = publications::ApplicationPublicationVersionRecord {
            id: deterministic_test_id(0x44444444444444440000000000000000, ordinal),
            application_id: input.application_id,
            flow_id: input.flow_id,
            flow_version_id: input.flow_version_id,
            mapping_snapshot: input.mapping_snapshot.clone(),
            compiled_plan_id: input.compiled_plan_id,
            version_sequence,
            active: true,
            api_enabled: input.api_enabled,
            flow_schema_version: input.flow_schema_version.clone(),
            document_hash: input.document_hash.clone(),
            document_snapshot: input.document_snapshot.clone(),
            runtime_profile_snapshot: input.runtime_profile_snapshot.clone(),
            output_selector: input.output_selector.clone(),
            dependency_snapshot: input.dependency_snapshot.clone(),
            created_by: input.actor_user_id,
            created_at: OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(ordinal as i64),
        };
        inner
            .application_api_enabled
            .insert(input.application_id, input.api_enabled);
        inner
            .publications
            .insert(publication.id, publication.clone());

        Ok(publication)
    }

    async fn get_application_publication_version(
        &self,
        publication_id: Uuid,
    ) -> Result<Option<publications::ApplicationPublicationVersionRecord>> {
        Ok(self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .publications
            .get(&publication_id)
            .cloned())
    }

    async fn list_application_publication_versions(
        &self,
        application_id: Uuid,
    ) -> Result<Vec<publications::ApplicationPublicationVersionRecord>> {
        let mut publications = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .publications
            .values()
            .filter(|publication| publication.application_id == application_id)
            .cloned()
            .collect::<Vec<_>>();
        publications.sort_by(|left, right| {
            right
                .version_sequence
                .cmp(&left.version_sequence)
                .then(right.id.cmp(&left.id))
        });
        Ok(publications)
    }

    async fn load_active_application_publication(
        &self,
        application_id: Uuid,
    ) -> Result<Option<publications::ApplicationPublicationVersionRecord>> {
        Ok(self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .publications
            .values()
            .find(|publication| publication.application_id == application_id && publication.active)
            .cloned())
    }

    async fn set_application_api_enabled(
        &self,
        input: &SetApplicationApiEnabledInput,
    ) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned");
        if !inner.applications.contains_key(&input.application_id) {
            return Err(ControlPlaneError::NotFound("application").into());
        }
        inner
            .application_api_enabled
            .insert(input.application_id, input.api_enabled);
        Ok(())
    }
}
