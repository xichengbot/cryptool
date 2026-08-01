use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey, pkcs8::DecodePrivateKey};

use crate::error::AppError;

/// 使用 Ed25519 PKCS#8 PEM 私钥为 canonical query 生成 Base64 签名。
///
/// 调用方必须传入已完成参数排序和编码的 query 字符串；本函数不会修改
/// 待签名内容。
pub fn sign_query(private_key_pem: &str, query: &str) -> Result<String, AppError> {
    let signing_key = SigningKey::from_pkcs8_pem(private_key_pem)
        .map_err(|err| AppError::Config(format!("invalid ed25519 private key: {err}")))?;
    let signature = signing_key.sign(query.as_bytes());
    Ok(STANDARD.encode(signature.to_bytes()))
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{SigningKey, pkcs8::EncodePrivateKey};

    use super::sign_query;

    #[test]
    fn signs_query_with_ed25519_private_key() {
        let private_key = SigningKey::from_bytes(&[7_u8; 32]);
        let private_key_pem = private_key.to_pkcs8_pem(Default::default()).unwrap();

        let signature = sign_query(
            private_key_pem.as_str(),
            "symbol=BTCUSDT&timestamp=1499827319559",
        )
        .unwrap();

        assert_eq!(
            signature,
            "7QuaYH2cHvrnNPBz4zKJFD4jRmn83V7zT7gVN0zi4Ne2ZheSNXwgkiDG1DILjMS7FSUmA9IT212qLNTv7URlAQ=="
        );
    }
}
