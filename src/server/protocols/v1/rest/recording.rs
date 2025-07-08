use crate::device::manager::UuidWrapper;
use crate::device::recording::{RecordingManagerCommand, RecordingsManagerHandler};
use crate::server::protocols::v1::errors::Error;
use actix_web::Responder;
use chrono::{DateTime, Utc};
use mime_guess::from_path;
use paperclip::actix::{
    api_v2_operation, get, post,
    web::{self, HttpResponse, Json},
    Apiv2Schema,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::debug;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct McapFileInfo {
    pub file_name: String,
    pub file_size: u64,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum RecordingsManagerPostOptionsV1 {
    StartRecording,
    StopRecording,
    GetRecordingStatus,
}

#[api_v2_operation(tags("Recordings Server"))]
#[get("/recordings/list")]
async fn list_mcap_recordings(req: web::HttpRequest) -> Result<Json<Vec<McapFileInfo>>, Error> {
    let recordings_dir = Path::new("recordings");
    debug!("Listing MCAP files in directory: {:?}", recordings_dir);

    let show_detailed_listing = req
        .headers()
        .get("show-listing")
        .and_then(|h| h.to_str().ok())
        .map(|v| v == "?1")
        .unwrap_or(false);

    let mut files = Vec::new();

    if !recordings_dir.exists() {
        debug!("Creating recordings directory: {:?}", recordings_dir);
        if let Err(e) = fs::create_dir_all(recordings_dir) {
            debug!("Failed to create recordings directory: {:?}", e);
            return Ok(Json(files));
        }
    }

    match fs::read_dir(recordings_dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        debug!("Found entry: {:?}", path);

                        // Filter for .mcap files or show all files if detailed listing is requested
                        let is_mcap = path.extension().map_or(false, |ext| ext == "mcap");
                        let should_include = is_mcap || (show_detailed_listing && path.is_file());

                        if should_include {
                            match entry.metadata() {
                                Ok(metadata) => {
                                    let modified = metadata
                                        .modified()
                                        .ok()
                                        .and_then(|mtime| {
                                            DateTime::<Utc>::from(mtime).to_rfc3339().into()
                                        })
                                        .unwrap_or_else(|| "unknown".to_string());

                                    debug!("Adding file: {:?}", path.file_name());
                                    files.push(McapFileInfo {
                                        file_name: path
                                            .file_name()
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string(),
                                        file_size: metadata.len(),
                                        modified,
                                    });
                                }
                                Err(e) => debug!("Failed to get metadata for {:?}: {:?}", path, e),
                            }
                        }
                    }
                    Err(e) => debug!("Failed to read entry: {:?}", e),
                }
            }
        }
        Err(e) => {
            debug!("Failed to read recordings directory: {:?}", e);
            // Try to create the directory if it doesn't exist
            if recordings_dir.parent().is_some() {
                let _ = fs::create_dir_all(recordings_dir);
            }
        }
    }

    // Sort files by modification time (newest first) when detailed listing is enabled
    if show_detailed_listing {
        files.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    debug!(
        "Total files found: {} (MCAP filter: {})",
        files.len(),
        !show_detailed_listing
    );
    Ok(Json(files))
}

#[api_v2_operation(tags("Recordings Server"))]
#[get("/recordings/download/{file_name}")]
async fn download_mcap_file(
    file_name: web::Path<String>,
    req: web::HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let recordings_dir = Path::new("recordings");
    let file_path = recordings_dir.join(&*file_name);

    let canonical_recordings = match recordings_dir.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            debug!("Failed to canonicalize recordings directory");
            return HttpResponse::InternalServerError().body("Invalid recordings directory");
        }
    };

    let canonical_file = match file_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            debug!("File not found or inaccessible: {:?}", file_path);
            return HttpResponse::NotFound().body("File not found");
        }
    };

    if !canonical_file.starts_with(&canonical_recordings) {
        debug!("Attempted path traversal attack blocked: {:?}", file_path);
        return HttpResponse::Forbidden().body("Access denied");
    }

    if canonical_file.exists() && canonical_file.is_file() {
        match fs::read(&canonical_file) {
            Ok(data) => {
                let mime = from_path(&canonical_file).first_or_octet_stream();
                let content_type = mime.as_ref();

                let is_inline = req
                    .headers()
                    .get("prefer-inline")
                    .and_then(|h| h.to_str().ok())
                    .map(|v| v == "true")
                    .unwrap_or(false)
                    || query.get("inline").is_some();

                let disposition = if is_inline {
                    format!("inline; filename=\"{}\"", file_name)
                } else {
                    format!("attachment; filename=\"{}\"", file_name)
                };

                debug!(
                    "Serving file: {:?} (size: {} bytes, type: {})",
                    canonical_file,
                    data.len(),
                    content_type
                );

                HttpResponse::Ok()
                    .content_type(content_type)
                    .append_header(("Content-Disposition", disposition))
                    .append_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
                    .append_header(("Pragma", "no-cache"))
                    .append_header(("Expires", "0"))
                    .body(data)
            }
            Err(e) => {
                debug!("Failed to read file {:?}: {:?}", canonical_file, e);
                HttpResponse::InternalServerError().body("Failed to read file")
            }
        }
    } else {
        debug!("File not found or not a regular file: {:?}", canonical_file);
        HttpResponse::NotFound().body("File not found")
    }
}

#[api_v2_operation(tags("Recordings Manager"))]
#[get("recordings_manager/list")]
async fn recording_manager_get(
    recording_tx: web::Data<RecordingsManagerHandler>,
) -> Result<Json<crate::device::recording::Answer>, Error> {
    let request = RecordingManagerCommand::GetAllRecordingStatus;
    let answer = recording_tx.send(request).await?;
    Ok(Json(answer))
}

#[api_v2_operation(tags("Recordings Manager : Device"))]
#[post("recordings_manager/{device}/{selection}")]
async fn recording_manager_post(
    recording_tx: web::Data<RecordingsManagerHandler>,
    info: web::Path<(Uuid, RecordingsManagerPostOptionsV1)>,
) -> Result<Json<crate::device::recording::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request: RecordingManagerCommand = match request {
        RecordingsManagerPostOptionsV1::StartRecording => {
            RecordingManagerCommand::StartRecording(UuidWrapper { uuid })
        }
        RecordingsManagerPostOptionsV1::StopRecording => {
            RecordingManagerCommand::StopRecording(UuidWrapper { uuid })
        }
        RecordingsManagerPostOptionsV1::GetRecordingStatus => {
            RecordingManagerCommand::GetRecordingStatus(UuidWrapper { uuid })
        }
    };

    let answer = recording_tx.send(request).await?;
    Ok(Json(answer))
}

#[api_v2_operation(tags("Recording Manager: Request"))]
#[post("recordings_manager/request")]
async fn recordings_manager_post_request(
    manager_handler: web::Data<RecordingsManagerHandler>,
    json: web::Json<crate::device::recording::RecordingManagerCommand>,
) -> Result<Json<crate::device::recording::Answer>, Error> {
    let request = json.into_inner();
    let answer = manager_handler.send(request).await?;
    Ok(Json(answer))
}
