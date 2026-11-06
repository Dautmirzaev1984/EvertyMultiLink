use super::*;

pub const PROFILE_FORMAT: &str = "everty.multilink";
pub const PROFILE_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileServer {
    pub id: String,
    pub name: String,
    pub uri: String,
}

/// Transport document, deliberately independent of persisted desktop Settings.
/// HTTPS and a revocable subscription token authorize delivery, not obscurity.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDocument {
    pub format: String,
    pub version: u32,
    pub profile_id: String,
    pub name: String,
    pub revision: u64,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub servers: Vec<ProfileServer>,
}
impl ProfileDocument {
    pub fn decode(text: &str) -> Result<Self, String> {
        if text.len() > 4 * 1024 * 1024 {
            return Err("Конфигурация превышает 4 МБ".into());
        }
        let doc: Self =
            serde_json::from_str(text).map_err(|_| "Некорректный формат конфигурации Everty")?;
        if doc.format != PROFILE_FORMAT || doc.version != PROFILE_VERSION {
            return Err("Неподдерживаемая версия конфигурации Everty".into());
        }
        if doc.expires_at.is_some_and(|expiry| expiry <= now()) {
            return Err("Срок действия конфигурации истёк; обновите подписку".into());
        }
        if doc.profile_id.is_empty()
            || doc.profile_id.len() > 128
            || doc.name.chars().count() > 80
            || doc.name.chars().any(char::is_control)
            || doc.servers.is_empty()
            || doc.servers.len() > 2000
        {
            return Err("Некорректные параметры конфигурации Everty".into());
        }
        doc.clone().into_nodes()?;
        Ok(doc)
    }
    pub fn into_nodes(self) -> Result<Vec<Node>, String> {
        let mut ids = HashSet::new();
        let mut nodes = Vec::with_capacity(self.servers.len());
        for server in self.servers {
            if server.id.is_empty()
                || server.id.len() > 128
                || !ids.insert(server.id)
                || server.name.is_empty()
                || server.name.chars().count() > 120
                || server.name.chars().any(char::is_control)
                || !server.uri.starts_with("vless://")
                || server.uri.chars().any(char::is_whitespace)
            {
                return Err("Некорректный сервер в конфигурации Everty".into());
            }
            let mut node = parse_uri(&server.uri)?;
            node.name = server.name;
            nodes.push(node);
        }
        Ok(nodes)
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn own_format_is_strict_versioned_and_importable() {
        let mut doc = ProfileDocument {
            format: PROFILE_FORMAT.into(),
            version: 1,
            profile_id: "demo".into(),
            name: "Demo".into(),
            revision: 2,
            issued_at: now(),
            expires_at: None,
            servers: vec![ProfileServer {
                id: "node-1".into(),
                name: "DE-demo".into(),
                uri: "vless://00000000-0000-4000-8000-000000000001@example.invalid:443#ignored"
                    .into(),
            }],
        };
        let text = serde_json::to_string(&doc).unwrap();
        let nodes = import_subscription(&text).unwrap().nodes;
        assert_eq!(nodes[0].name, "DE-demo");
        doc.version = 99;
        assert!(import_subscription(&serde_json::to_string(&doc).unwrap()).is_err());
        doc.version = 1;
        doc.expires_at = Some(1);
        assert!(ProfileDocument::decode(&serde_json::to_string(&doc).unwrap()).is_err());
        doc.expires_at = None;
        doc.servers.push(doc.servers[0].clone());
        assert!(ProfileDocument::decode(&serde_json::to_string(&doc).unwrap()).is_err());
    }
}
