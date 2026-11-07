use serde_json::{Value, json};

use super::{MachineAddParams, MachineEditParams, MachineList, MachineListParams, MachineRemoveParams, MachineRemoved, MachineResult};
use crate::Session;
use crate::schema::tests::{assert_keys, assert_strict_keeps_todays_errors, conforms};
use crate::schema::{ParamCheck, input_schema_for, output_schema_for};

fn router() -> Value {
    json!({"name": "Router", "post": "grbl", "axes": [
        {"type": "rotary", "name": "A", "axis": [1, 0, 0], "on": "table", "limits": [-360, 360]},
        {"type": "linear", "name": "X", "min": 0, "max": 300},
    ]})
}

#[test]
fn schemas_and_the_key_check_agree() {
    assert_keys(input_schema_for::<MachineAddParams>(), &["axes", "name", "post"]);
    assert_keys(input_schema_for::<MachineEditParams>(), &["axes", "machine", "name", "post"]);
    assert_keys(input_schema_for::<MachineRemoveParams>(), &["machine"]);
    assert_keys(input_schema_for::<MachineListParams>(), &[]);
}

#[test]
fn strict_keeps_todays_errors() {
    assert_strict_keeps_todays_errors(&[
        ("cam.machine.add", json!({"name": "Empty", "axes": []})),
        ("cam.machine.add", json!({"name": "Bad", "axes": [{"type": "linear", "name": "X", "min": 1, "max": 0}]})),
        ("cam.machine.add", json!({"name": "Bad", "axes": [{"type": "linear", "name": "A", "min": 0, "max": 1}]})),
        ("cam.machine.add", json!({"name": "Bad", "axes": [{"type": "linear", "name": "X", "min": 0, "max": 1}], "post": "fanuc"})),
        ("cam.machine.edit", json!({"machine": "Haas", "name": "Mill"})),
        ("cam.machine.remove", json!({"machine": "Haas"})),
    ]);
}

/// Adding, editing, listing and removing reply as their output schemas say, in Warn without
/// warnings: the edit clears the post with `null`, and the list carries both kinds of axis.
#[test]
fn machine_commands_reply_with_their_schemas() {
    let mut s = Session { param_check: ParamCheck::Warn, ..Session::default() };
    let run = |s: &mut Session, id: &str, p: Value| s.execute(id, &p).unwrap_or_else(|e| panic!("{id} {p}: {e}"));
    let matches = |schema: schemars::Schema, r: &Value| {
        let schema = schema.as_value();
        assert!(conforms(schema, schema, r), "{r} doesn't match {schema}");
        assert!(r.get("param_warnings").is_none(), "{r}");
    };
    matches(output_schema_for::<MachineResult>(), &run(&mut s, "cam.machine.add", router()));
    matches(output_schema_for::<MachineResult>(), &run(&mut s, "cam.machine.edit", json!({"machine": "Router", "post": null, "name": "Mill"})));
    let listed = run(&mut s, "cam.machine.list", json!({}));
    matches(output_schema_for::<MachineList>(), &listed);
    assert_eq!(listed["machines"][0]["axes"][0]["limits"], json!([-360.0, 360.0]));
    matches(output_schema_for::<MachineRemoved>(), &run(&mut s, "cam.machine.remove", json!({"machine": "Mill"})));

    let mut strict = Session { param_check: ParamCheck::Strict, ..Session::default() };
    let misspelt = strict.execute("cam.machine.add", &json!({"name": "R", "axes": [], "psot": "grbl"}));
    assert!(misspelt.is_err_and(|e| e.to_string().contains("psot")));
}
