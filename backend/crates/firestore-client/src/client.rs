//! Firestore REST API client. Implements the operations every context needs:
//!   - get(collection, doc_id)
//!   - set(collection, doc_id, fields)  [upsert]
//!   - delete(collection, doc_id)
//!   - list(collection, prefix?, limit)
//!
//! Firestore documents are represented as `Document { id, fields }` where
//! `fields` is a serde_json Value. Higher-level adapters map domain types to
//! this shape.

use crate::{auth::TokenSource, error::FirestoreError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub type FirestoreValue = Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub fields: FirestoreValue,
}

#[derive(Clone)]
pub struct FirestoreClient {
    http: reqwest::Client,
    tokens: TokenSource,
    project_id: String,
    database_id: String,
    timeout: Duration,
}

impl FirestoreClient {
    #[must_use]
    pub fn new(project_id: impl Into<String>, tokens: TokenSource) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest"),
            tokens,
            project_id: project_id.into(),
            database_id: "(default)".into(),
            timeout: Duration::from_secs(10),
        }
    }

    #[must_use]
    pub fn with_database(mut self, database_id: impl Into<String>) -> Self {
        self.database_id = database_id.into();
        self
    }

    fn base(&self) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents",
            self.project_id, self.database_id
        )
    }

    async fn auth_header(&self) -> Result<(&'static str, String), FirestoreError> {
        let token = self.tokens.access_token().await?;
        Ok(("authorization", format!("Bearer {token}")))
    }

    pub async fn get(
        &self,
        collection: &str,
        doc_id: &str,
    ) -> Result<Option<Document>, FirestoreError> {
        let url = format!("{}/{collection}/{doc_id}", self.base());
        let (k, v) = self.auth_header().await?;
        let resp = tokio::time::timeout(self.timeout, self.http.get(&url).header(k, v).send())
            .await
            .map_err(|_| FirestoreError::Timeout)?
            .map_err(|e| FirestoreError::Http(e.to_string()))?;
        match resp.status().as_u16() {
            200 => {
                let body: FirestoreRawDoc = resp
                    .json()
                    .await
                    .map_err(|e| FirestoreError::Decode(e.to_string()))?;
                Ok(Some(body.into_document(doc_id.into())))
            }
            404 => Ok(None),
            s => Err(FirestoreError::HttpStatus {
                status: s,
                body: resp.text().await.unwrap_or_default(),
            }),
        }
    }

    pub async fn set(
        &self,
        collection: &str,
        doc_id: &str,
        fields: FirestoreValue,
    ) -> Result<(), FirestoreError> {
        let url = format!("{}/{collection}?documentId={doc_id}", self.base());
        let payload = json!({ "fields": to_firestore_fields(&fields) });
        let (k, v) = self.auth_header().await?;
        let resp = tokio::time::timeout(
            self.timeout,
            self.http.post(&url).header(k, v).json(&payload).send(),
        )
        .await
        .map_err(|_| FirestoreError::Timeout)?
        .map_err(|e| FirestoreError::Http(e.to_string()))?;
        match resp.status().as_u16() {
            200 => Ok(()),
            409 => {
                // Document exists — upsert via PATCH.
                let patch_url = format!("{}/{collection}/{doc_id}", self.base());
                let (k, v) = self.auth_header().await?;
                let resp = tokio::time::timeout(
                    self.timeout,
                    self.http
                        .patch(&patch_url)
                        .header(k, v)
                        .json(&payload)
                        .send(),
                )
                .await
                .map_err(|_| FirestoreError::Timeout)?
                .map_err(|e| FirestoreError::Http(e.to_string()))?;
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(FirestoreError::HttpStatus {
                        status: resp.status().as_u16(),
                        body: resp.text().await.unwrap_or_default(),
                    })
                }
            }
            s => Err(FirestoreError::HttpStatus {
                status: s,
                body: resp.text().await.unwrap_or_default(),
            }),
        }
    }

    pub async fn delete(&self, collection: &str, doc_id: &str) -> Result<(), FirestoreError> {
        let url = format!("{}/{collection}/{doc_id}", self.base());
        let (k, v) = self.auth_header().await?;
        let resp = tokio::time::timeout(self.timeout, self.http.delete(&url).header(k, v).send())
            .await
            .map_err(|_| FirestoreError::Timeout)?
            .map_err(|e| FirestoreError::Http(e.to_string()))?;
        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            Err(FirestoreError::HttpStatus {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            })
        }
    }

    pub async fn list(
        &self,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<Document>, FirestoreError> {
        let url = format!("{}/{collection}?pageSize={limit}", self.base());
        let (k, v) = self.auth_header().await?;
        let resp = tokio::time::timeout(self.timeout, self.http.get(&url).header(k, v).send())
            .await
            .map_err(|_| FirestoreError::Timeout)?
            .map_err(|e| FirestoreError::Http(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(FirestoreError::HttpStatus {
                status: resp.status().as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        let body: ListResponse = resp
            .json()
            .await
            .map_err(|e| FirestoreError::Decode(e.to_string()))?;
        Ok(body
            .documents
            .unwrap_or_default()
            .into_iter()
            .map(|raw| {
                let id = raw_doc_id(&raw.name);
                raw.into_document(id)
            })
            .collect())
    }
}

// ---- wire types ----

#[derive(Deserialize)]
struct FirestoreRawDoc {
    #[serde(default)]
    name: String,
    #[serde(default)]
    fields: Value,
}

#[derive(Deserialize)]
struct ListResponse {
    #[serde(default)]
    documents: Option<Vec<FirestoreRawDoc>>,
}

impl FirestoreRawDoc {
    fn into_document(self, id: String) -> Document {
        Document {
            id,
            fields: from_firestore_fields(&self.fields),
        }
    }
}

fn raw_doc_id(name: &str) -> String {
    name.rsplit('/').next().unwrap_or("").to_string()
}

// ---- field conversion ----
//
// Firestore wraps each value in a type tag: { stringValue, integerValue,
// booleanValue, doubleValue, mapValue, arrayValue, timestampValue, nullValue }.
// We translate between serde_json::Value (flat) and that wrapped form.

pub fn to_firestore_fields(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                out.insert(k.clone(), to_firestore_value(val));
            }
            Value::Object(out)
        }
        _ => to_firestore_value(v),
    }
}

fn to_firestore_value(v: &Value) -> Value {
    match v {
        Value::Null => json!({ "nullValue": null }),
        Value::Bool(b) => json!({ "booleanValue": b }),
        Value::Number(n) if n.is_i64() => {
            json!({ "integerValue": n.as_i64().unwrap_or(0).to_string() })
        }
        Value::Number(n) => json!({ "doubleValue": n.as_f64().unwrap_or(0.0) }),
        Value::String(s) => json!({ "stringValue": s }),
        Value::Array(a) => json!({
            "arrayValue": { "values": a.iter().map(to_firestore_value).collect::<Vec<_>>() }
        }),
        Value::Object(_) => json!({ "mapValue": { "fields": to_firestore_fields(v) } }),
    }
}

pub fn from_firestore_fields(v: &Value) -> Value {
    let Value::Object(map) = v else {
        return Value::Null;
    };
    let mut out = serde_json::Map::new();
    for (k, val) in map {
        out.insert(k.clone(), from_firestore_value(val));
    }
    Value::Object(out)
}

fn from_firestore_value(v: &Value) -> Value {
    let Value::Object(tag) = v else {
        return v.clone();
    };
    if let Some(s) = tag.get("stringValue") {
        s.clone()
    } else if let Some(b) = tag.get("booleanValue") {
        b.clone()
    } else if let Some(i) = tag.get("integerValue").and_then(Value::as_str) {
        Value::Number(
            i.parse::<i64>()
                .map(Into::into)
                .unwrap_or_else(|_| 0.into()),
        )
    } else if let Some(d) = tag.get("doubleValue") {
        d.clone()
    } else if tag.contains_key("nullValue") {
        Value::Null
    } else if let Some(arr) = tag.get("arrayValue").and_then(|a| a.get("values")) {
        let Value::Array(items) = arr else {
            return Value::Null;
        };
        Value::Array(items.iter().map(from_firestore_value).collect())
    } else if let Some(m) = tag.get("mapValue").and_then(|m| m.get("fields")) {
        from_firestore_fields(m)
    } else if let Some(t) = tag.get("timestampValue") {
        t.clone()
    } else {
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_primitives() {
        let input = json!({
            "name": "alice",
            "age": 30,
            "active": true,
            "score": 0.95,
            "ghost": null,
            "tags": ["a", "b"],
            "meta": { "nested": "x" }
        });
        let wire = to_firestore_fields(&input);
        let back = from_firestore_fields(&wire);
        assert_eq!(back, input);
    }

    #[test]
    fn integer_is_stringified_on_wire() {
        let v = to_firestore_value(&Value::from(42));
        assert_eq!(v["integerValue"], Value::String("42".into()));
    }

    #[test]
    fn string_is_tagged_on_wire() {
        let v = to_firestore_value(&Value::from("hi"));
        assert_eq!(v["stringValue"], Value::String("hi".into()));
    }
}
