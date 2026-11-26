use serde_json::{Map, Value, json};

#[derive(Clone, Debug, PartialEq)]
pub struct FormPart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FormBody {
    pub parts: Vec<FormPart>,
    pub headers: Vec<(String, String)>,
}

fn escape(s: &str) -> String {
    s.replace('"', "%22").replace('\r', "%0D").replace('\n', "%0A")
}

fn crlf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n")
}

impl FormBody {
    pub fn text(mut self, name: &str, value: &str) -> Self {
        self.parts.push(FormPart { name: name.to_owned(), filename: None, content_type: None, data: crlf(value).into_bytes() });
        self
    }

    pub fn file(mut self, name: &str, filename: &str, content_type: &str, data: Vec<u8>) -> Self {
        self.parts.push(FormPart { name: name.to_owned(), filename: Some(filename.to_owned()), content_type: Some(content_type.to_owned()), data });
        self
    }

    pub fn header(mut self, name: &str, value: String) -> Self {
        self.headers.push((name.to_owned(), value));
        self
    }

    pub fn boundary() -> String {
        const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let tail: String = (0..16).map(|_| ALPHA[rand::random_range(0..ALPHA.len())] as char).collect();
        format!("----WebKitFormBoundary{tail}")
    }

    pub fn content_type(boundary: &str) -> String {
        format!("multipart/form-data; boundary={boundary}")
    }

    pub fn encode(&self, boundary: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for p in &self.parts {
            out.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{}\"", escape(&p.name)).as_bytes());
            if let Some(f) = &p.filename {
                out.extend_from_slice(format!("; filename=\"{}\"", escape(f)).as_bytes());
            }
            out.extend_from_slice(b"\r\n");
            if let Some(ct) = &p.content_type {
                out.extend_from_slice(format!("Content-Type: {ct}\r\n").as_bytes());
            }
            out.extend_from_slice(b"\r\n");
            out.extend_from_slice(&p.data);
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        out
    }
}

pub fn md5_hex(bytes: &[u8]) -> String {
    use md5::Digest;
    md5::Md5::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn original_md5_header(md5: Option<&str>) -> Option<String> {
    md5.filter(|m| m.len() == 32 && m.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))).map(str::to_owned)
}

#[derive(Clone, Debug, PartialEq)]
pub struct StickerUpload {
    pub name: String,
    pub tags: String,
    pub description: String,
    pub filename: String,
    pub content_type: String,
    pub file: Vec<u8>,
    pub original_md5: Option<String>,
}

impl StickerUpload {
    pub fn form(&self) -> FormBody {
        let mut f = FormBody::default().text("name", &self.name).text("tags", &self.tags).text("description", &self.description).file("file", &self.filename, &self.content_type, self.file.clone());
        if let Some(h) = original_md5_header(self.original_md5.as_deref()) {
            f = f.header("X-Discord-Original-MD5", h);
        }
        f
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundUpload {
    pub name: String,
    pub sound: String,
    pub volume: f64,
    pub emoji_id: Option<String>,
    pub emoji_name: Option<String>,
}

impl SoundUpload {
    pub fn body(&self) -> Value {
        let mut b = Map::new();
        b.insert("name".into(), json!(self.name));
        b.insert("sound".into(), json!(self.sound));
        let v = (self.volume * 100.).round() / 100.;
        b.insert("volume".into(), if v.fract() == 0. { json!(v as i64) } else { json!(v) });
        if let Some(id) = &self.emoji_id {
            b.insert("emoji_id".into(), json!(id));
        }
        if let Some(n) = &self.emoji_name {
            b.insert("emoji_name".into(), json!(n));
        }
        Value::Object(b)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn sticker() -> StickerUpload {
        StickerUpload {
            name: "cat hug".into(),
            tags: "😺".into(),
            description: "two cats".into(),
            filename: "cat.png".into(),
            content_type: "image/png".into(),
            file: vec![0x89, b'P', b'N', b'G'],
            original_md5: Some(md5_hex(b"original")),
        }
    }

    pub fn sound() -> SoundUpload {
        SoundUpload { name: "ding".into(), sound: "data:audio/ogg;base64,AA==".into(), volume: 0.85, emoji_id: None, emoji_name: Some("🔔".into()) }
    }

    #[test]
    fn sticker_form_matches_the_browser_body() {
        let f = sticker().form();
        let b = "----WebKitFormBoundaryAbCdEfGhIjKlMnOp";
        let mut want = Vec::new();
        want.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\ncat hug\r\n").as_bytes());
        want.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"tags\"\r\n\r\n😺\r\n").as_bytes());
        want.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"description\"\r\n\r\ntwo cats\r\n").as_bytes());
        want.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cat.png\"\r\nContent-Type: image/png\r\n\r\n").as_bytes());
        want.extend_from_slice(&[0x89, b'P', b'N', b'G']);
        want.extend_from_slice(format!("\r\n--{b}--\r\n").as_bytes());
        assert_eq!(String::from_utf8_lossy(&f.encode(b)), String::from_utf8_lossy(&want));
        assert_eq!(f.encode(b), want);
        assert_eq!(f.headers, vec![("X-Discord-Original-MD5".to_owned(), md5_hex(b"original"))]);
        assert_eq!(FormBody::content_type(b), format!("multipart/form-data; boundary={b}"));
    }

    #[test]
    fn form_escapes_like_chromium() {
        let f = FormBody::default().text("a\"b", "x\ny").file("f", "a\"\n.png", "image/png", vec![]);
        let s = String::from_utf8(f.encode("B")).unwrap();
        assert!(s.contains("name=\"a%22b\"\r\n\r\nx\r\ny\r\n"));
        assert!(s.contains("filename=\"a%22%0A.png\""));
    }

    #[test]
    fn boundary_and_md5_shapes() {
        let b = FormBody::boundary();
        assert_eq!(b.len(), 38);
        assert!(b.starts_with("----WebKitFormBoundary"));
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(original_md5_header(Some("XYZ")), None);
        let mut s = sticker();
        s.original_md5 = None;
        assert!(s.form().headers.is_empty());
    }

    #[test]
    fn sound_body_matches_the_client() {
        assert_eq!(sound().body(), json!({ "name": "ding", "sound": "data:audio/ogg;base64,AA==", "volume": 0.85, "emoji_name": "🔔" }));
        let custom = SoundUpload { volume: 1.0, emoji_id: Some("42".into()), emoji_name: None, ..sound() };
        let b = custom.body();
        assert_eq!(b, json!({ "name": "ding", "sound": "data:audio/ogg;base64,AA==", "volume": 1, "emoji_id": "42" }));
        assert!(b["volume"].is_i64());
        assert!(b.get("emoji_name").is_none());
    }
}
