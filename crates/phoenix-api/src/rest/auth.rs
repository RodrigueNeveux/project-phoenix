//! Extra auth endpoints: SMS / backup-code MFA and remote-auth (QR) login.

use reqwest::Method;
use serde::Deserialize;
use serde_json::json;

use super::{Body, RestClient};
use crate::error::Result;

#[derive(Deserialize)]
struct TokenBody {
    token: String,
}

impl RestClient {
    /// `POST /auth/mfa/sms/send`: texts a code, returns the redacted phone.
    pub async fn login_mfa_sms_send(&self, ticket: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct P {
            #[serde(default)]
            phone: String,
        }
        let p: P = self.call(Method::POST, "/auth/mfa/sms/send", Body::Json(json!({ "ticket": ticket }))).await?;
        Ok(p.phone)
    }

    /// `POST /auth/mfa/sms`
    pub async fn login_mfa_sms(&self, ticket: &str, code: &str) -> Result<String> {
        let t: TokenBody = self
            .call(
                Method::POST,
                "/auth/mfa/sms",
                Body::Json(json!({ "code": code, "ticket": ticket, "login_source": null, "gift_code_sku_id": null })),
            )
            .await?;
        Ok(t.token)
    }

    /// Backup codes go through the TOTP endpoint (8 characters, no dash).
    pub async fn login_mfa_backup(&self, ticket: &str, code: &str) -> Result<String> {
        let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        self.login_mfa_totp(ticket, &code).await
    }

    /// `POST /users/@me/remote-auth/login`: exchange a remote-auth ticket for
    /// the RSA-encrypted token (base64).
    pub async fn remote_auth_login(&self, ticket: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct E {
            encrypted_token: String,
        }
        let e: E = self
            .call(Method::POST, "/users/@me/remote-auth/login", Body::Json(json!({ "ticket": ticket })))
            .await?;
        Ok(e.encrypted_token)
    }
}
