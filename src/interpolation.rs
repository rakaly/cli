//! Variable interpolation for game and mod files.

use crate::json::{check_size, Encoding};
use jomini::text::syntax::{
    de::Document,
    parse,
    resolve::{resolve_with, Resolution, Variables},
    LineIndex, OwnedBlock, OwnedItem, OwnedValue, Parse, SyntaxError, SyntaxErrorKind, TextRange,
};
use jomini::text::Operator;
use std::path::PathBuf;

/// Collect the variables of `files`, with a warning for each definition that
/// has no value. Each file sees the variables of the files before it.
pub fn load_variables(
    files: &[PathBuf],
    encoding: Encoding,
) -> anyhow::Result<(Variables, Vec<String>)> {
    let mut variables = Variables::new();
    let mut warnings = Vec::new();
    for file in files {
        let context = |e: &dyn std::fmt::Display| anyhow::anyhow!("{}: {}", file.display(), e);
        let data = std::fs::read(file).map_err(|e| context(&e))?;
        check_size(&data).map_err(|e| context(&e))?;
        let parsed = parse(&data);
        let mut messages = check_syntax(&data, parsed.errors()).map_err(|e| context(&e))?;

        let resolution = resolve_with(parsed.tree(), &variables);
        messages.extend(resolve_messages(&resolution, &data, encoding));
        warnings.extend(messages.iter().map(|x| format!("{}:{}", file.display(), x)));

        // A definition that failed removes the name, so that a later file
        // can't use a value this file replaced
        for definition in resolution.definitions() {
            match definition.value() {
                Some(value) => variables.insert(definition.name(), value.clone()),
                None => variables.remove(definition.name()),
            };
        }
    }
    Ok((variables, warnings))
}

/// A file with its variables resolved.
pub struct Interpolated {
    /// The file with each definition removed, each reference replaced by its
    /// value, and `?=` and `==` as `=`.
    pub block: OwnedBlock,
    /// Variables and calculations that have no value, as `line:col: message`.
    pub warnings: Vec<String>,
}

/// Interpolate `parsed`. A variable that it does not define yet comes from
/// `scope`.
pub fn interpolate(
    parsed: &Parse,
    scope: &Variables,
    encoding: Encoding,
) -> anyhow::Result<Interpolated> {
    let tree = parsed.tree();
    let resolution = resolve_with(tree, scope);
    let warnings = resolve_messages(&resolution, tree.source(), encoding);
    let mut block: OwnedBlock = Document::new(tree, encoding)
        .with_resolution(resolution)?
        .deserialize()?;
    normalize_operators(&mut block);
    Ok(Interpolated { block, warnings })
}

/// Write `?=` and `==` as `=`. Blocks can nest deep, so this does not recurse.
fn normalize_operators(root: &mut OwnedBlock) {
    let mut blocks = vec![root];
    while let Some(block) = blocks.pop() {
        for item in block.items_mut() {
            let value = match item {
                OwnedItem::Field(field) => {
                    if matches!(field.op, Some(Operator::Exact | Operator::Exists)) {
                        field.op = Some(Operator::Equal);
                    }
                    &mut field.value
                }
                OwnedItem::Value(value) => value,
                OwnedItem::Parameter(parameter) => {
                    blocks.push(&mut parameter.body);
                    continue;
                }
                _ => continue,
            };
            if let OwnedValue::Block(block) | OwnedValue::HeaderedBlock(_, block) = value {
                blocks.push(block);
            }
        }
    }
}

/// Split `errors` into warnings (`line:col: message`) and a fatal error.
///
/// A stray `}` or `]` and a block that is never closed are warnings, as the
/// tape has the rest of the file and game files contain them. Recovery from
/// any other error can lose data, as in `a = b = c`, which drops `a`.
pub fn check_syntax(data: &[u8], errors: &[SyntaxError]) -> anyhow::Result<Vec<String>> {
    if errors.is_empty() {
        return Ok(Vec::new());
    }
    let lines = LineIndex::new(data);
    let mut warnings = Vec::new();
    for error in errors {
        let message = format!(
            "{}: syntax error: {}",
            position(&lines, error.range),
            error.message()
        );
        match error.kind {
            SyntaxErrorKind::UnmatchedCloseBrace
            | SyntaxErrorKind::UnclosedBlock
            | SyntaxErrorKind::UnmatchedCloseBracket => warnings.push(message),
            _ => anyhow::bail!(message),
        }
    }
    Ok(warnings)
}

fn resolve_messages(resolution: &Resolution, data: &[u8], encoding: Encoding) -> Vec<String> {
    let lines = LineIndex::new(data);
    let errors = resolution.errors().iter();
    errors
        .map(|error| {
            let text = &data[start(error.range)..u32::from(error.range.end()) as usize];
            let text = jomini::Encoding::decode(&encoding, text);
            let position = position(&lines, error.range);
            format!("{}: {} `{}`", position, error.message(), text)
        })
        .collect()
}

fn start(range: TextRange) -> usize {
    u32::from(range.start()) as usize
}

fn position(lines: &LineIndex, range: TextRange) -> String {
    let pos = lines.line_col(range.start());
    format!("{}:{}", pos.line + 1, pos.col + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jomini::json::{DuplicateKeyMode, JsonOptions};

    fn interpolate(data: &[u8], scope: &Variables, encoding: Encoding) -> Interpolated {
        let parsed = parse(data);
        check_syntax(data, parsed.errors()).unwrap();
        super::interpolate(&parsed, scope, encoding).unwrap()
    }

    fn to_json_with(data: &[u8], options: JsonOptions) -> String {
        to_json_scope(data, &Variables::new(), options)
    }

    fn to_json_scope(data: &[u8], scope: &Variables, options: JsonOptions) -> String {
        let out = interpolate(data, scope, Encoding::Utf8);
        assert_eq!(out.warnings, Vec::<String>::new());
        json_with(&out, options)
    }

    fn json_with(out: &Interpolated, options: JsonOptions) -> String {
        out.block.json().with_options(options).to_string()
    }

    fn json_of(out: &Interpolated) -> String {
        json_with(out, JsonOptions::new())
    }

    fn scope_of(source: &[u8]) -> Variables {
        let parsed = parse(source);
        resolve_with(parsed.tree(), &Variables::new())
            .variables()
            .clone()
    }

    /// A temporary directory that is removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("rakaly-{}-{}", name, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn to_json(data: &[u8]) -> String {
        to_json_with(data, JsonOptions::new())
    }

    #[test]
    fn test_basic_variable_interpolation() {
        let data = br#"
@my_var = 10
my_obj = {
  pos_x = @[100-my_var]
  pos_y = @my_var
}
"#;
        assert_eq!(to_json(data), r#"{"my_obj":{"pos_x":90,"pos_y":10}}"#);
    }

    #[test]
    fn test_string_variables() {
        // https://github.com/rakaly/cli: CK3 holding buildings
        let data = br#"
@holding_illustration_western = "gfx/interface/illustrations/holding_types/city_western.dds"
@holding_illustration_norse = "gfx/interface/illustrations/holding_types/fp1_city_norse.dds"
city_01 = {
    construction_time = slow_construction_time
    assets = {
        asset = {
            type = pdxmesh
            names = { "fp1_building_norse_city_01_a_mesh" }
            illustration = @holding_illustration_norse
        }
        asset = {
            illustration = @holding_illustration_western
        }
    }
}
"#;
        assert_eq!(
            to_json(data),
            r#"{"city_01":{"construction_time":"slow_construction_time","assets":{"asset":{"type":"pdxmesh","names":["fp1_building_norse_city_01_a_mesh"],"illustration":"gfx/interface/illustrations/holding_types/fp1_city_norse.dds"},"asset":{"illustration":"gfx/interface/illustrations/holding_types/city_western.dds"}}}}"#
        );
    }

    #[test]
    fn test_unquoted_string_variable_and_arrays() {
        let data = br#"
@name = Jean
@size = 2
ruler = @name
list = { @name @size @[size * 3] }
"#;
        assert_eq!(to_json(data), r#"{"ruler":"Jean","list":["Jean",2,6]}"#);
    }

    #[test]
    fn test_variable_key() {
        let data = b"@size = width\n@@size = 3";
        assert_eq!(to_json(data), r#"{"width":3}"#);
    }

    #[test]
    fn test_arithmetic_expressions() {
        let data = br#"
@half = @[1/2]
scale = @[1-half]
scale_mul = @[1*half]
scale_add = @[1+half]
scale_div = @[1/half]
my_list = { @[1-half] @half }
"#;
        assert_eq!(
            to_json(data),
            r#"{"scale":0.5,"scale_mul":0.5,"scale_add":1.5,"scale_div":2,"my_list":[0.5,0.5]}"#
        );
    }

    #[test]
    fn test_complex_expressions() {
        let data = b"@half = @[1/2]\nmy_calc = @[(-half-half)*half]";
        assert_eq!(to_json(data), r#"{"my_calc":-0.5}"#);
    }

    #[test]
    fn test_nested_variable_filtering() {
        let data = br#"
obj = { @half = 0.5 pos_x=@half pos_y=@[half*2] }
scale = @[1-0.25]
"#;
        assert_eq!(
            to_json(data),
            r#"{"obj":{"pos_x":0.5,"pos_y":1},"scale":0.75}"#
        );
    }

    #[test]
    fn test_comprehensive_interpolation() {
        let data = br#"@base = 10
@factor = @[1/4]
@doubled = @[base*2]
@halved = @[base/2]
width = @doubled
height = @base
ratio = @factor"#;
        assert_eq!(to_json(data), r#"{"width":20,"height":10,"ratio":0.25}"#);
    }

    #[test]
    fn test_parentheses_interpolation() {
        let data = br#"
@width = 768
@cross_x = @[ ( 333 / width ) + 0.001 ]
test_value = @cross_x
"#;
        assert_eq!(to_json(data), r#"{"test_value":0.43459375}"#);
    }

    #[test]
    fn test_chained_divisions() {
        let data = br#"
@test1 = @[1/3/2]
@test2 = @[8/4/2]
@test3 = @[12/3/2/2]
result1 = @test1
result2 = @test2
result3 = @test3
"#;
        assert_eq!(
            to_json(data),
            r#"{"result1":0.16666666666666666,"result2":1,"result3":1}"#
        );
    }

    #[test]
    fn test_unary_minus_with_addition() {
        let data = br#"
@half = @[1/2]
@two = @[2]
test1 = @[-half+2]
test2 = @[-half + half]
test3 = @[-2+half]
test4 = @[-half*2+1]
test5 = @[-(half)+2]
"#;
        assert_eq!(
            to_json(data),
            r#"{"test1":1.5,"test2":0,"test3":-1.5,"test4":0,"test5":1.5}"#
        );
    }

    #[test]
    fn test_comparison_operators() {
        let data = br#"
@width = 768
@height = 512
test_gt > @[ width ]
test_lt < @[ height ]
test_gte >= @[ 700 ]
test_lte <= @[ 500 ]
test_ne != @[ 999 ]
test_exact == @[ 42 ]
test_exists ?= @[ 100 ]
test_eq = @[ width ]
"#;
        assert_eq!(
            to_json(data),
            r#"{"test_gt":{"GREATER_THAN":768},"test_lt":{"LESS_THAN":512},"test_gte":{"GREATER_THAN_EQUAL":700},"test_lte":{"LESS_THAN_EQUAL":500},"test_ne":{"NOT_EQUAL":999},"test_exact":42,"test_exists":100,"test_eq":768}"#
        );
    }

    #[test]
    fn test_direct_variable_reference() {
        let data = br#"
@var_a = 0.7
@var_b = @var_a
test_obj = {
    edge_color_mult = @var_b
}
"#;
        assert_eq!(to_json(data), r#"{"test_obj":{"edge_color_mult":0.7}}"#);
    }

    #[test]
    fn test_use_before_definition_is_reported() {
        // The game substitutes variables in source order, so a use before
        // the definition has no value.
        let data = b"@derived_value = @base_value\n@base_value = 42\nresult = @derived_value";
        let out = interpolate(data, &Variables::new(), Encoding::Utf8);
        assert_eq!(
            out.warnings,
            vec!["1:18: undefined variable `@base_value`".to_string()]
        );
    }

    #[test]
    fn test_unresolved_variable_reference() {
        let data = b"@missing_ref = @nonexistent_var\ntest = @missing_ref";
        let out = interpolate(data, &Variables::new(), Encoding::Utf8);
        assert_eq!(
            out.warnings,
            vec!["1:16: undefined variable `@nonexistent_var`".to_string()]
        );

        // The unresolved reference is kept as written
        let json = json_of(&out);
        assert_eq!(json, r#"{"test":"@missing_ref"}"#);
    }

    #[test]
    fn test_scope_variables() {
        let scope = scope_of(b"@msg = \"event:/sfx/generic\"\n@size = 2");
        let data = b"project = { sound = @msg width = @[size * 3] }";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"project":{"sound":"event:/sfx/generic","width":6}}"#
        );
    }

    #[test]
    fn test_code_payload() {
        // EU5 templates: TextTape::from_slice rejects `code [[ ... ]]`
        let data = b"@size = 2\nt = { width = @size code = [[ a = $X$ ]] }";
        assert_eq!(to_json(data), r#"{"t":{"width":2,"code":"[[ a = $X$ ]]"}}"#);
    }

    #[test]
    fn test_windows1252_bytes_are_kept() {
        let data = b"@n = \"Jos\xe9\"\nruler = @n";
        let out = interpolate(data, &Variables::new(), Encoding::Windows1252);
        let json = json_of(&out);
        assert_eq!(json, r#"{"ruler":"José"}"#);
    }

    #[test]
    fn test_quoted_escapes_are_kept() {
        let data = br#"@n = "say \"hi\""
a = @n"#;
        assert_eq!(to_json(data), r#"{"a":"say \"hi\""}"#);
    }

    #[test]
    fn test_local_definition_shadows_scope() {
        let scope = scope_of(b"@illustration = shared.dds\n@other = other.dds");
        let data = b"@illustration = local.dds\na = @illustration\nb = @other";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":"local.dds","b":"other.dds"}"#
        );
    }

    #[test]
    fn test_scope_value_applies_until_local_definition() {
        let scope = scope_of(b"@x = 1");
        let data = b"a = @x\nc = @[x * 10]\n@x = 5\nb = @x\nd = @[x * 10]";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":1,"c":10,"b":5,"d":50}"#
        );
    }

    #[test]
    fn test_failed_local_definition_does_not_use_scope() {
        // After the local definition, the scope's value must not come back
        let scope = scope_of(b"@x = 1");
        let out = interpolate(b"@x = 0\na = @[1 / x]", &scope, Encoding::Utf8);
        assert_eq!(out.warnings, ["2:7: result is not a finite number `1 / x`"]);
        let json = json_of(&out);
        assert_eq!(json, r#"{"a":"@[1 / x]"}"#);

        let data = b"@y = @missing\n@x = @y\na = @x";
        let out = interpolate(data, &scope, Encoding::Utf8);
        assert!(out
            .warnings
            .iter()
            .all(|w| w.contains("undefined variable")));
        assert!(json_of(&out).contains(r#""a":"@x""#));
    }

    #[test]
    fn test_scope_errors_before_local_definition_are_reported() {
        let scope = scope_of(b"@x = 0");
        let data = b"a = @[1 / x]\n@x = 5\nb = @x";
        let out = interpolate(data, &scope, Encoding::Utf8);
        assert_eq!(out.warnings, ["1:7: result is not a finite number `1 / x`"]);
        let json = json_of(&out);
        assert_eq!(json, r#"{"a":"@[1 / x]","b":5}"#);

        let scope = scope_of(b"@x = text");
        let out = interpolate(b"a = @[x + 1]\n@x = 5", &scope, Encoding::Utf8);
        assert_eq!(out.warnings.len(), 1);
        assert!(
            out.warnings[0].contains("not a number"),
            "{:?}",
            out.warnings
        );
    }

    #[test]
    fn test_local_definition_before_redefinition_keeps_scope_value() {
        let scope = scope_of(b"@x = 1");
        let data = b"@y = @x\n@x = 5\nb = @y\n@z = @y\nc = @z\nd = @x";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"b":1,"c":1,"d":5}"#
        );

        // A second definition applies from its position
        let data = b"@y = @x\na = @y\n@x = 5\n@y = 2\nb = @y";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":1,"b":2}"#
        );
    }

    #[test]
    fn test_each_redefinition_is_independent() {
        let scope = scope_of(b"@x = 1\n@y = 2");
        // Redefining @y first must not hide the scope's @x before @x is
        // redefined
        let data = b"@y = 5\na = @x\nb = @[x + y]\n@x = 7\nc = @x\nd = @[x + y]";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":1,"b":6,"c":7,"d":12}"#
        );

        // A local that read the scope's @x before its redefinition keeps
        // that value after it
        let data = b"@w = @[x * 10]\n@y = 5\n@x = 7\na = @w\nb = @y";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":10,"b":5}"#
        );
    }

    #[test]
    fn test_redefinition_uses_old_value() {
        let scope = scope_of(b"@x = 1\n@name = old");
        let data = b"a = @x\n@x = @[x + 1]\nb = @x\n@name = @name\nc = @name";
        assert_eq!(
            to_json_scope(data, &scope, JsonOptions::new()),
            r#"{"a":1,"b":2,"c":"old"}"#
        );

        // Errors in the redefinition are reported, and the name is then
        // undefined rather than keeping the scope's value
        let out = interpolate(b"@x = @[x / 0]\nb = @x", &scope, Encoding::Utf8);
        assert_eq!(out.warnings.len(), 1);
        assert!(
            out.warnings[0].starts_with("1:8: result is not a finite number"),
            "{:?}",
            out.warnings
        );
        assert!(json_of(&out).contains(r#""b":"@x""#));
    }

    #[test]
    fn test_failed_local_stays_failed() {
        // @y fails before @x is redefined; that must not give it a value
        let scope = scope_of(b"@x = 1");
        let out = interpolate(b"@y = @z\n@z = 1\n@x = 5\na = @y", &scope, Encoding::Utf8);
        assert_eq!(out.warnings, ["1:6: undefined variable `@z`"]);
        assert!(json_of(&out).contains(r#""a":"@y""#));

        let scope = scope_of(b"@x = text");
        let data = b"@y = @[x + 1]\n@x = 5\na = @y\n@w = @y\nb = @w\nc = @[y * 2]";
        let out = interpolate(data, &scope, Encoding::Utf8);
        // Only the original failure is reported
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
        assert!(
            out.warnings[0].contains("not a number"),
            "{:?}",
            out.warnings
        );
        let json = json_of(&out);
        assert_eq!(json, r#"{"a":"@y","b":"@w","c":"@[y * 2]"}"#);

        // A failed redefinition stays failed too
        let scope = scope_of(b"@x = 1");
        let out = interpolate(
            b"@x = @missing\na = @x\nb = @[x + 1]",
            &scope,
            Encoding::Utf8,
        );
        assert_eq!(out.warnings, ["1:6: undefined variable `@missing`"]);
        let json = json_of(&out);
        assert_eq!(json, r#"{"a":"@x","b":"@[x + 1]"}"#);
    }

    #[test]
    fn test_substitution_does_not_join_tokens() {
        // Same structure as the file read without interpolation, where
        // `@[1+1]x` is a calculation then a value
        let data = b"@x = 1\nb = { @[x]@x }\nc = @[1+1]x\nd = { @[x]\"s\" }";
        assert_eq!(
            to_json(data),
            r#"{"b":[1,1],"c":2,"remainder":["x",{"d":[1,"s"]}]}"#
        );
    }

    #[test]
    fn test_copy_of_failed_variable() {
        let scope = scope_of(b"@x = 1");
        let data = b"@y = @missing\n@x = 5\n@z = @y\na = @[z + 1]";
        let out = interpolate(data, &scope, Encoding::Utf8);
        assert_eq!(out.warnings, ["1:6: undefined variable `@missing`"]);
    }

    #[test]
    fn test_code_payload_variables_are_template_parameters() {
        // Filled in where the template is used, so left as written, quietly
        let data = b"@x = 1\nt = { code = [[ a = @x ]] }";
        let out = interpolate(data, &Variables::new(), Encoding::Utf8);
        assert_eq!(out.warnings, Vec::<String>::new());
        assert_eq!(json_of(&out), r#"{"t":{"code":"[[ a = @x ]]"}}"#);
    }

    #[test]
    fn test_redefinition_in_file() {
        let data = b"@w = 1\na = @w\n@w = @[w + 1]\nb = @w";
        assert_eq!(to_json(data), r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn test_headers() {
        let data = b"@x = rgb\na = @x { 1 2 }\nb = \"text\" { c = @x }";
        assert_eq!(
            to_json(data),
            r#"{"a":{"rgb":[1,2]},"b":{"text":{"c":"rgb"}}}"#
        );
    }

    #[test]
    fn test_blocks_keep_their_shape() {
        for (data, expected) in [
            // As without the definition: the first entry has no operator
            ("x = { @w = 1 size { 2 3 } }", r#"{"x":[{"size":[2,3]}]}"#),
            ("x = { @n = 1 a { b = 2 } }", r#"{"x":[{"a":{"b":2}}]}"#),
            ("x = { @n = 1 a = { b = 2 } }", r#"{"x":{"a":{"b":2}}}"#),
            ("x = { @a = 1 }", r#"{"x":[]}"#),
            ("x = { 1 @a = 2 @a 3 }", r#"{"x":[1,2,3]}"#),
            ("x = { @a = 2 @a 3 }", r#"{"x":[2,3]}"#),
            ("@x = rgb\na = @x { @y = 1 }", r#"{"a":"rgb"}"#),
        ] {
            assert_eq!(to_json(data.as_bytes()), expected, "{}", data);
        }
    }

    #[test]
    fn test_operators_in_parameter_block() {
        let data = b"t = { [[p] b ?= 3 c == 4] }";
        assert_eq!(to_json(data), r#"{"t":{"[p]":{"b":3,"c":4}}}"#);
    }

    #[test]
    fn test_operators_in_nested_blocks() {
        let data = b"a = { { b ?= 1 } rgb { c == 2 } } d = hsv { e ?= { f == 3 } }";
        assert_eq!(
            to_json(data),
            r#"{"a":[{"b":1},{"rgb":{"c":2}}],"d":{"hsv":{"e":{"f":3}}}}"#
        );
    }

    #[test]
    fn test_windows1252_warning_text() {
        let data = b"a = @Jos\xe9";
        let out = interpolate(data, &Variables::new(), Encoding::Windows1252);
        assert_eq!(out.warnings, ["1:5: undefined variable `@José`"]);
    }

    #[test]
    fn test_unread_definition_does_not_shadow_scope() {
        // The resolver does not read a parameter block, so its `@x` must not
        // hide the scope's `@x`
        let scope = scope_of(b"@x = 1");
        let out = interpolate(b"a = @x\n[[p] @x = 5 ]", &scope, Encoding::Utf8);
        assert_eq!(out.warnings, Vec::<String>::new());
        assert!(json_of(&out).contains(r#""a":1"#));
    }

    #[test]
    fn test_interpolation_with_duplicate_keys() {
        let data = br#"
@var = 10
duplicate = @var
duplicate = @[var * 2]
test = @[var + 5]
"#;
        let opts = |mode| JsonOptions::new().with_duplicate_keys(mode);
        assert_eq!(
            to_json_with(data, opts(DuplicateKeyMode::Preserve)),
            r#"{"duplicate":10,"duplicate":20,"test":15}"#
        );
        assert_eq!(
            to_json_with(data, opts(DuplicateKeyMode::Group)),
            r#"{"duplicate":[10,20],"test":15}"#
        );
        assert_eq!(
            to_json_with(data, opts(DuplicateKeyMode::KeyValuePairs)),
            r#"{"type":"obj","val":[["duplicate",10],["duplicate",20],["test",15]]}"#
        );
    }

    #[test]
    fn test_interpolation_with_operators_and_grouping() {
        let data = br#"
@threshold = 50
condition > @threshold
condition < @[threshold * 2]
condition = @[threshold / 2]
"#;
        let options = JsonOptions::new().with_duplicate_keys(DuplicateKeyMode::Group);
        assert_eq!(
            to_json_with(data, options),
            r#"{"condition":[{"GREATER_THAN":50},{"LESS_THAN":100},25]}"#
        );
    }

    #[test]
    fn test_definitions() {
        for (data, expected) in [
            // As in jomini, any operator makes a definition
            ("@x = 5\nb = { @x < 3 }\na = @x", r#"{"b":[],"a":3}"#),
            ("@x ?= 5\na = @x", r#"{"a":5}"#),
            (
                "@:register_variable x = 2\na = @:load_variable x",
                r#"{"a":2}"#,
            ),
        ] {
            let out = interpolate(data.as_bytes(), &Variables::new(), Encoding::Utf8);
            assert_eq!(json_of(&out), expected, "{}", data);
        }
    }

    #[test]
    fn test_check_syntax() {
        let check = |data: &[u8]| check_syntax(data, parse(data).errors());
        assert_eq!(
            check(b"a = { b = 1 }}").unwrap(),
            ["1:14: syntax error: unmatched '}'"]
        );
        assert_eq!(check(b"a = { b = 1").unwrap().len(), 1);
        assert_eq!(check(b"a = 1 ] b = 2").unwrap().len(), 1);
        for data in [
            "a = b = c",
            "a =",
            "x = { = 1 }",
            "a = \"x",
            "a = \"x\nb = \"c\"",
        ] {
            assert!(check(data.as_bytes()).is_err(), "{}", data);
        }
    }

    #[test]
    fn test_load_variables() {
        let tmp = TempDir::new("vars");
        let write = |name: &str, text: &str| {
            let path = tmp.0.join(name);
            std::fs::write(&path, text).unwrap();
            path
        };
        let base = write("z.txt", "@x = 1\n@gone = 1");
        let next = write("a.txt", "@x = @[x + 1]\n@gone = @missing");

        // Read in the order given
        let (variables, warnings) =
            load_variables(&[base.clone(), next.clone()], Encoding::Utf8).unwrap();
        assert_eq!(variables.get("x").unwrap().as_bytes(), b"2");
        // A failed definition does not leave the earlier value
        assert_eq!(variables.get("gone"), None);
        assert_eq!(
            warnings,
            [format!(
                "{}:2:9: undefined variable `@missing`",
                next.display()
            )]
        );

        let broken = write("broken.txt", "@x = \"oops");
        assert!(load_variables(&[base, broken], Encoding::Utf8).is_err());
        assert!(load_variables(std::slice::from_ref(&tmp.0), Encoding::Utf8).is_err());
    }
}
