use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use lazy_static::lazy_static;
use thiserror::Error;
use xml::attribute::OwnedAttribute;
use xml::common::{Position, TextPosition};
use xml::EventReader;
use xml::name::OwnedName;
use xml::reader::{ErrorKind, XmlEvent};

use crate::{ConstraintViolationAction, CoreApi, DatabaseType, DockerConnectionInfo, DockerStepProvider, ImplicitDockerStepPosition, Location, parse_docker_image, TableConstraintType};
use rapid_utils::http_utils::HttpMethod;
use rapid_fs::vfs::Vfs;
use rapid_fs::vfs::BoundVfs;
use rapid_utils::err::{ErrorCode, HttpError};
use rapid_utils::http as http;

pub type Result<T> = std::result::Result<T, HamlError>;
lazy_static!{
static ref HAML_CODE_UNKNOWN_ATTR: ErrorCode =
    ErrorCode::new("haml_unknown_attr", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_INVALID_PROVIDER: ErrorCode = ErrorCode::new(
    "haml_invalid_provider",
    http::status::StatusCode::BAD_REQUEST,
);
static ref HAML_CODE_INVALID_STEP_LOC: ErrorCode = ErrorCode::new(
    "haml_invalid_step_loc",
    http::status::StatusCode::BAD_REQUEST,
);
static ref HAML_CODE_MISSING_IMPORT: ErrorCode =
    ErrorCode::new("haml_missing_import", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_UNKNOWN_WELL_KNOWN_TYPE: ErrorCode = ErrorCode::new(
    "haml_unknown_well_known_type",
    http::status::StatusCode::BAD_REQUEST,
);
static ref HAML_CODE_UNSUPPORTED_CHILD: ErrorCode = ErrorCode::new(
    "haml_unsupported_child",
    http::status::StatusCode::BAD_REQUEST,
);
static ref HAML_CODE_CANNOT_REPEAT: ErrorCode =
    ErrorCode::new("haml_cannot_repeat", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_UNKNOWN_EL: ErrorCode = ErrorCode::new(
    "haml_unknown_element",
    http::status::StatusCode::BAD_REQUEST,
);
static ref HAML_CODE_XML_SYNTAX: ErrorCode =
    ErrorCode::new("haml_xml_syntax", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_XML_IO: ErrorCode =
    ErrorCode::new("haml_xml_io", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_XML_UTF8: ErrorCode =
    ErrorCode::new("haml_xml_utf8", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_XML_EOF: ErrorCode =
    ErrorCode::new("haml_xml_eof", http::status::StatusCode::BAD_REQUEST);
static ref HAML_CODE_NO_ROOT: ErrorCode =
    ErrorCode::new("haml_no_root", http::status::StatusCode::BAD_REQUEST);
}
const EL_TABLE: &str = "table";
const EL_TABLES: &str = "tables";
const EL_APIS: &str = "apis";
// const EL_API: &str = "api";
const EL_DOCUMENT: &str = "document";
const EL_COLUMN: &str = "column";
const EL_COLUMN_PIPELINE: &str = "pipeline";
const EL_PIPELINE_ARGS: &str = "args";
const EL_PIPELINE_WRITE: &str = "write";
const EL_PIPELINE_READ: &str = "read";
const EL_HYPI: &str = "hypi";
const EL_MAPPING: &str = "mapping";
const EL_GLOBAL_OPTIONS: &str = "global-options";
const EL_CORE_API: &str = "core-api";
const EL_REST: &str = "rest";
const EL_ENDPOINT: &str = "endpoint";
const EL_QUERY_OPTIONS_RESPONSE: &str = "response";
const EL_PIPELINE: &str = "pipeline";
const EL_DB: &str = "db";
const EL_SCHEMA: &str = "schema";
const EL_ENV: &str = "env";
const EL_SQL: &str = "sql";
const EL_FN: &str = "fn";
const EL_STEP: &str = "step";
const EL_STEP_BUILDER: &str = "step-builder";
const EL_WEBSOCKET: &str = "websocket";
const EL_SCRIPT: &str = "script";
const EL_CALL: &str = "call";
const EL_GRAPHQL: &str = "graphql";
const EL_JOB: &str = "job";
const EL_META: &str = "meta";
const EL_PAIR: &str = "pair";
const EL_CONSTRAINT: &str = "constraint";
const EL_PROVIDER: &str = "provider";
const CORE_API_REGISTER: &str = "register";
const CORE_API_LOGIN_BY_EMAIL: &str = "login-by-email";
const CORE_API_LOGIN_BY_USERNAME: &str = "login-by-username";
const CORE_API_OAUTH: &str = "oauth";
const CORE_API_PASSWORD_RESET_TRIGGER: &str = "password-reset-trigger";
const CORE_API_PASSWORD_RESET: &str = "password-reset";
const CORE_API_VERIFY_ACCOUNT: &str = "verify-account";
const CORE_API_MAGIC_LINK: &str = "magic-link";
const CORE_API_2FA_EMAIL: &str = "2fa-email";
const CORE_API_2FA_SMS: &str = "2fa-sms";
const CORE_API_2FA_STEP2: &str = "2fa-step2";
const CORE_API_2FA_TOTP: &str = "2fa-totp";
const ATTR_NAME: &str = "name";
const ATTR_COLUMNS: &str = "columns";
const ATTR_DB_NAME: &str = "db_name";
const ATTR_HOST: &str = "host";
const ATTR_PORT: &str = "port";
const ATTR_USERNAME: &str = "username";
const ATTR_PASSWORD: &str = "password";
const ATTR_OPTIONS: &str = "options";
const ATTR_ASYNC: &str = "async";
const ATTR_DB: &str = "db";
const ATTR_LABEL: &str = "label";
const ATTR_SOURCE: &str = "source";
const ATTR_BASE: &str = "base";
// const ATTR_TABLE: &str = "table";
// const ATTR_COLUMN: &str = "column";
// const ATTR_ORDER: &str = "order";
// const ATTR_ASC: &str = "asc";
// const ATTR_DESC: &str = "desc";
const ATTR_PK: &str = "primary_key";
const ATTR_NULLABLE: &str = "nullable";
const ATTR_TYPE: &str = "type";
const ATTR_UNIQUE: &str = "unique";
const ATTR_DEFAULT: &str = "default";
const ATTR_KEY: &str = "key";
const ATTR_VALUE: &str = "value";
const ATTR_FROM: &str = "from";
const ATTR_ENABLE_SUBSCRIPTIONS: &str = "enable-subscriptions";
const ATTR_TO: &str = "to";
// const ATTR_JOIN: &str = "join";
const ATTR_IMPORT: &str = "import";
const ATTR_TARGET: &str = "target";
const ATTR_PATH: &str = "path";
const ATTR_PRODUCES: &str = "produces";
const ATTR_ACCEPTS: &str = "accepts";
// const ATTR_FIELD: &str = "field";
// const ATTR_OP: &str = "op";
const ATTR_STATUS: &str = "status";
const ATTR_WHEN: &str = "when";
const ATTR_YIELD: &str = "yield";
const ATTR_PUBLIC: &str = "public";
const ATTR_PIPELINE: &str = "pipeline";
const ATTR_INTERVAL_FREQUENCY: &str = "intervalfrequency";
const ATTR_INTERVAL: &str = "interval";
const ATTR_START: &str = "start";
const ATTR_END: &str = "end";
const ATTR_ENABLED: &str = "enabled";
const ATTR_REPEATS: &str = "repeats";
const ATTR_METHOD: &str = "method";
const ATTR_VERSION: &str = "version";
const ATTR_PROVIDER: &str = "provider";
const ATTR_BEFORE: &str = "before";
const ATTR_AFTER: &str = "after";
const ATTR_IMAGE: &str = "image";
const COL_TYPE_TEXT: &str = "text";
const COL_TYPE_INT: &str = "int";
const COL_TYPE_BIGINT: &str = "bigint";
const COL_TYPE_FLOAT: &str = "float";
const COL_TYPE_DOUBLE: &str = "double";
const COL_TYPE_TIMESTAMP: &str = "timestamp";
const COL_TYPE_BOOL: &str = "boolean";
const COL_TYPE_BYTEA: &str = "bytea";
const FK_TYPE_FOREIGN: &str = "foreign_key";
const FK_TYPE_UNIQUE: &str = "unique";
const ATTR_ON_DELETE: &str = "on_delete";
const ATTR_ON_UPDATE: &str = "on_update";

lazy_static! {
    static ref IGNORED_ATTRS: Vec<&'static str> = vec!["xmlns", "schemaLocation"];
}

type NodePtr<T> = Rc<RefCell<T>>;

pub fn new_node_ptr<T>(val: T) -> NodePtr<T> {
    NodePtr::new(RefCell::new(val))
}

#[derive(Debug, Error)]
pub enum HamlError {
    #[error("Invalid HAML. {0:?}")]
    ParseErr(ParseErr),
    // #[error("{0}")]
    // X(serde_xml_rs::Error),
    #[error("{msg}")]
    Semantics {
        msg: String,
        code: ErrorCode,
        ctx: Option<HashMap<String, String>>,
    },
}

impl From<HamlError> for HttpError {
    fn from(value: HamlError) -> Self {
        match value {
            HamlError::ParseErr(e) => HttpError {
                code: e.code,
                message: e.message,
                context: Some(HashMap::from([
                    ("line".to_owned(), e.line.to_string()),
                    ("column".to_owned(), e.column.to_string()),
                    ("element".to_owned(), e.element),
                    ("file".to_owned(), e.file),
                ])),
            },
            HamlError::Semantics { msg, code, ctx } => HttpError {
                code,
                message: msg.to_owned(),
                context: ctx,
            },
        }
    }
}

#[derive(Error, Debug)]
pub struct ParseErr {
    pub file: String,
    pub line: u64,
    pub column: u64,
    pub code: ErrorCode,
    pub element: String,
    pub message: String,
}

impl Display for ParseErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code.to_string().as_str())?;
        f.write_str(",")?;
        f.write_str(self.element.as_str())?;
        f.write_str(",")?;
        f.write_str(self.message.as_str())
    }
}

pub struct ParsedTablePtr(NodePtr<ParsedTable>);
//we know we only read from ParsedTablePtr so it is safe to send between threads
unsafe impl Sync for ParsedTablePtr {}
unsafe impl Send for ParsedTablePtr {}

impl Deref for ParsedTablePtr {
    type Target = NodePtr<ParsedTable>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ParsedSchemaPtr(NodePtr<ParsedTable>);
//we know we only read from SchemaPtr so it is safe to send between threads
unsafe impl Sync for ParsedSchemaPtr {}
unsafe impl Send for ParsedSchemaPtr {}

impl Deref for ParsedSchemaPtr {
    type Target = NodePtr<ParsedTable>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub enum ParsedHypiSchemaElement {
    ParsedDocument(NodePtr<ParsedDocument>),
    ParsedTables(NodePtr<ParsedTables>),
    ParsedTable(NodePtr<ParsedTable>),
    Column(NodePtr<ParsedColumn>),
    Apis(NodePtr<ParsedApis>),
    ColumnPipeline(NodePtr<ParsedColumnPipeline>),
    ColumnPipelineArgs(NodePtr<ParsedColumnPipelineArgs>),
    ColumnPipelineWrite(NodePtr<ParsedColumnPipelineWrite>),
    ColumnPipelineRead(NodePtr<ParsedColumnPipelineRead>),
    Hypi(NodePtr<ParsedHypi>),
    Mapping(NodePtr<ParsedMapping>),
    ApiGlobalOptions(NodePtr<ParsedGlobalOptions>),
    ApiCoreApi(NodePtr<ParsedCoreApiName>),
    ApiRest(NodePtr<ParsedRest>),
    ApiEndpoint(NodePtr<ParsedEndpoint>),
    ApiEndpointResponse(NodePtr<ParsedEndpointResponse>),
    ApiEndpointSql(NodePtr<ParsedEndpointSql>),
    ApiEndpointScript(NodePtr<ParsedEndpointScript>),
    ApiEndpointCall(NodePtr<ParsedCall>),
    ApiEndpointFn(NodePtr<ParsedEndpointFn>),
    DockerStep(NodePtr<ParsedDockerStep>),
    DockerStepBuilder(NodePtr<DockerConnectionInfo>),
    ApiEndpointWebsocket(NodePtr<ParsedEndpointWebsocket>),
    ApiGraphQL(NodePtr<ParsedGraphQL>),
    ApiJob(NodePtr<ParsedJob>),
    Pipeline(NodePtr<ParsedPipeline>),
    Env(NodePtr<ParsedEnv>),
    Db(NodePtr<ParsedDb>),
    ParsedSchema(NodePtr<ParsedSchema>),
    Constraint(NodePtr<ParsedConstraint>),
    Meta(NodePtr<ParsedMeta>),
    Pair(NodePtr<ParsedKeyValuePair>),
}

impl ParsedHypiSchemaElement {
    pub fn set_attr<F>(&mut self, ctx: &ParseCtx<F>, key: String, value: String) -> Result<()>
    where
        F: Vfs,
    {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ParsedTable(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::Column(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::Apis(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::ParsedTables(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ColumnPipeline(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::Hypi(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::Mapping(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiCoreApi(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiRest(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::ApiEndpoint(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiGraphQL(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiJob(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::ApiEndpointResponse(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::Pipeline(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::ApiEndpointScript(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiEndpointCall(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiEndpointSql(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiEndpointFn(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::DockerStep(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::DockerStepBuilder(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::Env(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::Db(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::Constraint(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::ParsedSchema(node) => {
                node.borrow_mut().set_attr(ctx, key, value)
            }
            ParsedHypiSchemaElement::Meta(node) => node.borrow_mut().set_attr(ctx, key, value),
            ParsedHypiSchemaElement::Pair(node) => node.borrow_mut().set_attr(ctx, key, value),
        }
    }
    pub fn append_child<F>(
        &mut self,
        ctx: &ParseCtx<F>,
        child: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()>
    where
        F: Vfs,
    {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ParsedTables(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ParsedTable(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::Column(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Apis(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ColumnPipeline(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::Hypi(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Mapping(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiCoreApi(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ApiRest(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ApiEndpoint(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::DockerStep(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::DockerStepBuilder(_node) => {
                // node.borrow_mut().append_child(ctx, child)
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointScript(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiEndpointCall(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::Pipeline(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ApiEndpointResponse(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiEndpointSql(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiEndpointFn(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::ApiGraphQL(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ApiJob(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Env(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Db(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Constraint(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::ParsedSchema(node) => {
                node.borrow_mut().append_child(ctx, child)
            }
            ParsedHypiSchemaElement::Meta(node) => node.borrow_mut().append_child(ctx, child),
            ParsedHypiSchemaElement::Pair(node) => node.borrow_mut().append_child(ctx, child),
        }
    }
    pub fn set_str_body<F>(&mut self, ctx: &ParseCtx<F>, value: String) -> Result<()>
    where
        F: Vfs,
    {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ParsedTables(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ParsedTable(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::Column(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Apis(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ColumnPipeline(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::Hypi(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Mapping(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiCoreApi(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ApiRest(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ApiEndpoint(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::DockerStep(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::DockerStepBuilder(_node) => {
                // node.borrow_mut().set_str_body(ctx, value)
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointCall(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiEndpointScript(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }

            ParsedHypiSchemaElement::ApiEndpointResponse(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiEndpointSql(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiEndpointFn(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::ApiGraphQL(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ApiJob(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Pipeline(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Env(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Db(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Constraint(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::ParsedSchema(node) => {
                node.borrow_mut().set_str_body(ctx, value)
            }
            ParsedHypiSchemaElement::Meta(node) => node.borrow_mut().set_str_body(ctx, value),
            ParsedHypiSchemaElement::Pair(node) => node.borrow_mut().set_str_body(ctx, value),
        }
    }
    pub fn validate<F>(&mut self, ctx: &ParseCtx<F>) -> Result<()>
    where
        F: Vfs,
    {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ParsedTables(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ParsedTable(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Column(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Apis(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ColumnPipeline(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Hypi(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Mapping(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiCoreApi(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiRest(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpoint(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpointResponse(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpointSql(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::DockerStep(_node) => {
                //node.borrow_mut().validate(ctx)
                Ok(())
            }
            ParsedHypiSchemaElement::DockerStepBuilder(_node) => {
                //node.borrow_mut().validate(ctx)
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointFn(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpointScript(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiEndpointCall(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiGraphQL(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ApiJob(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Pipeline(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Env(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Db(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Constraint(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::ParsedSchema(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Meta(node) => node.borrow_mut().validate(ctx),
            ParsedHypiSchemaElement::Pair(node) => node.borrow_mut().validate(ctx),
        }
    }
    pub fn set_location(
        &mut self,
        line: u64,
        column: u64,
        child_index: u64,
        file_name: String,
        is_start: bool,
    ) -> Result<()> {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ParsedTables(_) => {}
            ParsedHypiSchemaElement::ParsedTable(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Column(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Apis(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ColumnPipeline(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Hypi(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Mapping(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiCoreApi(_) => {}
            ParsedHypiSchemaElement::ApiRest(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpoint(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::DockerStep(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::DockerStepBuilder(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointResponse(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointSql(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointFn(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointScript(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiEndpointCall(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiGraphQL(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ApiJob(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Pipeline(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Env(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Db(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Constraint(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::ParsedSchema(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Meta(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
            ParsedHypiSchemaElement::Pair(node) => {
                let mref = &mut node.borrow_mut();
                let loc = if is_start {
                    &mut mref.start_pos
                } else {
                    &mut mref.end_pos
                };
                loc.line = line;
                loc.column = column;
                loc.child_index = child_index;
                loc.file_name = file_name;
            }
        }
        Ok(())
    }
    pub fn name(&self) -> &str {
        match self {
            ParsedHypiSchemaElement::ParsedDocument(_) => EL_DOCUMENT,
            ParsedHypiSchemaElement::ParsedTables(_) => EL_TABLES,
            ParsedHypiSchemaElement::ParsedTable(_) => EL_TABLE,
            ParsedHypiSchemaElement::Column(_) => EL_COLUMN,
            ParsedHypiSchemaElement::Apis(_) => EL_APIS,
            ParsedHypiSchemaElement::ColumnPipeline(_) => EL_COLUMN_PIPELINE,
            ParsedHypiSchemaElement::ColumnPipelineArgs(_) => EL_PIPELINE_ARGS,
            ParsedHypiSchemaElement::ColumnPipelineWrite(_) => EL_PIPELINE_WRITE,
            ParsedHypiSchemaElement::ColumnPipelineRead(_) => EL_PIPELINE_READ,
            ParsedHypiSchemaElement::Hypi(_) => EL_HYPI,
            ParsedHypiSchemaElement::Mapping(_) => EL_MAPPING,
            ParsedHypiSchemaElement::ApiGlobalOptions(_) => EL_GLOBAL_OPTIONS,
            ParsedHypiSchemaElement::ApiCoreApi(_) => EL_CORE_API,
            ParsedHypiSchemaElement::ApiRest(_) => EL_REST,
            ParsedHypiSchemaElement::ApiEndpoint(_) => EL_ENDPOINT,
            ParsedHypiSchemaElement::ApiEndpointResponse(_) => EL_QUERY_OPTIONS_RESPONSE,
            ParsedHypiSchemaElement::ApiEndpointSql(_) => EL_SQL,
            ParsedHypiSchemaElement::DockerStep(_) => EL_STEP,
            ParsedHypiSchemaElement::DockerStepBuilder(_) => EL_STEP_BUILDER,
            ParsedHypiSchemaElement::ApiEndpointFn(_) => EL_FN,
            ParsedHypiSchemaElement::ApiEndpointWebsocket(_) => EL_WEBSOCKET,
            ParsedHypiSchemaElement::ApiEndpointScript(_) => EL_SCRIPT,
            ParsedHypiSchemaElement::ApiEndpointCall(_) => EL_CALL,
            ParsedHypiSchemaElement::ApiGraphQL(_) => EL_GRAPHQL,
            ParsedHypiSchemaElement::ApiJob(_) => EL_JOB,
            ParsedHypiSchemaElement::Pipeline(_) => EL_COLUMN_PIPELINE,
            ParsedHypiSchemaElement::Env(_) => EL_ENV,
            ParsedHypiSchemaElement::Db(_) => EL_DB,
            ParsedHypiSchemaElement::Constraint(_) => EL_CONSTRAINT,
            ParsedHypiSchemaElement::ParsedSchema(_) => EL_SCHEMA,
            ParsedHypiSchemaElement::Meta(_) => EL_META,
            ParsedHypiSchemaElement::Pair(_) => EL_PAIR,
        }
    }
}

pub trait HypiSchemaNode<F>
where
    F: Vfs,
{
    fn set_attr(&mut self, _ctx: &ParseCtx<F>, _name: String, _value: String) -> Result<()> {
        Ok(())
    }
    fn append_child(
        &mut self,
        _ctx: &ParseCtx<F>,
        _node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        Ok(())
    }
    fn set_str_body(&mut self, _ctx: &ParseCtx<F>, _value: String) -> Result<()> {
        Ok(())
    }
    fn validate(&mut self, _ctx: &ParseCtx<F>) -> Result<()> {
        Ok(())
    }
}

pub fn new_node<F>(
    parent: Option<NodePtr<ParsedHypiSchemaElement>>,
    ctx: &ParseCtx<F>,
    name: &str,
) -> Result<ParsedHypiSchemaElement>
where
    F: Vfs,
{
    let parent_name = parent.map(|v| v.borrow().name().to_owned());
    match name {
        EL_DOCUMENT => Ok(ParsedHypiSchemaElement::ParsedDocument(new_node_ptr(
            ParsedDocument {
                start_pos: Location::default(),
                end_pos: Location::default(),
                meta: new_node_ptr(ParsedMeta {
                    start_pos: Default::default(),
                    end_pos: Default::default(),
                    key_value_pairs: new_node_ptr(vec![]),
                }),
                apis: new_node_ptr(ParsedApis {
                    start_pos: Location::default(),
                    end_pos: Location::default(),
                    global_options: None,
                    rest: None,
                    graphql: None,
                    pipelines: new_node_ptr(vec![]),
                    jobs: new_node_ptr(vec![]),
                }),
                databases: new_node_ptr(vec![]),
                env: new_node_ptr(vec![]),
                step_builders: new_node_ptr(vec![]),
            },
        ))),
        EL_TABLES => Ok(ParsedHypiSchemaElement::ParsedTables(new_node_ptr(vec![]))),
        EL_TABLE => Ok(ParsedHypiSchemaElement::ParsedTable(new_node_ptr(
            ParsedTable {
                start_pos: Location::default(),
                end_pos: Location::default(),
                hypi: None,
                columns: new_node_ptr(vec![]),
                constraints: new_node_ptr(vec![]),
                name: "".to_string(),
            },
        ))),
        EL_APIS => Ok(ParsedHypiSchemaElement::Apis(new_node_ptr(ParsedApis {
            start_pos: Location::default(),
            end_pos: Location::default(),
            global_options: None,
            rest: None,
            graphql: None,
            pipelines: new_node_ptr(vec![]),
            jobs: new_node_ptr(vec![]),
        }))),
        EL_COLUMN => Ok(ParsedHypiSchemaElement::Column(new_node_ptr(
            ParsedColumn {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                typ: ColumnType::TEXT,
                nullable: true,
                unique: false,
                default: None,
                primary_key: false,
                pipeline: None,
            },
        ))),
        EL_COLUMN_PIPELINE if parent_name == Some(EL_COLUMN.to_owned()) => Ok(
            ParsedHypiSchemaElement::ColumnPipeline(new_node_ptr(ParsedColumnPipeline {
                start_pos: Location::default(),
                end_pos: Location::default(),
                args: None,
                write: None,
                read: None,
            })),
        ),
        EL_PIPELINE_ARGS => Ok(ParsedHypiSchemaElement::ColumnPipelineArgs(new_node_ptr(
            ParsedColumnPipelineArgs {
                start_pos: Location::default(),
                end_pos: Location::default(),
                value: String::new(),
            },
        ))),
        EL_ENV => Ok(ParsedHypiSchemaElement::Env(new_node_ptr(ParsedEnv {
            start_pos: Location::default(),
            end_pos: Location::default(),
            name: "".to_string(),
            value: String::new(),
        }))),
        EL_DB => Ok(ParsedHypiSchemaElement::Db(new_node_ptr(ParsedDb {
            start_pos: Location::default(),
            end_pos: Location::default(),
            label: "".to_string(),
            db_name: "".to_string(),
            host: "".to_string(),
            port: None,
            typ: DatabaseType::MekaDb,
            username: "".to_string(),
            password: "".to_string(),
            options: None,
            schemas: new_node_ptr(vec![]),
        }))),
        EL_SCHEMA => Ok(ParsedHypiSchemaElement::ParsedSchema(new_node_ptr(
            ParsedSchema {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                tables: new_node_ptr(vec![]),
            },
        ))),
        EL_CONSTRAINT => Ok(ParsedHypiSchemaElement::Constraint(new_node_ptr(
            ParsedConstraint {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                columns: vec![],
                typ: TableConstraintType::Unique,
                mappings: new_node_ptr(vec![]),
            },
        ))),
        EL_META => Ok(ParsedHypiSchemaElement::Meta(new_node_ptr(ParsedMeta {
            start_pos: Location::default(),
            end_pos: Location::default(),
            key_value_pairs: new_node_ptr(vec![]),
        }))),
        EL_PAIR => Ok(ParsedHypiSchemaElement::Pair(new_node_ptr(
            ParsedKeyValuePair {
                start_pos: Location::default(),
                end_pos: Location::default(),
                key: "".to_string(),
                value: "".to_string(),
            },
        ))),
        EL_PIPELINE_WRITE => Ok(ParsedHypiSchemaElement::ColumnPipelineWrite(new_node_ptr(
            ParsedColumnPipelineWrite {
                start_pos: Location::default(),
                end_pos: Location::default(),
                value: String::new(),
            },
        ))),
        EL_PIPELINE_READ => Ok(ParsedHypiSchemaElement::ColumnPipelineRead(new_node_ptr(
            ParsedColumnPipelineRead {
                start_pos: Location::default(),
                end_pos: Location::default(),
                value: String::new(),
            },
        ))),
        EL_HYPI => Ok(ParsedHypiSchemaElement::Hypi(new_node_ptr(ParsedHypi {
            start_pos: Location::default(),
            end_pos: Location::default(),
            well_known: None,
            mappings: vec![],
        }))),
        EL_MAPPING => Ok(ParsedHypiSchemaElement::Mapping(new_node_ptr(
            ParsedMapping {
                start_pos: Location::default(),
                end_pos: Location::default(),
                from: "".to_string(),
                to: None,
                children: vec![],
                typ: None,
            },
        ))),
        EL_GLOBAL_OPTIONS => Ok(ParsedHypiSchemaElement::ApiGlobalOptions(new_node_ptr(
            ParsedGlobalOptions {
                start_pos: Location::default(),
                end_pos: Location::default(),
                core_apis: vec![],
                explicitly_enabled_crud_tables: vec![],
                implicit_steps: new_node_ptr(vec![]),
            },
        ))),
        EL_CORE_API => Ok(ParsedHypiSchemaElement::ApiCoreApi(new_node_ptr(
            String::new(),
        ))),
        EL_REST => Ok(ParsedHypiSchemaElement::ApiRest(new_node_ptr(ParsedRest {
            start_pos: Location::default(),
            end_pos: Location::default(),
            base: "/".to_string(),
            endpoints: vec![],
        }))),
        EL_ENDPOINT => Ok(ParsedHypiSchemaElement::ApiEndpoint(new_node_ptr(
            ParsedEndpoint::default(),
        ))),
        EL_SCRIPT => Ok(ParsedHypiSchemaElement::ApiEndpointScript(new_node_ptr(
            ParsedEndpointScript {
                start_pos: Location::default(),
                end_pos: Location::default(),
                file: "".to_string(),
                label: None,
                typ: ScriptType::JavaScript,
                is_async: false,
            },
        ))),
        EL_CALL => Ok(ParsedHypiSchemaElement::ApiEndpointCall(new_node_ptr(
            ParsedCall {
                start_pos: Location::default(),
                end_pos: Location::default(),
                target: "".to_string(),
                label: None,
                mappings: vec![],
                is_async: false,
            },
        ))),
        EL_GRAPHQL => Ok(ParsedHypiSchemaElement::ApiGraphQL(new_node_ptr(
            ParsedGraphQL {
                start_pos: Location::default(),
                end_pos: Location::default(),
                base: "".to_string(),
                from: "".to_string(),
                enable_subscriptions: true,
            },
        ))),
        EL_JOB => Ok(ParsedHypiSchemaElement::ApiJob(new_node_ptr(ParsedJob {
            start_pos: Location::default(),
            end_pos: Location::default(),
            name: "".to_string(),
            pipeline: "".to_string(),
            start: "".to_string(),
            end: "".to_string(),
            interval: "".to_string(),
            interval_frequency: "".to_string(),
            enabled: false,
            repeats: false,
        }))),
        EL_QUERY_OPTIONS_RESPONSE => Ok(ParsedHypiSchemaElement::ApiEndpointResponse(
            new_node_ptr(ParsedEndpointResponse {
                start_pos: Location::default(),
                end_pos: Location::default(),
                status: 0,
                when: None,
                yield_expr: None,
                body: None,
                mappings: vec![],
            }),
        )),

        EL_SQL => Ok(ParsedHypiSchemaElement::ApiEndpointSql(new_node_ptr(
            ParsedEndpointSql {
                start_pos: Location::default(),
                end_pos: Location::default(),
                sql: "".to_string(),
                db_name: None,
                label: None,
                mappings: vec![],
                is_async: false,
            },
        ))),
        EL_FN => Ok(ParsedHypiSchemaElement::ApiEndpointFn(new_node_ptr(
            ParsedEndpointFn {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                label: None,
                version: "".to_string(),
                mappings: vec![],
                is_async: false,
            },
        ))),
        EL_STEP => Ok(ParsedHypiSchemaElement::DockerStep(new_node_ptr(
            ParsedDockerStep {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                mappings: new_node_ptr(vec![]),
                implicit_before_position: None,
                provider: DockerStepProvider::Dockerfile {
                    path: ".".to_string(),
                },
                implicit_after_position: None,
            },
        ))),
        EL_STEP_BUILDER => Ok(ParsedHypiSchemaElement::DockerStepBuilder(new_node_ptr(
            DockerConnectionInfo {
                start_pos: Location::default(),
                end_pos: Location::default(),
                username: None,
                password: None,
                image: "".to_string(),
                tag: None,
            },
        ))),
        EL_WEBSOCKET => Ok(ParsedHypiSchemaElement::ApiEndpointWebsocket(new_node_ptr(
            ParsedEndpointWebsocket {
                start_pos: Location::default(),
                end_pos: Location::default(),
                base: "/".to_string(),
                sources: vec![],
            },
        ))),
        EL_PIPELINE => Ok(ParsedHypiSchemaElement::Pipeline(new_node_ptr(
            ParsedPipeline {
                start_pos: Location::default(),
                end_pos: Location::default(),
                name: "".to_string(),
                label: None,
                steps: new_node_ptr(vec![]),
                docker_steps: new_node_ptr(vec![]),
                is_async: false,
            },
        ))),
        _ => Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_EL.clone(),
            element: name.to_owned(),
            message: format!("Unsupported XML node - {}", name),
        })),
    }
}

pub type ParsedTables = Vec<NodePtr<ParsedTable>>;
pub type Mappings = Vec<NodePtr<ParsedMapping>>;
// pub type Apis = Vec<NodePtr<ParsedApi>>;

/// Hypi Application Markup Language = HAML
#[derive(Debug)]
pub struct ParsedDocument {
    pub start_pos: Location,
    pub end_pos: Location,
    pub meta: NodePtr<ParsedMeta>,
    pub apis: NodePtr<ParsedApis>,
    pub databases: NodePtr<Vec<NodePtr<ParsedDb>>>,
    pub env: NodePtr<Vec<NodePtr<ParsedEnv>>>,
    pub step_builders: NodePtr<Vec<NodePtr<DockerConnectionInfo>>>,
}

impl<F> HypiSchemaNode<F> for ParsedDocument
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, _value: String) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_ATTR.clone(),
            element: EL_DOCUMENT.to_owned(),
            message: format!("document does not support an attribute called '{}'...in fact, it doesn't support any attributes at all!", name),
        }))
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Apis(node) => {
                self.apis = node.clone();
                Ok(())
            }
            ParsedHypiSchemaElement::Env(node) => {
                self.env.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::DockerStepBuilder(node) => {
                self.step_builders.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::Db(node) => {
                self.databases.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::Meta(node) => {
                self.meta = node.clone();
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_DOCUMENT.to_owned(),
                message: format!(
                    "The document element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

pub struct ParseCtx<F>
where
    F: Vfs,
{
    file_name: String,
    line_number: u64,
    column: u64,
    ///Used to resolve imports
    ///file name -> file contents
    fs: Arc<BoundVfs<F>>,
    attributes: Vec<OwnedAttribute>,
}

impl<F> ParseCtx<F>
where
    F: Vfs,
{
    fn new(
        file_name: String,
        position: TextPosition,
        fs: Arc<BoundVfs<F>>,
        attributes: Vec<OwnedAttribute>,
    ) -> Self {
        let line = position.row.wrapping_add(1);
        let col = position.column.wrapping_add(1);
        ParseCtx {
            file_name,
            fs,
            attributes,
            line_number: line,
            column: col,
        }
    }
}

impl ParsedDocument {
    pub fn to_str(&self) -> Result<String> {
        //serde_xml_rs::to_string(self).map_err(HamlError::X)
        panic!()
    }
    #[allow(unused_assignments)]
    pub fn from_str<F>(
        file_name: String,
        fs: Arc<BoundVfs<F>>,
    ) -> Result<NodePtr<ParsedHypiSchemaElement>>
    where
        F: Vfs,
    {
        let xml = match fs.read_schema_file(file_name.as_str()) {
            Ok(val) => val,
            Err(e) => {
                return Err(HamlError::ParseErr(ParseErr {
                    file: file_name.clone(),
                    line: 0,
                    column: 0,
                    code: HAML_CODE_MISSING_IMPORT.clone(),
                    element: EL_ENDPOINT.to_owned(),
                    message: format!("Imported file not found {}. {:?}", file_name, e),
                }));
            }
        };
        let mut root: Option<NodePtr<ParsedHypiSchemaElement>> = None;
        let mut q: Vec<NodePtr<ParsedHypiSchemaElement>> = vec![];
        let mut parser: EventReader<&[u8]> = EventReader::new(xml.as_bytes().into());
        let mut child_index = vec![];
        loop {
            let e = parser.next();
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    child_index.push(child_index.len() as u64);
                    let mut ctx =
                        ParseCtx::new(file_name.clone(), parser.position(), fs.clone(), attributes);
                    match name {
                        OwnedName { local_name, .. } => {
                            let parent = q.last().map(|v| v.clone());
                            let mut node = new_node(parent, &ctx, local_name.as_str())?;
                            let mut child_index = child_index.last_mut().unwrap();
                            node.set_location(
                                ctx.line_number,
                                ctx.column,
                                *child_index,
                                file_name.clone(),
                                true,
                            )?;
                            child_index = &mut ((*child_index) + 1);
                            let ctx = &mut ctx;
                            for attr in &ctx.attributes {
                                if IGNORED_ATTRS.contains(&attr.name.local_name.as_str()) {
                                    continue;
                                }
                                node.set_attr(
                                    ctx,
                                    attr.name.local_name.to_owned(),
                                    attr.value.to_owned(),
                                )?;
                            }
                            let node = Rc::new(RefCell::new(node));
                            if root.is_none() {
                                root = Some(node.clone());
                                q.push(node.clone());
                            } else {
                                let old = q.last().map(|v| v.clone());
                                q.push(node.clone());
                                if let Some(current) = old {
                                    let clone = current.clone();
                                    let mut m: RefMut<'_, _> = (*clone).borrow_mut();
                                    m.append_child(ctx, node)?;
                                }
                            }
                        }
                    }
                }
                Ok(XmlEvent::Characters(chars)) => {
                    let mut ctx =
                        ParseCtx::new(file_name.clone(), parser.position(), fs.clone(), vec![]);
                    if let Some(current) = q.last().clone() {
                        (*current).borrow_mut().set_str_body(&mut ctx, chars)?;
                    }
                }
                Ok(XmlEvent::EndElement { .. }) => {
                    let mut ctx =
                        ParseCtx::new(file_name.clone(), parser.position(), fs.clone(), vec![]);
                    if let Some(current) = q.pop().clone() {
                        let mut node = (*current).borrow_mut();
                        node.set_location(
                            ctx.line_number,
                            ctx.column,
                            child_index.pop().unwrap(),
                            file_name.clone(),
                            false,
                        )?;
                        node.validate(&mut ctx)?;
                    }
                }
                Ok(XmlEvent::EndDocument) => {
                    //once emitted, the parser always emits it when next is called so break out of the loop
                    break;
                }
                Err(e) => {
                    let mut msg: String = String::new();
                    let code = match e.kind() {
                        ErrorKind::Syntax(s) => {
                            msg.push_str(s);
                            HAML_CODE_XML_SYNTAX.clone()
                        }
                        ErrorKind::Io(io) => {
                            msg.push_str(io.to_string().as_str());
                            HAML_CODE_XML_IO.clone()
                        }
                        ErrorKind::Utf8(e) => {
                            msg.push_str(e.to_string().as_str());
                            HAML_CODE_XML_UTF8.clone()
                        }
                        ErrorKind::UnexpectedEof => {
                            msg.push_str("Unexpected end of HAML");
                            HAML_CODE_XML_EOF.clone()
                        }
                    };
                    let pos = parser.position();
                    return Err(HamlError::ParseErr(ParseErr {
                        file: file_name.clone(),
                        line: pos.row,
                        column: pos.column,
                        code,
                        element: "<>".to_owned(),
                        message: msg,
                    }));
                }
                // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
                _ => {}
            }
        }
        if let Some(root) = root {
            Ok(root)
        } else {
            let pos = parser.position();
            Err(HamlError::ParseErr(ParseErr {
                file: file_name.clone(),
                line: pos.row,
                column: pos.column,
                code: HAML_CODE_NO_ROOT.clone(),
                element: "".to_owned(),
                message: "I mean...you gotta pass something in!".to_owned(),
            }))
        }
    }
}

#[derive(Debug)]
pub struct ParsedTable {
    pub start_pos: Location,
    pub end_pos: Location,
    pub columns: NodePtr<Vec<NodePtr<ParsedColumn>>>,
    pub constraints: NodePtr<Vec<NodePtr<ParsedConstraint>>>,
    pub name: String,
    pub hypi: Option<NodePtr<ParsedHypi>>,
}

impl<F> HypiSchemaNode<F> for ParsedTable
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        if attr_name == ATTR_IMPORT && ctx.attributes.len() > 1 {
            return Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_MISSING_IMPORT.clone(),
                element: EL_ENDPOINT.to_owned(),
                message: format!(
                    "The import attribute cannot be combined with any others. Attempting to import '{}' and mixing it with '{:?}'.",
                    value,
                    ctx.attributes.iter().filter(|v| v.name.local_name.to_lowercase() != ATTR_IMPORT).map(|v| v.name.local_name.clone()).collect::<Vec<_>>().join(",")
                ),
            }));
        }
        match attr_name {
            ATTR_IMPORT => match ParsedDocument::from_str(value.clone(), ctx.fs.clone()) {
                Ok(node) => match &*(&*node).borrow() {
                    ParsedHypiSchemaElement::ParsedTable(table) => {
                        let table = table.replace(ParsedTable {
                            start_pos: Location::default(),
                            end_pos: Location::default(),
                            columns: new_node_ptr(vec![]),
                            constraints: new_node_ptr(vec![]),
                            name: "".to_string(),
                            hypi: None,
                        });
                        let _ = std::mem::replace(self, table);
                        Ok(())
                    }
                    _ => Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_MISSING_IMPORT.clone(),
                        element: EL_ENDPOINT.to_owned(),
                        message: format!(
                            "Imported file '{}' found but it was not an endpoint as expected",
                            value
                        ),
                    })),
                },
                Err(err) => Err(err),
            },
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            val => {
                return Err(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_TABLE.to_owned(),
                    message: format!(
                        "table elements do not support an attribute called '{}'",
                        val
                    ),
                }));
            }
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Column(node) => {
                self.columns.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::Hypi(node) => {
                self.hypi = Some(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::Constraint(node) => {
                self.constraints.borrow_mut().push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_TABLE.to_owned(),
                message: format!(
                    "The table element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

fn parse_column_type<F>(ctx: &ParseCtx<F>, value: &String) -> Result<ColumnType>
where
    F: Vfs,
{
    Ok(match value.to_lowercase().as_str() {
        COL_TYPE_TEXT => ColumnType::TEXT,
        COL_TYPE_INT => ColumnType::INT,
        COL_TYPE_BIGINT => ColumnType::BIGINT,
        COL_TYPE_FLOAT => ColumnType::FLOAT,
        COL_TYPE_DOUBLE => ColumnType::DOUBLE,
        COL_TYPE_TIMESTAMP => ColumnType::TIMESTAMP,
        COL_TYPE_BOOL => ColumnType::BOOL,
        COL_TYPE_BYTEA => ColumnType::BYTEA,
        _ => return Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_ATTR.clone(),
            element: EL_COLUMN.to_owned(),
            message: format!("Column type does not support '{}'. Supported types are text,int,bigint,float,double,timestamp,bool,bytea", value),
        }))
    })
}

#[derive(Debug, PartialEq, Clone)]
pub enum ColumnType {
    TEXT,
    INT,
    BIGINT,
    FLOAT,
    DOUBLE,
    TIMESTAMP,
    BOOL,
    BYTEA,
}

#[derive(Debug, Clone)]
pub enum ColumnDefault {
    UniqueSqid,
    UniqueUlid,
    UniqueSnowflake,
}

#[derive(Debug)]
pub struct ParsedColumn {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub typ: ColumnType,
    pub nullable: bool,
    pub unique: bool,
    pub default: Option<ColumnDefault>,
    pub primary_key: bool,
    pub pipeline: Option<NodePtr<ParsedColumnPipeline>>,
}

impl<F> HypiSchemaNode<F> for ParsedColumn
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_NAME => {
                self.name = value;
            }
            ATTR_PK => {
                self.primary_key = value.to_lowercase() == "true";
            }
            ATTR_NULLABLE => {
                self.nullable = value.to_lowercase() == "true";
            }
            ATTR_TYPE => {
                self.typ = parse_column_type(ctx, &value)?;
            }
            ATTR_UNIQUE => {
                self.unique = value.to_lowercase() == "true";
            }
            ATTR_DEFAULT => {
                let default;
                let value = value.to_lowercase();
                if value.contains("(") && value.replace(&[' ', '\t'], "").contains("(sqid)") {
                    default = ColumnDefault::UniqueSqid;
                } else if value == "unique" {
                    default = ColumnDefault::UniqueUlid;
                } else {
                    return Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_UNKNOWN_ATTR.clone(),
                        element: EL_COLUMN.to_owned(),
                        message: format!("Column type does not support '{}'. Supported types are text,int,bigint,float,double,timestamp,bool,bytea", value),
                    }));
                }
                self.default = Some(default);
            }
            val => {
                return Err(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_COLUMN.to_owned(),
                    message: format!(
                        "Column elements do not support an attribute called '{}'",
                        val
                    ),
                }));
            }
        }
        Ok(())
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ColumnPipeline(node) => {
                if self.pipeline.is_some() {
                    return Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_CANNOT_REPEAT.clone(),
                        element: EL_COLUMN.to_owned(),
                        message: "The column element does support multiple pipeline elements."
                            .to_owned(),
                    }));
                }
                self.pipeline = Some(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_COLUMN.to_owned(),
                message: format!(
                    "The column element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedColumnPipeline {
    pub start_pos: Location,
    pub end_pos: Location,
    pub args: Option<NodePtr<ParsedColumnPipelineArgs>>,
    pub write: Option<NodePtr<ParsedColumnPipelineWrite>>,
    pub read: Option<NodePtr<ParsedColumnPipelineRead>>,
}

impl<F> HypiSchemaNode<F> for ParsedColumnPipeline
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, _value: String) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_ATTR.clone(),
            element: EL_COLUMN_PIPELINE.to_owned(),
            message: format!("The pipeline element of a column does not support an attribute called '{}'...in fact, it doesn't support any attributes at all.", name),
        }))
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ColumnPipelineArgs(node) => {
                if self.args.is_none() {
                    self.args = Some(node.clone());
                    Ok(())
                } else {
                    Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_CANNOT_REPEAT.clone(),
                        element: EL_PIPELINE_ARGS.to_owned(),
                        message: "Only 1 args element can appear inside a column pipeline"
                            .to_owned(),
                    }))
                }
            }
            ParsedHypiSchemaElement::ColumnPipelineWrite(node) => {
                if self.write.is_none() {
                    self.write = Some(node.clone());
                    Ok(())
                } else {
                    Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_CANNOT_REPEAT.clone(),
                        element: EL_PIPELINE_ARGS.to_owned(),
                        message: "Only 1 write element can appear inside a column pipeline"
                            .to_owned(),
                    }))
                }
            }
            ParsedHypiSchemaElement::ColumnPipelineRead(node) => {
                if self.read.is_none() {
                    self.read = Some(node.clone());
                    Ok(())
                } else {
                    Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_CANNOT_REPEAT.clone(),
                        element: EL_PIPELINE_ARGS.to_owned(),
                        message: "Only 1 read element can appear inside a column pipeline"
                            .to_owned(),
                    }))
                }
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_COLUMN_PIPELINE.to_owned(),
                message: format!(
                    "The pipeline element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedColumnPipelineArgs {
    pub start_pos: Location,
    pub end_pos: Location,
    pub value: String,
}

impl<F> HypiSchemaNode<F> for ParsedColumnPipelineArgs
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_VALUE => {
                self.value = value;
                Ok(())
            }
            name => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PIPELINE_ARGS.to_owned(),
                message: format!("The args element of a column pipeline does not support an attribute called '{}'.", name),
            }))
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
            element: EL_PIPELINE_ARGS.to_owned(),
            message: format!("The args element of a column pipeline does not support '{}' elements inside it. In fact, it does not support any children at all", (*node).borrow().name()),
        }))
    }
}

#[derive(Debug)]
pub struct ParsedColumnPipelineWrite {
    pub start_pos: Location,
    pub end_pos: Location,
    pub value: String,
}

impl<F> HypiSchemaNode<F> for ParsedColumnPipelineWrite
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_VALUE => {
                self.value = value;
                Ok(())
            }
            name => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PIPELINE_WRITE.to_owned(),
                message: format!("The write element of a column pipeline does not support an attribute called '{}'.", name),
            }))
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
            element: EL_PIPELINE_WRITE.to_owned(),
            message: format!("The write element of a column pipeline does not support '{}' elements inside it. In fact, it does not support any children at all", (*node).borrow().name()),
        }))
    }
}

#[derive(Debug)]
pub struct ParsedColumnPipelineRead {
    pub start_pos: Location,
    pub end_pos: Location,
    pub value: String,
}

impl<F> HypiSchemaNode<F> for ParsedColumnPipelineRead
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_VALUE => {
                self.value = value;
                Ok(())
            }
            name => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PIPELINE_READ.to_owned(),
                message: format!("The read element of a column pipeline does not support an attribute called '{}'.", name),
            }))
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_ATTR.clone(),
            element: EL_PIPELINE_READ.to_owned(),
            message: format!("The read element of a column pipeline does not support '{}' elements inside it. In fact, it does not support any children at all", (*node).borrow().name()),
        }))
    }
}

#[derive(Debug)]
pub struct ParsedDockerStep {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub provider: DockerStepProvider,
    pub mappings: NodePtr<Mappings>,
    pub implicit_before_position: Option<ImplicitDockerStepPosition>,
    pub implicit_after_position: Option<ImplicitDockerStepPosition>,
}

impl<F> HypiSchemaNode<F> for ParsedDockerStep
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_BEFORE => {
                self.implicit_before_position = Some(value.parse().map_err(|e| {
                    HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_INVALID_STEP_LOC.clone(),
                        element: EL_STEP.to_owned(),
                        message: format!("Invalid 'before' value. {}. Supported values are first OR each OR last", e),
                    })
                })?);
                Ok(())
            }
            ATTR_AFTER => {
                self.implicit_before_position = Some(value.parse().map_err(|e| {
                    HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_INVALID_STEP_LOC.clone(),
                        element: EL_STEP.to_owned(),
                        message: format!(
                            "Invalid 'after' value. {}. Supported values are first OR each OR last",
                            e
                        ),
                    })
                })?);
                Ok(())
            }
            ATTR_PROVIDER => {
                self.provider = value.parse().map_err(|e| {
                    HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_INVALID_PROVIDER.clone(),
                        element: EL_PROVIDER.to_owned(),
                        message: format!("Invalid provider value. {}. Supported formats are file:path/to/src/dir OR file:path/to/src/Dockerfile OR docker:image-name:tag", e),
                    })
                })?;
                Ok(())
            }
            name => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PROVIDER.to_owned(),
                message: format!(
                    "The step element of a pipeline does not support an element called '{}'.",
                    name
                ),
            })),
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(node) => {
                self.mappings.borrow_mut().push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PROVIDER.to_owned(),
                message: format!(
                    "The step element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

impl<F> HypiSchemaNode<F> for DockerConnectionInfo
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            ATTR_IMAGE => {
                let info=parse_docker_image(value.as_str()).map_err(|e| {
                    HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_INVALID_STEP_LOC.clone(),
                        element: EL_STEP.to_owned(),
                        message: format!("Invalid 'before' value. {}. Supported values are first OR each OR last", e),
                    })
                })?;
                let old=std::mem::replace(self,info);
                self.start_pos=old.start_pos;
                self.end_pos=old.end_pos;
                Ok(())
            } 
            name => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PROVIDER.to_owned(),
                message: format!(
                    "The step-builder element of a pipeline does not support an element called '{}'.",
                    name
                ),
            })),
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PROVIDER.to_owned(),
                message: format!(
                    "The step-builder element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

pub type ParsedCoreApiName = String;

impl<F> HypiSchemaNode<F> for ParsedCoreApiName
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            "name" => {
                self.clear();
                self.clone_from(&value);
                Ok(())
            }
            _ => {
                Err(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_GLOBAL_OPTIONS.to_owned(),
                    message: format!("The core-api element of global-options does not support an attribute called '{}'.", name),
                }))
            }
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
            element: EL_GLOBAL_OPTIONS.to_owned(),
            message: format!("The core-api element does not support '{}' elements inside it... In fact, it doesn't support any children at all!", (*node).borrow().name()),
        }))
    }
}

#[derive(Debug)]
pub struct ParsedGlobalOptions {
    pub start_pos: Location,
    pub end_pos: Location,
    pub core_apis: Vec<CoreApi>,
    pub explicitly_enabled_crud_tables: Vec<String>,
    pub implicit_steps: NodePtr<Vec<NodePtr<ParsedDockerStep>>>,
}

impl<F> HypiSchemaNode<F> for ParsedGlobalOptions
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            "enable-crud-on-tables" => {
                for table_name in value.split(',') {
                    self.explicitly_enabled_crud_tables
                        .push(table_name.to_owned());
                }
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_GLOBAL_OPTIONS.to_owned(),
                message: format!(
                    "The global-options element of apis does not support an attribute called '{}'.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::DockerStep(node) => {
                self.implicit_steps.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiCoreApi(node) => {
                match (*node).borrow().to_lowercase().as_str() {
                    CORE_API_REGISTER => Ok(self.core_apis.push(CoreApi::Register)),
                    CORE_API_LOGIN_BY_EMAIL => Ok(self.core_apis.push(CoreApi::LoginByEmail)),
                    CORE_API_LOGIN_BY_USERNAME => Ok(self.core_apis.push(CoreApi::LoginByUsername)),
                    CORE_API_OAUTH => Ok(self.core_apis.push(CoreApi::OAuth)),
                    CORE_API_PASSWORD_RESET_TRIGGER => {
                        Ok(self.core_apis.push(CoreApi::PasswordResetTrigger))
                    }
                    CORE_API_PASSWORD_RESET => Ok(self.core_apis.push(CoreApi::PasswordReset)),
                    CORE_API_VERIFY_ACCOUNT => Ok(self.core_apis.push(CoreApi::VerifyAccount)),
                    CORE_API_MAGIC_LINK => Ok(self.core_apis.push(CoreApi::MagicLink)),
                    CORE_API_2FA_EMAIL => Ok(self.core_apis.push(CoreApi::TwoFactorAuthEmail)),
                    CORE_API_2FA_SMS => Ok(self.core_apis.push(CoreApi::TwoFactorAuthSms)),
                    CORE_API_2FA_STEP2 => Ok(self.core_apis.push(CoreApi::TwoFactorStep2)),
                    CORE_API_2FA_TOTP => Ok(self.core_apis.push(CoreApi::TwoFactorTotp)),
                    name => Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                        element: EL_CORE_API.to_owned(),
                        message: format!("No core api supported with the name '{}'.", name),
                    })),
                }
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_CORE_API.to_owned(),
                message: format!(
                    "The global-options element does not support '{}' elements inside it.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedApis {
    pub start_pos: Location,
    pub end_pos: Location,
    pub global_options: Option<NodePtr<ParsedGlobalOptions>>,
    pub rest: Option<NodePtr<ParsedRest>>,
    pub graphql: Option<NodePtr<ParsedGraphQL>>,
    pub pipelines: NodePtr<Vec<NodePtr<ParsedPipeline>>>,
    pub jobs: NodePtr<Vec<NodePtr<ParsedJob>>>,
}

impl<F> HypiSchemaNode<F> for ParsedApis
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, _value: String) -> Result<()> {
        return match name.as_str() {
            val => {
                Err(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_APIS.to_owned(),
                    message: format!("The apis element does not support an attribute called '{}'...in fact, it doesn't support any attributes at all.", val),
                }))
            }
        };
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ApiGlobalOptions(node) => {
                self.global_options = Some(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiRest(node) => {
                self.rest = Some(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::Pipeline(node) => {
                self.pipelines.borrow_mut().push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiGraphQL(node) => {
                self.graphql = Some(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiJob(node) => {
                self.jobs.borrow_mut().push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_APIS.to_owned(),
                message: format!(
                    "The apis element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

impl<F> HypiSchemaNode<F> for ParsedTables
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, _value: String) -> Result<()> {
        Err(HamlError::ParseErr(ParseErr {
            file: ctx.file_name.clone(),
            line: ctx.line_number.clone(),
            column: ctx.column.clone(),
            code: HAML_CODE_UNKNOWN_ATTR.clone(),
            element: EL_TABLES.to_owned(),
            message: format!("The tables element does not support an attribute called '{}'...in fact, it doesn't support any attributes at all.", name),
        }))
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ParsedTable(tbl) => {
                self.push(tbl.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_TABLES.to_owned(),
                message: format!(
                    "The tables element does not support child elements of type '{}'.",
                    node.borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum WellKnownType {
    Account,
    File,
    Permission,
    Role,
}

#[derive(Debug)]
pub struct ParsedHypi {
    pub start_pos: Location,
    pub end_pos: Location,
    pub well_known: Option<WellKnownType>,
    pub mappings: Mappings,
}

impl<F> HypiSchemaNode<F> for ParsedHypi
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.as_str() {
            "well-known" => {
                self.well_known = Some(match value.to_lowercase().as_str() {
                    "account" => WellKnownType::Account,
                    "file" => WellKnownType::File,
                    _ => {
                        return Err(HamlError::ParseErr(ParseErr {
                            file: ctx.file_name.clone(),
                            line: ctx.line_number.clone(),
                            column: ctx.column.clone(),
                            code: HAML_CODE_UNKNOWN_WELL_KNOWN_TYPE.clone(),
                            element: EL_HYPI.to_owned(),
                            message: format!(
                                "The hypi element does not support a well known type called '{}'.",
                                value
                            ),
                        }));
                    }
                });
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_TABLE.to_owned(),
                message: format!(
                    "The hypi element does not support an attribute called '{}'.",
                    name
                ),
            })),
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(node) => {
                self.mappings.push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_HYPI.to_owned(),
                message: format!(
                    "The hypi element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedMapping {
    pub start_pos: Location,
    pub end_pos: Location,
    pub from: String,
    pub to: Option<String>,
    pub typ: Option<ColumnType>,
    pub children: Vec<NodePtr<ParsedMapping>>,
}

impl<F> HypiSchemaNode<F> for ParsedMapping
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_FROM => {
                self.from = value;
                Ok(())
            }
            ATTR_TO => {
                self.to = Some(value);
                Ok(())
            }
            ATTR_TYPE => {
                self.typ = Some(parse_column_type(ctx, &value)?);
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_TABLE.to_owned(),
                message: format!(
                    "The mapping element does not support an attribute called '{}'.",
                    name
                ),
            })),
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(node) => {
                self.children.push(node.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_MAPPING.to_owned(),
                message: format!(
                    "The mapping element does not support '{}' elements inside it.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedRest {
    pub start_pos: Location,
    pub end_pos: Location,
    pub base: String,
    pub endpoints: Vec<NodePtr<ParsedEndpoint>>,
}

impl<F> HypiSchemaNode<F> for ParsedRest
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_BASE => {
                self.base = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_REST.to_owned(),
                message: format!(
                    "The rest element does not support an attribute called '{}'.",
                    name
                ),
            })),
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ApiEndpoint(node) => {
                self.endpoints.push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_REST.to_owned(),
                message: format!(
                    "The rest element does not support '{}' elements inside it.",
                    (*el).name()
                ),
            })),
        }
    }
}

#[derive(Debug, Default)]
pub struct ParsedEndpoint {
    pub start_pos: Location,
    pub end_pos: Location,
    pub method: HttpMethod,
    pub path: Option<String>,
    pub name: Option<String>,
    pub public: Option<bool>,
    pub accepts: Option<String>,
    pub produces: Option<String>,
    ///The name of the pipeline which is executed when this endpoint is called
    pub pipeline: Option<String>,
    pub responses: Vec<NodePtr<ParsedEndpointResponse>>,
    pub websockets: Vec<NodePtr<ParsedEndpointWebsocket>>,
}

impl<F> HypiSchemaNode<F> for ParsedEndpoint
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        if attr_name == ATTR_IMPORT && ctx.attributes.len() > 1 {
            return Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_MISSING_IMPORT.clone(),
                element: EL_ENDPOINT.to_owned(),
                message: format!(
                    "The import attribute cannot be combined with any others. Attempting to import '{}' and mixing it with '{:?}'.",
                    value,
                    ctx.attributes.iter().filter(|v| v.name.local_name.to_lowercase() != ATTR_IMPORT).map(|v| v.name.local_name.clone()).collect::<Vec<_>>().join(",")
                ),
            }));
        }
        match attr_name {
            ATTR_ACCEPTS => {
                self.accepts = Some(value);
                Ok(())
            }
            ATTR_PRODUCES => {
                self.produces = Some(value);
                Ok(())
            }
            ATTR_PATH => {
                self.path = Some(value);
                Ok(())
            }
            ATTR_NAME => {
                self.name = Some(value);
                Ok(())
            }
            ATTR_PUBLIC => {
                self.public = Some(value.to_lowercase() == "true");
                Ok(())
            }
            ATTR_PIPELINE => {
                self.pipeline = Some(value);
                Ok(())
            }
            ATTR_METHOD => {
                self.method = HttpMethod::from(&value).ok_or(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_ENDPOINT.to_owned(),
                    message: format!(
                        "An endpoint does not support '{}' in the method attribute",
                        value
                    ),
                }))?;
                Ok(())
            }
            ATTR_IMPORT => {
                match ParsedDocument::from_str(value.clone(), ctx.fs.clone()) {
                    Ok(node) => {
                        match &*(&*node).borrow() {
                            ParsedHypiSchemaElement::ApiEndpoint(endpoint) => {
                                //todo need to take the node out, maybe make endpoint an enum with a Endpoint::None for cases like this??
                                let endpoint = endpoint.replace(ParsedEndpoint::default());
                                let _ = std::mem::replace(self, endpoint);
                                Ok(())
                            }
                            _ => {
                                Err(HamlError::ParseErr(ParseErr {
                                    file: ctx.file_name.clone(),
                                    line: ctx.line_number.clone(),
                                    column: ctx.column.clone(),
                                    code: HAML_CODE_MISSING_IMPORT.clone(),
                                    element: EL_ENDPOINT.to_owned(),
                                    message: format!("Imported file '{}' found but it was not an endpoint as expected", value),
                                }))
                            }
                        }
                    }
                    Err(err) => Err(err),
                }
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_ENDPOINT.to_owned(),
                message: format!(
                    "The endpoint element does not support an attribute called '{}'.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ApiEndpointResponse(node) => {
                self.responses.push(node.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointWebsocket(node) => {
                self.websockets.push(node.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_ENDPOINT.to_owned(),
                message: format!(
                    "The endpoint element does not support '{}' elements inside it.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}
#[derive(Debug)]
pub struct ParsedEndpointResponse {
    pub start_pos: Location,
    pub end_pos: Location,
    pub status: u16,
    pub when: Option<String>,
    pub yield_expr: Option<String>,
    ///A response body template
    pub body: Option<String>,
    pub mappings: Mappings,
}

impl<F> HypiSchemaNode<F> for ParsedEndpointResponse
where
    F: Vfs,
{
    fn set_str_body(&mut self, _ctx: &ParseCtx<F>, value: String) -> Result<()> {
        self.body = Some(value);
        Ok(())
    }
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_STATUS => {
                self.status = match value.parse() {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(HamlError::ParseErr(ParseErr {
                            file: ctx.file_name.clone(),
                            line: ctx.line_number.clone(),
                            column: ctx.column.clone(),
                            code: HAML_CODE_UNKNOWN_ATTR.clone(),
                            element: EL_QUERY_OPTIONS_RESPONSE.to_owned(),
                            message: format!(
                                "The response status attribute must be a number - got '{}'. {:?}",
                                value, e
                            ),
                        }));
                    }
                };
                Ok(())
            }
            ATTR_WHEN => {
                self.when = Some(value);
                Ok(())
            }
            ATTR_YIELD => {
                self.yield_expr = Some(value);
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_QUERY_OPTIONS_RESPONSE.to_owned(),
                message: format!(
                    "The response element does not support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(mapping) => {
                self.mappings.push(mapping.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_ENDPOINT.to_owned(),
                message: format!(
                    "The response element doesn't support '{}' as a child.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedEndpointSql {
    pub start_pos: Location,
    pub end_pos: Location,
    pub sql: String,
    pub db_name: Option<String>,
    pub label: Option<String>,
    pub mappings: Mappings,
    pub is_async: bool,
}

impl<F> HypiSchemaNode<F> for ParsedEndpointSql
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_ASYNC => {
                self.is_async = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_DB => {
                self.db_name = Some(value);
                Ok(())
            }
            ATTR_LABEL => {
                self.label = Some(value);
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_SQL.to_owned(),
                message: format!("The sql element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn set_str_body(&mut self, _ctx: &ParseCtx<F>, value: String) -> Result<()> {
        self.sql = value;
        Ok(())
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(mapping) => {
                self.mappings.push(mapping.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_SQL.to_owned(),
                message: format!(
                    "The SQL element doesn't support '{}' as a child.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedEndpointFn {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub label: Option<String>,
    pub version: String,
    pub mappings: Mappings,
    pub is_async: bool,
}

impl<F> HypiSchemaNode<F> for ParsedEndpointFn
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_LABEL => {
                self.label = Some(value);
                Ok(())
            }
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_ASYNC => {
                self.is_async = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_VERSION => {
                self.version = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_FN.to_owned(),
                message: format!("The fn element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(mapping) => {
                self.mappings.push(mapping.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_FN.to_owned(),
                message: format!(
                    "The fn element doesn't support '{}' as a child.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedEndpointWebsocket {
    pub start_pos: Location,
    pub end_pos: Location,
    pub base: String,
    pub sources: Vec<String>,
}

impl<F> HypiSchemaNode<F> for ParsedEndpointWebsocket
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_BASE => {
                self.base = value;
                Ok(())
            }
            ATTR_SOURCE => {
                for src in value.split("|") {
                    self.sources.push(src.to_owned());
                }
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_WEBSOCKET.to_owned(),
                message: format!(
                    "The websocket element doesn't support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_WEBSOCKET.to_owned(),
                message: format!(
                    "The websocket element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedGraphQL {
    pub start_pos: Location,
    pub end_pos: Location,
    pub base: String,
    pub from: String,
    pub enable_subscriptions: bool,
}

impl<F> HypiSchemaNode<F> for ParsedGraphQL
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_BASE => {
                self.base = value;
                Ok(())
            }
            ATTR_FROM => {
                self.from = value;
                Ok(())
            }
            ATTR_ENABLE_SUBSCRIPTIONS => {
                self.enable_subscriptions = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_GRAPHQL.to_owned(),
                message: format!(
                    "The graphql element doesn't support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_GRAPHQL.to_owned(),
                message: format!(
                    "The graphql element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedJob {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub pipeline: String,
    pub start: String,
    pub end: String,
    pub interval: String,
    pub interval_frequency: String,
    pub enabled: bool,
    pub repeats: bool,
}

impl<F> HypiSchemaNode<F> for ParsedJob
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_PIPELINE => {
                self.pipeline = value;
                Ok(())
            }
            ATTR_ENABLED => {
                self.enabled = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_REPEATS => {
                self.repeats = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_START => {
                self.start = value;
                Ok(())
            }
            ATTR_END => {
                self.end = value;
                Ok(())
            }
            ATTR_INTERVAL => {
                self.interval = value;
                Ok(())
            }
            ATTR_INTERVAL_FREQUENCY => {
                self.interval_frequency = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_JOB.to_owned(),
                message: format!("The job element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_JOB.to_owned(),
                message: format!(
                    "The job element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScriptType {
    ///Pure V8 based JavaScript
    JavaScript,
}
#[derive(Debug)]
pub struct ParsedEndpointScript {
    pub start_pos: Location,
    pub end_pos: Location,
    ///the name of the file containing the script's contents
    pub file: String,
    pub label: Option<String>,
    pub typ: ScriptType,
    pub is_async: bool,
}

impl<F> HypiSchemaNode<F> for ParsedEndpointScript
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_LABEL => {
                self.label = Some(value);
                Ok(())
            }
            ATTR_ASYNC => {
                self.is_async = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_IMPORT => {
                self.file = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_JOB.to_owned(),
                message: format!("The script element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_JOB.to_owned(),
                message: format!(
                    "The script element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedCall {
    pub start_pos: Location,
    pub end_pos: Location,
    pub target: String,
    pub label: Option<String>,
    pub mappings: Mappings,
    pub is_async: bool,
}

impl<F> HypiSchemaNode<F> for ParsedCall
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        match name.to_lowercase().as_str() {
            ATTR_LABEL => {
                self.label = Some(value);
                Ok(())
            }
            ATTR_ASYNC => {
                self.is_async = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            ATTR_TARGET => {
                self.target = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_CALL.to_owned(),
                message: format!("The call element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(mapping) => {
                self.mappings.push(mapping.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_CALL.to_owned(),
                message: format!(
                    "The call element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub enum ParsedStep {
    Sql(NodePtr<ParsedEndpointSql>),
    Fn(NodePtr<ParsedEndpointFn>),
    Call(NodePtr<ParsedCall>),
    Script(NodePtr<ParsedEndpointScript>),
    Pipeline(NodePtr<ParsedPipeline>),
}
#[derive(Debug)]
pub struct ParsedPipeline {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub label: Option<String>,
    pub steps: NodePtr<Vec<ParsedStep>>,
    pub docker_steps: NodePtr<Vec<NodePtr<ParsedDockerStep>>>,
    pub is_async: bool,
}

impl<F> HypiSchemaNode<F> for ParsedPipeline
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        if attr_name == ATTR_IMPORT && ctx.attributes.len() > 1 {
            return Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_MISSING_IMPORT.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!(
                    "The import attribute cannot be combined with any others. Attempting to import '{}' and mixing it with '{:?}'.",
                    value,
                    ctx.attributes.iter().filter(|v| v.name.local_name.to_lowercase() != ATTR_IMPORT).map(|v| v.name.local_name.clone()).collect::<Vec<_>>().join(",")
                ),
            }));
        }
        match attr_name {
            ATTR_IMPORT => match ParsedDocument::from_str(value.clone(), ctx.fs.clone()) {
                Ok(node) => match &*(&*node).borrow() {
                    ParsedHypiSchemaElement::Pipeline(pipeline) => {
                        let pipeline = pipeline.replace(ParsedPipeline {
                            start_pos: Default::default(),
                            end_pos: Default::default(),
                            name: "".to_string(),
                            label: None,
                            steps: new_node_ptr(vec![]),
                            docker_steps: new_node_ptr(vec![]),
                            is_async: false,
                        });
                        let _ = std::mem::replace(self, pipeline);
                        Ok(())
                    }
                    _ => Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_MISSING_IMPORT.clone(),
                        element: EL_PIPELINE.to_owned(),
                        message: format!(
                            "Imported file '{}' found but it was not an endpoint as expected",
                            value
                        ),
                    })),
                },
                Err(err) => Err(err),
            },
            ATTR_LABEL => {
                self.label = Some(value);
                Ok(())
            }
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_ASYNC => {
                self.is_async = value.to_ascii_lowercase() == "true";
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!(
                    "The pipeline element doesn't support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::DockerStep(f) => {
                self.docker_steps.borrow_mut().push(f.clone());
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointFn(f) => {
                self.steps.borrow_mut().push(ParsedStep::Fn(f.clone()));
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointSql(f) => {
                self.steps.borrow_mut().push(ParsedStep::Sql(f.clone()));
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointCall(f) => {
                self.steps.borrow_mut().push(ParsedStep::Call(f.clone()));
                Ok(())
            }
            ParsedHypiSchemaElement::ApiEndpointScript(f) => {
                self.steps.borrow_mut().push(ParsedStep::Script(f.clone()));
                Ok(())
            }
            ParsedHypiSchemaElement::Pipeline(f) => {
                self.steps
                    .borrow_mut()
                    .push(ParsedStep::Pipeline(f.clone()));
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!(
                    "The pipeline element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedMeta {
    pub start_pos: Location,
    pub end_pos: Location,
    pub key_value_pairs: NodePtr<Vec<NodePtr<ParsedKeyValuePair>>>,
}

impl<F> HypiSchemaNode<F> for ParsedMeta
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, _value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            val => {
                return Err(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_META.to_owned(),
                    message: format!("meta elements do not support an attribute called '{}'", val),
                }));
            }
        }
    }

    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Pair(node) => {
                self.key_value_pairs.borrow_mut().push(node.clone());
                Ok(())
            }
            el => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_META.to_owned(),
                message: format!(
                    "The meta element does not support '{}' elements inside it.",
                    el.name()
                ),
            })),
        }
    }
}

#[derive(Debug)]
pub struct ParsedKeyValuePair {
    pub start_pos: Location,
    pub end_pos: Location,
    pub key: String,
    pub value: String,
}

impl<F> HypiSchemaNode<F> for ParsedKeyValuePair
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            ATTR_KEY => {
                self.key = value;
                Ok(())
            }
            ATTR_VALUE => {
                self.value = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PAIR.to_owned(),
                message: format!("The pair element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PAIR.to_owned(),
                message: format!(
                    "The pair element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }

    fn validate(&mut self, _ctx: &ParseCtx<F>) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ParsedSchema {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub tables: NodePtr<ParsedTables>,
}

impl<F> HypiSchemaNode<F> for ParsedSchema
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_SCHEMA.to_owned(),
                message: format!(
                    "The db schema element doesn't support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ParsedTables(node) => {
                self.tables = node.clone();
                Ok(())
            }
            ParsedHypiSchemaElement::ParsedTable(node) => {
                self.tables.borrow_mut().push(node.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_SCHEMA.to_owned(),
                message: format!(
                    "The db schema element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }

    fn validate(&mut self, _ctx: &ParseCtx<F>) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ParsedConstraint {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub columns: Vec<String>,
    pub typ: TableConstraintType,
    pub mappings: NodePtr<Mappings>,
}

impl<F> HypiSchemaNode<F> for ParsedConstraint
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_COLUMNS => {
                self.columns = value.split(",").map(|v| v.to_string()).collect();
                Ok(())
            }
            ATTR_ON_DELETE => {
                let action = match value.to_lowercase().as_str() {
                    "cascade" => {ConstraintViolationAction::Cascade}
                    "restrict" => {ConstraintViolationAction::Restrict}
                    _ => return Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_UNKNOWN_ATTR.clone(),
                        element: EL_SCHEMA.to_owned(),
                        message: format!(
                            "The on_delete attr doesn't support '{}', only cascade OR restrict are allowed.",
                            name
                        ),
                    }))
                };
                match &mut self.typ {
                    TableConstraintType::Unique => {
                        //if it is uniq, replace
                        self.typ = TableConstraintType::ForeignKey {
                            on_delete: Some(action),
                            on_update: None,
                        }
                    }
                    TableConstraintType::ForeignKey { on_delete, .. } => *on_delete = Some(action),
                }
                Ok(())
            }
            ATTR_ON_UPDATE => {
                let action = match value.to_lowercase().as_str() {
                    "cascade" => {ConstraintViolationAction::Cascade}
                    "restrict" => {ConstraintViolationAction::Restrict}
                    _ => return Err(HamlError::ParseErr(ParseErr {
                        file: ctx.file_name.clone(),
                        line: ctx.line_number.clone(),
                        column: ctx.column.clone(),
                        code: HAML_CODE_UNKNOWN_ATTR.clone(),
                        element: EL_SCHEMA.to_owned(),
                        message: format!(
                            "The on_update attr doesn't support '{}', only cascade OR restrict are allowed.",
                            name
                        ),
                    }))
                };
                match &mut self.typ {
                    TableConstraintType::Unique => {
                        //if it is uniq, replace
                        self.typ = TableConstraintType::ForeignKey {
                            on_delete: None,
                            on_update: Some(action),
                        }
                    }
                    TableConstraintType::ForeignKey { on_update, .. } => *on_update = Some(action),
                }
                Ok(())
            }
            ATTR_TYPE => {
                match value.to_lowercase().as_str() {
                    FK_TYPE_UNIQUE => {
                        self.typ = TableConstraintType::Unique;
                    }
                    FK_TYPE_FOREIGN => {
                        match self.typ {
                            TableConstraintType::Unique => {
                                //if it is uniq, replace
                                self.typ = TableConstraintType::ForeignKey {
                                    on_delete: None,
                                    on_update: None,
                                }
                            }
                            //if it is already FK no action needed
                            TableConstraintType::ForeignKey { .. } => {}
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_SCHEMA.to_owned(),
                message: format!(
                    "The table constraint element doesn't support a '{}' attribute.",
                    name
                ),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::Mapping(node) => {
                self.mappings.borrow_mut().push(node.clone());
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_SCHEMA.to_owned(),
                message: format!(
                    "The db schema element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }

    fn validate(&mut self, _ctx: &ParseCtx<F>) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub struct ParsedDb {
    pub start_pos: Location,
    pub end_pos: Location,
    pub label: String,
    pub db_name: String,
    pub host: String,
    pub port: Option<u16>,
    pub typ: DatabaseType,
    pub username: String,
    pub password: String,
    pub options: Option<String>,
    pub schemas: NodePtr<Vec<NodePtr<ParsedSchema>>>,
}

impl<F> HypiSchemaNode<F> for ParsedDb
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            ATTR_LABEL => {
                self.label = value;
                Ok(())
            }
            ATTR_DB_NAME => {
                self.db_name = value;
                Ok(())
            }
            ATTR_HOST => {
                self.host = value;
                Ok(())
            }
            ATTR_PORT => {
                self.port = value.parse().ok();
                Ok(())
            }
            ATTR_USERNAME => {
                self.username = value;
                Ok(())
            }
            ATTR_PASSWORD => {
                self.password = value;
                Ok(())
            }
            ATTR_OPTIONS => {
                self.options = Some(value);
                Ok(())
            }
            ATTR_TYPE => {
                self.typ = DatabaseType::from(&value).ok_or(HamlError::ParseErr(ParseErr {
                    file: ctx.file_name.clone(),
                    line: ctx.line_number.clone(),
                    column: ctx.column.clone(),
                    code: HAML_CODE_UNKNOWN_ATTR.clone(),
                    element: EL_DB.to_owned(),
                    message: format!(
                        "The db element doesn't support '{}' as a database type.",
                        value
                    ),
                }))?;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_DB.to_owned(),
                message: format!("The db element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            ParsedHypiSchemaElement::ParsedSchema(schema) => {
                Ok(self.schemas.borrow_mut().push(schema.clone()))
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!(
                    "The db element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }

    fn validate(&mut self, ctx: &ParseCtx<F>) -> Result<()> {
        if self.db_name.trim().is_empty() {
            Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_SQL.to_owned(),
                message: "db_name is required.".to_string(),
            }))
        } else if self.host.trim().is_empty() {
            Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_SQL.to_owned(),
                message: "host is required.".to_string(),
            }))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct ParsedEnv {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub value: String,
}

impl<F> HypiSchemaNode<F> for ParsedEnv
where
    F: Vfs,
{
    fn set_attr(&mut self, ctx: &ParseCtx<F>, name: String, value: String) -> Result<()> {
        let attr_name = name.to_lowercase();
        let attr_name = attr_name.as_str();
        match attr_name {
            ATTR_NAME => {
                self.name = value;
                Ok(())
            }
            ATTR_VALUE => {
                self.value = value;
                Ok(())
            }
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNKNOWN_ATTR.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!("The env element doesn't support a '{}' attribute.", name),
            })),
        }
    }
    fn append_child(
        &mut self,
        ctx: &ParseCtx<F>,
        node: NodePtr<ParsedHypiSchemaElement>,
    ) -> Result<()> {
        match &*(*node).borrow() {
            _ => Err(HamlError::ParseErr(ParseErr {
                file: ctx.file_name.clone(),
                line: ctx.line_number.clone(),
                column: ctx.column.clone(),
                code: HAML_CODE_UNSUPPORTED_CHILD.clone(),
                element: EL_PIPELINE.to_owned(),
                message: format!(
                    "The env element does not support '{}' child elements.",
                    (*node).borrow().name()
                ),
            })),
        }
    }
}
