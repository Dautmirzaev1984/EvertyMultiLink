use multilink_core::{
    ClientAccess, PendingActivation, ProfileDocument, ACCESS_CONFIG, ACCESS_SERVER,
};
use reqwest::blocking::{Client, Response};
use std::{io::Read, time::Duration};
pub enum AccessError {
    Denied,
    Message(String),
}
fn client() -> Result<Client, AccessError> {
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AccessError::Message("Не удалось подготовить запрос доступа".into()))
}
fn response(response: Result<Response, reqwest::Error>) -> Result<String, AccessError> {
    let response = response.map_err(|_| {
        AccessError::Message(
            "Сервер доступа недоступен. Проверьте интернет и повторите попытку".into(),
        )
    })?;
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Err(AccessError::Denied),
        429 => {
            return Err(AccessError::Message(
                "Слишком много попыток. Попробуйте позже; ключ не изменён".into(),
            ))
        }
        503 => {
            return Err(AccessError::Message(
                "Серверы временно недоступны. Повторите обновление позже".into(),
            ))
        }
        _ => {
            return Err(AccessError::Message(
                "Сервер пока не готов выдать доступ. Повторите попытку позже".into(),
            ))
        }
    }
    let mut text = String::new();
    response
        .take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(|_| {
            AccessError::Message("Ответ прерван. Повторите запрос с тем же ключом".into())
        })?;
    if text.len() > 4 * 1024 * 1024 {
        return Err(AccessError::Message(
            "Ответ сервера превышает допустимый размер".into(),
        ));
    }
    Ok(text)
}
pub fn activate_access(
    pending: &PendingActivation,
) -> Result<(String, ProfileDocument), AccessError> {
    let text = response(
        client()?
            .post(format!("{ACCESS_SERVER}/v1/activate"))
            .json(&serde_json::json!({"code":pending.code,"nonce":pending.nonce}))
            .send(),
    )?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| AccessError::Message("Некорректный ответ сервера доступа".into()))?;
    let token = value["access_token"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| AccessError::Message("Сервер не выдал токен доступа".into()))?
        .to_owned();
    let profile =
        ProfileDocument::decode(&value["profile"].to_string()).map_err(AccessError::Message)?;
    Ok((token, profile))
}
pub fn refresh_access(access: &ClientAccess) -> Result<(String, ProfileDocument), AccessError> {
    let text = response(
        client()?
            .get(ACCESS_CONFIG)
            .bearer_auth(&access.token)
            .send(),
    )?;
    let doc = ProfileDocument::decode(&text).map_err(AccessError::Message)?;
    if doc.profile_id != access.profile_id {
        return Err(AccessError::Message(
            "Сервер вернул другой профиль доступа".into(),
        ));
    }
    Ok((access.token.clone(), doc))
}
