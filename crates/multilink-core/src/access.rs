use super::*;
pub const ACCESS_SERVER: &str = "https://multilink.tolkovmeste.com";
pub const ACCESS_CONFIG: &str = "https://multilink.tolkovmeste.com/v1/config";
pub const ACCESS_ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
#[derive(Clone, Serialize, Deserialize)]
pub struct PendingActivation {
    pub code: String,
    pub nonce: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ClientAccess {
    #[serde(default)]
    pub blocked: bool,
    pub subscription_id: u64,
    pub profile_id: String,
    pub name: String,
    pub token: String,
    pub expires_at: u64,
}
pub fn access_code(input: &str) -> Result<String, String> {
    let code: String = input
        .trim()
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if code.len() != 6 || !code.bytes().all(|b| ACCESS_ALPHABET.contains(&b)) {
        return Err("Введите 6 символов ключа: латинские буквы и цифры, без 0, 1, I и O".into());
    }
    Ok(code)
}
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn exclude_expired_access(settings: &mut Settings) {
    if let Some(access) = &settings.access {
        if access.blocked || access.expires_at <= unix_now() {
            for node in &mut settings.nodes {
                if !node.manually_added && node.subscription_ids == [access.subscription_id] {
                    node.enabled = false;
                }
            }
        }
    }
}
pub fn install_access(
    settings: &mut Settings,
    token: String,
    document: ProfileDocument,
) -> Result<(), String> {
    let expiry = document
        .expires_at
        .filter(|expiry| *expiry > unix_now())
        .ok_or("Сервер не передал действительный срок доступа")?;
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Некорректный ответ сервера доступа".into());
    }
    let profile_id = document.profile_id.clone();
    let name = document.name.clone();
    let nodes = ProfileDocument::decode(
        &serde_json::to_string(&document).map_err(|_| "Ошибка конфигурации доступа")?,
    )?
    .into_nodes()?;
    let id = update_subscription(
        settings,
        settings.access.as_ref().map(|a| a.subscription_id),
        ACCESS_CONFIG,
        nodes,
    )?;
    if let Some(source) = settings.subscriptions.iter_mut().find(|s| s.id == id) {
        if source.name == subscription_host(ACCESS_CONFIG) {
            source.name = "Everty · доступ по ключу".into();
        }
    }
    settings.access = Some(ClientAccess {
        blocked: false,
        subscription_id: id,
        profile_id,
        name,
        token,
        expires_at: expiry,
    });
    settings.pending_activation = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_code_normalizes_paste_and_rejects_confusable_characters() {
        assert_eq!(access_code(" ab7-k9x ").unwrap(), "AB7K9X");
        for code in ["ABCDEF7", "123456", "OOOIII", "АВ7К9Х"] {
            assert!(access_code(code).is_err());
        }
    }
    #[test]
    fn install_is_atomic_and_expiration_excludes_only_exclusive_managed_nodes() {
        let mut settings = Settings::default();
        let doc = ProfileDocument::decode(include_str!("../fixtures/profile.example.everty.json"))
            .unwrap();
        let mut doc = doc;
        doc.expires_at = Some(unix_now() + 86400);
        let before = serde_json::to_string(&settings).unwrap();
        assert!(install_access(&mut settings, "bad-token".into(), doc.clone()).is_err());
        assert_eq!(serde_json::to_string(&settings).unwrap(), before);
        install_access(&mut settings, "a".repeat(64), doc).unwrap();
        assert_eq!(settings.subscriptions[0].url, ACCESS_CONFIG);
        let mut manual = settings.nodes[0].clone();
        manual.manually_added = true;
        manual.name = "Manual".into();
        settings.nodes.push(manual);
        let mut shared = settings.nodes[0].clone();
        shared.subscription_ids.push(999);
        shared.name = "Shared".into();
        settings.nodes.push(shared);
        settings.access.as_mut().unwrap().expires_at = 1;
        exclude_expired_access(&mut settings);
        assert!(!settings.nodes[0].enabled);
        assert!(settings.nodes[1].enabled);
        assert!(settings.nodes[2].enabled);
        let exported = RoutingConfig::from_settings(&settings).encode().unwrap();
        assert!(!exported.contains(&"a".repeat(64)));
    }
}
