use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use rapid_fs::vfs::*;
use haml::{ColumnType, CoreApi, DatabaseType, ParsedDocument, ParsedHypiSchemaElement, WellKnownType};

mod common;

#[test]
fn can_parse_haml() -> haml::Result<()> {
    let node = ParsedDocument::from_str(
        "schema.xml".to_owned(),
        Arc::new(BoundVfs::new(
            DomainOptions {
                service_id: 123,
                version: "v1".to_string(),
            },
            Arc::new(MemoryVfs {
                root: PathBuf::from("/private/path/to/services"), //cannot be empty, all paths must start with this
                data: HashMap::from([
                    (
                        "/private/path/to/services/123/versions/v1/schema.xml".to_owned(),
                        common::read_str_resource("schema.xml"),
                    ),
                    (
                        "/private/path/to/services/123/versions/v1/pipeline_register.xml"
                            .to_owned(),
                        common::read_str_resource("pipeline_register.xml"),
                    ),
                    (
                        "/private/path/to/services/123/versions/v1/pipeline2.xml".to_owned(),
                        common::read_str_resource("pipeline2.xml"),
                    ),
                    (
                        "/private/path/to/services/123/versions/v1/endpoint_subscription.xml"
                            .to_owned(),
                        common::read_str_resource("endpoint_subscription.xml"),
                    ),
                    (
                        "/private/path/to/services/123/versions/v1/table_team_icon.xml".to_owned(),
                        common::read_str_resource("table_team_icon.xml"),
                    ),
                ]),
            }),
        )),
    )?;
    match &*node.borrow() {
        ParsedHypiSchemaElement::ParsedDocument(schema) => {
            let doc = schema.borrow();
            let dbs = &*doc.databases.borrow();
            let db8 = &*dbs[0].borrow();
            let schema = &*db8.schemas.borrow();
            let schema = &*schema[0].borrow();
            let tables = &*schema.tables.borrow();
            let apis = doc.apis.borrow();
            let env = &*doc.env.borrow();
            let step_builders = &*doc.step_builders.borrow();
            assert_eq!(step_builders.len(), 1);
            assert_eq!(
                (&*step_builders[0].borrow()).username,
                Some("user".to_string())
            );
            assert_eq!(
                (&*step_builders[0].borrow()).password,
                Some("pass".to_string())
            );
            assert_eq!(
                (&*step_builders[0].borrow()).image,
                "docker.host.com/image".to_string()
            );
            assert_eq!((&*step_builders[0].borrow()).tag, Some("tag".to_string()));
            assert_eq!(env.len(), 1);
            assert_eq!(dbs.len(), 6);
            assert_eq!(env[0].borrow().name, "API_KEY");
            assert_eq!(env[0].borrow().value, "abc.123");
            let types = vec![
                DatabaseType::MekaDb,
                DatabaseType::Postgres,
                DatabaseType::MySQL,
                DatabaseType::MariaDB,
                DatabaseType::Oracle,
                DatabaseType::MsSql,
            ];
            for i in 1..7 {
                assert_eq!(dbs[i - 1].borrow().label, format!("db{}", i));
                assert_eq!(dbs[i - 1].borrow().typ, types[i - 1]);
                assert_eq!(dbs[i - 1].borrow().username, format!("user{}", i));
                assert_eq!(dbs[i - 1].borrow().password, format!("pass{}", i));
            }
            assert_eq!(tables.len(), 13);
            assert_eq!(tables[0].borrow().name, "account".to_owned());
            assert_eq!(tables[1].borrow().name, "file".to_owned());
            assert_eq!(tables[2].borrow().name, "conversation".to_owned());
            assert_eq!(tables[3].borrow().name, "conversation_member".to_owned());
            assert_eq!(tables[4].borrow().name, "conversation_purpose".to_owned());
            assert_eq!(tables[5].borrow().name, "conversation_topic".to_owned());
            assert_eq!(tables[6].borrow().name, "message".to_owned());
            assert_eq!(tables[7].borrow().name, "block".to_owned());
            assert_eq!(tables[8].borrow().name, "message_block".to_owned());
            assert_eq!(tables[9].borrow().name, "team".to_owned());
            assert_eq!(tables[10].borrow().name, "team_icon".to_owned());
            assert_eq!(tables[11].borrow().name, "team_member".to_owned());
            assert_eq!(tables[12].borrow().name, "team_name_reservation".to_owned());
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[0].borrow()).name,
                "username".to_owned()
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[0].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[0].borrow()).nullable,
                true
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[0].borrow()).primary_key,
                true
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[1].borrow()).name,
                "email".to_owned()
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[1].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[1].borrow()).nullable,
                false
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[2].borrow()).name,
                "password".to_owned()
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[2].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[0].borrow().columns.borrow()[2].borrow()).nullable,
                false
            );
            if let Some(pipeline) = (&*tables[0].borrow().columns.borrow()[2].borrow())
                .pipeline
                .clone()
            {
                assert_eq!(
                    &*(&pipeline.borrow().args).clone().unwrap().borrow().value,
                    "bcrypt1"
                );
                assert_eq!(
                    &*(&pipeline.borrow().write).clone().unwrap().borrow().value,
                    "bcrypt2"
                );
                assert_eq!(
                    &*(&pipeline.borrow().read).clone().unwrap().borrow().value,
                    "null"
                );
            } else {
                panic!("Password pipeline missing");
            };
            assert_eq!(
                tables[0]
                    .borrow()
                    .hypi
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .well_known
                    .as_ref(),
                Some(&WellKnownType::Account)
            );
            assert_eq!(
                tables[0].borrow().hypi.as_ref().unwrap().borrow().mappings[0]
                    .borrow()
                    .from,
                "username".to_owned()
            );
            assert_eq!(
                tables[0].borrow().hypi.as_ref().unwrap().borrow().mappings[0]
                    .borrow()
                    .to
                    .as_ref()
                    .unwrap()
                    .clone(),
                "xyz".to_owned()
            );

            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[0].borrow()).name,
                "name".to_owned()
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[0].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[0].borrow()).nullable,
                true
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[0].borrow()).primary_key,
                false
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[1].borrow()).name,
                "path".to_owned()
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[1].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[1].borrow()).nullable,
                true
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[2].borrow()).name,
                "type".to_owned()
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[2].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[2].borrow()).nullable,
                true
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[3].borrow()).name,
                "size_in_bytes".to_owned()
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[3].borrow()).typ,
                ColumnType::BIGINT
            );
            assert_eq!(
                (&*tables[1].borrow().columns.borrow()[3].borrow()).nullable,
                true
            );
            assert_eq!(
                tables[1]
                    .borrow()
                    .hypi
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .well_known
                    .as_ref()
                    .unwrap(),
                &WellKnownType::File
            );

            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[0].borrow()).name,
                "label".to_owned()
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[0].borrow()).typ,
                ColumnType::TEXT
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[0].borrow()).nullable,
                false
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[0].borrow()).primary_key,
                false
            );

            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[1].borrow()).name,
                "is_archived".to_owned()
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[1].borrow()).typ,
                ColumnType::BOOL
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[1].borrow()).nullable,
                false
            );
            assert_eq!(
                (&*tables[2].borrow().columns.borrow()[1].borrow()).primary_key,
                false
            );
            assert_eq!(tables[2].borrow().columns.borrow().len(), 16);

            assert_eq!(
                apis.global_options
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .core_apis
                    .len(),
                12
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[0],
                CoreApi::Register
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[1],
                CoreApi::LoginByEmail
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[2],
                CoreApi::LoginByUsername
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[3],
                CoreApi::OAuth
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[4],
                CoreApi::PasswordResetTrigger
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[5],
                CoreApi::PasswordReset
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[6],
                CoreApi::VerifyAccount
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[7],
                CoreApi::MagicLink
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[8],
                CoreApi::TwoFactorAuthEmail
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[9],
                CoreApi::TwoFactorAuthSms
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[10],
                CoreApi::TwoFactorStep2
            );
            assert_eq!(
                apis.global_options.as_ref().unwrap().borrow().core_apis[11],
                CoreApi::TwoFactorTotp
            );
            //
            assert_eq!(apis.rest.as_ref().unwrap().borrow().base, "/api");
            assert_eq!(
                apis.rest.as_ref().unwrap().borrow().endpoints[0]
                    .borrow()
                    .name,
                Some("create_team".to_owned())
            );
            assert_eq!(
                apis.rest.as_ref().unwrap().borrow().endpoints[0]
                    .borrow()
                    .path,
                Some("team".to_owned())
            );
            assert_eq!(
                apis.rest.as_ref().unwrap().borrow().endpoints[0]
                    .borrow()
                    .accepts,
                Some("application/json".to_owned())
            );
            assert_eq!(
                apis.rest.as_ref().unwrap().borrow().endpoints[0]
                    .borrow()
                    .produces,
                Some("application/json".to_owned())
            );
            //
            //assert_eq!(apis.rest.as_ref().unwrap().borrow().endpoints[1].borrow().post.as_ref().unwrap().borrow().input.as_ref().unwrap().borrow().pipeline.borrow().steps[2].borrow().target, "endpoint.claim_domain.post");
        }
        _ => panic!("Expected a schema"),
    };
    Ok(())
}
