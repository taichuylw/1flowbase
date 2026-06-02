use super::*;

#[async_trait]

impl ApiKeyRepository for ApplicationPublicApiTestRepository {
    async fn create_api_key(&self, input: &CreateApiKeyInput) -> Result<domain::ApiKeyRecord> {
        let now = OffsetDateTime::now_utc();
        let api_key = domain::ApiKeyRecord {
            id: input.id,
            name: input.name.clone(),
            token_hash: input.token_hash.clone(),
            token_prefix: input.token_prefix.clone(),
            key_kind: input.key_kind,
            application_id: input.application_id,
            creator_user_id: input.creator_user_id,
            tenant_id: input.tenant_id,
            scope_kind: input.scope_kind,
            scope_id: input.scope_id,
            enabled: input.enabled,
            expires_at: input.expires_at,
            last_used_at: None,
            created_at: now,
            updated_at: now,
        };
        self.inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .api_keys
            .insert(api_key.id, api_key.clone());
        Ok(api_key)
    }

    async fn replace_api_key_data_model_permissions(
        &self,
        api_key_id: Uuid,
        permissions: &[UpsertApiKeyDataModelPermissionInput],
    ) -> Result<Vec<domain::ApiKeyDataModelPermissionRecord>> {
        let records = permissions
            .iter()
            .map(|permission| domain::ApiKeyDataModelPermissionRecord {
                api_key_id,
                data_model_id: permission.data_model_id,
                allow_list: permission.allow_list,
                allow_get: permission.allow_get,
                allow_create: permission.allow_create,
                allow_update: permission.allow_update,
                allow_delete: permission.allow_delete,
            })
            .collect::<Vec<_>>();
        self.inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .permissions
            .insert(api_key_id, records.clone());
        Ok(records)
    }

    async fn find_api_key_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<domain::ApiKeyRecord>> {
        Ok(self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .api_keys
            .values()
            .find(|api_key| api_key.token_hash == token_hash)
            .cloned())
    }

    async fn mark_api_key_used(&self, api_key_id: Uuid) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned");
        if inner.fail_mark_api_key_used {
            anyhow::bail!("mark_api_key_used failed for test");
        }
        let api_key = inner
            .api_keys
            .get_mut(&api_key_id)
            .ok_or(ControlPlaneError::NotFound("api_key"))?;
        api_key.last_used_at = Some(OffsetDateTime::now_utc());
        *inner
            .api_key_last_used_write_counts
            .entry(api_key_id)
            .or_default() += 1;
        Ok(())
    }

    async fn list_application_api_keys(
        &self,
        application_id: Uuid,
        creator_user_id: Uuid,
    ) -> Result<Vec<domain::ApiKeyRecord>> {
        let mut keys = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .api_keys
            .values()
            .filter(|api_key| api_key.key_kind == domain::ApiKeyKind::ApplicationApiKey)
            .filter(|api_key| api_key.application_id == Some(application_id))
            .filter(|api_key| api_key.creator_user_id == creator_user_id)
            .filter(|api_key| api_key.enabled)
            .cloned()
            .collect::<Vec<_>>();
        keys.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then(right.id.cmp(&left.id))
        });
        Ok(keys)
    }

    async fn revoke_application_api_key(
        &self,
        api_key_id: Uuid,
        application_id: Uuid,
        creator_user_id: Uuid,
    ) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned");
        let can_delete = inner
            .api_keys
            .get(&api_key_id)
            .filter(|api_key| api_key.key_kind == domain::ApiKeyKind::ApplicationApiKey)
            .filter(|api_key| api_key.application_id == Some(application_id))
            .filter(|api_key| api_key.creator_user_id == creator_user_id)
            .is_some();
        if !can_delete {
            return Err(ControlPlaneError::NotFound("application_api_key").into());
        }
        inner.api_keys.remove(&api_key_id);
        Ok(())
    }

    async fn list_api_key_data_model_permissions(
        &self,
        api_key_id: Uuid,
    ) -> Result<Vec<domain::ApiKeyDataModelPermissionRecord>> {
        Ok(self
            .inner
            .lock()
            .expect("application public api test repo mutex poisoned")
            .permissions
            .get(&api_key_id)
            .cloned()
            .unwrap_or_default())
    }
}
