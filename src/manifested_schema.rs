use rapid_utils::http_utils::HttpMethod;

use crate::{
    CoreApi, DatabaseType, DockerConnectionInfo, DockerStepProvider, ImplicitDockerStepPosition,
    Location, TableConstraintType,
};
use crate::haml_parser::{ColumnDefault, ColumnType, ParsedColumn, ParsedColumnPipeline, ParsedConstraint, ParsedDb, ParsedDockerStep, ParsedDocument, ParsedEndpoint, ParsedEndpointResponse,  ParsedEnv, ParsedGraphQL, ParsedHypi, ParsedJob, ParsedKeyValuePair, ParsedMapping, ParsedMeta, ParsedPipeline, ParsedRest, ParsedSchema, ParsedTable, WellKnownType};

#[derive(Clone, Debug)]
pub struct DocumentDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub crud_enabled_tables: Vec<String>,
    pub enabled_core_apis: Vec<CoreApi>,
    pub rest: Option<RestApiDef>,
    pub graphql: Option<GraphQLApiDef>,
    pub jobs: Vec<JobDef>,
    pub databases: Vec<DatabaseDef>,
    pub env: Vec<EnvVar>,
    pub step_builders: Vec<DockerConnectionInfo>,
    pub meta: MetaDef,
}

impl From<&ParsedDocument> for DocumentDef {
    fn from(value: &ParsedDocument) -> Self {
        let apis = &*value.apis.borrow();
        let doc = DocumentDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            crud_enabled_tables: apis
                .global_options
                .as_ref()
                .map(|v| (&*v.borrow()).explicitly_enabled_crud_tables.clone())
                .unwrap_or_else(|| vec![]),
            enabled_core_apis: apis
                .global_options
                .as_ref()
                .map(|v| (&*v.borrow()).core_apis.clone())
                .unwrap_or_else(|| vec![]),
            rest: apis.rest.as_ref().map(|v| (&*v.borrow()).into()),
            graphql: apis.graphql.as_ref().map(|v| (&*v.borrow()).into()),
            jobs: (&*apis.jobs.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
            databases: (&*value.databases.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
            env: (&*value.env.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
            step_builders: (&*value.step_builders.borrow())
                .iter()
                .map(|v| (&*v.borrow()).clone())
                .collect(),
            meta: (&*value.meta.borrow()).into(),
        };
        doc
    }
}

#[derive(Clone, Debug)]
pub struct MetaDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub pairs: Vec<PairDef>,
}

impl From<&ParsedMeta> for MetaDef {
    fn from(value: &ParsedMeta) -> Self {
        MetaDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),

            pairs: value
                .key_value_pairs
                .borrow()
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PairDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub key: String,
    pub value: String,
}

impl From<&ParsedKeyValuePair> for PairDef {
    fn from(value: &ParsedKeyValuePair) -> Self {
        PairDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            key: value.key.clone(),
            value: value.value.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphQLApiDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub base: String,
    pub from: String,
    pub enable_subscriptions: bool,
}

impl From<&ParsedGraphQL> for GraphQLApiDef {
    fn from(value: &ParsedGraphQL) -> Self {
        GraphQLApiDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            base: value.base.clone(),
            from: value.from.clone(),
            enable_subscriptions: value.enable_subscriptions,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JobDef {
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

impl From<&ParsedJob> for JobDef {
    fn from(value: &ParsedJob) -> Self {
        JobDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.clone(),
            pipeline: value.pipeline.clone(),
            start: value.start.clone(),
            end: value.end.clone(),
            interval: value.interval.clone(),
            interval_frequency: value.interval_frequency.clone(),
            enabled: value.enabled,
            repeats: value.repeats,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RestApiDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub base: String,
    pub endpoints: Vec<EndpointDef>,
}

impl From<&ParsedRest> for RestApiDef {
    fn from(value: &ParsedRest) -> Self {
        RestApiDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            base: value.base.clone(),
            endpoints: value
                .endpoints
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EndpointDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub method: HttpMethod,
    pub path: Option<String>,
    pub name: Option<String>,
    pub public: Option<bool>,
    pub accepts: Option<String>,
    pub produces: Option<String>,
    ///The name of the pipeline which is executed when this endpoint is called
    pub pipeline: Pipeline,
    pub responses: Vec<ResponseDef>,
}

impl From<&ParsedEndpoint> for EndpointDef {
    fn from(value: &ParsedEndpoint) -> Self {
        EndpointDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            method: value.method.clone(),
            path: value.path.clone(),
            name: value.name.clone(),
            public: value.public.clone(),
            accepts: value.accepts.clone(),
            produces: value.produces.clone(),
            pipeline: (&*value.pipeline.borrow()).into(),
            responses: value
                .responses
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResponseDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub status: u16,
    pub when: Option<String>,
    pub yield_expr: Option<String>,
    ///A response body template
    pub body: Option<String>,
    pub mappings: Vec<Mapping>,
}

impl From<&ParsedEndpointResponse> for ResponseDef {
    fn from(value: &ParsedEndpointResponse) -> Self {
        ResponseDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            status: value.status,
            when: value.when.clone(),
            yield_expr: value.yield_expr.clone(),
            body: value.body.clone(),
            mappings: value
                .mappings
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TableDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<ConstraintDef>,
    pub hypi: Option<HypiDef>,
}

impl From<&ParsedTable> for TableDef {
    fn from(value: &ParsedTable) -> Self {
        TableDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.to_owned(),
            columns: (&*value.columns.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
            constraints: (&*value.constraints.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
            hypi: value.hypi.as_ref().map(|v| (&*v.borrow()).into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub typ: ColumnType,
    pub nullable: bool,
    pub unique: bool,
    pub default: Option<ColumnDefault>,
    pub primary_key: bool,
    pub pipeline: Option<ColumnPipeline>,
}

impl From<&ParsedColumn> for ColumnDef {
    fn from(value: &ParsedColumn) -> Self {
        ColumnDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.clone(),
            typ: value.typ.clone(),
            nullable: value.nullable,
            unique: value.unique,
            default: value.default.clone(),
            primary_key: value.primary_key,
            pipeline: value.pipeline.as_ref().map(|v| (&*v.borrow()).into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConstraintDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub columns: Vec<String>,
    pub typ: TableConstraintType,
    pub mappings: Vec<Mapping>,
}

impl From<&ParsedConstraint> for ConstraintDef {
    fn from(value: &ParsedConstraint) -> Self {
        ConstraintDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.clone(),
            typ: value.typ.clone(),
            columns: value.columns.clone(),
            mappings: (&*value.mappings.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ColumnPipeline {
    pub args_start_pos: Option<Location>,
    pub args_end_pos: Option<Location>,
    pub write_start_pos: Option<Location>,
    pub write_end_pos: Option<Location>,
    pub read_start_pos: Option<Location>,
    pub read_end_pos: Option<Location>,
    ///always apply
    pub args: Vec<String>,
    ///apply if writing
    pub write: Vec<String>,
    ///apply if reading
    pub read: Vec<String>,
}

impl From<&ParsedColumnPipeline> for ColumnPipeline {
    fn from(value: &ParsedColumnPipeline) -> Self {
        ColumnPipeline {
            args_start_pos: value
                .args
                .as_ref()
                .map(|v| (&*v.borrow()).start_pos.clone()),
            args_end_pos: value.args.as_ref().map(|v| (&*v.borrow()).end_pos.clone()),
            write_start_pos: value
                .write
                .as_ref()
                .map(|v| (&*v.borrow()).start_pos.clone()),
            write_end_pos: value.write.as_ref().map(|v| (&*v.borrow()).end_pos.clone()),
            read_start_pos: value
                .read
                .as_ref()
                .map(|v| (&*v.borrow()).start_pos.clone()),
            read_end_pos: value.read.as_ref().map(|v| (&*v.borrow()).end_pos.clone()),
            args: value
                .args
                .as_ref()
                .map(|v| {
                    (&*v.borrow())
                        .value
                        .split("|")
                        .map(|v| v.to_string())
                        .collect()
                })
                .clone()
                .unwrap_or_else(|| vec![]),
            write: value
                .args
                .as_ref()
                .map(|v| {
                    (&*v.borrow())
                        .value
                        .split("|")
                        .map(|v| v.to_string())
                        .collect()
                })
                .clone()
                .unwrap_or_else(|| vec![]),
            read: value
                .args
                .as_ref()
                .map(|v| {
                    (&*v.borrow())
                        .value
                        .split("|")
                        .map(|v| v.to_string())
                        .collect()
                })
                .clone()
                .unwrap_or_else(|| vec![]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HypiDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub well_known: Option<WellKnownType>,
    pub mappings: Vec<Mapping>,
}

impl From<&ParsedHypi> for HypiDef {
    fn from(value: &ParsedHypi) -> Self {
        HypiDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            well_known: value.well_known.as_ref().map(|v| v.clone()),
            mappings: value
                .mappings
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mapping {
    pub start_pos: Location,
    pub end_pos: Location,
    pub from: String,
    pub to: Option<String>,
    pub typ: Option<ColumnType>,
    pub children: Vec<Mapping>,
}

impl From<&ParsedMapping> for Mapping {
    fn from(value: &ParsedMapping) -> Self {
        Mapping {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            from: value.from.clone(),
            to: value.to.clone(),
            typ: value.typ.clone(),
            children: value
                .children
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub label: Option<String>,
    pub steps: Vec<DockerStep>,
    pub is_async: bool,
}

impl From<&ParsedPipeline> for Pipeline {
    fn from(value: &ParsedPipeline) -> Self {
        Pipeline {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.to_owned(),
            label: value.label.to_owned(),
            is_async: value.is_async,
            steps: value
                .steps
                .borrow()
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DockerStep {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub provider: DockerStepProvider,
    pub mappings: Vec<Mapping>,
    pub implicit_before_position: Option<ImplicitDockerStepPosition>,
    pub implicit_after_position: Option<ImplicitDockerStepPosition>,
}

impl From<&ParsedDockerStep> for DockerStep {
    fn from(value: &ParsedDockerStep) -> Self {
        DockerStep {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.to_owned(),
            provider: value.provider.to_owned(),
            implicit_before_position: value.implicit_before_position.clone(),
            implicit_after_position: value.implicit_after_position.clone(),
            mappings: value
                .mappings
                .borrow()
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchemaDef {
    pub name: String,
    pub tables: Vec<TableDef>,
}

impl From<&ParsedSchema> for SchemaDef {
    fn from(value: &ParsedSchema) -> Self {
        Self {
            name: value.name.clone(),
            tables: (&*value.tables.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseDef {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub typ: DatabaseType,
    pub username: String,
    pub password: String,
    pub db_name: String,
    pub host: String,
    pub port: Option<u16>,
    pub schemas: Vec<SchemaDef>,
}

impl From<&ParsedDb> for DatabaseDef {
    fn from(value: &ParsedDb) -> Self {
        DatabaseDef {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.label.to_owned(),
            typ: value.typ.to_owned(),
            username: value.username.to_owned(),
            password: value.password.to_owned(),
            db_name: value.db_name.to_owned(),
            host: value.host.to_owned(),
            port: value.port.to_owned(),
            schemas: (&*value.schemas.borrow())
                .iter()
                .map(|v| (&*v.borrow()).into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub start_pos: Location,
    pub end_pos: Location,
    pub name: String,
    pub value: String,
}

impl From<&ParsedEnv> for EnvVar {
    fn from(value: &ParsedEnv) -> Self {
        EnvVar {
            start_pos: value.start_pos.clone(),
            end_pos: value.end_pos.clone(),
            name: value.name.to_owned(),
            value: value.value.to_owned(),
        }
    }
}
