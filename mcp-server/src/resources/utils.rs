use rmcp::model::{AnnotateAble, RawResource, Resource};
use rmcp::model::{RawResourceTemplate, ResourceTemplate};

use urlencoding::decode;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    pub fn resource_template(
        uri_template: impl Into<String>,
        name: impl Into<String>,
        title: Option<String>,
        description: Option<String>,
        mime_type: Option<String>,
    ) -> ResourceTemplate {
        RawResourceTemplate {
            uri_template: uri_template.into(),
            name: name.into(),
            title,
            description,
            mime_type,
        }
        .no_annotation()
    }

    pub fn resource(
        uri: impl Into<String>,
        name: impl Into<String>,
        title: Option<String>,
        description: Option<String>,
        mime_type: Option<String>,
    ) -> Resource {
        RawResource {
            uri: uri.into(),
            name: name.into(),
            title,
            description,
            size: None,
            mime_type,
            icons: None,
        }
        .no_annotation()
    }

    pub fn decode_uri_path(path: &str) -> String {
        decode(path)
            .map(|cow| cow.into_owned())
            .unwrap_or_else(|_| path.to_string())
    }
}
